# HANDOFF — Asteria / Mineclone

> Handoff corrente e operacional. Histórico anterior ao Cut 16: `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico antigo: `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura: `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline de develop: `5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante os cutovers internos.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar.
- Old saves/legacy compatibility não são prioridade.
- CI obrigatório = audits + Clippy + Check; `cargo test` não é gate automático.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues possuem owner e bound explícitos;
- trabalho frame-sensitive precisa ser incremental/budgetado;
- generation não possui side effects de render/UI/ECS;
- presentation é derivada e descartável; `ChunkRenderPool` nunca é world truth;
- cada cut coerente termina com HANDOFF atualizado e CI verde;
- não reintroduzir Hydrology legado.

## Fases

- Phases 1–6 concluídas: core boundaries, authoritative storage, biome/structure metadata, Streaming scheduler v2, Terrain generation v2 e structures/connectors/feature planning.
- **Phase 7 em andamento: voxel presentation / meshing v2 + dívida de performance/correctness descoberta durante validação.**

## Phase 7 — estado consolidado

### Presentation/render

- initial meshing e remesh usam snapshots imutáveis + stale checks;
- content/halo e lighting possuem stamps separados, section-aware;
- full presentation reset é descartável e reconstruído do `VoxelWorld` residente;
- publication main-thread é barata na maior parte das janelas e não explica os grandes hitches;
- RenderApp está instrumentado por stages e `Prepare` pelos sub-sets oficiais do Bevy 0.19.1.

### Cold-start conhecido

- Cut 15 aqueceu a world camera ainda em Loading e reduziu parcialmente o primeiro frame;
- Cut 16 localizou o cold-start remanescente em `RenderSystems::PrepareResources`;
- lighting warm-up do Cut 17 foi refutado e removido;
- `ClusterConfig::None` na viewmodel do Cut 18 é inválido nesse backend/wgpu e foi revertido;
- cold-start de `PrepareResources` segue separado do hitch de movimento e deve ser retomado depois do pacote atual.

### Streaming/hitch

- Cut 19 instrumentou main-world stages e localizou os hitches de movimento em `streaming`;
- Cut 21 adicionou warnings direcionados para rebuild, scheduler snapshots/cancelamentos e direct-light seed;
- `ChunkTaskQueue` usa `check_ready`, portanto polling não espera worker terminar.

### Surface biome size correctness

- Cut 20 (`3525c290f13e5c361507f5097dd1c28569ffe98d`) passou a aplicar `size.max` à continuidade normal de surface biomes;
- Cut 22 (`7ce183d8491ee43b516ed060b84f7ce2f060d7cd`) separou `size.max` das hard constraints de adjacency para não transformar ausência de fallback em panic indevido;
- Cut 23 (`35313349c08c39dab6249afa35a56323117ce8ae`, CI #10321 success) corrigiu `size.min`: jitter independente ±32% por site podia comprimir sites vizinhos até 36% do spacing nominal. Surface sites agora usam jitter lateral determinístico por linha; domain warp continua owner da organicidade. Validar em mundo novo.

## Cut 24 — limitar scans stale do generation frontier

Commit `75f476f54942f69ccd08df819b8b6ad5479099d8`, CI #10322 success.

Gameplay `2026-09-30_16-23-13-909518300.txt` reproduziu o hitch com frame máximo ~100.9 ms, `main_work_max_us` ~97.5 ms e `streaming_max_us` ~93.9 ms, enquanto os demais buckets ficaram muito menores e nenhum warning direcionado do Cut 21 disparou.

Root cause e correção:

- `select_generation_wave` usa `pop_pending_by_priority`, que faz o scan caro da pending frontier;
- stale/already-owned entries eram descartadas sem `budget.record(1)`, permitindo scans O(n) repetidos fora do item cap;
- agora todo `pop_pending_by_priority()` bem-sucedido consome imediatamente uma unidade do budget de 1 ms / máximo 16 scans;
- prioridades, wave size, worker limits e world truth não mudaram.

Ainda não há claim de ganho até gameplay pós-Cut 24.

## Pacote Worldgen coherence — execução autorizada agora

### Cut 25 — ocean/coast height influence lower-only

Implementação preparada em `terrain.rs`:

- terrain influences agora possuem política explícita de composição: `Blend` ou `LowerOnly`;
- `BiomeTerrain::Ocean` resolve para `LowerOnly`; todos os demais terrains permanecem `Blend`;
- o compositor calcula a altura blended normal e, quando existem influências unrestricted, limita o resultado ao baseline que existiria sem as influências lower-only;
- consequência/invariante: adicionar ocean/coast influence nunca pode produzir altura maior que o terrain blend não-oceânico anterior;
- ocean continua livre para abaixar/truncar/coastal-soften qualquer terrain, inclusive mountains/alps/mountain belt/volcano/gorge, sem enumerar IDs/famílias;
- quando a região é somente ocean, o shape oceânico permanece inalterado;
- shoreline material/surfaceMargin identity não foi alterada.

Regressões unitárias cobrem: lower-only não eleva, lower-only ainda reduz, composição only-lower-only preserva shape e Ocean resolve para a política correta.

### Próximos workstreams

1. **Volume → surface constraints:** volume biomes podem declarar em quais surface biomes podem existir; determinístico e chunk-order invariant.
2. **Volume surface indicator:** volume biome pode declarar structure/structure group opcional no surface biome; usar structure planner generalizado.
3. **Floating islands rewrite:** ilhas grandes, irregulares e coerentes, smooth lobe transitions e grass -> dirt -> stone.

Princípios:

- metadata/data-driven para relações de conteúdo;
- sem hardcode de biome IDs quando primitive geral resolve;
- determinismo, chunk seams e generation-order invariance obrigatórios;
- Hydrology legado continua proibido;
- validar mudanças de worldgen em mundo novo.

## Próximos passos

1. fechar CI do Cut 25;
2. implementar constraints volume→surface;
3. implementar surface indicator via structures;
4. reescrever floating islands e estratificação;
5. gameplay em mundo novo para validar pacote + novo log pós-Cut 24;
6. retomar cold-start de `PrepareResources` depois do pacote.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não force-push/rewrite de commits publicados; usar fast-forward;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
