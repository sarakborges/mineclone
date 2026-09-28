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
- **Phase 3 — Deterministic world metadata em andamento.**

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

### Phase 2 exit criteria

Atendidos:

- chunks podem existir/mutar em testes sem render entities;
- todas as mutations de world truth passam pela boundary autoritativa `VoxelWorld`, com facades especializados apenas coordenando side effects;
- presentation pode ser removida sem remover world truth; mesh-pressure/distance render retirement são independentes de resident chunk lifetime;
- storage/revision/lookup/eviction owners estão explícitos e testados.

## Phase 3 — progresso

Meta: deterministic world metadata precisa responder “o que pertence aqui?” sem materializar/renderizar chunk.

### Biome metadata vs cache

`BiomeField` já era capaz de consultar surface/volume biome em posições arbitrárias sem resident/render chunk, mas misturava metadata determinístico com um `Arc<RwLock<HashMap<...>>>` cru de surface-site selection.

O primeiro cut da Phase 3 extraiu esse storage para `SurfaceSiteCache`:

- `BiomeField` agora contém `surface_site_cache: SurfaceSiteCache`, em vez do lock/map cru;
- clones de `BiomeField` continuam compartilhando o mesmo cache, preservando reuse entre generation snapshots;
- cache miss continua recalculando a mesma seleção determinística;
- `retain_around` possui explicitamente o bound derivado da janela de streaming;
- `spawn_oceans` e single-biome invalidam o cache através do owner;
- sampling mantém read/write batching, portanto a extração não adiciona lock por site;
- testes cobrem compartilhamento/invalidation entre clones e eviction de sites fora do bound.

`499d33aa1098da8edb63c9b85f5462f545840a45` introduziu o owner. O primeiro gate `36498239706` encontrou apenas um mismatch de visibilidade (`private_interfaces`); `2c12c9e67abb8d24bb310e9654ec3f658ad6f33a` alinhou a visibilidade sem `allow`. CI `36498438529` success.

### Spawn metadata

`src/creatures/natural_spawn.rs` já usa o metadata layer correto:

- resolve `generation_region_coord`;
- consulta `WorldFeatureFields::volume_biome_region`;
- aplica `BiomeField::volume_selection_in_region`;
- usa volume biome quando aplicável e só cai para surface metadata quando nenhum volume domina.

Não inferir biome a partir de chunk visual/renderizado e não refatorar esse path só por estética.

### Structure metadata — diagnóstico atual

- `StructureMetadata` já é separado dos disposable `FeatureCaches` e possui o deterministic `StructureField`.
- `StructureField` pode visitar candidate surface anchors/bounds sem materializar chunks.
- `generation/structures.rs` resolve/filtra accepted structure candidates deterministicamente e só depois rasteriza em `VoxelChunk`.
- O gap atual é de boundary/API: generation ainda chega ao metadata via `WorldFeatureFields::structure_field()`, fazendo metadata lógico parecer parte do cache facade.
- Não mover o resolver inteiro nem duplicar structure planning. O próximo cut deve tornar a query de `StructureMetadata` explícita, preservando exatamente os caches de accepted placement/forest onde eles são derivados e descartáveis.

## Checkpoints verdes mais recentes

- `59de357997883148ffcc5df581f916f4632683b7` — `ResidentChunkStore`; CI `36490280435` success.
- `3f072dd12db3f456b1060a890730f4457a7a847c` — `ObjectRevisionState`; CI `36491139143` success.
- `d240f50368cf76147638cf00039f5de8eb84eec6` — object revisions nominais internamente; CI `36491654054` success.
- `90319e4031c4db656f685667f626a7cf39a332b7` — dead mesh revision cleanup finalizado; CI `36492502160` success.
- `e50519bb6e60feb9b393368c6171bc916d1458e4` — restaura block-targeting revision após CI revelar caller; CI `36493480313` success.
- `8f729481cd5ab14521a2f4d404ca4c82daedbde1` — `BlockTopologyRevision` + `BlockRevisionState`; CI `36493814557` success.
- `a26a8304823e17acb6b798581595c3f2585b65ca` + `b753efe537b1b806159af358f76cb64cfb445177` — `ContentRevisionState`; CI `36494360805` success.
- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation explicita `Resident / Archived / Absent`; CI `36496987975` success.
- `99d53e940a6cdc2eb9b94f3b5a5ca11d7d6b125a` + `73383a52f8f8c9c4a4043274161dc36cb5ec1340` — eviction coordinator separa authoritative vs presentation capabilities; CI `36497504684` success.
- `499d33aa1098da8edb63c9b85f5462f545840a45` + `2c12c9e67abb8d24bb310e9654ec3f658ad6f33a` — `SurfaceSiteCache` separa biome metadata de cache descartável; CI `36498438529` success.

## Próximo corte

1. tornar `StructureMetadata` a API explícita para surface structure intent queries, sem mudar placement/conflict algorithms;
2. manter accepted-placement/forest caches em `WorldFeatureFields` como derivados descartáveis, não promovê-los a world truth;
3. adicionar teste de que fresh feature caches preservam o mesmo structure metadata/intent;
4. continuar exigindo queries independentes de resident/render chunks;
5. depois revisar volume-structure intent para garantir o mesmo contract antes de encerrar Phase 3.

Não criar um `WorldMetadata` mega-container só para agrupar resources existentes.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.
