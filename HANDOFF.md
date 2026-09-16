# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy `0.19.1`. **VERSION `0.18.0`** (bump `6559f32933bb91d84a60f03fc35e9574e406ee0e`). `Cargo.toml` tem versão independente `0.10.16`; não harmonizar. Consultar o HEAD antes de gravar. **Histórico anterior integral preservado**, incluindo todas as regras, detalhes de código, commits, links e nove defeitos: [snapshot completo imediatamente anterior à alteração da CI, v0.18.0](https://github.com/sarakborges/mineclone/blob/07eb75dc99f9e57eb83e19a4143857c4d1dcad22/HANDOFF.md); [merge v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md); [histórico de mundo e branches na v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Compactar documentação não fecha defeitos nem apaga histórico.

## Regras do projeto

- `continua` / `go` / roadmap: ler HEAD, VERSION, CI, código real e este handoff; executar trabalho fundamentado. Trabalho normal solicitado pode ir direto à `develop`. Branches isolados eram exigência **exclusiva** das features de criaturas, chat e editor; não generalizar.
- Incrementar `VERSION` por bloco de código, dados jogáveis ou testes relevantes segundo SemVer; documentação isolada e esta alteração de configuração da CI não exigem bump. Atualizar HANDOFF na mesma sessão com alterações, SHA, validação efetiva, bugs e próxima ação. `ARCHITECTURE.md` é a referência de arquitetura. Preservar PNGs/GLBs originais; não editar assets fornecidos só para conectá-los à UI.
- **Política de CI explicitamente solicitada pelo usuário em 16/09/2026:** `.github/workflows/ci.yml` deve executar **Clippy** (`cargo clippy --all-targets --all-features -- -D warnings`) e **Check** (`cargo check`), **sem etapa de execução de testes unitários (`cargo test`)**. Commit [`a08d890c2772fb5198bb7b7b672362252afb7af6`](https://github.com/sarakborges/mineclone/commit/a08d890c2772fb5198bb7b7b672362252afb7af6) removeu somente a etapa Tests e comentários relacionados; não apagou testes Rust do repositório. Clippy `--all-targets` ainda analisa alvos de teste, mas não executa asserções. Runs iniciados antes da alteração mantêm a definição antiga: não interpretar `Tests` em um run antigo como regressão do workflow atual nem declarar que testes foram executados no novo. Não reintroduzir a etapa sem novo pedido.
- Não confundir CI com QA de gameplay/Windows/FPS, não afirmar sucesso quando job ainda está pendente ou em execução, não silenciar warnings nem prometer trabalho assíncrono. Evitar consultas repetitivas à CI sem informação nova; comunicar resultado e bloqueio concretos ao usuário.

## Direção de arte — regra obrigatória para TODOS os modelos (16/09/2026)

**Regra do usuário, global e não opcional:** todos os modelos 3D, inclusive criaturas, corpo e detalhes internos/externos, devem manter linguagem estritamente quadrada/cúbica e pixelada. Proibidos cantos arredondados, bevels/chanfros arredondados, subdivisão que suavize silhueta, esferas, elipsoides, olhos ovais, tubos ou curvas orgânicas. Construir volumes com superfícies planas, arestas retas e cantos nítidos; face, olhos, boca, núcleo e elementos decorativos também seguem esta regra. Animações não podem introduzir curvatura ou suavização na geometria. A resolução 64×64 foi exigida especificamente para a skin do slime neste pedido; não presumir automaticamente 64×64 como resolução obrigatória de todas as outras criaturas sem especificação. Pixel art deve ser exibida sem borramento, observando UVs, filtro nearest e mip/filter de acordo com o pipeline real. Esta regra se aplica à criação, revisão e correção de todos os assets futuros.

### Pendência de asset — refazer slime compartilhado (NÃO CONCLUÍDA)

**Pedido:** slime com corpo/silhueta quadrada, núcleo quadrado e face completamente quadrada (olhos, boca, bochechas, brilhos e demais detalhes), textura/skin pixelada **64×64**. Não marcar o GLB como corrigido por ter escrito este documento. O arquivo `assets/models/creatures/slime/slime.glb` NÃO foi alterado nesta atualização documental. Meadow e Ember referenciam esse MESMO GLB por `data/creatures/slimes/meadow.json` e `ember.json`; a correção precisa preservar compatibilidade das duas espécies e os nomes de materiais para tinting.

**Causa concreta identificada no gerador atual:** `assets/models/creatures/slime/generate_slime.py` constrói a casca com `rounded_box(..., .135, 12)`, núcleo/olhos/brilhos/bochechas com `ellipsoid(...)`, sorriso com `mouth_curve()` tubular e normais suaves; cria materiais de cor PBR, mas não define atlas/UV/texture 64×64. Portanto o gerador e o GLB gerado precisam ser revisados juntos, não apenas mudar parâmetros ou trocar a imagem. O diagnóstico é baseado no gerador do repositório; não foi feita inspeção visual em jogo.

**Checklist técnico para implementação e aceite (todos pendentes):**

- [ ] Substituir `rounded_box` da `SlimeShell` por cubo/caixa de seis faces planas, quinas 90°, sem raio, bevel ou subdivisão suavizante. Em pose neutra a silhueta deve ser cúbica/quadrada; se houver squash/stretch, preservar planos e arestas retas, sem deformar para formas arredondadas.
- [ ] Substituir `ellipsoid` da `SlimeCore` por volume cúbico/quadrado, preservando `InnerCore` e animações associadas.
- [ ] Refazer a face no plano frontal `-Z`: olhos e reflexos quadrados/pixelados, boca feita de pixels/retângulos ortogonais sem sorriso tubular ou curva contínua; bochechas e cada detalhe também quadrados. Não manter geometria oval mesmo se ela parecer pixelada na textura.
- [ ] Produzir/mapear uma skin/atlas **64×64 pixels exatos** para o slime, com UVs válidas e detalhe desenhado em pixels inteiros; especificar separação/alpha de casca, núcleo e face de modo compatível com os materiais. Configurar amostragem nearest e evitar blur/interpolação na renderização; validar no pipeline Bevy/glTF, não só na prévia do editor.
- [ ] Preservar nomes de materiais `SlimeShell`, `SlimeCore`, `SlimeCheeks`, `SlimeEyes`, `SlimeHighlights` e as entradas HSI por espécie; evitar alterar material glTF compartilhado de modo que Meadow e Ember recebam a mesma cor. Manter transparência da casca somente se compatível com a leitura pixelada e sem arredondar volume.
- [ ] Preservar hierarquia/nomes `SlimeRoot`, `Visual`, `BodyPivot`, `InnerCore`, a frente `-Z`, a origem nos pés (`y=0`) e o collider AABB no root NÃO animado. Se dimensões do novo modelo mudarem, sincronizar `slime.collider.json` e os colliders em `meadow.json`/`ember.json`; não ajustar física cegamente apenas pela aparência.
- [ ] Preservar clips nomeados `Idle`, `Anticipate`, `Airborne`, `Land`, `Hurt`, `Death`; revisar todas as poses para confirmar arestas/cantos retos, sem root motion/collider animado.
- [ ] Regenerar o GLB pelo gerador corrigido, verificar geometria, textura 64×64, materiais, UVs e animações; conferir Meadow e Ember em jogo em repouso, salto, dano e morte, de frente/lado/cima, em várias distâncias. Só então registrar prova visual, bump SemVer por alteração do asset jogável e fechar esta pendência.

**Status da presente alteração:** somente documentação/direção de arte e checklist, sem edição de `.glb`, `.py`, `.json`, PNG, Rust ou CI; `VERSION` permanece `0.18.0`. Não declarar QA, compilação ou correção do slime executados a partir desta regra documental.

## Bloco 16/09/2026 — v0.18.0: Grass, Bassalt e Brush

Commit do usuário [`545a7b0`](https://github.com/sarakborges/mineclone/commit/545a7b041b4462e68b321fc182fec537029631eb) adicionou `assets/textures/blocks/bassalt.png`, `assets/textures/tools/brush.png` e `assets/textures/tools/brush-tint.png`. **Os três PNGs permaneceram inalterados** desde o commit de origem; o diff até o código da feature não contém assets. `data/blocks/grass.json` muda somente o nome exibido para `Grass`, mantendo `asteria:grass`. Novo `data/blocks/bassalt.json` define `asteria:bassalt`, seis faces do PNG fornecido e categoria `stone_blocks`, sem apagar `asteria:stone`. `data/tools/brush.json` aponta `icon` para brush.png e `tintIcon` para brush-tint.png; `ToolDefinition` aceita o segundo caminho opcional.

O pincel já aplicava/removia a propriedade secundária `dyed` em blocos compatíveis com clique esquerdo, e abria a paleta com clique direito. Agora inicia em `BrushSelection::Clear` (sem cor; nesse estado o uso remove dye existente). O renderer `src/hud/tool_icon.rs` e a integração em hotbar, catálogo, inventário e item arrastado mostram somente brush.png quando não há cor e aplicam brush-tint.png como segunda camada colorida pelo `SecondaryPropertyRegistry` quando há cor. `sync_brush_tint_icons` atualiza visuais existentes sob mudanças relevantes. Código integrado no commit [`9cbee3e`](https://github.com/sarakborges/mineclone/commit/9cbee3edf5a3d665f2fe95f2809dbcddf7f2b7ce); workflow temporário de aplicação executou com sucesso no run [`35159303985`](https://github.com/sarakborges/mineclone/actions/runs/35159303985) e se removeu do HEAD, juntamente com os scripts. Bump para `0.18.0` em `6559f329`. A validação Rust do run v0.18.0 [`35159457536`](https://github.com/sarakborges/mineclone/actions/runs/35159457536) estava em progresso no Clippy na última consulta; Check e Tests ainda pendentes **na configuração antiga daquele run**. Não tratar como aprovação da versão. Nova CI pós-remoção de testes deve ser avaliada por seu próprio SHA.

**Histórico integrado:** merge `b421c5e` inclui PRs #9 criaturas e #10 chat/editor, preservando hidrologia `.42–.51`, correções de luz `.41` e GLB original do slime; v0.17.3 consolidada em `274b5f7`. Criaturas registradas por JSON, Meadow/Ember Slime, chat T/Enter/Esc, `/spawn_creature`, histórico 64 mensagens/15 linhas visuais, scrollbars condicionais e editor nativo `EditableText` + clipboard. QA real de GLB, animação, chat, Windows, seleção, IME e desempenho continua sem confirmação. Branches experimentais de luz, propriedades ópticas e diagnósticos não foram incluídos. Consultar snapshot v0.17.3 para conteúdo/limitações completos.

## Nove bugs reportados pelo usuário — TODOS ABERTOS até validação no jogo

| # | Prioridade | Sintoma |
|---|---|---|
| 1 | P0 | Chunks pretos e costuras de iluminação/AO; medir transiente, persistência e FPS. |
| 2 | P0 | Rios terminam no nada; testar conectividade de nascentes a oceano em várias seeds. |
| 3 | P0 | 2000 blocos exibem apenas Plains/Mountains; amostrar IDs de bioma versus render. |
| 4 | P0 | Margem do rio afunda antes de alcançar a água, sem permitir água suspensa. |
| 5 | P1 | Confluências rio–lago abruptas. |
| 6 | P1 | Túneis com paredes retas e paredes extras em interseções com rios. |
| 7 | P1 | Lua parece acompanhar câmera. |
| 8 | P1 | Lua visível durante o dia. |
| 9 | P1 | Target highlight não cobre todas as camadas de textura. |

Outros itens pendentes: fogColor, benchmark de streaming/FPS, held/R, avatar/vida do player HUD e posicionamento do target HUD. Única correção visual confirmada anteriormente pelo usuário: céu por bioma `.24`. Feature nova não encerra bugs antigos.

## Próxima execução

Primeiro, corrigir a pendência visual do slime segundo a regra de arte e checklist acima quando a edição do asset for solicitada: ajustar gerador e GLB juntos, validar textura/UV 64×64, manter collider e materiais/clips compartilhados, bump VERSION e atualizar este documento; não declarar feito sem gerar/inspecionar asset. Validar **o workflow atual sem testes unitários** por execução iniciada depois de `a08d890`, verificando Clippy e Check separadamente e corrigindo somente erros concretos. Fazer QA no jogo para Bassalt, nome Grass, ícones brush base/tint/cor/clear em todas as telas, sem editar PNGs. Retomar P0 rios, biomas, luz e margem; depois P1. Cada novo bloco altera versão/handoff conforme a natureza da mudança.
