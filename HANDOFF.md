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

- **Phase 1 concluída.** Core types/boundaries necessários para a reconstrução já existem.
- **Phase 2 — Authoritative chunk/world storage em andamento.**

## Ownership já separado

### Streaming / async / presentation

O antigo `ChunkStreamingState` já perdeu ownership direto de:

- logical residency (`desired` / `retained` / retirement);
- pending generation queue + priority caches;
- ready/initial-presentation queue;
- generation-wave lifecycle (targets, prefetch, staged, settling, publication);
- initial presentation state;
- mesh-pressure residency;
- selection pose / selection caches;
- priority diagnostics.

Generation e presentation possuem snapshot boundaries explícitos. Async work usa revisions tipadas e stale-result rejection.

### Core identities / revisions

Existem e devem ser preservados como domínios distintos:

- `DimensionId`
- `WorldId`
- `ChunkCoord`
- `VoxelCoord`
- `GenerationRegionCoord`
- `TaskInputRevision`
- `ResidencySelectionRevision`
- `ChunkContentRevision`
- `BlockTopologyRevision`

Não colapsar revisions semanticamente diferentes em `u64` genérico só porque hoje ambos são counters.

### Authoritative world storage

`VoxelWorld` continua como facade/coordenador, mas não possui mais diretamente vários containers:

- `ResidentChunkStore`: resident chunks + vertical column index.
- `LoadedChunkColumnIndex`: query/index de Y por coluna, encapsulado no resident store.
- `ChunkPersistenceState`: persistent set + archived payloads.
- `ContentRevisionState`: map/counter de `ChunkContentRevision` por resident chunk.
- `ObjectRevisionState`: scene revision + per-chunk object revision.
- `BlockRevisionState`: global block-topology revision.

`VoxelWorld` continua coordenando invariantes multi-owner: insert/archive/restore, persistence promotion, semantic revisions e mutations autoritativas.

## Read / mutation boundaries

- `ChunkMeshSnapshot` / `ChunkMeshDependencies` dependem de `ChunkSnapshotSource`, não do container inteiro.
- `VoxelRead` cobre leitura voxel estreita.
- `VoxelTopologyRead` adiciona object lookup.
- `VoxelTopologyReader` é o view entregue por `VoxelTopologyRuntime::read()`.
- Os antigos `VoxelMutationRuntime::world()` / `VoxelTopologyRuntime::world()` foram removidos.
- Collision, raycast, targeting/tool reads etc. não devem recuperar acesso irrestrito ao `VoxelWorld` através do facade de mutation.

Gameplay/interação usa `VoxelMutationRuntime` para combinar mutation autoritativa com side effects de simulation/presentation:

- block edit -> lighting + remesh + fluid scheduling;
- layer edit -> remesh;
- interactive fluid edit -> lighting + remesh + fluid scheduling.

Bypasses diretos já inspecionados e considerados **intencionais**:

- `world_objects.rs`: `set_object_at` / `remove_object_at`; object presentation é separada e invalidada por `ObjectRevisionState`.
- `voxel/lighting/propagation.rs`: escreve light diretamente; publication real vem de changed positions -> meshlet masks -> `ChunkRemeshTasks`/`ChunkRemeshQueue`.
- `world/fluid_updates.rs`: runtime de fluid simulation escreve `set_fluid_at` diretamente e possui seu próprio lighting/remesh/reschedule pipeline.
- generated-fluid settling usa caminho derivado próprio e reconcilia lighting/remesh/frontier na publication; não deve ser forçado pelo gameplay facade.

Não criar um `DirtyState` genérico para fundir esses domínios.

## Revision / dirty ownership atual

### `ChunkContentRevision`

- Representa mudança de conteúdo voxel de um resident chunk relevante para immutable snapshots/dependencies.
- Lighting **não** muda content revision.
- Blocks, layers, objects e fluids mudam content revision.
- Map/counter agora pertencem a `ContentRevisionState`; `VoxelWorld` apenas valida resident ownership e delega bump/remove/read.

### `BlockTopologyRevision`

O antigo `block_content_revision: u64` **não era estado morto**.

Durante um cut experimental (`7e97ec39`) ele foi removido; CI falhou porque `src/targeting/scene.rs` realmente o usa para invalidar o `BlockTargetingVisualSnapshot`. O estado funcional foi restaurado em `e50519bb`, CI `36493480313` success.

O domínio foi então corrigido semanticamente:

- tipo nominal: `BlockTopologyRevision`;
- owner: `BlockRevisionState`;
- targeting armazena o tipo nominal, não `u64`;
- avança em insert/archive/restore de chunk e mudança real de block;
- **não** avança por layer/fluid/object/light.

Commit: `8f729481cd5ab14521a2f4d404ca4c82daedbde1`; CI `36493814557` success.

### Object revisions

- `ObjectRevisionState` possui scene invalidation + per-chunk object revision.
- Scene e chunk revisions são tipos internos distintos; adapter `u64` externo ainda existe em `world_objects.rs` e pode ser removido em cut posterior se trouxer valor real.

### Mesh revisions

O antigo `chunk_mesh_revisions` foi confirmado como estado legado e removido.

- não existia reader de produção;
- presentation dirtiness real já pertence a `ChunkRemeshTasks` / `ChunkRemeshQueue`;
- lighting invalida meshlets diretamente;
- interactive lighting publica changed positions por frame;
- generated-fluid settling mantém mudanças privadas até convergir e só depois publica.

`66511702` removeu o estado; `90319e40` corrigiu um teste residual; CI `36492502160` success.

## Storage semantics preservadas

- Chunk gerado e não modificado pode ser descartado ao archive.
- Chunk promovido a persistent é arquivado/restaurável.
- Save serializa somente chunks persistent.
- Generated-fluid convergence não pode promover chunk para persistent pelo caminho derivado.
- `ResidentChunkStore::insert` não substitui resident chunk silenciosamente.
- Archive remove content/object revisions do resident chunk.
- Restore recebe revisions novas.

Generation/streaming agora torna explícita a distinção que antes era inferida por duas queries:

- `Resident`
- `Archived` / known-but-not-resident
- `Absent`

Por enquanto `ChunkAvailability` fica local em `streaming/generation.rs`, porque esse é o único consumidor conhecido que precisa dos três estados. Não promover para um enum global sem segundo consumidor real.

## Últimos checkpoints relevantes

- `bb2afe5e8d4fc17a423b9d8545d74de48b10ba05` — fecha full-world mutation escape hatch; CI `36485808944` success.
- `576b770822ee8919b6cd7a1864308e159d4d3547` — content revision nominal no storage; CI `36488836153` success.
- `fc011ac18d3f0de80fdd2551cf0ff6430377b99b` — `ChunkPersistenceState`; CI `36489367228` success.
- `ba0d53326c2beb1d4203d82f57443e9da7a543f4` — loaded column index; CI `36489860888` success.
- `59de357997883148ffcc5df581f916f4632683b7` — `ResidentChunkStore`; CI `36490280435` success.
- `3f072dd12db3f456b1060a890730f4457a7a847c` — `ObjectRevisionState`; CI `36491139143` success.
- `d240f50368cf76147638cf00039f5de8eb84eec6` — object revisions nominal internally; CI `36491654054` success.
- `90319e4031c4db656f685667f626a7cf39a332b7` — dead mesh revision cleanup finalizado; CI `36492502160` success.
- `e50519bb6e60feb9b393368c6171bc916d1458e4` — restaura a real block-targeting revision após CI revelar o caller; CI `36493480313` success.
- `8f729481cd5ab14521a2f4d404ca4c82daedbde1` — `BlockTopologyRevision` + `BlockRevisionState`; CI `36493814557` success.
- `a26a8304823e17acb6b798581595c3f2585b65ca` + `b753efe537b1b806159af358f76cb64cfb445177` — `ContentRevisionState` passa a possuir map/counter de content revisions; CI `36494360805` success.
- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation distingue `Resident / Archived / Absent` explicitamente e testa a transição de storage; CI `36496987975` success.

## Próximo corte — Phase 2

Lookup semantics e os mutation bypasses relevantes já foram classificados. O próximo bloco deve revisar **cache/eviction ownership** sem acoplar logical/resident world a render entities.

Prioridades:

1. localizar caches autoritativos/derived ainda sem owner/bound explícito;
2. separar cache eviction de render entity lifetime quando ainda estiver misturado;
3. preservar `VoxelWorld` como facade/coordenador e evitar megadiff;
4. não criar abstração nova sem consumidor/invariante real;
5. encerrar Phase 2 somente quando storage, revisions, lookup semantics e eviction ownership estiverem claros e testados.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.
