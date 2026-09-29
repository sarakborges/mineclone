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
- **Phase 4 — código do Streaming scheduler v2 concluído.** Bounds, cancellation e stale-result guards estão cobertos por código/testes. O critério empírico de responsividade em gameplay/warp continua pendente de logs reais e não deve ser tratado como performance comprovada.
- **Phase 5 — Terrain generation v2 em andamento.**

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

## Phase 4 — estado final do código

Meta: desired-state scheduling explícito, bounded e stale-safe sem reescrever o priority algorithm antes de provar os owners atuais.

### Boundedness audit

- `PendingChunkQueue`: não possui cap numérico próprio, mas membership é mantida como subset da seleção atual. Full rebuild reconstrói de `desired`; incremental rebuild remove `no_longer_desired`; requeue exige `keeps_loaded`.
- `ReadyChunkQueue`: tinha growth por histórico de viagem porque entries fora de residency eram apenas ignoradas no `pop`. `84955d6b6496548cd24fce37a0bed87aea368af7` adicionou retention pelo owner e `mark_selection_rebuilt()` agora garante `ready ⊆ desired ∪ retained`. CI `36502882704` success.
- generation async: máximo 8 tasks in-flight; critical generation wave usa até 4 targets e normal wave até 8; prefetch segue o mesmo target limit.
- initial mesh async: máximo 8 tasks in-flight.
- remesh async: máximo 4 terrain + 4 fluid tasks; shared `ChunkAsyncWorkLimiter` aplica limite global/adaptativo.
- loading async usa queue depth de 4 por worker via limiter.
- `ChunkTaskQueue` é deduplicada; caps pertencem aos schedulers, não ao container genérico.
- retired scan limita tanto inspeção quanto retorno útil por poll/frame path: `MAX_RETIRED_SCAN_STEPS_PER_POLL=16` e `MAX_RETIRED_RESULTS_BEFORE_YIELD=16`. `1442fa3350c2a7d30eb9e30db4341c1f48dace16` força yield após progresso bounded; CI `36582996658` success.

### Stale-work audit

- generation nova pertence apenas a `desired`; `retained` preserva world truth/materialization temporária, mas não autoriza novo trabalho de generation.
- `39ed517067eedf8506c8417fe4a57e032117b8a7` moveu cancellation generation stale para selection rebuild e usa membership em `desired`. CI `36586304834` success.
- `37920ff51c4add96f08d2a471163e1cf07dbb98b` removeu o scan redundante de cancellation generation por frame; pending/prefetch/results revalidam `wants_generation`. CI `36587932705` success.
- initial mesh tasks são canceladas quando deixam `retains_render_mesh` e publication revalida selection + task revision + content dependencies.
- remesh publication/dispatch exigem `streaming.retains_render_mesh(coord)` além de resident/render-pool checks (`17402d31753de978ac9499ee3e85784d279976ac`).
- `c1f7b6b3fe63ed75a9abe44de7be5383a5dddabe` impede acúmulo de halo remesh para chunks realmente ausentes. CI `36581123163` success.
- `1ab39af00954862af6ff5b8471790ab10f22f2a9` dá metadata explícita a requests remesh em voo e cancela/re-enfileira trabalho que sai de `retains_render_mesh`; `cancel_coord` continua destrutivo para retirement/eviction. CI `36593145821` success.

### Disjoint warp / prefetch

- generation task-backed fora de `desired` é cancelada no rebuild; unscheduled wave pending continua bounded e revalida `wants_generation`.
- chunks staged/settling já são world truth em convergência; publication posterior passa por residency/ready guards.
- ready retained-work não compete com warp disjunto porque `pop_ready` exige show radius atual.
- remesh em voo usa a seleção de presentation e não mantém permits ocupados por requests antigos após warp disjunto.
- `GenerationWaveState::finish()` só promove reservations ainda presentes em `prefetch_targets`.
- selection rebuild cancela task-backed prefetch fora de `desired`; `abandon_target` remove a reservation antes de `finish()`.
- `b1312808c3851865d5216d06e6b16a90dc1986a3` adiciona regressão garantindo que prefetch abandonado não seja promovido, enquanto reservation ainda válida é promovida. CI `36595941232` success.

### Observabilidade / dívida empírica

O diagnóstico existente cobre pending/ready/retired, wave/prefetch/staged, async task counts/shared limiter, remesh queues, priority scan count/avg/max/max queue e frame/main/render work percentiles com slow-frame context.

Não há log de gameplay desta HEAD provando ainda o último exit criterion de Phase 4 (“gameplay remains responsive while background work progresses incrementally”). Por decisão explícita de seguir para Phase 5, isso fica registrado como **runtime validation debt**, não como performance já comprovada. Não adicionar métricas nem reabrir scheduler sem hipótese sustentada por logs.

## Phase 5 — Terrain generation v2

Meta: generation deve ser um job determinístico que produz authoritative world data sem possuir scheduling, ECS publication ou render side effects.

### Audit inicial

- `generation::generate_chunk(coord, context) -> VoxelChunk` já é cálculo de world data; no caminho auditado não cria Bevy entities, meshes, UI ou render state.
- `GenerationSnapshot` captura/clona a dependency surface permitida para background generation: registries/content, dimension, world-generation settings, biome field e feature metadata/caches.
- `GenerationScheduler` despacha generation no `AsyncComputeTaskPool`; o frame path integra resultados depois de revision/selection relevance guards.
- não foi encontrado caller síncrono de generation no frame path auditado.
- não existe harness de benchmark independente (`benches/`/Criterion ausente no audit atual); o crate é hoje binary-only, então não fazer um refactor amplo para `lib.rs` apenas para satisfazer benchmark tooling.

### Job boundary

- `f602bdbd6b3c74cb77d54c291fd7b163effde6bc` introduziu `ChunkGenerationJob`, com `ChunkCoord + Arc<GenerationSnapshot>` como inputs imutáveis. CI `36598783588` success.
- `ChunkGenerationJob::run()` é o owner do cálculo `generate_chunk` para async task generation.
- `GenerationScheduler` continua proprietário apenas de snapshot revision, dedup/cap, async permit, cancellation e result revision.
- o job não recebe permit/revision e não publica no `VoxelWorld`; scheduling/publication ficam fora do cálculo.

### Cut atual — determinismo / testabilidade

- `GenerationSnapshot::from_context` passa a capturar inputs a partir de `ChunkGenerationContext`, sem exigir Bevy `SystemParam`; `capture` permanece o adapter runtime e delega ao mesmo boundary.
- regressão executa duas vezes o mesmo `ChunkGenerationJob` para o mesmo snapshot/coord usando conteúdo real do Overworld.
- a comparação usa `DiskChunk` serializado, que representa blocks/layers/objects/fluids autoritativos e deliberadamente exclui lighting/runtime caches derivados; não adicionar `PartialEq` artificial em `VoxelChunk`.
- usar o mesmo snapshot nas duas execuções também trava a invariant de que warming dos caches derivados não pode mudar o resultado autoritativo.
- CI pendente para este cut.

### Publication audit

- result integration já rejeita `TaskInputRevision` stale e revalida `wants_generation` antes de inserir world data.
- `VoxelWorld::insert_chunk` é uma operação única e recusa overwrite de chunk resident/persisted; publication do resultado calculado não acontece via mutações voxel-a-voxel.
- chunks que entram em generated-fluid settling já são world truth em convergência; se deixarem retention antes da publication final, são arquivados em vez de apresentados como ready.
- falta apenas decidir se essas invariants precisam de regressão adicional específica nesta fase; não reescrever o caminho que já satisfaz o contract sem evidência.

## Checkpoints verdes mais recentes

- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation explicita `Resident / Archived / Absent`; CI `36496987975` success.
- `73383a52f8f8c9c4a4043274161dc36cb5ec1340` — eviction coordinator separa authoritative vs presentation capabilities; CI `36497504684` success.
- `499d33aa1098da8edb63c9b85f5462f545840a45` + `2c12c9e67abb8d24bb310e9654ec3f658ad6f33a` — `SurfaceSiteCache`; CI `36498438529` success.
- `d976bf536c9980a28e971ab5f5869c81b931ee3b` — `StructureMetadata` vira contract explícito e fresh caches preservam metadata; CI `36498902285` success.
- `12963a65547ec148725a259195c3bd05d40609ac` — structure reference bounds viram deterministic metadata; CI `36499367474` success.
- `bb2cfb1002c3dac37d139a000ec123301dffa013` — fecha Phase 3 e inicia audit da Phase 4; CI `36499528861` success.
- `84955d6b6496548cd24fce37a0bed87aea368af7` — ready queue residency-bound; CI `36502882704` success.
- `17402d31753de978ac9499ee3e85784d279976ac` — remesh dispatch/publication rejeita presentation work fora da seleção; CI `36503076178` success.
- `c1f7b6b3fe63ed75a9abe44de7be5383a5dddabe` — prune local de halo remesh ausente; CI `36581123163` success.
- `1442fa3350c2a7d30eb9e30db4341c1f48dace16` — retired path força yield após progresso bounded; CI `36582996658` success.
- `39ed517067eedf8506c8417fe4a57e032117b8a7` — selection rebuild cancela generation fora de `desired`; CI `36586304834` success.
- `37920ff51c4add96f08d2a471163e1cf07dbb98b` — remove cancellation scan generation redundante; CI `36587932705` success.
- `1ab39af00954862af6ff5b8471790ab10f22f2a9` — cancela/re-enfileira remesh stale preservando dirty metadata; CI `36593145821` success.
- `b1312808c3851865d5216d06e6b16a90dc1986a3` — trava invariável de prefetch promotion sob troca de seleção; CI `36595941232` success.
- `f602bdbd6b3c74cb77d54c291fd7b163effde6bc` — separa generation calculation em `ChunkGenerationJob`; CI `36598783588` success.

## Phase 5 — próximos cortes

1. fechar CI da regressão de determinismo e do constructor de snapshot por contexto de domínio;
2. adicionar benchmark independente do `ChunkGenerationJob`, medindo somente `run()` depois do setup/warm-up e sem scheduler/render;
3. continuar auditando callers para confirmar ausência de generation síncrona fora do caminho já inspecionado;
4. adicionar teste de publication stale/atomic somente se a cobertura existente não travar a invariant de revisão/relevância de forma suficiente;
5. fechar os exit criteria da Phase 5 e só então iniciar structures/connectors da Phase 6.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.
