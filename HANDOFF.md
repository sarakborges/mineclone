# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico integral anterior foi preservado em `HANDOFF_ARCHIVE_2026-09-25.md`. Para continuidade normal, comece por este arquivo.

## 2026-09-25/26 — 0.68.47 reconstrói Electro GLBs e adiciona auditoria estrutural

- QA da 0.68.46 ainda panicou no preload do Electro normal, agora em `slime_electro.glb`:
  `Gltf(Binary(Length { length: 8220, length_read: 8172 }))`.
- A investigação confirmou que o problema era o asset gerado, não a referência nem o preload:
  - o GLB normal declarava 8220 bytes no header, mas tinha só 8172 bytes; faltavam exatamente 48 bytes no final;
  - o GLB large também estava truncado, com uma diferença muito maior;
  - os `.gltf` anteriores também eram estruturalmente inválidos: o normal continha 7948 bytes decodificados no data-URI do buffer, mas declarava `byteLength=5228`; todos os bufferViews válidos terminavam em 5228, então havia lixo extra depois do buffer autoritativo.
- Para não reparar binário por adivinhação, foi criado temporariamente um workflow one-shot que:
  - faz checkout com histórico completo;
  - recupera os `.gltf` fonte do commit `35b8aaca906e1fdf48bae56feb4bb94b84814861`;
  - respeita o `buffers[0].byteLength` declarado e descarta bytes excedentes que não pertencem a nenhum bufferView;
  - valida todos os bufferViews/accessors;
  - move imagens data-URI para bufferViews GLB válidos;
  - escreve GLB 2.0 com chunks JSON/BIN alinhados e header final consistente;
  - valida novamente o container antes de escrever/commitar.
- Primeiro run do rebuild falhou de forma segura ao detectar a inconsistência 7948 != 5228 e não publicou assets.
- Segundo run passou e commitou os dois assets reconstruídos:
  - `18e48e0625a8a61c54e8866a8510b9c6ccc34e0a` — `Rebuild Electro GLB containers`;
  - Electro normal: header declara 11480 bytes e o arquivo tem exatamente 11480 bytes;
  - Electro large: header declara 11780 bytes e o arquivo tem exatamente 11780 bytes.
- O workflow one-shot foi removido depois do rebuild.
- Para impedir recorrência, foi adicionado `tools/check_glb_assets.py` e o CI agora executa `Audit GLB assets` antes de instalar Rust. O audit verifica:
  - magic/version/length do header;
  - chunks JSON/BIN e alinhamento;
  - parse do JSON;
  - tamanho do buffer embedded;
  - ranges de bufferViews;
  - ranges/tamanho de accessors.
- As definitions Electro continuam usando `.glb` + face externa `textures/creatures/slime_electro/face.png`.
- A instrumentação de performance da 0.68.45 (`frame`, `main_work`, `render work`) permanece intacta.
- CI funcional da 0.68.47: run `36211664885` — success em localizations, structure content references, novo `Audit GLB assets`, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.

VERSION: `0.68.47`.

## 2026-09-25/26 — 0.68.46 corrige referência de preload dos Electro Slimes

- A 0.68.45 não chegou ao gameplay porque `setup_world` panicou no preload de `slime_electro.gltf`.
- `slime_electro.json` e `slime_electro_large.json` foram alterados para `.glb`.
- Ambos receberam override explícito do material `SlimeFace` para `textures/creatures/slime_electro/face.png`.
- Os `.gltf` quebrados foram removidos do HEAD.
- Essa versão revelou que os `.glb` também haviam sido gerados incorretamente; a correção estrutural definitiva está na 0.68.47.

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

1. Rodar a **0.68.47** e confirmar que Loading entra em Gameplay sem panic do Electro normal ou large.
2. Gerar log de gameplay com período parado e movimento/streaming normal.
3. Comparar no mesmo intervalo `frame_*`, `main_work_*` e `render work` para escolher o próximo domínio de otimização.
4. Antes de cada novo bloco de alteração, manter CI sem erros e sem warnings.
