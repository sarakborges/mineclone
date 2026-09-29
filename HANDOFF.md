# HANDOFF — Asteria / Mineclone

> Handoff corrente. Histórico anterior está em `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura em `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-29

- Repo: `sarakborges/mineclone`.
- Branch: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante estes cutovers internos.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar. Rivers/lakes/cave entrances futuros pertencem a structures/connectors/structure groups + deterministic metadata. Dynamic fluids continuam separados.
- Old saves/legacy compatibility não são prioridade.
- CI deste branch = audits + Clippy + Check. `cargo test` não roda automaticamente; testes só quando explicitamente solicitados.
- Não avançar com audits/Clippy/Check vermelhos ou em andamento.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues têm owner e bound explícitos;
- trabalho frame-sensitive é incremental/budgetado;
- cancellation é normal;
- generation não possui side effects de render/UI/ECS;
- não esconder full scan/mutation cross-layer em path aparentemente barato;
- cada migration block termina com HANDOFF + CI verde.

## Fases

- Phase 1 concluída: core types/boundaries.
- Phase 2 concluída: authoritative storage, revisions, lookup e eviction ownership.
- Phase 3 concluída: biome/structure metadata independente de render/materialization; caches derivados bounded.
- Phase 4 concluída: Streaming scheduler v2 correctness + boundedness + responsiveness validados.
- Phase 5 concluída: Terrain generation v2 isolada em jobs determinísticos e stale-safe.
- Phase 6 em andamento: structures/connectors/feature planning.

## Contracts que não devem regredir

### Authoritative storage

`VoxelWorld` coordena owners separados para resident chunks, persistence/archive, content revisions, object revisions e block topology revisions. Gameplay mutations passam por `VoxelMutationRuntime`; presentation é derivada e descartável. `ChunkRenderPool` nunca é world truth.

### Deterministic metadata

- `BiomeField` consulta surface/volume biomes sem resident/render chunks.
- `SurfaceSiteCache` é derivado, descartável e spatially bounded.
- `StructureMetadata` possui deterministic `StructureField`, separado de caches derivados.
- structure reference bounds são metadata precomputado por conteúdo.
- `WorldFeatureFields::clone_with_fresh_caches()` preserva metadata e troca somente caches.
- natural spawn consulta volume-biome metadata antes de surface fallback.

## Phase 4 — Streaming scheduler v2

Principais cuts fechados:

- ready/pending queues residency-bound;
- generation/remesh async com caps e stale/relevance checks;
- selection delta incremental em movimento adjacente (`e78d4808...`);
- remoção de remesh full scan em selection churn (`3cacbd6a...`);
- full-chunk lighting scan lazy (`ba7ddfdf...`);
- unload boundary lighting scan lazy (`a57b3fea...`);
- lint fix Rust 1.98 (`0346ae55...`).

Gameplay final `2026-09-29_18-51-38-488208300.txt`: janela final 58.7 FPS médio, p95 18.947 ms, p99 20.856 ms e `main_work_max_us=11149` mesmo com backlog pesado. Os stalls anteriores de 100–500 ms de main_work não retornaram.

Há spike separado de startup/render (~217 ms com ~188.7 ms em render work), dívida de presentation/rendering, não reabre Phase 4.

**Phase 4 concluída.**

## Phase 5 — Terrain generation v2

- `ChunkGenerationJob` recebe `ChunkCoord + Arc<GenerationSnapshot>` e retorna `VoxelChunk` (`f602bdbd...`).
- scheduler possui revision/dedup/cap/permit/cancellation; job possui somente calculation.
- determinism regression usa output autoritativo canonicalizado (`77cba7c7...`).
- benchmark manual/ignored de `ChunkGenerationJob::run` (`d6a9f7bd...`).
- runtime/loading usam `GenerationScheduler -> ChunkGenerationJob -> generate_chunk`.
- publication rejeita task revision stale e coord fora de `desired` antes do insert.
- resident não é sobrescrito; archived restaura persistência; somente absent recebe generation output.

**Phase 5 concluída.**

### Política de CI

O step automático `cargo test --locked` foi removido em `7b02f1b0ac1adb7efe3fea29f2e8e609bfd736d9`. CI #10254 confirmou audits + Clippy + Check sem Test. `cargo test` permanece somente manual/explícito.

## Phase 6 — Structures, connectors and feature planning

Meta: generalized structures são o único mecanismo authored para world features long-form; planning/reservation não pertence ao cache nem à presentation.

### Audit inicial

- `StructureField` já enumera roots deterministicamente sem materializar chunks.
- connector/group resolution já é deterministic por seed/id/anchor/rotation e suporta target de structure/group.
- `generation/structures.rs` ainda mistura coleta terrain-dependent com materialization; deterministic placement resolution está sendo removido desse owner.
- antes dos cuts da Phase 6, `StructureField` importava planning de `world::generation`, invertendo a dependency direction.
- connector graph possuía somente `MAX_CONNECTOR_CHAIN_DEPTH = 64`; total de pieces/nodes não era bounded explicitamente.
- resolved placement plans são valores de domínio; caches podem armazená-los, mas não são owner da decisão.

### Cut 1 — resolved plan ownership

Commit `3434909e3a730b8a4108a190c19a35ffb76011b5` (`Define resolved structure plan ownership`), CI #10259 success.

- `ResolvedStructurePlanPiece` representa `structure_id + rotation + anchor + origin_y`.
- `ResolvedStructurePlan` contém pieces e bounds horizontal/vertical do placement inteiro.
- os tipos vivem em `structure_metadata.rs`, não em `WorldFeatureFields`.
- `WorldFeatureFields` mantém aliases temporários `CachedStructureForest*` durante migration de call sites.
- hashing/selection/conflict/rasterization não mudaram nesse cut.

### Cut 2 — planning sai de generation

Commit `a32795957d707e08042d891b80e0cc2c606ff91d` (`Move structure planning out of generation`).

- planning puro passa para `src/world/structure_metadata/planning/`;
- owner inclui `connectors`, `placement`, `set`, `hash` e `geometry`;
- `StructureField` passa a importar `connected_horizontal_bounds_for_reference` e `structure_candidate_anchor` diretamente de `structure_metadata::planning`, removendo a dependency `metadata -> generation`;
- os paths antigos em `generation/structures/{connectors,placement,set,hash,geometry}.rs` tornam-se adapters mínimos de re-export para manter `generation/structures.rs` estável neste migration block;
- algorithms, hashes, connector resolution, set selection, bounds e rasterization permanecem semanticamente iguais nesse cut.

CI #10262 falhou apenas no Clippy por um re-export morto em `generation.rs`; `5b00587b47d922aee09fbd306c96b566ff4df06c` removeu esse re-export. CI #10263 expôs o mesmo símbolo ainda sem consumidor dentro de `generation/structures.rs`; `c6954f0efbfd4eac08d350af8ad0c0fd764efa22` manteve temporariamente esse adapter type-checked por um `const _` sem runtime cost. CI #10268 passou audits + Clippy + Check.

### Cut 3 — materialization consome placements resolvidos

Commit `df52ad68bda7ad07774ba61ac6c8b05ec4699a69` (`Make structure materialization consume resolved plans`), CI #10275 success.

- `ResolvedStructurePlacement` é o resultado aceito após deterministic conflict resolution: mantém `placement_id + placement_anchor + placement_y + ResolvedStructurePlan`;
- biome identity, priority, `reserve_space` e conflict groups permanecem exclusivamente no resolver; não vazam para materialization;
- o cache horizontal deixa de armazenar uma lista flat por piece e passa a armazenar `Vec<ResolvedStructurePlacement>`;
- o filtro final por piece/chunk saiu do resolver: o placement completo permanece no cache quando seus bounds cruzam o chunk, e cada consumidor filtra somente as pieces que realmente intersectam o chunk atual;
- `rasterize_structures`, `maximum_potential_structure_top_y_for_chunk` e `located_structure_origins_in_chunk` consomem `placement.plan.pieces`;
- raster voxel, hashing, structure selection, connector expansion, terrain fitting e conflict policy não foram alterados nesse cut.

### Cut 4 — connector expansion bounded por total de pieces

Commit `972db610bb50566b15504b665331da5ad6ddd9ee` (`Bound connector expansion by piece budget`), CI #10276 success.

- `MAX_RESOLVED_CONNECTOR_PIECES = 256` limita explicitamente o forest inteiro, contando roots e filhos conectados;
- roots acima do budget são truncadas deterministicamente na ordem de entrada; o budget restante é compartilhado entre todas as branches do forest, então fan-out não multiplica o limite por root;
- cada child aceito consome exatamente uma unidade antes de poder entrar na pending queue; quando o budget chega a zero, nenhuma seleção, ground-fit ou overlap adicional é executado;
- `MAX_CONNECTOR_CHAIN_DEPTH = 64` continua como bound ortogonal de profundidade;
- hashing, ordem de connector traversal, group selection, collision rejection e strength semantics permanecem iguais enquanto há budget;
- `connected_horizontal_bounds_for_reference` permanece conservador e depth-bounded; qualquer futuro cap de estados do cálculo de bounds precisa preservar conservadorismo para não criar clipping falso.

### Cut 5 — conflict resolution pertence ao planner

Commit `70c04704383f789eff935e0525337be1a71302fd` (`Move structure conflict resolution into planner`).

- novo `structure_metadata/planning/resolver.rs` possui `StructureCandidate`, identity/dedup, priority ordering, conflict/reservation checks e agrupamento final em `ResolvedStructurePlacement`;
- `generation/structures.rs` não possui mais `Ordering`, `HashMap`, `HashSet`, `candidate_identity`, `candidate_order`, `candidates_conflict` ou grouping de accepted pieces;
- generation fornece apenas um collector callback `(bounds -> candidates)` porque biome sampling, ground-fit, volume restrictions e caches terrain-dependent ainda pertencem ao generation input adapter;
- o planner chama esse collector tanto para o target inicial quanto para competitors que podem cruzar os bounds do placement, preservando exatamente a semântica cross-chunk anterior;
- a saída continua sendo `Vec<ResolvedStructurePlacement>`; materialization e cache não mudam neste cut;
- raster voxel, connector resolution, hashing, terrain fitting, volume eligibility e conflict policy permanecem semanticamente iguais;
- a dependency continua inward: `planning::resolver` não conhece `ChunkGenerationContext`, terrain, render, ECS ou cache owner.

CI #10277 passou todos os audits e falhou apenas no Clippy `needless_lifetimes` da assinatura do adapter `resolve_structure_placements_uncached`. Commit `127c76c3f64aead39381745abd8645f2ba86fc48` (`Fix planner resolver Clippy gate`) removeu somente o lifetime explícito redundante; CI #10278 passou audits + Clippy + Check.

### Cut 6 — adapters de planning removidos de generation

Commit `fda250504219b413e27ac2e472d4615e9dafbeb6` (`Remove generation planning adapters`).

- `generation/structures.rs` importa `connectors`, `geometry`, `hash`, `placement` e `set` diretamente de `structure_metadata::planning`;
- os cinco arquivos `generation/structures/{connectors,geometry,hash,placement,set}.rs`, que continham apenas re-exports de uma linha, foram removidos;
- `generation.rs` mantém seus re-exports públicos atuais de APIs de structures para não misturar cleanup de ownership com migração de consumidores externos neste cut;
- `restrictions.rs` e `support.rs` permanecem em generation porque ainda consomem terrain/`ChunkGenerationContext` e não são planning puro;
- nenhum hashing, connector traversal, candidate selection, conflict policy, ground-fit, rasterização ou materialization mudou neste cut.

CI #10279 passou os audits e falhou somente no Clippy porque `connected_horizontal_bounds_for_reference`, agora importado diretamente do planner por `generation/structures.rs`, ainda é um re-export sem consumidor runtime. O corretivo mantém temporariamente um `const _` de type-check em `generation.rs`; ele não executa trabalho e será removido quando os re-exports públicos restantes de structures forem auditados/migrados. Os cinco arquivos-adapter continuam removidos.

### Próximos cuts

1. formalizar reservation/conflict semantics atravessando chunk boundaries no planner e adicionar cobertura específica de boundary/priority sem ativar testes automáticos no CI;
2. avaliar cap explícito de estados em connector bounds somente com fallback conservador; não trocar boundedness por under-bounds/clipping;
3. auditar os re-exports públicos restantes de `generation` e eliminar também o `const _` temporário sem reintroduzir dependency inversion;
4. migrar cave entrances/tunnels e futuros rivers/lakes exclusivamente por structures/connectors, sem hydrology paralelo.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- cada bloco coerente deve terminar como um único commit com código + `HANDOFF.md`;
- commits já publicados nesse branch não devem ser reescritos/force-pushed; montar cuts em branch temporária e publicar somente por fast-forward quando necessário;
- verificar CI antes de iniciar o próximo migration block;
- gate = audits + Clippy + Check; não aguardar/executar `cargo test` automaticamente;
- se o gate falhar, abrir logs e corrigir root cause; não esconder warnings com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não reintroduzir hydrology legado;
- não inventar performance claims sem logs reais.
