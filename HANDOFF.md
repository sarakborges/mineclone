# HANDOFF — Asteria / Mineclone

> Handoff corrente. Histórico anterior preservado em `HANDOFF_ARCHIVE_2026-09-25.md`. Para continuidade arquitetural, leia também `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-29

- Repo: `sarakborges/mineclone`.
- Branch ativa: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline da reconstrução: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante estes cutovers internos.
- Rust + Bevy permanecem. Bevy é host/framework; Asteria deve possuir world core, metadata/generation, runtime streaming, simulation boundaries e voxel presentation.
- Não ressuscitar hydrology legado. Rivers/lakes/cave entrances futuros pertencem ao sistema generalizado de structures/connectors/structure groups + deterministic metadata. Dynamic fluid simulation continua separada.
- Não preservar legacy/compat scaffolding sem necessidade. Old saves não são prioridade.
- CI precisa ficar verde entre migration blocks; não avançar sobre Clippy/check vermelho.

## Fase

- **Phase 1 concluída.** Core types/boundaries necessários para a reconstrução existem.
- **Phase 2 concluída.** Authoritative chunk/world storage, revisions, lookup semantics e eviction ownership estão explícitos.
- **Phase 3 concluída.** Biome/structure metadata é consultável sem render/materialization e caches derivados possuem ownership/bounds explícitos.
- **Phase 4 — Streaming scheduler v2 em andamento.**

## Phase 2 — estado final

### Authoritative storage owners

`VoxelWorld` continua como facade/coordenador, mas os containers concretos têm owners separados:

- `ResidentChunkStore`: resident chunks + vertical column index.
- `LoadedChunkColumnIndex`: query/index de Y por coluna.
- `ChunkPersistenceState`: persistent set + archived payloads.
- `ContentRevisionState`: map/counter de `ChunkContentRevision` por resident chunk.
- `ObjectRevisionState`: scene revision + per-chunk object revision.
- `BlockRevisionState`: global `BlockTopologyRevision`.

`VoxelWorld` coordena apenas invariantes multi-owner: insert/archive/restore, persistence promotion, revisions semânticas e mutation autoritativa.

### Mutation / dirty ownership

Gameplay/interação usa `VoxelMutationRuntime` para combinar mutation autoritativa com side effects de simulation/presentation:

- block edit -> lighting + remesh + fluid scheduling;
- layer edit -> remesh;
- interactive fluid edit -> lighting + remesh + fluid scheduling.

Bypasses diretos inspecionados e considerados **intencionais**:

- `world_objects.rs`: object mutation + `ObjectRevisionState`; object presentation é separada de voxel mesh.
- `voxel/lighting/propagation.rs`: light data muda diretamente; changed positions alimentam meshlet remesh.
- `world/fluid_updates.rs`: fluid simulation possui seu próprio lighting/remesh/reschedule pipeline.
- generated-fluid settling usa mutation derivada e só reconcilia lighting/remesh/frontier na publication.

Não criar `DirtyState` genérico. Simulation dirtiness hoje é representada por owners/queues específicos (`PendingLightingUpdates`, `PendingFluidUpdates`) porque não existe consumidor real para uma revisão escalar genérica.

### Revision domains

- `ChunkContentRevision`: conteúdo voxel de resident chunk; lighting não avança; block/layer/object/fluid avançam.
- `BlockTopologyRevision`: invalida block targeting visual; avança em chunk insert/archive/restore e mudança real de block; layer/fluid/object/light não avançam.
- Object scene/chunk revisions continuam semanticamente separadas internamente.
- O antigo `chunk_mesh_revisions` foi removido: presentation dirtiness pertence ao remesh scheduler/queue.

Importante: o experimento `7e97ec39` removeu erroneamente a antiga block revision. CI revelou o caller real em targeting; `e50519bb` restaurou a semantic e `8f729481` a transformou em `BlockTopologyRevision` + `BlockRevisionState`. Não repetir o diagnóstico de “estado morto”.

### Read / lookup boundaries

- `ChunkMeshSnapshot` / `ChunkMeshDependencies` dependem de `ChunkSnapshotSource`, não do container inteiro.
- `VoxelRead` e `VoxelTopologyRead` são capabilities estreitas.
- `VoxelMutationRuntime::world()` / `VoxelTopologyRuntime::world()` não existem mais.
- Generation distingue explicitamente `Resident / Archived / Absent`; `ChunkAvailability` fica local em `streaming/generation.rs` enquanto só esse consumidor precisar dos três estados.

### Residency / eviction ownership

- `ChunkResidencyState` possui `desired`, `retained`, retired queue e selection revision. Render entity lifetime não define world residency.
- Resident retention é finita em função da streaming selection + unload retention radius.
- `ChunkRenderPool` é presentation-only.
- `evict_distant_chunks` é o cross-boundary coordinator final: residency decide retirement; `ChunkEvictionRuntime` possui authoritative world/state/budget; `ChunkEvictionPresentationRuntime` possui lighting/remesh cleanup. Os dois capability sets executam no mesmo budgeted system para não abrir um frame de split-brain.
- Mesh-pressure eviction continua presentation-only e não arquiva world truth.

## Phase 3 — estado final

### Biome metadata

- `BiomeField` responde surface e volume biomes sem resident/render chunks.
- `SurfaceSiteCache` é um owner descartável separado do campo determinístico.
- clones de `BiomeField` compartilham cache para async generation, mas cache miss/eviction não altera a decisão determinística.
- `retain_around` explicita o bound espacial do surface-site cache.
- natural spawn consulta `volume_biome_region` + `volume_selection_in_region` e só cai para surface metadata quando nenhum volume biome domina.

### Structure metadata

- `StructureMetadata` é separado de `FeatureCaches` e mantém o deterministic `StructureField`.
- `StructureField` fornece surface candidate roots sem materializar chunks.
- structure reference bounds agora são precomputados no metadata para todas as referências usadas por biome structure placements, inclusive volume placements.
- `structure_placement_bounds` saiu de `FeatureCaches`; bounds de conteúdo não crescem com histórico de viagem.
- `WorldFeatureFields::clone_with_fresh_caches()` troca caches derivados e preserva o mesmo `StructureMetadata`/field storage.
- accepted candidate/forest/origin caches continuam derivados e descartáveis; não foram promovidos a world truth.

### Connector / group planning contract

- connector resolution usa seed/ids/anchors/rotation e deterministic hashing;
- connector chains possuem `MAX_CONNECTOR_CHAIN_DEPTH` explícito;
- structure-set resolution usa seed + set id + anchor e deterministic tie/attempt hashes;
- esses planners operam sobre metadata/content e não dependem de render entity ou chunk visível.

### Phase 3 exit criteria

Atendidos:

- biome e structure intent podem ser consultados sem materializar render chunks;
- spawn usa biome-volume metadata, não contexto visual de chunk;
- generation usa deterministic metadata/content + disposable feature caches;
- spatial caches possuem retention explícita; content-keyed bounds são metadata finito por conteúdo.

## Phase 4 — progresso

Meta: desired-state scheduling explícito, bounded e stale-safe sem reescrever o priority algorithm antes de provar os owners atuais.

### Boundedness audit

- `PendingChunkQueue`: não possui cap numérico próprio, mas membership é mantida como subset da seleção atual. Full rebuild reconstrói de `desired`; incremental rebuild remove `no_longer_desired`; requeue exige `keeps_loaded`.
- `ReadyChunkQueue`: tinha growth por histórico de viagem porque entries fora de residency eram apenas ignoradas no `pop`. `84955d6b6496548cd24fce37a0bed87aea368af7` adicionou retention pelo owner e `mark_selection_rebuilt()` agora garante `ready ⊆ desired ∪ retained`. CI `36502882704` success.
- generation async: máximo 8 tasks in-flight; critical generation wave usa até 4 targets e normal wave até 8; prefetch segue o mesmo target limit.
- initial mesh async: máximo 8 tasks in-flight.
- remesh async: máximo 4 terrain + 4 fluid tasks; shared `ChunkAsyncWorkLimiter` aplica limite global/adaptativo.
- loading async usa queue depth de 4 por worker via limiter.
- `ChunkTaskQueue` é deduplicada; caps pertencem aos schedulers, não ao container genérico.
- retired scan agora limita tanto inspeção quanto retorno útil por poll/frame path: `MAX_RETIRED_SCAN_STEPS_PER_POLL=16` e `MAX_RETIRED_RESULTS_BEFORE_YIELD=16`. `1442fa3350c2a7d30eb9e30db4341c1f48dace16` força yield após progresso bounded; CI `36582996658` success.

### Stale-work audit

- generation nova pertence apenas a `desired`; `retained` preserva world truth/materialization temporária, mas não autoriza novo trabalho de generation.
- `39ed517067eedf8506c8417fe4a57e032117b8a7` moveu o cancelamento de generation stale para o selection rebuild e usa membership em `desired`, não `keeps_loaded`. Em warp disjunto, jobs da seleção anterior liberam worker capacity assim que deixam o desired set. CI `36586304834` success.
- `37920ff51c4add96f08d2a471163e1cf07dbb98b` removeu o `cancel_generation_outside_desired` redundante de `collect_generated_chunks`; remoções de `desired` pertencem ao selection rebuild e pending/prefetch/results continuam revalidando `wants_generation`. CI `36587932705` success.
- initial mesh tasks são canceladas quando deixam `retains_render_mesh` e publication revalida selection + task revision + content dependencies.
- remesh publication/dispatch exigem `streaming.retains_render_mesh(coord)` além de resident/render-pool checks (`17402d31753de978ac9499ee3e85784d279976ac`).
- `c1f7b6b3fe63ed75a9abe44de7be5383a5dddabe` impede acúmulo de halo remesh para chunks realmente ausentes sem confundir ausência de world residency com ausência de render mesh. O prune local acompanha voxel/fluid edits e o retain global roda no máximo uma vez por selection revision. CI `36581123163` success.
- `1ab39af00954862af6ff5b8471790ab10f22f2a9`: `ChunkRemeshTasks` passa a possuir metadata explícita de cada request em voo (`coord + kind + meshlet mask`). Na mudança de selection revision, `process_chunk_remesh_queue` cancela tasks que saíram de `retains_render_mesh`; se o render allocation e world chunk ainda existem, re-enfileira exatamente os dirty meshlets. `cancel_coord` continua destrutivo para retirement/eviction. CI `36593145821` success.

### Disjoint warp audit

- generation tasks/prefetch reservations task-backed agora são cancelados desired-only no rebuild; unscheduled wave pending continua bounded pelo wave cap e revalida `wants_generation` antes de schedule.
- chunks já staged/settling são world truth em processo de convergência e não são descartados por mudança de seleção; publication posterior ainda passa por residency/ready guards.
- ready retained-work não compete com um warp disjunto: `pop_ready` exige estar dentro do show radius atual. Mesh task em voo só sobrevive dentro de `retains_render_mesh`/hide radius, preservando hysteresis sem publicar trabalho distante.
- remesh em voo agora segue a mesma seleção de presentation: request metadata permite cancel/requeue sem perder invalidation, eliminando a ocupação de shared async permits por até 4 terrain + 4 fluid tasks da seleção antiga durante warp disjunto.

### Prefetch promotion audit

- `GenerationWaveState::finish()` só promove reservations que ainda permanecem em `prefetch_targets`.
- selection rebuild cancela generation task-backed fora de `desired` e chama `abandon_target`, que também remove a reservation de `prefetch_targets` antes de `finish()`.
- staged/settling/publication chunks não podem ser abandonados como async work porque já pertencem a world truth em convergência; os guards posteriores de publication/residency continuam responsáveis por eles.
- não foi encontrado production hole nessa transição. Cut atual adiciona regressão focada provando que prefetch abandonado durante a wave não é promovido quando a wave termina, enquanto reservation ainda válida é promovida. CI pendente para este cut.

### Observabilidade existente

Não criar outro metrics owner sem necessidade. O diagnóstico periódico atual já cobre:

- `pending`, `ready`, `retired`, generation wave/prefetch/staged;
- generation/initial-mesh/remesh tasks e shared async limiter;
- remesh geometry/lighting/fluid queue counts;
- pending/ready priority scan count, média, máximo e maior queue observada;
- frame time percentiles + top slow-frame contexts com `selection_revision` e `warp_active`;
- main/render work percentiles.

Adicionar nova métrica somente quando um próximo cut tiver uma hipótese que os sinais acima não consigam provar/refutar.

## Checkpoints verdes mais recentes

- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation explicita `Resident / Archived / Absent`; CI `36496987975` success.
- `73383a52f8f8c9c4a4043274161dc36cb5ec1340` — eviction coordinator separa authoritative vs presentation capabilities; CI `36497504684` success.
- `499d33aa1098da8edb63c9b85f5462f545840a45` + `2c12c9e67abb8d24bb310e9654ec3f658ad6f33a` — `SurfaceSiteCache`; CI `36498438529` success.
- `d976bf536c9980a28e971ab5f5869c81b931ee3b` — `StructureMetadata` vira o contract explícito e fresh caches preservam metadata; CI `36498902285` success.
- `12963a65547ec148725a259195c3bd05d40609ac` — structure reference bounds deixam `FeatureCaches` e viram deterministic metadata; CI `36499367474` success.
- `bb2cfb1002c3dac37d139a000ec123301dffa013` — fecha Phase 3 e inicia audit da Phase 4; CI `36499528861` success.
- `84955d6b6496548cd24fce37a0bed87aea368af7` — ready queue passa a ser residency-bound; CI `36502882704` success.
- `17402d31753de978ac9499ee3e85784d279976ac` — remesh dispatch/publication rejeita presentation work fora da seleção atual; CI `36503076178` success.
- `c1f7b6b3fe63ed75a9abe44de7be5383a5dddabe` — prune local de halo remesh ausente; CI `36581123163` success.
- `1442fa3350c2a7d30eb9e30db4341c1f48dace16` — retired candidate path passa a forçar yield após progresso bounded; CI `36582996658` success.
- `39ed517067eedf8506c8417fe4a57e032117b8a7` — selection rebuild cancela generation fora de `desired`; CI `36586304834` success.
- `37920ff51c4add96f08d2a471163e1cf07dbb98b` — remove scan redundante de cancellation generation por frame; CI `36587932705` success.
- `1ab39af00954862af6ff5b8471790ab10f22f2a9` — cancela/re-enfileira remesh em voo obsoleto preservando dirty metadata; CI `36593145821` success.

## Phase 4 — próximos cortes

1. fechar o gate do teste de regressão de prefetch promotion; se verde, considerar a invariável de promotion coberta;
2. usar gameplay logs reais para validar responsividade de warp/background work e os bounds já observáveis antes de declarar Phase 4 completa;
3. só adicionar diagnóstico ou alterar priority scanning se esses logs mostrarem uma hipótese concreta não coberta pelos sinais existentes; não reescrever prioridade por estética.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.