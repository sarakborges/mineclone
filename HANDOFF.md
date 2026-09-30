# HANDOFF — Asteria / Mineclone

> Handoff operacional atual. Histórico anterior ao Cut 16: `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico antigo: `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura: `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch de continuação: `architecture/asteria-core-rebuild-phase7`.
- Draft PR: #24 — `Continue Asteria Phase 7 presentation cold-start work`.
- Asteria core rebuild original foi integrado em `develop` pelo PR #22 / merge `48197a30ed78cc6b3eadd5f2be7fcf0d2a9c4204`.
- `develop` foi sincronizado novamente no branch de continuação até `5ad2263b01e45be87215719eccc88c08234f433c`.
- `VERSION`: `0.68.61`; mudanças paralelas de slime/assets continuam preservadas.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar.
- Old saves/legacy compatibility não são prioridade.
- CI obrigatório = audits + Clippy + Check; `cargo test` não é gate automático.
- `HANDOFF.md` é `paths-ignore` no workflow, portanto commits exclusivamente documentais não disparam novo gate.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues possuem owner e bound explícitos;
- trabalho frame-sensitive precisa ser incremental/budgetado;
- generation não possui side effects de render/UI/ECS;
- presentation é derivada e descartável; `ChunkRenderPool` nunca é world truth;
- cada cut coerente termina com HANDOFF atualizado e último gate aplicável verde;
- não reintroduzir Hydrology legado.

## Fases

- Phases 1–6 concluídas: core boundaries, authoritative storage, biome/structure metadata, Streaming scheduler v2, Terrain generation v2 e structures/connectors/feature planning.
- **Phase 7 em andamento: voxel presentation / meshing v2 + dívida de performance/correctness descoberta durante validação.**

## Cuts recentes

### Cut 23 — surface `size.min`

Commit `35313349c08c39dab6249afa35a56323117ce8ae`, CI #10321 success.

- jitter independente ±32% por site podia comprimir vizinhos até 36% do spacing nominal;
- surface sites passaram a usar jitter lateral determinístico por linha; domain warp continua owner da organicidade.

### Cut 24 — limitar scans stale do generation frontier

Commit `75f476f54942f69ccd08df819b8b6ad5479099d8`, CI #10322 success.

- baseline antigo `2026-09-30_16-23-13-909518300.txt`: frame ~100.9 ms, `main_work` ~97.5 ms, `streaming` ~93.9 ms;
- `select_generation_wave` fazia priority scans O(n) e stale/already-owned entries escapavam de `budget.record(1)`;
- cada `pop_pending_by_priority()` bem-sucedido agora consome uma unidade do budget de 1 ms / máximo 16 scans.

Validação pós-Cut 24 nos logs 0.68.55–0.68.59 mostra `streaming` normalmente na faixa sub-ms/baixo-ms; o antigo hitch de ~94 ms não é mais o gargalo dominante. O problema restante migrou para o cold-start do RenderApp.

### Cut 25 — ocean/coast height influence lower-only

Commit `cc116cb8d477259bc0ccc6d954177b6b2d7805a0`, CI #10323 success.

- terrain influences possuem política `Blend` ou `LowerOnly`;
- `BiomeTerrain::Ocean` é `LowerOnly`; demais terrains continuam `Blend`;
- ocean pode abaixar/truncar/soften coast, mas nunca elevar mountain/alps/mountain belt/volcano/gorge ou terrain futuro blended;
- região somente ocean preserva o shape oceânico.

### Cut 26 — volume biome constraints por surface biome

Commits principais `0c659fa3376fcc7f59594fe29dde97ca40b7ce22`, `4d84cae3783bdbfc20020ca5b7ee8ec424d8b472`, `c4b62f5bdf103a63807d0340234be5169c2e1bb9`; correção de gate `99cd6456af1f77b670f1e771d3b372cc46dac735`; sync `12806bd94b08498d15ac92111b74e22863935b93`, CI #10341 success.

- `BiomeDefinition.tags` para surface biomes;
- `BiomeDefinition.surfaceConstraints` opcional para volume biomes;
- selectors por `ids`/`tags`, allow/deny e deny precedence;
- density pass reutiliza a surface identity já amostrada sem novo surface sample por voxel;
- volume biome sem constraints preserva comportamento irrestrito.

### Cut 27 — volume biome surface indicators

Commit `5121da7ea2fb59d37b71c24ad12f38a1c722b9e6`, CI #10342 success.

- `VolumeStructurePlacementRules.mode` suporta `volume` e `surface_indicator`;
- indicator reutiliza o planner generalizado de `BiomeStructure`, incluindo groups/chance/priority/conflicts/reserve-space/connectors;
- identidade/chance continuam derivadas do volume site original e o root é projetado no surface Y;
- `surfaceConstraints` continuam aplicadas;
- nenhuma indicator concreta foi inventada para conteúdo atual.

### Cut 28 — floating islands como ilhas estratificadas

Consolidado no core rebuild antes do merge do PR #22.

- core central obrigatório + 3–5 lobes secundários determinísticos e conectados;
- smooth union `1 - (1-a)(1-b)` entre massas;
- footprint/altura/underside variam pelo seed com borda orgânica;
- `surfaceLayers` permitido em volume biome apenas para `floating_island`;
- Floating Islands autoram `grass_block` depth 1 -> `dirt` depth 4 -> `stone`;
- footprint autorado aumentado para `48..96` X/Z e `12..24` Y;
- early-out de chunks altos reconhece `FloatingIsland` como solid volume modifier;
- nenhuma regra de surface terrain, structure ou streaming foi alterada.

### Cut 29 — prewarm da apresentação antes de revelar Gameplay

Branch: `architecture/asteria-core-rebuild-phase7`; draft PR #24.

Baseline principal: `2026-09-30_21-05-23-706780000.txt` (0.68.59).

- primeiro bloco de Gameplay ainda mostrava `stage_prepare_max_us ~= 76 ms`, `stage_render_max_us ~= 62 ms` e frame máximo ~147 ms;
- `publish_initial_count=3193`, enquanto streaming/main-world já não explicavam o spike;
- root cause alvo: o mundo/câmera eram revelados enquanto caches e recursos render-side ainda aqueciam.

Implementação:

- player + world camera continuam sendo criados durante `WorldLoadingPhase::Spawning`;
- visibilidade exata dos chunks iniciais é primada ainda sob a tela de loading;
- novo `InitialPresentationPrewarm` é `Local<>` derivado, não world truth;
- após primar, o loading permanece por 12 frames completos antes de solicitar a transição para `Gameplay`;
- nenhuma mudança em worldgen, streaming, residency, meshing budgets, save ou authoritative state;
- implementação passou audits + Clippy + Check no CI #10496;
- sync final com `develop` 0.68.61 passou audits + Clippy + Check no CI #10500;
- commits posteriores são exclusivamente de HANDOFF/no-op e não alteram o código validado.

**Cut 29 estruturalmente fechado. Sem claim de ganho ainda:** precisa de log runtime novo dessa build.

## Próximos passos

1. rodar gameplay em build do Cut 29 e comparar o primeiro bloco de Gameplay com `2026-09-30_21-05-23-706780000.txt`;
2. validar visualmente em mundo novo: ocean/coast, volume constraints/indicator e floating islands;
3. se o spike de cold-start continuar, instrumentar readiness/sequence do RenderApp e decompor `PrepareResources`/`Render` antes do próximo corte;
4. depois retomar a dívida restante de voxel presentation / meshing v2.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild-phase7`, nunca direto em `develop`;
- sincronizar avanços paralelos de `develop` por merge real; não rebasear nem force-pushar commits publicados;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais;
- atualizar este HANDOFF em cada cut coerente.

## Conteúdo paralelo preservado

- 0.68.56: Cryo Slime reference rebuild.
- 0.68.57: Hydro Slime reference rebuild.
- 0.68.58: Pyro Slime reference rebuild.
- 0.68.59: Hydro/Pyro silhouette refinements.
- 0.68.60: Hydro exposed-face voxel continuation + Pyro root material refinement.
- 0.68.61: Hydro droplet vertically compressed; Pyro unchanged.
