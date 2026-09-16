# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone` · branch de trabalho: `develop` · Rust + Bevy 0.19.1.

**Esta é a fonte canônica persistente.** Anexos/exports são snapshots. O histórico detalhado anterior continua recuperável integralmente no [HANDOFF.md do commit 72b60b5](https://github.com/sarakborges/mineclone/blob/72b60b5a4e70a9960b3108debdf77d7a8510c4b1/HANDOFF.md); esta edição compacta a cronologia, sem encerrar bugs antigos não confirmados.

## Regras obrigatórias

- Trabalhar diretamente em `develop`, sem feature branch sem solicitação. Antes de escrever, consultar HEAD, `VERSION` raiz e código real; commits pequenos e coerentes.
- Cada **bloco de código** sobe `VERSION` no próprio bloco: PATCH correção/refactor/otimização compatível; MINOR feature compatível; MAJOR quebra de contrato. Documento isolado não sobe versão. `Cargo.toml` em 0.10.16 está deliberadamente antigo: não confundir com a versão operacional.
- Atualizar **este HANDOFF na mesma sessão** de toda alteração material de código, versão, arquitetura, roadmap, processo ou feedback; manter versão, HEAD, estado, riscos e próximo passo corretos. `go`/`continua` pede execução, não novo planejamento.
- Erros e warnings reais e feedback de gameplay têm precedência sobre refactors de roadmap; limpar warnings dos arquivos tocados.
- CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`; `fmt` não é gate; CI não executa `cargo test`. Não repetir avisos de cargo local/espera por `cargo run`. Teste manual só quando usuário solicitar. **Separar código publicado, CI aprovado e gameplay visual comprovado.**
- Não gerar imagens sem pedido. Assets binários do usuário devem ser usados exatamente como enviados.

## Canon arquitetural a preservar

`ARCHITECTURE.md` é a autoridade arquitetural. Um owner por fato de gameplay; compartilhar invariants reais, não formas parecidas; SystemParams por domínio, UI compartilhada em `src/ui`, targeting único `TargetedBlock`; sistemas e HUD change-driven. Usar `DeduplicatedQueue<T>`/`VoxelUpdateQueue`, `FrameWorkBudget`; evitar scans globais, alocações e dirty writes sem trabalho.

Pipeline async: seleção visible/preload -> generation task -> integração -> seed de luz direta inicial -> captura halo -> mesh task -> alocação render -> visibilidade -> retenção -> archive/unload; main thread orçamentada, resultados stale descartados/reagendados por revisões. Visible obrigatório supera preload; preempção devolve tarefa não crítica. A seed direta inicial ocorre uma vez por residência, não por retry de mesh. Terrain e Fluid são meshes e filas independentes. `ChunkRemeshTasks::lighting_revisions` é diferente de `VoxelWorld::chunk_mesh_revisions`. Terrain, Lighting e Fluid incorporam luz nos vértices e usam halo 3×3×3; meshing inicial deve preservar time-to-visible **e** eventualmente reparar luz/geometria capturada antes da chegada de vizinhos. Não provocar starvation invalidando infinitamente a primeira mesh.

Stack HDR: world camera order0 Skip, viewmodel HDR order1 tonemap final, UI SDR order2. Fog readiness frontier, histerese/retention sem void/popping/fog breathing e sem esconder throughput insuficiente com fog. Céu: bioma/relógio produzem `EnvironmentVisualState.sky_color`; `SkyPlugin` escreve essa cor exata em `ClearColor`, com rastreamento do bioma antes da atualização visual no mesmo frame. `fogColor` artístico é paleta separada e ainda não aplicado no renderer; fundo plano exige fog terminal alinhada ao sky, sem mistura fixa fog→sky. Nuvens world-space, vento independente e altura por seaLevel da dimensão.

Hidrologia: rios incidentes compartilham X/Z/Y do nó de drenagem/lago, endpoints não alterados pelo ajuste de terreno, segmento reproduzível entre regiões. **Preservar margens gerais corrigidas em 0.15.12**, sem confundir com novo defeito em descidas. Natural hydrology gerada autoritativamente; não enfileirar em massa no solver dinâmico. Held block observa hotbar/visual/tint, sem depender de `PlacementOrientation`; R afeta só colocação.

## Estado confirmado em 16/09/2026

**VERSION: 0.15.31.** HEAD do último commit de código `1965466e35683c0775d3a7b79e3bfaf37a49606f`; bump `ea2f9b338c7576e8086e975c7675276434e311ef`. Commit documental desta atualização é posterior; consultar HEAD antes de editar de novo.

**0.15.31 — correção estrutural PARCIAL de invalidação diagonal da luz:** em `src/world/lighting_updates.rs`, `process_dynamic_lighting` agora considera também as 20 posições diagonais do halo 3×3×3 em torno de cada chunk cuja luz realmente mudou. Só agenda Geometry se o vizinho carregado tem conteúdo nas faces orientadas para o chunk alterado; só agenda Fluid se tem fluido nas respectivas faces, preservando separação de filas e evitando remesh cego de 20 chunks. O revisionamento de iluminação existente invalida outputs async stale. Teste unitário acrescentado para condição de borda diagonal. **Não declarar que chunks pretos/costuras estão resolvidos sem CI e gameplay.** Não afirmar que esse teste foi executado: CI não roda unit tests.

**CI:** .24 run35121054358 success; .25 run35122730177 success; .26 run35123049209 success; .27 run35123661593 success; .28 run35124362037 success; .29 run35132844975 success; .30 run35133472565 success (Clippy+Check, não runtime). .22/.23 falharam antes de correções subsequentes; não contar como aprovadas. **.31 push run35134931146 estava `in_progress` na consulta inicial; reconsultar antes de afirmar sucesso.**

### Achados de código sobre P0 iluminação, ainda abertos

1. **Confirmado no código e parcialmente corrigido em .31:** `face_lighting` em `src/voxel/mesh_lighting.rs` lê base, lados e cantos do halo; `enqueue_lighting_change` de `src/world/chunk_remesh.rs` notificava só centro + seis cardinais. .31 cobre diagonais para mudanças detectadas pela relaxação, sem refazer toda a fila existente.
2. **Lacuna independente confirmada no código:** `ChunkMeshDependencies::is_current` em `src/voxel/mesh_snapshot.rs` deliberadamente ignora vizinho ausente na captura mesmo que carregue durante a task inicial, para priorizar time-to-visible. `collect_built_chunk_meshes` em `src/world/streaming.rs` publica a mesh e notifica apenas vizinhos cardinais já renderizados, mas NÃO reavalia a própria mesh recém-publicada se halo/iluminação mudaram durante a task. Isso pode deixar geometria/iluminação inicial stale quando o vizinho chega antes de sua publicação, especialmente diagonais. Implementar um catch-up barato após publicar a primeira mesh sem invalidar ou starvar tasks. Não considerar isso comprovado como explicação de todo chunk preto.
3. **Outra lacuna a auditar:** `notify_loaded_chunk_neighbors` usa somente `CARDINAL_NEIGHBORS` e conteúdo da face, enquanto AO/lighting dependem de diagonais. Seed direta inicial de chunk pode mudar luz sem aparecer em `changed_chunks` da relaxação; checar atualização de vizinhos e revisão das tasks em voo na chegada de chunk, incluindo vazio, diagonal e fluido. .28 consertou apenas notificação cardinal para vizinho renderizado com conteúdo na face.
4. Checar se chunk inteiramente preto envolve seed/snapshot de luz, `average_shader_light_levels` quando nenhum sample carregado/vazio, iluminação assíncrona e revisão de mesh. Não modificar arbitrariamente luz global, sombreamento ou fog para esconder sintomas; avaliar custo de remesh sob caminhada/voo.

### Nove defeitos do teste de gameplay de 16/09 — TODOS ABERTOS

Prioridade de investigação; não é ordem de causa comprovada. Cada item precisa de reprodução/correção/aceite físico ou visual. CI de .31 não fecha automaticamente nenhum.

| Ordem | Prioridade | Relato e aceite |
| --- | --- | --- |
| 1 | P0 | Chunks inteiros pretos e costuras/sombras entre chunks apesar de .28. Investigar seed, luz assíncrona, halo 3×3×3, revisões, primeiro mesh e vizinhos; continuidade visual sem perda de FPS. .31 é correção parcial da notificação diagonal na relaxação. |
| 2 | P0 | Rios morrem no nada. Provar por seed/coords grafo, leito E água física até lago/rio/oceano através de chunks e regiões. .15/.16/.19/.20 não resolveram no gameplay. |
| 3 | P0 | Trajeto de 2.000 blocos só mostrou Plains/Mountains. Amostrar bioma FINAL vs aparência, seleção, suitability e distribuição regional; .25 só otimizou alocações. |
| 4 | P0 | Na descida abrupta, margem do rio abaixa ANTES da água. Medir leito, nível, perfil margem e fluidos; vazamento lateral é HIPÓTESE, e não há causa comprovada para água não espalhar. Preservar margens gerais .12. |
| 5 | P1 | Junção rio↔lago abrupta. Curvatura gradual em XY e perfil de largura, profundidade, margem e altura sem quebrar endpoints/conectividade. |
| 6 | P1 | Túneis às vezes abrem paredes retas na superfície. Auditar cave connector/density/surface carver, transição suave preservando rios/margens; distinto da parede no cruzamento rio-túnel. |
| 7 | P1 | Lua parece seguir câmera. Auditar referencial, transform e órbita data-driven; preservar sol e sequência por dimensão. |
| 8 | P1 | Lua visível de dia. Conferir sequência, fases e janela data-driven, sem assumir ciclo 12h–12h ou órbita igual ao sol. |
| 9 | P1 | Highlight do target não encapsula todas as camadas de textura. Examinar bounds e overlays/alpha sem selecionar vizinho. |

### Pendências e decisões que não podem regredir

- FPS/carregamento melhoraram segundo usuário, mas FPS ~60 não foi medido; investigar throughput andando/voando, remesh Geometry da .28 e custo Fluid da .22. Não trocar cache owner-correct por segunda fonte de verdade stale.
- Margens gerais de rio: **corrigidas conforme usuário em .12**; descida antecipada é novo defeito. Parede de rio/túnel ainda aberta apesar do carver .14. Hidrologia usa cache seletivo, SmallVec inline4 .26, distância única carve+shore .27, filtro conservador envelope irregular+margem .30; preservar geometria e água na fronteira.
- Biomas .7 alterou peso/espessura mountain belt, sem comprovar variedade; .25 remove alocações (array de oito vizinhos e único Vec). Plains e Wasteland podem parecer semelhantes; verificar identidade real.
- Sky .24 corrigiu ordenação `track_current_biome` antes dos visuais e manteve céu independente do fog; fog artístico pendente, não misturar fogColor fixo em sky. Fog/frontier/nuvens antigas informadas concluídas pelo usuário; preservar HDR .15.0 e nuvens world-space .9/.10.
- Hotbar vazia, ghost alpha uniforme, dye e HUD histórica informados concluídos; highlight multilayer é novo. R/held block .18 tem CI aprovado, gameplay não confirmado. HUD futura: player inferior esquerdo `Yogg'Sara` com vida `50 / 100` na barra, status acima; target acima da crosshair com bloco à esquerda no estilo slot de inventário e texto à direita com shadow de tooltip.
- Códigos importantes: .15.11 outlet oceano wet; .12 shore grading; .13 lazy water cache; .14 proteção do leito; .15 confluence meander; .16 outlet de lago; .17 seed uma vez/residência; .19 endpoints Y; .20 trace-limit não cacheia falso; .21 Geometry captura halo luz; .22 Terrain/Fluid queues distintas; .24 sky correto; .25 seleção bioma; .26/.27/.30 otimizações hidrologia; .28 remesh cardinal ao chegar chunk vazio; .29 elimina lookup duplicado de definição por face; .31 diagonais na relaxação. SHA, CI e detalhes de .14.70–.30 disponíveis no handoff histórico linkado no topo.

## Próxima execução concreta

1. Reconsultar CI .31. Se falhar, corrigir todos erros/warnings tocados sem avançar o roadmap; nova alteração de código exige novo bump.
2. Atacar lacuna do primeiro mesh: comparar snapshot capturado vs world atual após publicação, agendar catch-up de Geometry/Fluid para halo que mudou (incluindo vizinho antes ausente e luz seed/relax), mantendo time-to-visible e evitando starvation. Testar cenário vizinho ausente na captura e presente na integração; cenário luz muda enquanto task está em voo. Investigar notificação diagonal na integração de chunk novo com filtro de conteúdo/fluido e budget.
3. Se CI e implementação permitirem, investigar iluminação inteiramente preta independentemente da costura, sem atribuir tudo ao mesmo defeito.
4. Depois: continuidade física end-to-end dos rios por seed/coords; diversidade de biomas em trajetória real; perfil de descida e junção rio-lago; túneis; lua; highlight. Preservar margens .12 e desempenho.

Ao fechar qualquer bloco, bump `VERSION`, registrar commits, CI com estado **verificado** e HANDOFF na mesma sessão. Não declarar gameplay resolvido por inspeção ou CI.
