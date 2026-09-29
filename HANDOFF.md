# HANDOFF — Asteria / Mineclone

> Handoff corrente. Histórico anterior está em `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura também estão em `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-29

- Repo: `sarakborges/mineclone`.
- Branch ativa: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline da reconstrução: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante estes cutovers internos.
- Rust + Bevy permanecem. Bevy é host/framework; Asteria possui world core, metadata/generation, streaming, simulation boundaries e voxel presentation.
- Não ressuscitar hydrology legado. Rivers/lakes/cave entrances futuros pertencem a structures/connectors/structure groups + deterministic metadata. Dynamic fluids continuam separados.
- Old saves/legacy compatibility não são prioridade.
- CI deve ficar verde entre migration blocks. Não avançar sobre Clippy/test/check vermelho ou em andamento.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes da reconstrução

- authoritative world != resident world != rendered presentation;
- scheduling não possui generation calculation;
- jobs async usam inputs imutáveis e resultados stale são descartáveis;
- cancellation é caminho normal;
- caches/queues precisam de owner e bound explícitos;
- trabalho frame-sensitive precisa ser incremental/budgetado;
- publication revalida revision + relevância/residência;
- nenhum subsystem deve esconder full scan/mutation cross-layer em um path aparentemente barato.

## Fases

- **Phase 1 concluída:** core types/boundaries.
- **Phase 2 concluída:** authoritative storage, revisions, lookup e eviction ownership.
- **Phase 3 concluída:** biome/structure metadata independente de render/materialization; caches derivados bounded.
- **Phase 4:** correctness do Streaming scheduler v2 concluído; responsividade empírica ainda aberta enquanto removemos stalls reais de main thread.
- **Phase 5:** Terrain generation v2 em andamento; boundary/determinismo/benchmark prontos, publication/caller audit ainda precisa fechamento após Phase 4 empírica.

## Phase 2 — contracts que não devem regredir

### Storage / revisions

`VoxelWorld` coordena owners separados:

- `ResidentChunkStore`: resident chunks + vertical column index;
- `ChunkPersistenceState`: persistent set + archived payloads;
- `ContentRevisionState`: `ChunkContentRevision`;
- `ObjectRevisionState`: scene/per-chunk object revisions;
- `BlockRevisionState`: `BlockTopologyRevision`.

`ChunkContentRevision` cobre conteúdo voxel resident; lighting é derivado. `BlockTopologyRevision` existe porque targeting depende de mudanças topológicas. Não repetir a remoção equivocada da antiga block revision (`7e97ec39`); `e50519bb`/`8f729481` corrigiram esse contract.

### Mutation / presentation

Gameplay usa `VoxelMutationRuntime` para mutation autoritativa + side effects de lighting/remesh/fluid. Bypasses intencionais continuam limitados aos owners especializados (`world_objects`, lighting propagation, fluid simulation/generated settling).

`ChunkRenderPool` é presentation-only. `ChunkResidencyState` possui desired/retained/retired e selection revision. `evict_distant_chunks` é o coordinator de cutover resident -> archived/absent e faz teardown de presentation no mesmo path budgetado.

## Phase 3 — contracts que não devem regredir

- `BiomeField` consulta surface/volume biomes sem resident/render chunks.
- `SurfaceSiteCache` é derivado, descartável e spatially bounded.
- `StructureMetadata` mantém deterministic `StructureField` separado de feature caches.
- structure reference bounds são metadata precomputado por conteúdo.
- `WorldFeatureFields::clone_with_fresh_caches()` preserva metadata e troca apenas caches derivados.
- natural spawn consulta volume-biome metadata antes de surface fallback.
- connector/group planning usa seed/ids/anchors/rotation, deterministic hashing e depth bound explícito.

## Phase 4 — Streaming scheduler v2

### Correctness / boundedness já fechados

- `PendingChunkQueue` é mantida subset da seleção atual; requeue exige residency relevante.
- `ReadyChunkQueue` é residency-bound (`84955d6b6496548cd24fce37a0bed87aea368af7`, CI `36502882704`).
- generation async: máximo 8 tasks; wave/prefetch bounded.
- initial mesh async: máximo 8 tasks.
- remesh async: máximo 4 terrain + 4 fluid; shared `ChunkAsyncWorkLimiter` aplica limite global/adaptativo.
- retired scan limita inspeção e retorno útil a 16 por poll (`1442fa3350c2a7d30eb9e30db4341c1f48dace16`, CI `36582996658`).
- generation fora de `desired` é cancelada no selection rebuild (`39ed517067eedf8506c8417fe4a57e032117b8a7`); scan redundante por frame removido em `37920ff51c4add96f08d2a471163e1cf07dbb98b`.
- remesh dispatch/publication exige `retains_render_mesh`; stale in-flight requests preservam dirty metadata ao cancelar/re-enfileirar (`1ab39af00954862af6ff5b8471790ab10f22f2a9`, CI `36593145821`).
- prefetch abandonado não é promovido após troca de seleção (`b1312808c3851865d5216d06e6b16a90dc1986a3`, CI `36595941232`).

### Gameplay log 1 — selection rebuild

`2026-09-29_17-02-50-263036500.txt` mostrou steady state ~53–58 FPS, mas stalls de 117/230/488 ms praticamente inteiros em `main_work`.

Causa encontrada: movimento adjacente fazia full rebuild da seleção ao mudar direção ou cruzar generation-region boundary. `e78d4808eb55573ab72f0d0bd3e4687472b2e3b3` mantém esses movimentos no delta path; CI `36604894331` success.

### Gameplay log 2 — remesh residency scan

`2026-09-29_17-31-55-122428800.txt` não mostrou mais `slow streaming selection rebuild`, confirmando o primeiro fix. Ainda houve stalls ~103/343/423/128 ms, com remesh backlog em milhares e priority scans abaixo de ~4,5 ms.

Causa encontrada: `process_chunk_remesh_queue` chamava `ChunkRemeshQueue::retain_resident` em toda `selection_revision`, percorrendo/sort/dedup geometry/fluid/lighting fora do budget. Selection change não é residency change; eviction/retirement já removem as entries no owner correto.

`3cacbd6aca82d36448d3130f8cd983b1fe976553` remove o full scan do churn normal e mantém apenas reconciliação defensiva em revision regression. CI `36607710543` / #10244 success.

### Gameplay log 3 — lighting work materializado durante unload

`2026-09-29_17-55-12-178083400.txt`, já com `3cacbd6`, melhorou o cenário, mas ainda não fecha Phase 4:

- janela final: ~53.1 FPS médio, p95 ~21.4 ms, p99 ~24.8 ms;
- stall máximo: 235.464 ms, com `main_work_max_us=234758`;
- outro stall: 109.656 ms;
- no pior frame: `retired=5917`, `remesh_geometry=1331`, `selection_revision=31`;
- `pending_priority_max_us=2304` e `ready_priority_max_us=141`, insuficientes para explicar 235 ms;
- startup teve stall de render separado: frame 180 ms com main work ~26.7 ms, portanto não confundir com o stall de viagem.

Audit do unload encontrou um overrun estrutural do budget:

1. `evict_distant_chunks` tem budget de 4 ms e checa o deadline entre chunks;
2. ao arquivar um chunk não-vazio, chama `PendingLightingUpdates::enqueue_loaded_column_below`;
3. para cada chunk carregado abaixo, esse método chamava `LightingQueue::enqueue_chunk_voxels`;
4. `enqueue_chunk_voxels` materializava imediatamente os 4096 voxels de um chunk 16³ em `VoxelUpdateQueue`;
5. logo um único item do unload podia fazer dezenas de milhares de hash lookups/inserts antes da próxima checagem do budget.

### Cut atual — lazy full-chunk lighting scan

`LightingQueue::enqueue_chunk_voxels` passa a registrar um full-chunk scan virtual em vez de inserir 4096 posições imediatamente.

- o scan guarda apenas o `origin` do chunk até ser consumido;
- cada `pop` produz um voxel na mesma ordem y/z/x do eager path;
- `relax_budgeted` continua sendo o consumidor e já checa budget a cada 64 voxels, então o custo pesado migra para dentro do budget existente;
- interactive e settling continuam preemptando background;
- a fronteira FIFO com background já existente é preservada por um watermark: trabalho anterior ao request fica antes do scan; trabalho adicionado depois fica atrás;
- requests duplicados antes de iniciar o scan coalescem;
- um novo request do mesmo chunk enquanto o scan está ativo agenda uma segunda passagem, evitando perder uma invalidação ocorrida no meio da primeira;
- quando o scan virtual alcança uma posição explicitamente queued depois dele, remove a duplicata do background queue;
- boundary-voxel/boundary-neighbor enqueue continua eager por enquanto; não ampliar o cut sem log que mostre necessidade.

Regressões unitárias do cut cobrem: full 4096-voxel traversal lazy, ordem contra background preexistente, dedup antes do scan e requeue durante scan ativo.

**Phase 4 continua empiricamente aberta até gameplay log da HEAD com este cut.** Se stalls severos permanecerem, investigar o próximo custo síncrono demonstrado pelo log; não voltar a otimizar selection/cache/remesh por hipótese.

## Phase 5 — Terrain generation v2

Meta: generation é job determinístico de authoritative world data, sem scheduling/ECS/render publication.

### Concluído

- `f602bdbd6b3c74cb77d54c291fd7b163effde6bc`: `ChunkGenerationJob` recebe `ChunkCoord + Arc<GenerationSnapshot>`; CI `36598783588` success.
- scheduler possui revision/dedup/cap/permit/cancellation; job possui somente calculation.
- `77cba7c7bcb8fe49aadd0f64a16d657277030c73`: `GenerationSnapshot::from_context` + regressão de determinismo autoritativo usando `DiskChunk`; CI `36600832740` success.
- caches derivados podem aquecer sem alterar o resultado autoritativo.
- `d6a9f7bdbc0822d956eb9195dd8d9522088fc6ef`: benchmark manual/ignored só de `ChunkGenerationJob::run`; CI `36602768937` success.
- benchmark: `cargo test --release --locked benchmark_chunk_generation_job -- --ignored --nocapture --test-threads=1`.
- publication audit já confirmou guards de `TaskInputRevision` + `wants_generation` antes de `VoxelWorld::insert_chunk`; insert é atômico no nível de chunk e recusa overwrite resident/persisted.
- não foi encontrado caller síncrono de generation no frame path já auditado.

### Depois de fechar Phase 4 empírica

1. concluir caller audit de generation;
2. decidir se coverage atual já trava publication stale/atomic ou adicionar regressão específica;
3. fechar exit criteria da Phase 5;
4. iniciar Phase 6 structures/connectors sem reintroduzir sistemas legados.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- cada bloco coerente deve atualizar este `HANDOFF.md` no mesmo commit;
- conferir diff de arquivos grandes regravados;
- verificar CI antes do próximo migration block;
- se CI falhar, abrir logs e corrigir root cause; não esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não reintroduzir hydrology legado;
- não inventar performance claims sem logs reais.
