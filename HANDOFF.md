# HANDOFF — Asteria / Mineclone

> Handoff corrente. Histórico anterior preservado em `HANDOFF_ARCHIVE_2026-09-25.md`. Para continuidade arquitetural, leia também `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-28

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
- **Phase 4 — Streaming scheduler v2 iniciando.**

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

Bypasses diretos inspecionados e considerados intencionais:

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

## Checkpoints verdes mais recentes

- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation explicita `Resident / Archived / Absent`; CI `36496987975` success.
- `73383a52f8f8c9c4a4043274161dc36cb5ec1340` — eviction coordinator separa authoritative vs presentation capabilities; CI `36497504684` success.
- `499d33aa1098da8edb63c9b85f5462f545840a45` + `2c12c9e67abb8d24bb310e9654ec3f658ad6f33a` — `SurfaceSiteCache`; CI `36498438529` success.
- `d976bf536c9980a28e971ab5f5869c81b931ee3b` — `StructureMetadata` vira o contract explícito e fresh caches preservam metadata; CI `36498902285` success.
- `12963a65547ec148725a259195c3bd05d40609ac` — structure reference bounds deixam `FeatureCaches` e viram deterministic metadata; CI `36499367474` success.

## Phase 4 — próximo corte

Meta: desired-state scheduling explícito, bounded e stale-safe.

Antes de criar scheduler novo, auditar o que já existe após os splits da Phase 1/2:

- `ChunkResidencyState`: desired/retained/retired;
- pending generation priority queue/cache;
- `GenerationWaveState`: active targets/prefetch/staged/publication;
- ready/initial-presentation queues;
- generation/mesh/remesh task schedulers e `TaskInputRevision`;
- shared `ChunkAsyncWorkLimiter`;
- frame-work budgets e priority diagnostics.

Próximas decisões:

1. provar quais queues são bounded por residency vs quais precisam cap/backpressure próprio;
2. verificar warp/selection changes e stale-task rejection end-to-end;
3. remover rescans/cache invalidations redundantes se diagnostics mostrarem owner duplicado;
4. não reescrever priority algorithm enquanto ownership/backpressure estiver correto;
5. fechar Phase 4 apenas quando obsolete work não puder publicar sobre seleção atual e queue growth estiver explicitamente bounded/observable.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.
