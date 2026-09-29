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

### Stale-work audit

- generation tasks são canceladas no selection rebuild quando deixam `keeps_loaded`; stale input revision só é requeued se ainda pertencer à seleção/wave atual.
- initial mesh tasks são canceladas quando deixam `retains_render_mesh` e publication revalida selection + task revision + content dependencies.
- remesh tinha uma lacuna: publication/dispatch dependiam de `ChunkRenderPool` membership, então trabalho concluído podia aplicar durante o backlog de render retirement depois que a seleção já não queria mais o mesh.
- `17402d31753de978ac9499ee3e85784d279976ac` corrige isso: dispatch e publication exigem `streaming.retains_render_mesh(coord)` além de resident/render-pool checks. CI `36503076178` success.

### Próximo problema já localizado

`ChunkRemeshQueue` pode receber halo invalidations para coordenadas ausentes. Geometry/fluid/lighting scans ignoram chunks não renderizados, mas uma coord realmente sem resident chunk pode ficar na queue/mask maps até alguma remoção externa. O próximo cut deve impedir/prunar **apenas chunks ausentes**, preservando resident preload/unrendered work que ainda pode ser necessário para initial-presentation catchup.

Não podar remesh queue simplesmente por `ChunkRenderPool`: render residency e world residency são domínios diferentes.

Também auditar retired queue: candidatos não residentes devem consumir budget ou ser podados explicitamente; um backlog de selection-history não pode gerar scan loop não-orçado durante warp.

## Checkpoints verdes mais recentes

- `74c45f7aabfacbeb50b747b37d6799f168708f51` — generation explicita `Resident / Archived / Absent`; CI `36496987975` success.
- `73383a52f8f8c9c4a4043274161dc36cb5ec1340` — eviction coordinator separa authoritative vs presentation capabilities; CI `36497504684` success.
- `499d33aa1098da8edb63c9b85f5462f545840a45` + `2c12c9e67abb8d24bb310e9654ec3f658ad6f33a` — `SurfaceSiteCache`; CI `36498438529` success.
- `d976bf536c9980a28e971ab5f5869c81b931ee3b` — `StructureMetadata` vira o contract explícito e fresh caches preservam metadata; CI `36498902285` success.
- `12963a65547ec148725a259195c3bd05d40609ac` — structure reference bounds deixam `FeatureCaches` e viram deterministic metadata; CI `36499367474` success.
- `bb2cfb1002c3dac37d139a000ec123301dffa013` — fecha Phase 3 e inicia audit da Phase 4; CI `36499528861` success.
- `84955d6b6496548cd24fce37a0bed87aea368af7` — ready queue passa a ser residency-bound; CI `36502882704` success.
- `17402d31753de978ac9499ee3e85784d279976ac` — remesh dispatch/publication rejeita presentation work fora da seleção atual; CI `36503076178` success.

## Phase 4 — próximos cortes

1. impedir/prunar remesh queue entries de chunks realmente ausentes, preservando resident preload work;
2. garantir que retired candidates inválidos sejam bounded/charged pelo frame budget durante warp/selection churn;
3. adicionar observabilidade onde falta para provar queue growth bounds, sem criar metrics owners duplicados;
4. verificar disjoint warp/selection changes end-to-end para generation wave/prefetch/ready/remesh;
5. só depois considerar remoção de priority rescans redundantes; não reescrever a prioridade por estética.

## Regras de continuidade

- Trabalhar na branch `architecture/asteria-core-rebuild`, nunca direto em `develop`.
- Commitar blocos coerentes e conferir diff quando arquivos grandes forem regravados.
- Se CI falhar, abrir logs e corrigir root cause; não usar `allow` para esconder warning.
- Não reintroduzir hydrology legado.
- Não inventar performance claims sem gameplay logs reais.
- Preservar gameplay/content/UI/assets válidos enquanto a fundação é substituída por boundaries explícitos.