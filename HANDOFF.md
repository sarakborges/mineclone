# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico integral anterior a este arquivo foi preservado byte por byte em `HANDOFF_ARCHIVE_2026-09-25.md` (blob original `c06357d3f869a09effb14de27c6712dc11dc086c`). Consulte o arquivo histórico apenas quando a tarefa exigir contexto antigo; para continuidade normal, comece por este arquivo.

## 2026-09-25/26 — 0.68.46 corrige crash de preload dos Electro Slimes

- QA da 0.68.45 não chegou ao gameplay porque `setup_world` panicou no preload de `models/creatures/slime_electro/slime_electro.gltf` com `AssetLoaderPanic` do `bevy_gltf::loader::GltfLoader`.
- O diretório Electro tinha dois formatos ao mesmo tempo (`.gltf` e `.glb`), mas as definitions normal/large eram as únicas da família recente apontando para `.gltf`; os demais slimes data-driven usam `.glb`.
- `data/creatures/slime_electro.json` agora aponta para `models/creatures/slime_electro/slime_electro.glb`.
- `data/creatures/slime_electro_large.json` agora aponta para `models/creatures/slime_electro_large/slime_electro_large.glb`.
- As duas definitions também receberam o mesmo contrato de face externa usado pelos outros slimes:
  `"textures": { "SlimeFace": "textures/creatures/slime_electro/face.png" }`.
  Isso garante que a face Electro editada recentemente seja realmente usada pelo runtime, em vez de depender da imagem embedded no modelo.
- Os dois `.gltf` quebrados foram removidos para evitar regressão por referência futura acidental; os `.glb` permanecem como assets autoritativos.
- Commits do fix:
  - `588dbe971d308a52b7f7913af401201bcde24df3` — Electro normal usa `.glb` + face externa;
  - `ae5efce8cc8d156b048719d618594f59b2f6e521` — Electro large usa `.glb` + face externa;
  - `9a62a460e72c944fa945c9358b961209196fde4a` — remove `.gltf` quebrado normal;
  - `fbdcffb187b979e73703fa1621659c13f790f499` — remove `.gltf` quebrado large;
  - `0372fd8811591fcff11268017b1f2d31fef65a93` — VERSION 0.68.46.
- Antes de retomar qualquer otimização de FPS, CI deste HEAD precisa ficar verde sem warnings e o runtime deve confirmar que Loading entra em Gameplay.
- A instrumentação `render work` da 0.68.45 permanece intacta; o próximo log útil de performance deve portanto ser produzido já na 0.68.46.

VERSION: `0.68.46`.

## 2026-09-25/26 — Rendering diagnostics 0.68.45 mede o Render schedule separadamente

- Continuação direta da investigação de FPS após 0.68.44.
- O log 0.68.42 já havia mostrado slow frames grandes mesmo com streaming/generation/mesh/remesh zerados, portanto o pipeline de chunks sozinho não explicava os spikes.
- A 0.68.43 adicionou `main_work_*` para medir o trabalho do Main Schedule e mudou o Windows/DX12 padrão para `PresentMode::Mailbox`, removendo o degrau rígido FIFO 60 -> 30 quando um frame perde vblank.
- A 0.68.44 reduziu custo objetivo de renderização removendo o segundo anel de chunks que permanecia visível completamente atrás do fog: no render distance default 12, `show/hide` passou de `14/15` para `13/14`.
- A 0.68.45 adiciona uma terceira medição independente para separar melhor CPU/Main de renderer/present/GPU:
  - novo módulo `src/world/render_work_diagnostics.rs`;
  - o timer inicia no schedule `Render` antes de `RenderSystems::ExtractCommands`;
  - termina depois de `RenderSystems::PostCleanup`;
  - portanto mede wall time do **Render schedule**, não deve ser interpretado como timestamp de GPU e não inclui trabalho externo a esse schedule;
  - o resultado atravessa do RenderApp para o world principal por `Arc<AtomicU64>`, sem `Mutex`, fila ou alocação por frame;
  - a bridge é latest-only e usa um sequence number; se o main app não observar alguma amostra, `skipped_samples` torna isso explícito em vez de esconder a perda;
  - amostras observadas são acumuladas em janela de até 4096 e resumidas em avg/p50/p95/p99/max;
  - o runtime log agora recebe uma linha adicional a cada ciclo de diagnostics:
    `render work: samples=... skipped_samples=... avg_us=... p50_us=... p95_us=... p99_us=... max_us=...`.
- `FrameTimeSamples` e `MainFrameWorkSamples` existentes não foram substituídos; a nova medição é complementar.
- Interpretação esperada do próximo log:
  - `frame` alto + `main_work` alto => gargalo continua no Main Schedule e deve ser perfilado por systems;
  - `frame` alto + `main_work` baixo + `render work` alto => custo está no lado CPU/submit do Render schedule;
  - `frame` alto + `main_work` baixo + `render work` baixo => espera de presentation/GPU ou trabalho fora das duas janelas medidas fica muito mais provável;
  - `skipped_samples > 0` => considerar a bridge latest-only ao comparar contagens, mas os percentis das amostras recebidas continuam úteis.
- O diagnóstico é resetado ao entrar em Loading/Gameplay para não misturar sessões.
- Implementação funcional no `develop`:
  - `b88a18fee3a062eb4545eb58c80e0ac3435aea60` — primeira implementação do RenderApp timing;
  - `4d05911084079a15f58db7b12e927d7534009c7d` — restaura wiring correto de `world/mod.rs` após o primeiro CI encontrar integração incorreta;
  - `07bc32b256da7434417c63cb7aa5b9cd93d68469` — corrige const mask para Rust 1.98.1.
- CI funcional final: run `36207686674` — success em localization audit, structure content reference audit, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.

VERSION: `0.68.45`.

## 2026-09-25 — Rendering 0.68.44 remove segundo anel invisível além do fog

- A auditoria não encontrou prepasses, SSAO, Bloom, TAA/SMAA/FXAA ou MSAA ativos na câmera do mundo; GPU occlusion/frustum culling experimental continua desligado por causa do flicker já confirmado em QA.
- HDR não foi removido por hipótese; faz parte do stack explícito world -> viewmodel -> UI e já teve regressão histórica quando alterado isoladamente.
- `chunk_visibility_radii` mantinha no default RD12 `show=14` e `hide=15`, enquanto o fog nominal termina em ~11.76 chunks.
- Como o centro de streaming é um chunk inteiro, um único chunk de margem cobre o deslocamento interno do player: os raios foram fixados em `show = nominal + 1` e `hide = show + 1`.
- Default RD12: `14/15 -> 13/14`; área horizontal potencial do show circle cai ~13.8% e hide ~12.9%, sem reduzir a distância nominal visível.
- O guard dinâmico do fog continua recuando durante streaming incompleto quando faltam colunas dentro do raio nominal.

VERSION: `0.68.44`.

## 2026-09-25 — Runtime 0.68.43 remove degrau FIFO 60→30 e mede Main Schedule

- O leak de font atlas da 0.68.42 ficou estabilizado no log seguinte (`font_atlas_faces=1`, `font_atlases=10..11`, ~11 MiB).
- Streaming não explicava a queda persistente: havia períodos com backlog renderável e generation/mesh/remesh zerados ainda próximos de 30 FPS.
- O padrão 16.7 ms -> ~30 ms tinha assinatura compatível com FIFO/VSync quando um vblank era perdido.
- Windows/DX12 padrão passou a `PresentMode::Mailbox`; backend Windows explicitamente escolhido por `WGPU_BACKEND` e plataformas não-Windows mantêm `AutoVsync`.
- Diagnostics passaram a medir `main_work_avg_us`, p50/p95/p99/max do Main Schedule entre `First` e `Last`.
- CI funcional: run `36204035283` — success.

VERSION: `0.68.43`.

## 2026-09-25 — UI 0.68.42 fecha race de system-font atlas

- O log 0.68.41 mostrou crescimento progressivo de `font_atlas_faces` e `font_atlas_bytes` mesmo com streaming ocioso.
- `pin_ui_font_handles` saiu de `Update`/`Added<TextFont>` para `PostUpdate`/`Changed<TextFont>`, explicitamente antes do carregamento da font collection e de `UiSystems::Content`.
- Isso garante que TextFonts novas/modificadas recebam handles concretos estáveis antes de layout/atlas, fechando a janela que recriava system-font faces.
- CI funcional: run `36200522205` — success.

VERSION: `0.68.42`.

## Continuidade imediata

1. Confirmar CI verde da **0.68.46**.
2. Rodar/obter um log de gameplay da 0.68.46 com período parado e movimento/streaming normal; Loading precisa chegar a Gameplay sem o panic Electro.
3. Comparar, no mesmo intervalo, `frame_*`, `main_work_*` e a nova linha `render work`.
4. Só então escolher o próximo alvo:
   - Main Schedule systems;
   - Render schedule/submit;
   - presentation/GPU/driver;
   - ou, se os dados contradisserem a hipótese atual, ampliar a instrumentação antes de otimizar.
5. Manter a regra do projeto: antes de cada próximo bloco de alteração, CI sem erros e sem warnings.
