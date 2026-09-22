# Auditoria de rendering, sombras, meshing, remeshing e fluidos

## Escopo e evidência

Base auditada: `sarakborges/mineclone`, `develop`,
`bd2c152702e37998d3dc4c12926982c8509b990c`, `VERSION 0.50.14`.
Referências normativas: `ARCHITECTURE.md`, `ENGINEERING_PRACTICES.md`,
checkpoints 181/182 e histórico preservado em `docs/handoffs/`.

Pedido de 2026-09-22: analisar antes de mudar, implementar por prioridade,
atualizar handoff e acompanhar/corrigir CI. O acompanhamento de CI está
explicitamente autorizado nesta rodada. Não foi acrescentado `cargo test`
ao workflow. Os testes Rust de regressão são compilados pelo Clippy
`--all-targets`; compilação não significa execução desses testes.

Esta é uma auditoria do código e dos logs de CI. Não há captura de GPU,
medição de FPS, benchmark do executável ou QA Windows nesta sessão.
Reduções abaixo são de trabalho identificável no código; não são percentuais
de melhoria do jogo. O ambiente local não tem Rust/Cargo e o acesso Git direto
não respondeu; leitura/publicação e validação usam a conexão GitHub e Actions.

## Caminho de execução verificado

- Geração assíncrona compartilha `ChunkAsyncWorkLimiter` com initial mesh/remesh.
- Streaming integra uma wave, converge fluidos gerados, semeia iluminação,
  captura snapshot e despacha initial mesh; publicação integra assets/entities.
- `PostUpdate`: fluidos -> iluminação -> remesh -> residency -> visibilidade.
- Cada chunk 16³ possui oito regiões de remesh 8³. Elas são regiões CPU de
  invalidação, não os meshlets da funcionalidade GPU homônima do Bevy.
- Malhas agrupam materiais compatíveis e preservam os canais compactados:
  posição, UV, payload inteiro e luz/AO `Unorm8x4` (28 bytes por vértice).
- O patch mantém entidades/handles quando as chaves materiais permitem;
  mudanças de composição podem exigir rebuild completo. A preparação dos
  patches é atômica antes de escrever em `Assets<Mesh>`.
- Conteúdo, iluminação, publicação visual e persistência têm revisões/owners
  diferentes. Não transformar caches/diagnósticos em estado autoritativo.

## Prioridades com caminho definido

| Ordem | Achado comprovado | Mudança delimitada | Validação |
| --- | --- | --- | --- |
| P0 | CI da base falha em duas assinaturas de `player/model.rs` | Aliases privados para as queries de root/head; manter filtros e mutabilidade | Clippy rigoroso + check do SHA publicado |
| P1 | Remesh conta somente despachos bem-sucedidos no orçamento | Contar candidatos examinados; parar/reter trabalho quando o executor recusa | Limite de tentativas, fila preservada, CI |
| P2 | Mudança só de luz descarta também fluid geometry válida | Publicar fluido com conteúdo atual e reenfileirar iluminação; rejeitar terrain obsoleto | Regressão de política de publicação + CI |
| P3 | Patch parcial clona todos os atributos/índices antes até de decidir `Unchanged` | Ler slices emprestados; alocar apenas a saída que realmente muda | Quads preservados, índices U16/U32, entradas inválidas, CI |
| P4 | Greedy terrain percorre 96 planos mesmo sem fontes naquele plano; fluid top varre níveis vazios | Pular planos sem candidatos antes da máscara/varredura | Mesma seleção/ordem de quads; CI |
| P5 | Shader consulta sombra solar antes de saber se a incidência é zero | Avaliar contribuição primeiro e retornar a mesma parcela ambiente quando zero | Identidade algébrica, fonte Bevy 0.19.1, CI; QA GPU pendente |

Cada bloco atualiza `VERSION` por patch e o checkpoint do `HANDOFF.md`.
Após cada publicação, observar o run de push do SHA exato. Corrigir regressão
antes de avançar. Publicar sem force e parar se `develop` tiver mudança
concorrente que precise ser conciliada.

## Shadowing

Arquivos: `rendering/directional_shadows.rs`, `dynamic_lights.rs`,
`world/chunk_rendering/spawn.rs`, `assets/shaders/terrain_*.wgsl`.

- Sol: três cascatas de 1024, primeira até 16 unidades, teto atual de nove
  chunks de alcance (144 unidades). Não aumentar mapas/cascatas sem medir
  GPU e qualidade. O limite de sombras é separado da render distance.
- Luz segurada: cubemap de 512, alcance 8; sombras são desligadas quando
  não há emissão. Reaproveitar esse caminho e o buffer global já existentes.
- O shader já evita sombras em fluidos e quando não há contribuição de sky.
  Ainda chama `fetch_directional_shadow` em faces com incidência zero;
  `mix(SUN_AMBIENT_SHARE, 1, shadow * 0)` independe da consulta.
- Confirmada a assinatura da fonte oficial Bevy **v0.19.1**:
  `crates/bevy_pbr/src/render/shadows.wgsl`. A consulta pode amostrar duas
  cascatas na faixa de transição; evitar a consulta elimina ambas nesse caso.
- Preservar `casts_shadow=false`, particularmente folhas, e os grupos por
  semântica de material. Não juntar foliage e sólido perdendo esse contrato.
- Revisar em GPU futuramente acne/peter-panning, transições das cascatas,
  movimento da luz segurada e estabilidade temporal. Não alterar bias ou
  distância somente por uma hipótese de FPS.

## Rendering

Arquivos: `world/chunk_rendering/{materials,pool,spawn,refresh}.rs`,
`rendering/terrain_material.rs`, `player/camera.rs`, shaders de terreno.

- Texture array, interning, batching, vértices compactos, buffer de luz
  compartilhado, DepthPrepass e OcclusionCulling já existem. Reimplementá-los
  não constitui uma otimização nova.
- CPU culling continua nos transparentes; `NoCpuCulling` fica nos caminhos
  opaque/mask. Alterar isso exige validar fallback de GPU/backend.
- Custom alpha cutoffs diferentes de 0.5 usam fallback de material; não
  assumir que todos os blocos podem usar a mesma máscara.
- O fragment shader faz `alpha_discard` tarde. Antecipar descarte de foliage
  é um candidato, mas precisa preservar opaque/blend/alpha-to-coverage e
  uniformidade de derivatives das amostras seguintes. Não mover às cegas.
- O prepass customizado merece QA específico em todos os modos de alpha e
  na câmera de terceira pessoa. Clippy/check não compilam WGSL em runtime.
- `pool.mesh_bytes()` soma allocations; residency e aposentadoria ainda
  fazem scans recorrentes. Um total incremental só deve entrar junto de
  todas as transições insert/replace/patch/detach/append/take/clear, com
  reconciliação verificável. Evitar um contador incompleto.
- Os tetos atuais 192/160/128 MiB controlam residency. A prioridade de
  eviction e recuperação deve ser auditada com RD24/World Tree; não resolver
  pressão escondendo chunks ou aproximando fog.

## Meshing

Arquivos: `voxel/{mesh,fluid_mesh,layer_mesh,mesh_lighting,mesh_buffer}.rs`.

- Iteração sparse de voxels ocupados e seleção 8³ já reduzem scans. Greedy
  deve continuar limitado ao meshlet para tornar patches locais corretos.
- Planos vazios são custo comprovado: um único cubo ocupa um plano por eixo,
  logo precisa de seis planos de faces, mas percorre 96. P4 elimina as 90
  varreduras vazias desse exemplo; não muda o número de faces nem prova FPS.
- Em fluid top, um lago plano de uma camada ocupa só um dos 16 níveis.
- `VoxelMeshSource` na base atual ainda armazena `VoxelCell` por valor,
  apesar da descrição de zero-copy no checkpoint 182. Migrar para referências
  exige explicitar o lifetime da visita sparse para não deixar referências
  escaparem de callbacks com lifetime curto. Não tratar o texto histórico
  como prova de que a otimização já está presente.
- Cache denso de iluminação 18³ somente em rebuild total e chunks com pelo
  menos 512 células de conteúdo. O ponto de corte precisa de benchmark com
  sólido, folha sparse, Chisel e fluidos; não ajustar por palpite.
- Greedy terrain preserva quantização final de iluminação uniforme;
  greedy fluid top mantém altura plana, fluido/tint/luz iguais. Mesclar
  gradientes ou bordas inclinadas sem equivalência altera a imagem.

## Remeshing e trabalho por frame

Arquivos: `world/chunk_remesh.rs`, `chunk_remesh/{queue}.rs`,
`chunk_remesh_tasks.rs`, `voxel/{mesh_snapshot,meshlet}.rs`, `work_budget.rs`.

- O orçamento de dispatch possui mínimo de um item e teto de quatro,
  mas só incrementa em sucesso. Quando o limiter está ocupado, todos os
  candidatos podem ser retirados, ter snapshots construídos e ser devolvidos
  sem ativar o limite de tempo. Isso é especialmente ruim com RD alto.
- As tarefas já em voo também não eram cobradas como trabalho examinado.
  P1 trata esse caminho sem perder máscaras coalescidas ou prioridade.
- Geometry+lighting já se unem antes do dispatch; fluido mantém fila e
  capacidade independentes. Manter a seleção justa entre tipos.
- `content_is_current` vem antes da publicação; essa rejeição permanece
  obrigatória. O problema P2 é a rejeição adicional por luz para fluidos,
  contrariando o contrato explícito da arquitetura de publicação seguida
  por refresh de iluminação.
- Patches parciais copiam **todos** os arrays de entrada para `MeshArrays`,
  depois copiam os quads sobreviventes novamente para a saída. Até um patch
  sem interseção copia o mesh inteiro. P3 elimina essas cópias de entrada,
  preservando a validação e a publicação atômica do pool.
- P3 não elimina o upload do asset completo pelo renderer nem transforma o
  patch em atualização de subfaixa de GPU. Isso exigiria outro contrato.
- O dispatch de initial mesh também conta somente sucesso em alguns ramos
  de descarte; revisar esse orçamento depois de P1, preservando preempção
  nearest-first e o piso de progresso.
- A publicação inicial atual já possui gating por prioridade próxima em
  `streaming/meshing.rs`; essa pendência do handoff histórico não deve ser
  reimplementada. Há risco de head-of-line blocking a medir na World Tree.
- Há divergências entre a arquitetura de halo de luz estabilizado e a
  implementação atual de notificações por slice. Reintroduzir barreiras
  globais pode travar streaming; a correção exige readiness por seção/halo
  e métricas de latência, não bloquear tudo enquanto houver qualquer luz.

## Fluids

Arquivos: `world/fluid_updates/{solver,state,frontier,settling}.rs`.

- Scheduler já tem ticks lógicos, due buckets, dormant por chunk, wakes
  justos, scratch reutilizado e diagnósticos de BFS. Preservar `spreadSpeed`,
  `maxSpread`, gravidade e ownership dos chunks em settling.
- Generated settling faz verificação sparse de ponto fixo após drenar a
  fila local; remover essa fase mudaria a convergência de rotas downhill.
- O BFS consulta o mundo antes de verificar se um vizinho já foi visitado
  em distância menor. Essa consulta é dispensável para tais vizinhos durante
  uma busca sobre mundo imutável. Candidato posterior com regressões de
  caminhos empatados, quedas e fontes; não mudar a propagação de máscaras.
- Cache de rotas entre avaliações não está aprovado: exigiria identidade
  de origem/fluido/range e invalidação de todo o domínio consultado, inclusive
  quedas e mudanças de residency. Um cache local mal invalidado muda água.
- `maxSpread` pode tornar uma única busca cara mesmo com orçamento externo.
  Medir `downhill_nodes / downhill_searches` antes de limitar comportamento
  authored ou transformar o solver em estado incremental.

## Validação e próxima decisão baseada em runtime

CI baseline: push run `35680336531`, FAIL no Clippy por `type_complexity`
em `player/model.rs` (duas ocorrências); check não chegou a executar.

Gates de cada bloco: revisar diff, manter lint sem novos `allow`, preservar
lockfile/caches, Clippy `--locked --all-targets --all-features -- -D warnings`
e `cargo check --locked` no SHA publicado. Só marcar CI verde após conclusão.

QA posterior, com mesma seed/posição, build e configurações:

1. RD24, World Tree inteira, câmera parada e percurso repetível: frame time
   mediano/p95/p99, geração/meshing/remesh, filas e bytes de mesh.
2. Edições interiores/bordas/cantos, Chisel e layers: ausência de buracos,
   handles reaproveitados e nenhuma face de meshlet vizinho perdida.
3. Água/lava em bordas diagonais, queda e luz em mudança: geometria publicada
   sem starvation e iluminação convergindo; cadência lógica preservada.
4. Foliage alpha, céu/noite/caverna, luz segurada e terceira pessoa: conferir
   sombras, prepass, transparência e render distance/fog.
5. GPU capture para separar custo de sombras, depth prepass, fragment shading
   e upload. Esses resultados decidem mudanças maiores de renderer.

Ganhos de FPS, VRAM e qualidade visual permanecem **não medidos** até esse QA.
