# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico integral anterior foi preservado em `HANDOFF_ARCHIVE_2026-09-25.md`. Para continuidade normal, comece por este arquivo.

## 2026-09-25/26 — 0.68.48 repara semanticamente os Electro GLBs e endurece auditoria

- QA mostrou que a correção estrutural da 0.68.47 ainda não bastava: o preload do Electro normal continuava panicando dentro do `GltfLoader`, mesmo com header/chunks GLB formalmente válidos.
- O validator oficial da Khronos foi executado contra os assets e revelou a causa real:
  - Electro normal: 258 erros semânticos;
  - Electro large: 47 erros semânticos;
  - havia `bufferView`/accessor metadata stale apontando para regiões erradas do buffer, produzindo índices OOB, bounds incorretos e keyframes de animação lidos de bytes que não eram keyframes.
- A investigação recuperou os dados válidos diretamente dos `.gltf` históricos do commit `35b8aaca906e1fdf48bae56feb4bb94b84814861`:
  - no normal, body/face permaneciam válidos nos offsets originais e todos os 12 accessors de animação estavam intactos, deslocados +2720 bytes em relação ao metadata stale;
  - o bloco de details do normal pertencia a outra revisão: metadata dizia 54 vértices, enquanto o bloco real de índices referenciava `0..167`; ele não foi remendado por alteração artificial do count;
  - no large, body/face/details estavam recuperáveis; o details válido possui 62 vértices e exatamente 252 índices, todos dentro de range e sem triângulos degenerados;
  - o metadata large dizia 276 índices e acabava lendo 24 valores de dentro do bloco de animação.
- O reparo definitivo repacotou cada accessor em um buffer novo e sequencial, reconstruiu todos os `bufferViews`, recalculou bounds a partir dos bytes reais e validou relações entre primitives/accessors antes de escrever o GLB.
- O details Electro válido do large foi usado como topologia canônica também no normal, remapeado para o bounding box authored do details normal. Body, face e animações normais continuam usando os dados recuperados do próprio normal; não foi copiado outro tipo de slime.
- A face embedded histórica foi removida dos GLBs porque as definitions já aplicam `textures/creatures/slime_electro/face.png` externamente ao material `SlimeFace`; isso também eliminou os únicos warnings restantes do validator GLB.
- Resultado do one-shot antes de publicar:
  - `tools/check_glb_assets.py`: 22 assets validados;
  - Khronos glTF Validator: Electro normal `0 errors / 0 warnings`;
  - Khronos glTF Validator: Electro large `0 errors / 0 warnings`.
- Assets finais publicados em `develop`:
  - `6d8b22bdfcdaf8ba21592e961b03b5639efe94b2` — `Repair Electro GLB accessor layout`.
- O audit permanente `tools/check_glb_assets.py` foi ampliado para validar, além do container:
  - bounds reais de accessors contra `min/max` declarados;
  - contagem de atributos de vertex contra `POSITION`;
  - índices de primitives dentro do count de vértices;
  - `COLOR_0` float dentro de `[0, 1]`;
  - animation input como `FLOAT SCALAR`;
  - key times finitos e estritamente crescentes.
- O audit semântico novo passou em todos os 22 GLBs existentes e o CI completo do bloco passou no run `36212679301`, incluindo Clippy `-D warnings` e `cargo check --locked`.
- Os três workflows temporários de validator/inspeção/reparo foram removidos após o diagnóstico. A proteção permanente ficou em `tools/check_glb_assets.py` + CI normal.
- A instrumentação de performance da 0.68.45 (`frame`, `main_work`, `render work`) permanece intacta; depois de confirmar que Loading entra em Gameplay, a investigação de FPS volta exatamente desse ponto.

VERSION: `0.68.48`.

## 2026-09-25/26 — 0.68.47 corrigiu container GLB, mas não metadata semântico

- QA da 0.68.46 panicou no preload do Electro normal com:
  `Gltf(Binary(Length { length: 8220, length_read: 8172 }))`.
- A investigação inicial confirmou que os GLBs tinham header/chunks truncados e foi feito um rebuild estrutural.
- Commit do rebuild de container:
  - `18e48e0625a8a61c54e8866a8510b9c6ccc34e0a` — `Rebuild Electro GLB containers`.
- Também foi criado `tools/check_glb_assets.py` e o CI passou a executar `Audit GLB assets` antes de instalar Rust.
- Essa auditoria inicial verificava magic/version/length, chunks JSON/BIN, alinhamento, buffer size e ranges de bufferViews/accessors.
- O container reconstruído passou nessa auditoria, mas QA mostrou um novo `AssetLoaderPanic`; o validator Khronos então revelou que o conteúdo semântico continuava corrompido.
- Portanto a 0.68.47 deve ser considerada a correção da camada de container, não a correção definitiva do Electro. O reparo semântico completo está na 0.68.48.

VERSION: `0.68.47`.

## 2026-09-25/26 — 0.68.46 corrige referência de preload dos Electro Slimes

- A 0.68.45 não chegou ao gameplay porque `setup_world` panicou no preload de `slime_electro.gltf`.
- `slime_electro.json` e `slime_electro_large.json` foram alterados para `.glb`.
- Ambos receberam override explícito do material `SlimeFace` para `textures/creatures/slime_electro/face.png`.
- Os `.gltf` quebrados foram removidos do HEAD.
- Essa versão revelou que os `.glb` também haviam sido gerados incorretamente; a correção definitiva está na 0.68.48.

VERSION: `0.68.46`.

## 2026-09-25/26 — Rendering diagnostics 0.68.45 mede o Render schedule separadamente

- Continuação da investigação de FPS após os logs mostrarem slow frames com streaming/generation/mesh/remesh zerados.
- A 0.68.43 adicionou `main_work_*` e mudou Windows/DX12 para `PresentMode::Mailbox`.
- A 0.68.44 reduziu o segundo anel de chunks invisíveis atrás do fog (`show/hide` default 14/15 -> 13/14).
- A 0.68.45 adicionou `src/world/render_work_diagnostics.rs` para medir wall time do schedule `Render` separadamente do `Main`:
  - timer antes de `RenderSystems::ExtractCommands`;
  - fim depois de `RenderSystems::PostCleanup`;
  - bridge latest-only por `Arc<AtomicU64>`, sem mutex/alocação por frame;
  - logs `render work: samples=... skipped_samples=... avg_us=... p50_us=... p95_us=... p99_us=... max_us=...`.
- Interpretação do próximo log:
  - frame alto + main alto => perfilar Main systems;
  - frame alto + main baixo + render alto => perfilar Render schedule/submit;
  - frame alto + main baixo + render baixo => presentation/GPU/driver ou trabalho fora das janelas medidas.
- CI funcional da implementação: run `36207686674` verde.

VERSION: `0.68.45`.

## Continuidade imediata

1. Rodar a **0.68.48** e confirmar que Loading entra em Gameplay sem panic do Electro normal ou large.
2. Gerar log de gameplay com período parado e movimento/streaming normal.
3. Comparar no mesmo intervalo `frame_*`, `main_work_*` e `render work` para escolher o próximo domínio de otimização.
4. Antes de cada novo bloco de alteração, manter CI sem erros e sem warnings.
