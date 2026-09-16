# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy `0.19.1`. **VERSION `0.18.0`** (bump `6559f32933bb91d84a60f03fc35e9574e406ee0e`). `Cargo.toml` tem versão independente `0.10.16`; não harmonizar. Consultar o HEAD antes de gravar. **Histórico anterior integral preservado**, incluindo todas as regras, detalhes de código, commits, links e nove defeitos: [snapshot completo imediatamente anterior à alteração da CI, v0.18.0](https://github.com/sarakborges/mineclone/blob/07eb75dc99f9e57eb83e19a4143857c4d1dcad22/HANDOFF.md); [merge v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md); [histórico de mundo e branches na v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Compactar documentação não fecha defeitos nem apaga histórico.

## Regras do projeto

- `continua` / `go` / roadmap: ler HEAD, VERSION, CI, código real e este handoff; executar trabalho fundamentado. Trabalho normal solicitado pode ir direto à `develop`. Branches isolados eram exigência **exclusiva** das features de criaturas, chat e editor; não generalizar.
- Incrementar `VERSION` por bloco de código, dados jogáveis ou testes relevantes segundo SemVer; documentação isolada e esta alteração de configuração da CI não exigem bump. Atualizar HANDOFF na mesma sessão com alterações, SHA, validação efetiva, bugs e próxima ação. `ARCHITECTURE.md` é a referência de arquitetura. Preservar PNGs/GLBs originais; não editar assets fornecidos só para conectá-los à UI.
- **Política de CI explicitamente solicitada pelo usuário em 16/09/2026:** `.github/workflows/ci.yml` deve executar **Clippy** (`cargo clippy --all-targets --all-features -- -D warnings`) e **Check** (`cargo check`), **sem etapa de execução de testes unitários (`cargo test`)**. Commit [`a08d890c2772fb5198bb7b7b672362252afb7af6`](https://github.com/sarakborges/mineclone/commit/a08d890c2772fb5198bb7b7b672362252afb7af6) removeu somente a etapa Tests e comentários relacionados; não apagou testes Rust do repositório. Clippy `--all-targets` ainda analisa alvos de teste, mas não executa asserções. Runs iniciados antes da alteração mantêm a definição antiga: não interpretar `Tests` em um run antigo como regressão do workflow atual nem declarar que testes foram executados no novo. Não reintroduzir a etapa sem novo pedido.
- Não confundir CI com QA de gameplay/Windows/FPS, não afirmar sucesso quando job ainda está pendente ou em execução, não silenciar warnings nem prometer trabalho assíncrono. Evitar consultas repetitivas à CI sem informação nova; comunicar resultado e bloqueio concretos ao usuário.

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

Validar **o workflow atual sem testes unitários** por uma execução iniciada depois do commit `a08d890`, verificando Clippy e Check separadamente e corrigindo somente erros concretos. Fazer QA no jogo para Bassalt, nome Grass, ícones brush base/tint/cor/clear em todas as telas, sem editar PNGs. Em seguida retomar P0 rios, biomas, luz e margem; depois P1. Cada novo bloco altera versão/handoff conforme a natureza da mudança.