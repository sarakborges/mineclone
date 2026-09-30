# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico detalhado anterior ao Cut 16 está preservado em `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico ainda mais antigo em `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura em `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante estes cutovers internos.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar.
- Old saves/legacy compatibility não são prioridade.
- CI = audits + Clippy + Check; `cargo test` não roda automaticamente.
- Não avançar com gate vermelho ou em andamento.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues têm owner e bound explícitos;
- trabalho frame-sensitive é incremental/budgetado;
- generation não possui side effects de render/UI/ECS;
- presentation é derivada e descartável; `ChunkRenderPool` nunca é world truth;
- cada migration block termina com HANDOFF atualizado e CI verde;
- não reintroduzir hydrology legado.

## Fases

- Phase 1 concluída: core types/boundaries.
- Phase 2 concluída: authoritative storage/revisions/lookup/eviction ownership.
- Phase 3 concluída: biome/structure metadata independente de render/materialization.
- Phase 4 concluída: Streaming scheduler v2.
- Phase 5 concluída: Terrain generation v2.
- Phase 6 concluída: structures/connectors/feature planning.
- **Phase 7 em andamento: voxel presentation / meshing v2.**

## Phase 7 — estado consolidado

### Ownership/correctness já fechado

- initial meshing e remesh usam snapshots imutáveis e stale checks antes de publication;
- content/halo source e lighting source são stamps separados;
- `ChunkRenderPool` mantém source stamps section-aware por terrain/fluid meshlet;
- render section identity é explícita (`coord + kind + meshlet_index`);
- chunks vazios também recebem source stamp sem snapshot pesado;
- presentation scheduler é o único owner nominal do scheduling inicial;
- full presentation reset drena entities/meshes/stamps/accounting e avança `presentation_reset_revision`;
- streaming observa cada reset uma vez e requeueia presentation ausente a partir do `VoxelWorld` residente, sem regenerar world truth nem repetir side effects once-per-residency.

### Observabilidade fechada até aqui

- meshing async, publication main-thread e RenderApp são medidos separadamente;
- publication inicial/remesh apply são microsegundos na maior parte das janelas e não explicam o stall principal;
- RenderApp é decomposto em `ExtractCommands`, `PrepareAssets`, `PrepareMeshes`, views, queue, `Prepare`, `Render` e cleanup;
- Cut 16 subdivide `Prepare` nos sub-sets oficiais do Bevy 0.19.1.

### Cut 15 — world camera warm-up

Commit `47959b90c2e7f09774d1f24329dab48ff5a8bbae` (`Warm world camera before gameplay`), CI #10309 success.

- world camera nasce ativa ainda em Loading/Spawning, mantendo `CameraOutputMode::Skip` atrás do overlay;
- loading camera usa ordem explícita acima da world camera;
- pior frame de entrada caiu de ~272 ms para ~201 ms, mas `stage_prepare_max` permaneceu dominante (~133 ms);
- nenhum meshing, generation, budget ou world truth mudou.

### Cut 16 — timing interno de `RenderSystems::Prepare`

Código: commit `d3a8778e8e0b891e1413b40c126a4b518bce5fe2` (`Split render prepare stage timing`), CI #10310 success. Handoff/archive: `a9c75a9c03cdfcf292d5c64c9ea66c68b27f4639`, CI #10311 success.

- `render_prepare_diagnostics` mede `PrepareResources`, batch phases, write phase buffers, collect phase buffers, flush, bind groups, total e tail;
- lifecycle reset limpa essas métricas ao entrar em Loading e Gameplay;
- somente diagnóstico; não altera camera, assets, batching, budgets, presentation ou gameplay.

### Evidência após Cut 16

Gameplay `2026-09-30_04-17-04-401157000.txt` + vídeo `Gravando 2026-09-30 012214.mp4`:

- Loading: `render prepare total_max_us=8373`, `resources_max_us=1000`, `bind_groups_max_us=7891`;
- primeiro intervalo de Gameplay: `render prepare total_max_us=70971`, **`resources_max_us=67598`**, `bind_groups_max_us=3169`, `flush_max_us=1189`;
- no mesmo intervalo: `stage_prepare_max_us=70985`, `stage_render_max_us=50495`, `render work max_us=101426`, frame max `129088`, `main_work_max_us=7388`;
- portanto o cold-start restante está especificamente em **Bevy `RenderSystems::PrepareResources`**;
- existe um problema separado durante movimento: uma janela registrou frame max `102812 us` com **`main_work_max_us=102244`**, enquanto `stage_prepare_max_us=5166` e `stage_render_max_us=19341`. Esse stall é main-world e deve ser investigado em cut separado após o startup.

### Cut 17 — hipótese de prewarm de lighting PBR

Commit `0b215f029d7e107f58dd57342c9fc752eda4f803` (`Warm PBR lighting before gameplay`), CI #10312 success.

- experimento criou `DirectionalLight` e `PointLight` zero-intensity durante Loading para tentar pagar o cold-start de lighting atrás do overlay;
- as entidades eram separadas das luzes reais de Gameplay e não alteravam valores funcionais, shadows, world truth ou streaming;
- gameplay log `2026-09-30_05-09-17-966417700.txt` **refutou a hipótese**:
  - Loading ficou com `resources_max_us=1659`, `bind_groups_max_us=12197`;
  - primeiro intervalo de Gameplay ficou com `render prepare total_max_us=97863`, **`resources_max_us=93610`**, `stage_render_max_us=111069` e frame max `169143`;
  - depois do burst, `resources_max_us` volta a ~2–3 ms;
- conclusão: lighting warm-up não remove o cold-start de `PrepareResources`; não deve permanecer como complexidade especulativa.

### Audit pós-Cut 17 — clustering per-view

A leitura do Bevy 0.19.1 localizou uma operação com perfil compatível dentro de `PrepareResources`:

- `prepare_clusters_for_gpu_clustering` roda em `RenderSystems::PrepareResources` quando GPU clustering está habilitado;
- por view 3D ele cria `ViewClusterBindings` e `ViewGpuClusteringBuffers`, reserva cluster storage, lista inicial de até 65.536 índices e scratch buffers e escreve buffers GPU;
- a world camera já existe durante Loading por causa do Cut 15;
- a **viewmodel camera só nasce no primeiro Update de Gameplay**, portanto cria uma nova view 3D justamente na janela do cold-start;
- a viewmodel não necessita clustered lighting:
  - o braço usa `StandardMaterial` explicitamente `unlit=true`;
  - held-block usa `BlockModelMaterial` com display shading próprio;
  - a câmera já usa render layer dedicada e não deve compartilhar iluminação/world visibility da world camera;
- Bevy 0.19.1 fornece `ClusterConfig::None` com semântica explícita de desabilitar cálculos de cluster para aquela view.

### Cut 18 — remover warm-up refutado e desligar clustering da viewmodel

- remover `LightingWarmupPlugin` do `RenderingPlugin` e deletar `src/rendering/lighting_warmup.rs`;
- restaurar visibilidade mínima dos helpers de `sun_lighting` / `dynamic_lights` que haviam sido ampliados somente para o experimento;
- adicionar `ClusterConfig::None` exclusivamente à segunda `Camera3d` usada pela viewmodel;
- world camera continua com clustering normal e iluminação PBR real;
- nenhuma mudança em meshing, streaming, generation, presentation ownership ou world truth;
- objetivo mensurável: evitar alocação/preparo de clustered-light resources para uma view que não usa luz PBR;
- **não declarar ganho até gameplay log novo**.

## Próximos passos

1. passar audits + Clippy + Check do Cut 18;
2. coletar gameplay log novo e comparar primeiro Gameplay `resources_max_us`, `stage_prepare_max_us`, `stage_render_max_us` e frame max contra 93.610 ms / 97.902 ms / 111.069 ms / 169.143 ms do log pós-Cut 17 e também contra o baseline pré-Cut 17 de 67.598 ms em `PrepareResources`;
3. se o pico cair, manter `ClusterConfig::None` na viewmodel e continuar separando o custo residual de startup;
4. se não cair, instrumentar/alterar outro candidato dentro de `PrepareResources`, sem reintroduzir warm-ups especulativos;
5. depois do startup, instrumentar o main schedule para localizar o hitch de movimento de ~102 ms;
6. executar audit final da Phase 7 quando a dívida de performance estiver localizada/endereçada.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não reescrever/force-push commits publicados; usar fast-forward;
- cada bloco coerente deve terminar com HANDOFF e gate verde;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
