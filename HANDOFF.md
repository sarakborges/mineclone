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
- **Phase 3 — Deterministic world metadata iniciando.**

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

## Checkpoints verdes mais recentes

- `59de357997883148ffcc5df581f916f4632683b7` — `ResidentChunkStore`; CI `36490280435` success.
- `3f072dd12db3f456b1060a890730f4457a7a847c` — `ObjectRevisionState`; CI `36491139143` success.
- `d240f50368cf76147638cf00039f5de8eb84eec6` — object revisions nominais internamente; CI `36491654054` success.
- `90319e4031c4db656f685667f626a7cf39a332b7` — dead mesh revision cleanup finalizado; CI `36492502160` success.
- `e50519bb6e60feb9b393368c6171bc916d1458e4` — restaura block-targeting revision após CI revelar caller; CI `36493480313` success.
- `8f729481cd5ab14521a2f4d404ca4c82daedbde1` — `BlockTopologyRevision` + `BlockRevisionState`; CI `36493814557` success.
- `a26a8304823e17acb6b798581595c3f2585b65ca` + `b753efe537b1b806159af358f76cb64cfb445177` — `ContentRevisionState`; CI `36494360805` success.
- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation explicita `Resident / Archived / Absent`; CI `36496987975` success.
- `99d53e940a6cdc2eb9b94f3b5a5ca11d7d6b125a` + `73383a52f8f8c9c4a4043274161dc36cb5ec1340` — eviction coordinator separa authoritative vs presentation capabilities e corrige naming do system; CI `36497504684` success.

## Phase 3 — próximo corte

Meta: deterministic world metadata precisa poder responder “o que pertence aqui?” sem materializar/renderizar chunk.

Primeiro mapear o que já existe antes de criar qualquer abstração:

- `BiomeField`: biome-volume metadata e caches;
- `WorldFeatureFields`: deterministic feature calculations/caches;
- `StructureMetadata` / `StructureField`: structure intent existente;
- generation snapshot e spawn consumers que já consultam esses owners.

Próximas decisões devem distinguir:

1. metadata lógico determinístico de caches derivados descartáveis;
2. queries que podem rodar sem resident/render chunk;
3. cache validity + memory bounds explícitos;
4. structure intent/reservation vs structure materialization;
5. biome-volume query como fonte para spawn rules, nunca biome visual inferido de chunk renderizado.

Não criar um `WorldMetadata` mega-container só para agrupar resources existentes. Primeiro localizar invariantes e consumers reais.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.
