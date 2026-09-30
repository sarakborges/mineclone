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
- RenderApp já é decomposto em `ExtractCommands`, `PrepareAssets`, `PrepareMeshes`, views, queue, `Prepare`, `Render` e cleanup.

### Evidência pré-Cut 15

Gameplay `2026-09-30_01-46-22-083021400.txt`:

- primeiro intervalo de Gameplay: `frame_max_us=272327`, `render work max_us=228307`, `main_work_max_us=12128`;
- `stage_prepare_max_us=122187`, `stage_render_max_us=80060`;
- steady-state: `Prepare` ~3 ms e `Render` ~11–12 ms;
- publication main-thread e async initial meshing não são o gargalo dominante.

### Cut 15 — world camera warm-up

Commit `47959b90c2e7f09774d1f24329dab48ff5a8bbae` (`Warm world camera before gameplay`), CI #10309 success.

- world camera nasce ativa ainda em Loading/Spawning, mantendo `CameraOutputMode::Skip` atrás do overlay;
- loading camera usa ordem explícita acima da world camera;
- nenhum meshing, generation, budget ou world truth mudou.

### Evidência após Cut 15

Gameplay novo mostrou melhora parcial, não resolução:

- pior frame de entrada caiu de ~272 ms para ~201 ms;
- `stage_render_max` caiu de ~80 ms para ~68 ms;
- `stage_prepare_max` permaneceu dominante e subiu para ~133 ms;
- o slow frame ocorreu com `generation_tasks=0`, `mesh_tasks=0`, `remesh_tasks=0`, `async_work=0`;
- steady-state permaneceu aproximadamente 58–61 FPS, `Prepare` ~3 ms e `Render` ~11–12 ms.

Conclusão: o warm-up da world camera removeu parte do cold-start, mas o maior stall restante é `RenderSystems::Prepare` e precisa ser subdividido antes de qualquer nova otimização.

### Cut 16 — timing interno de `RenderSystems::Prepare`

Código: commit `d3a8778e8e0b891e1413b40c126a4b518bce5fe2` (`Split render prepare stage timing`), CI #10310 success.

- novo módulo `src/world/render_prepare_diagnostics.rs` instrumenta somente diagnóstico;
- usa os sub-sets oficiais do Bevy 0.19.1 dentro de `RenderSystems::Prepare`:
  - `PrepareResources`;
  - `PrepareResourcesBatchPhases`;
  - `PrepareResourcesWritePhaseBuffers`;
  - `PrepareResourcesCollectPhaseBuffers`;
  - `PrepareResourcesFlush`;
  - `PrepareBindGroups`;
- também mede `total` e `tail` entre o último subset explícito e o fim de `Prepare`, para não esconder trabalho fora dos sub-sets;
- lifecycle reset limpa as métricas em Loading e na entrada de Gameplay, junto da instrumentação já existente;
- o log novo começa com `render prepare:` e expõe `*_avg_us` / `*_max_us` para cada subfase;
- não altera camera, assets, batching, render budgets, presentation, gameplay ou world truth.

## Próximo passo

1. rodar gameplay desta build e enviar o log `.txt`;
2. localizar qual campo de `render prepare:` contém o spike ~133 ms no primeiro intervalo de Gameplay;
3. fazer **um único cut de performance direcionado à subfase dominante**;
4. medir novamente contra os baselines pré/pós-Cut 15;
5. depois da dívida localizada/endereçada, executar audit final da Phase 7 e decidir encerramento.

### Interpretação esperada do próximo log

- `resources` dominante: investigar criação/preparo de buffers/texturas/uniforms;
- `batch_phases` dominante: investigar batching e cardinalidade de phase items;
- `write_phase_buffers` dominante: investigar volume de batch/phase data enviado ao GPU;
- `collect_phase_buffers` dominante: investigar coleta/readback interno de buffers de phase;
- `flush` dominante: investigar flush/upload/synchronization de resources;
- `bind_groups` dominante: investigar criação/churn de bind groups e resources Gameplay-only;
- `tail` dominante: existe trabalho de Prepare fora dos sub-sets oficiais e será necessário instrumentá-lo separadamente.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não reescrever/force-push commits publicados; usar fast-forward;
- cada bloco coerente deve terminar com HANDOFF e gate verde;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
