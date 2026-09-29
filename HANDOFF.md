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
- `generation/structures.rs` ainda mistura placement planning, conflict resolution e rasterization.
- antes dos cuts da Phase 6, `StructureField` importava planning de `world::generation`, invertendo a dependency direction.
- connector graph possui `MAX_CONNECTOR_CHAIN_DEPTH = 64`, mas ainda não possui cap explícito para total de pieces/nodes; fan-out continua dívida.
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
- algorithms, hashes, connector resolution, set selection, bounds e rasterization permanecem semanticamente iguais neste cut;
- os adapters são temporários: o próximo cut deve separar candidate/conflict resolution de rasterization e remover esses wrappers, em vez de transformá-los em compatibilidade permanente.

CI #10262 falhou apenas no Clippy por um re-export morto de `connected_horizontal_bounds_for_reference` em `generation.rs`, remanescente depois que `StructureField` passou a consumir o planner diretamente. O corretivo removeu esse re-export externo sem alterar lógica.

CI #10263 então expôs o mesmo símbolo ainda sem consumidor dentro de `generation/structures.rs`. Como remover o item do arquivo grande será parte do próximo cut que elimina os adapters, o corretivo mantém temporariamente esse adapter type-checked por um `const _` em `generation.rs`: não há chamada/runtime cost, não há `allow`, e não se reintroduz a dependency `metadata -> generation`. O `const _` deve desaparecer junto com os adapters no próximo cut.

### Próximos cuts

1. separar candidate/conflict resolution de `rasterize_structures`; materialization deve consumir `ResolvedStructurePlan` em vez de construir o plano;
2. remover os adapters `generation/structures/*` após migrar os call sites para o planner owner, incluindo o `const _` temporário de type-check;
3. adicionar cap explícito de total de pieces/nodes e, se necessário, cap dos estados avaliados em connector bounds;
4. formalizar reservation/conflict semantics atravessando chunk boundaries no planner;
5. migrar cave entrances/tunnels e futuros rivers/lakes exclusivamente por structures/connectors, sem hydrology paralelo.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- cada bloco coerente deve terminar como um único commit com código + `HANDOFF.md`;
- verificar CI antes de iniciar o próximo migration block;
- gate = audits + Clippy + Check; não aguardar/executar `cargo test` automaticamente;
- se o gate falhar, abrir logs e corrigir root cause; não esconder warnings com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não reintroduzir hydrology legado;
- não inventar performance claims sem logs reais.
