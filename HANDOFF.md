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
- portanto o cold-start restante está especificamente em **Bevy `RenderSystems::PrepareResources`**. Não é publication, async meshing, bind-group creation nem mesh-instance buffer flush;
- steady-state parado permanece ~60–61 FPS, `Prepare` ~3.1 ms e `Render` ~11.6 ms;
- existe um problema separado durante movimento: uma janela registrou frame max `102812 us` com **`main_work_max_us=102244`**, enquanto `stage_prepare_max_us=5166` e `stage_render_max_us=19341`. O vídeo mostra o hitch correspondente. Esse stall é main-world e deve ser investigado em cut separado após o startup;
- outro spike posterior de render mostrou `flush_max_us=11337`, mas é menor e distinto do cold-start inicial.

### Audit de hipóteses antes do Cut 17

Hipóteses descartadas com código/evidência antes de mudar lifecycle:

- viewmodel camera: target/view preparation pertence a `PrepareViews`, não à subfase culpada `PrepareResources`;
- milhares de mesh instances: o write/flush do buffer de mesh instances cai em `PrepareResourcesFlush`, mas o startup mediu só ~1.2 ms de flush;
- player GLB/skin: `player.glb` não usa skinning (`JOINTS_0`/`WEIGHTS_0`/node `skin` ausentes), portanto `prepare_skins` não explica o spike;
- `publish_initial_count` cruza Loading -> Gameplay por design, enquanto as métricas do Cut 16 são zeradas em `OnEnter(Gameplay)`; não usar esse contador acumulado como prova de publication no frame do spike.

Delta de lifecycle que resta dentro de `PrepareResources`:

- `SunLightingPlugin` cria o `DirectionalLight` somente em `OnEnter(Gameplay)`;
- `DynamicLightsPlugin` cria o `PointLight` da mão somente quando seu system começa a rodar em Gameplay;
- Bevy extrai essas luzes para `prepare_lights` / clustered-light preparation dentro de `PrepareResources`;
- ambas já têm shadow maps desativados, portanto o cold-start não é shadow rendering.

### Cut 17 — prewarm da infraestrutura PBR de lighting durante Loading

- novo `LightingWarmupPlugin` cria durante Loading uma `DirectionalLight` de illuminance zero, visível para o renderer e sem shadows;
- quando `GameplayCamera` aparece durante Loading, o mesmo warm-up cria como child uma `PointLight` de intensity zero, usando range/radius e shadow policy da luz dinâmica real;
- ambos warm-up entities usam `DespawnOnExit(GameState::Loading)`: eles desaparecem na transição e as luzes reais de Gameplay continuam sendo criadas pelos owners atuais, sem compartilhar gameplay state com o warm-up;
- `sun_directional_light` e `held_point_light` viram helpers internos reutilizados pelos owners reais e pelo warm-up, evitando configuração duplicada;
- zero intensity/illuminance mantém o warm-up visualmente inerte; `Visibility::Visible` é deliberado para Bevy extrair as lights e preparar o path PBR atrás do overlay;
- nenhum lighting value real, day/night behavior, terrain lighting buffer, shadow policy, camera, meshing, streaming ou world truth é alterado;
- **não declarar ganho até gameplay log novo**. Métrica primária: primeiro Gameplay `resources_max_us`; comparar com 67.598 ms desta evidência.

## Próximos passos

1. passar audits + Clippy + Check do Cut 17;
2. coletar gameplay log novo e comparar primeiro Gameplay `resources_max_us`, `stage_prepare_max_us`, `stage_render_max_us` e frame max;
3. se `PrepareResources` cair, manter o warm-up e então separar/atacar o `stage_render` steady-state;
4. se não cair, remover/repensar o warm-up em vez de empilhar outras hipóteses;
5. depois do startup, instrumentar o main schedule para localizar o hitch de movimento de ~102 ms;
6. executar audit final da Phase 7 quando a dívida de performance estiver localizada/endereçada.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não reescrever/force-push commits publicados; usar fast-forward;
- cada bloco coerente deve terminar com HANDOFF e gate verde;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
