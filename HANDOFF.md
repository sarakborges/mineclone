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
- Phase 6 concluída: structures/connectors/feature planning com ownership, boundedness e authored feature path auditados.
- Phase 7 em andamento: voxel presentation / meshing v2.

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
- hashing, ordem de connector traversal, group selection, collision rejection e strength semantics permanecem iguais enquanto há budget.

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

CI #10279 passou os audits e falhou somente no Clippy porque `connected_horizontal_bounds_for_reference`, agora importado diretamente do planner por `generation/structures.rs`, ainda é um re-export sem consumidor runtime. Commit `b7c3a68fd25c0967927e2b443c713df9e898e860` (`Fix planning adapter cleanup Clippy gate`) manteve temporariamente um `const _` de type-check em `generation.rs`; ele não executa trabalho. CI #10280 passou audits + Clippy + Check. Os cinco arquivos-adapter continuam removidos.

### Cut 7 — conflict/reservation contract cross-chunk

Commit `5f400382132001e162225f1c365ba95f8fcaded3` (`Formalize cross-chunk structure conflict contract`), CI #10281 success.

- `planning/resolver.rs` documenta explicitamente que conflict resolution é **intent-based**, não acceptance-based: um target candidate perde para qualquer intent conflitante que o outranque, mesmo se esse intent estiver fora do target e não for materializado por aquela consulta;
- o collector contract exige retornar todos os candidates cujos bounds completos intersectem o retângulo pedido; o resolver expande a consulta pelos bounds completos de cada direct candidate para incluir reservations que cruzam chunk boundaries;
- ranking fica formalizado como priority descendente e, em empate, `placement_id`, `biome_id`, anchor X/Z e placement Y ascendentes;
- `reserve_space` é direcional: somente um intent de maior rank com reserva bloqueia um inferior apenas por reservation; `conflict_groups` compartilhados são simétricos depois que ranking escolhe o vencedor;
- cinco regressões cobrem higher-priority reservation fora do target, shared conflict group fora do target, intent não relacionado, reservation inferior não bloqueando superior e deterministic tie-break em prioridade igual;
- nenhuma linha da política de decisão foi alterada; o cut formaliza e prova a semântica existente.

### Cut 8 — connector metadata bounds com state-space explícito

Commit `dc9f3c6a8b9449a53bebd1e138827c222a1c3f47` (`Bound connector metadata state space`), CI #10282 success.

- `connected_horizontal_bounds_for_reference` deixa de usar `f32::to_bits()` como parte da memo key de remaining strength e passa a usar `CONNECTOR_BOUND_STRENGTH_BUCKETS = 64`;
- strength é sempre arredondada **para cima** ao bucket seguinte, então o estado reutilizado representa um upper bound da força real e pode apenas ampliar o bounds calculado, nunca reduzir;
- junto de `MAX_CONNECTOR_CHAIN_DEPTH = 64`, cada `structure + rotation` possui no máximo `65 strength buckets × 65 depth states` no memo, em vez de um conjunto praticamente ilimitado de bit patterns de `f32`;
- a cardinalidade total continua multiplicada apenas pelo conteúdo carregado/rotações válidas, não por histórico de traversal;
- o resolver/materializer real de connectors continua usando `f32` exato; bucketization existe somente no cálculo conservador de metadata bounds;
- o teste `connector_strength_bucket_never_underestimates_remaining_strength` cobre o invariant de arredondamento conservador; o teste existente da chain continua exigindo bounds exato `(0,0)..(8,0)` no caso simples;
- o fixture antigo de `explicit_piece_budget_caps_connector_expansion` foi corrigido de loss inválido para `0.25`, preservando cadeia suficiente para provar o budget de três pieces e respeitando o máximo de 64 loops;
- nenhuma alteração em hashing, seleção de group member, connector traversal runtime, overlap rejection, ground-fit, conflict policy ou rasterization;
- o arquivo acidental `tmp/placeholder` que acompanhou o commit foi identificado como resíduo de workspace e removido em `15c005ee0f5ab9694e52a06ab9f73ddb7a79c489` (`Remove accidental phase 6 placeholder`); CI #10283 success.

### Cut 9 — bridge final generation -> planning removido

Commit `ef651573aa8fa611e146ac21c23dab6c9d53e634` (`Remove final generation planning bridge`), CI #10284 success.

- o `const _` temporário em `generation.rs`, criado somente para manter type-checked um re-export morto durante a migração, foi removido;
- `generation/structures.rs` não re-exporta mais `connected_horizontal_bounds_for_reference`; o uso real continua direto via `structure_metadata::planning::connectors` dentro da coleta de candidates de volume;
- os demais re-exports de structures em `generation.rs` foram auditados e mantidos porque continuam compondo API realmente consumida; não houve churn artificial só para mudar paths;
- nenhuma lógica de planning, metadata bounds, candidate collection, materialization, rasterization ou terrain fitting mudou neste cut;
- com isso não resta bridge artificial criado pela migração Phase 6 entre `generation` e o owner de planning.

### Audit final / exit criteria

- conteúdo authored de cavernas já usa o mecanismo generalized: `asteria:overworld/caverns` referencia o group `asteria:cavern_entrance_start`; as cinco entrance variants pertencem a esse group e conectam para `asteria:cavern_tunnel_segment`; as nove tunnel variants pertencem a esse segundo group, incluindo straight, diagonals, curves e spirals;
- não existe gerador especial de cave entrance/tunnel em `src/`; a cadeia authored entra pelo volume-structure collector, usa deterministic member selection + connector expansion + planner conflict resolution e só então materialization por chunk;
- cavern carving em `density_sampling` é terrain/volume-biome shaping, não authored feature planning; ele só define o espaço cavernoso e não possui entrance/tunnel/connector graph;
- `BiomeSurfaceFluid` contém somente `VolcanoCrater`; `generation/fluids.rs` trata sea + crater/spill local. Não existe river/lake escondido nesse path;
- árvore de `src/` e `data/` não contém `hydrology`, `river` ou `lake`; não há subsystem legado/paralelo a remover no HEAD atual;
- **intent sem render**: `StructureField` é construído de seed + biome/structure/set content, guarda reference bounds e enumera anchors intersecting sem resident/render chunks; `WorldFeatureFields` mantém `StructureMetadata` separado de caches descartáveis;
- **materialization chunk-boundary-safe**: o cache mantém `ResolvedStructurePlacement` completo e consumers filtram pieces pela interseção do chunk; load/render order não decide o placement;
- **connectors deterministic e bounded**: selection é seed/hash-driven, depth cap = 64, runtime forest cap = 256 pieces e metadata bounds usam strength buckets conservadores;
- **interactions previsíveis**: priority/reservation/conflict groups pertencem ao planner e o contract cross-chunk está coberto no Cut 7;
- futuros rivers/lakes continuam explicitamente destinados ao mesmo structures/connectors model; eles não são requisito de conteúdo para encerrar esta infraestrutura e hydrology paralelo continua proibido.

**Phase 6 concluída.** Todos os exit criteria formais de `docs/asteria-core-rebuild.md` estão satisfeitos pelo architecture/content audit atual.

## Phase 7 — Voxel presentation / meshing v2

### Audit inicial

- initial mesh jobs já capturam `PresentationContentSnapshot` imutável + `ChunkMeshSnapshot` com center chunk e halo; workers não leem `VoxelWorld` vivo durante o build;
- `ChunkMeshDependencies` captura `ChunkContentRevision` do center/halo e publication de initial mesh rejeita task input revision stale, render irrelevante e snapshot dependency stale antes de criar entities/assets;
- remesh também usa snapshot imutável e rejeita content dependency stale; lighting possui revision própria por meshlet, separada de `ChunkContentRevision`;
- `ChunkContentRevision` não é incrementada por `set_light_at`, portanto uma futura render source revision não pode fingir que content revision sozinho representa lighting;
- `ChunkRenderPool` ainda guarda allocation por chunk + entities/mesh handles/bytes, mas não preserva explicitamente a identidade/source revisions que produziram o estado publicado;
- spike conhecido de startup/render (~217 ms frame max, ~188.7 ms render work no último gameplay log) continua dívida mensurada desta fase; causa ainda não deve ser atribuída sem profiling específico de submission/assets.

### Cut 1 — source stamp explícito para initial presentation

Commit `5811dc516fab40e47d157aa5d707dee585353478` (`Define initial presentation source stamp`), CI #10287 success.

- `ChunkPresentationSource` passa a agrupar `ChunkCoord` + `ChunkMeshDependencies`, em vez de o output carregar somente a matriz de revisions do snapshot;
- o check `is_current` agora possui também identidade explícita do center chunk e mantém a mesma validação de revisions do halo;
- initial halo catch-up continua delegado ao mesmo `ChunkMeshDependencies`, sem mudança de semântica;
- `TaskInputRevision` continua separado no scheduler para authored/content snapshot changes; o novo source stamp representa somente a fonte autoritativa do world snapshot;
- lighting não foi artificialmente fundido nesse tipo: remesh continua usando sua revision por meshlet, que será parte do próximo desenho section-aware;
- nenhuma lógica de mesh building, prioridade, cancelamento, publication, entity spawning ou asset retirement muda neste cut.

### Cut 2 — source stamp passa ao owner de presentation snapshot

Commit `af2e326b244359ff26aa73df62cae0f9c4ee8d6a` (`Move presentation source to snapshot owner`).

- `ChunkPresentationSource` deixa de pertencer ao scheduler de initial mesh e passa a `presentation_snapshot.rs`, ao lado do snapshot imutável de conteúdo de apresentação;
- `PresentationScheduler` passa a consumir esse tipo compartilhado e não define mais a identidade/source stamp localmente;
- lighting continua deliberadamente fora deste stamp; sua revision por meshlet permanece no owner de remesh até a composição section-aware;
- nenhuma lógica de meshing, task scheduling, publication, render allocation ou asset lifetime muda neste cut;
- CI #10288 passou os audits e falhou apenas no Clippy `dead_code`: `for_meshlets` havia sido introduzido antes do primeiro consumidor. Commit `ccd6183219d3bad1a376f6b99cf7f7286ba995a5` (`Fix presentation source Clippy gate`) removeu o método sem `allow`; CI #10289 passou audits + Clippy + Check.

### Cut 3 — remesh compartilha content source stamp

Commit `c3d35e3281f3e4b1abae7b2ef965d734102d06f6` (`Share presentation source with remesh`), CI #10290 success.

- `ChunkPresentationSource::for_meshlets` volta junto de seu primeiro consumidor e reduz as content/halo dependencies exatamente aos meshlets reconstruídos;
- `ChunkRemeshDependencies.content` deixa de armazenar `ChunkMeshDependencies` cru e passa a armazenar `ChunkPresentationSource`, usando a mesma identidade/source contract do initial mesh;
- lighting permanece separado e continua capturando revisions por meshlet; content e lighting não são colapsados em um contador artificial;
- publication behavior permanece igual: content stale sempre retry; lighting stale causa lighting retry para terrain e catch-up para fluid;
- nenhuma lógica de mesh building, queue priority, cancellation, asset patching ou entity lifetime muda neste cut.

### Cut 4 — lighting source ganha owner semântico de presentation

Commit `73d7d72c8dec0539cd2f6711a5b836697eebe2de` (`Define presentation lighting source ownership`), CI #10291 success.

- `PresentationLightingRevisions` passa a encapsular o mapa de revisions por chunk/meshlet e `PresentationLightingSource` representa o stamp imutável capturado por uma task;
- bump, removal, capture e freshness check passam ao domínio `presentation_snapshot`, em vez de serem implementados como detalhes internos do scheduler de remesh;
- `ChunkRemeshTasks` ainda hospeda temporariamente `PresentationLightingRevisions` para manter o cut pequeno e preservar os call sites atuais, mas deixa de definir a semântica desse estado;
- `ChunkRemeshDependencies` passa a compor explicitamente `ChunkPresentationSource` + `PresentationLightingSource` como owners independentes de content/halo e lighting;
- partial remesh continua section-aware: o lighting stamp contém mask + oito slots de revision, então publicar um subset não afirma freshness dos meshlets não reconstruídos;
- nenhuma política de retry/publication, queue scheduling, asset patching ou render lifetime muda neste cut.

### Cut 5 — lighting revisions viram resource físico de presentation

Commit `eebf02f723bb6b49191e977c542430da3f0c7c5c` (`Move lighting revisions to presentation resource`).

- `PresentationLightingRevisions` agora é `Resource` do `WorldPlugin`, com lifecycle/reset explícito em Loading, Gameplay e saída do mundo;
- `ChunkRemeshTasks` não armazena mais o mapa de revisions: ele volta a possuir somente scheduler/task metadata e uma fila transitória de cleanup para manter o retirement API estável durante a migração;
- dynamic lighting incrementa diretamente o resource compartilhado por meshlet;
- remesh captura e valida `PresentationLightingSource` contra o resource compartilhado tanto no dispatch quanto na publication;
- retirement/unload continuam chamando `remove_lighting_revision` no scheduler apenas como bridge: o pedido é drenado no começo de `process_chunk_remesh_queue`, no mesmo frame para retirement de Update e no frame seguinte para mesh-pressure retirement de PostUpdate;
- o mapa real deixa de estar acoplado ao scheduler, abrindo o mesmo owner para initial meshing e para os stamps persistidos no `ChunkRenderPool`;
- nenhuma regra de lighting propagation, remesh retry, queue priority, mesh building, asset patching ou render lifetime foi alterada neste cut;
- CI #10292 passou os audits e falhou somente no Clippy `too_many_arguments` de `collect_completed_remesh_tasks`; commit `d29645a675a6fe41a3005d9a00a846b21e2617d0` (`Fix lighting resource Clippy gate`) agrupou os inputs imutáveis em `RemeshCollectionContext`, sem `allow` e sem mudança de comportamento. CI #10293 passou audits + Clippy + Check.

### Cut 6 — initial mesh captura lighting source stamp

Commit `bbe356f44ac956252bbd74d9baf6375a00300792` (`Track lighting source for initial meshes`), CI #10294 success.

- `ChunkMeshTaskOutput` passa a carregar `content_source` e `lighting_source` explicitamente, em vez de um campo genérico `dependencies` que escondia o fato de existirem dois owners de freshness;
- dispatch de initial mesh em Loading e Gameplay captura `PresentationLightingSource` para `ChunkMeshletMask::ALL` do mesmo resource compartilhado usado pelo remesh;
- publication de initial mesh rejeita tanto content/halo stale quanto lighting stale antes de criar/alterar render allocation; Loading reschedules imediatamente e streaming devolve o chunk à ready queue;
- o direct-light seed de streaming continua once-per-residency: retry por lighting stale não reseed e mantém o catch-up de halo/seed já existente;
- Loading continua com lighting totalmente serializado antes de Meshing, portanto revision zero é um baseline legítimo até existir mudança dinâmica posterior;
- chunks vazios continuam no caminho síncrono sem async task; o stamp deles será capturado diretamente no momento da futura persistência no `ChunkRenderPool`;
- nenhuma lógica de mesh building, prioridade, preemption, lighting propagation, entity spawning ou asset lifetime muda neste cut.

Correção de processo imediatamente após o Cut 6: uma chamada errada criou `tmp/placeholder2` em `728abdfb764d4b91f7adb31c3eca62a732cdcffd`; `d149f26bd08382c99e572547a9d110bb20579a85` removeu o arquivo sem force-push. `bbe356f4..d149f26b` termina com **0 arquivos diferentes**, e CI #10296 passou audits + Clippy + Check.

### Cut 7 — `ChunkRenderPool` possui published source stamps section-aware

Commit `b532c841cb852e6a2d61d93ecdd822800e1fc784` (`Persist section-aware presentation source stamps`), CI #10297 success.

- `ChunkRenderPool` ganha metadata `published_sources` keyed por chunk, separada da allocation GPU mas com o mesmo lifecycle owner;
- cada chunk mantém oito slots independentes para terrain e oito para fluid; cada slot guarda o par exato `(ChunkPresentationSource, PresentationLightingSource)` da publication que o produziu;
- initial publication assíncrona em Loading e Gameplay grava o mesmo source pair nos oito meshlets de terrain e fluid somente depois que freshness foi validada e o render allocation foi criado;
- partial remesh expõe o source pair capturado pela própria task e só grava provenance depois que `apply_built_chunk_*_meshlets` retorna sucesso;
- geometry/lighting remesh atualiza somente os slots terrain selecionados pela mask; fluid remesh atualiza somente os slots fluid selecionados, preservando integralmente os meshlets e a section não tocados;
- `FluidWithLightingCatchup` persiste deliberadamente o lighting stamp que realmente produziu o mesh visível e mantém o follow-up já enfileirado; não mente que o fluid observou uma revision de lighting posterior;
- `insert` limpa metadata antiga antes de uma nova allocation; `take`/retirement e `clear` removem a metadata junto do allocation, portanto source stamps não sobrevivem ao render residency que possuíam;
- o stamp salvo por slot permanece o stamp exato do batch que publicou aquele meshlet; ele não é artificialmente reescrito para uma revision global;
- chunks vazios síncronos continuam sem stamp neste cut; a ausência representa source ainda não registrado, não freshness presumida;
- nenhuma lógica de mesh building, patch/replace, queue priority, entity lifetime, asset retirement ou lighting propagation muda neste cut.

### Cut 8 — chunks vazios recebem source stamp sem snapshot

Commit `0a847fa1adcd75c254ea2253834eb6007723b586` (`Stamp empty chunk presentation sources`).

- `ChunkPresentationSource` distingue internamente o modo de halo completo (`mesh_revisions`) do modo center-only (`center_revision`) dentro do owner de presentation;
- jobs de meshing/remesh continuam usando `mesh_revisions`, preservando halo revisions, meshlet reduction e initial catch-up exatamente como antes;
- publications síncronas de chunks vazios usam `capture_center`: capturam somente a revision autoritativa do center chunk, sem clonar `VoxelChunk`, halo ou construir `ChunkMeshSnapshot`;
- o modo center-only exige somente que a `ChunkContentRevision` do center permaneça igual; sua validação não chama `snapshot_chunk`, então continua revision-only;
- Loading e gameplay streaming capturam também `PresentationLightingSource::ALL` no momento da publication vazia e gravam o pair no `ChunkRenderPool` depois que a allocation é criada;
- o caminho gameplay continua executando direct-light seed antes da publication vazia; o stamp representa o estado observado naquele ponto, sem transformar o retry/seed lifecycle em meshing async;
- initial catch-up para center-only é vazio por definição; neighbor arrival continua invalidando/remeshando os chunks que realmente possuem boundary geometry, não a allocation vazia;
- nenhum trabalho async adicional, clone de snapshot, mesh build, entity churn ou asset allocation foi introduzido neste cut;
- CI #10298 passou os audits e falhou somente no Clippy `large_enum_variant` da primeira representação `Mesh(ChunkMeshDependencies) | Center(ChunkContentRevision)`. A correção preserva `Copy` e evita heap allocation representando os modos por `Option<ChunkMeshDependencies> + Option<ChunkContentRevision>` no mesmo struct, mantendo o tamanho na mesma ordem do source original e sem `allow`.

### Cut 9 — snapshot aliases transitórios removidos

Commit `88671dd37c0b89d5f4326d82c316c74ce6d82549` (`Remove presentation snapshot aliases`), CI #10300 success.

- `PresentationScheduler` deixa de armazenar `Arc<MeshContentSnapshot>` e passa a usar `Arc<PresentationContentSnapshot>` diretamente;
- `ChunkRemeshTasks` usa o mesmo `PresentationContentSnapshot` diretamente, sem passar por `chunk_mesh_tasks`;
- o re-export `MeshContentSnapshot` foi removido de `chunk_mesh_tasks.rs`;
- o compatibility shim `PresentationContentSnapshot::from_content` foi removido; initial mesh e remesh usam `PresentationContentSnapshot::capture`;
- não houve mudança em snapshot contents, task revision, scheduling, queue caps, publication, stale checks ou meshing;
- o alias `ChunkMeshTasks = PresentationScheduler` permaneceu como o último bridge transitório até o Cut 10.

### Cut 10 — scheduler de presentation é o único owner nominal

Commit `909a193c4e41113cb5a4cb0add5c5915cbbe59db` (`Use presentation scheduler owner directly`).

- lifecycle do `WorldPlugin`, Loading `SystemParam`/meshing e gameplay streaming passam a referenciar `PresentationScheduler` diretamente;
- o alias `ChunkMeshTasks = PresentationScheduler` é removido de `chunk_mesh_tasks.rs`; não resta compatibility name para initial presentation scheduling;
- init/reset de resource continua nos mesmos lifecycle hooks; somente o tipo nominal muda;
- métodos de sync/schedule/cancel/poll, caps, revision semantics e publication behavior não mudam;
- `streaming.rs` teve somente o import e o `ResMut` do SystemParam renomeados; `mod.rs` teve somente import + init/resets equivalentes;
- com isso os bridges transitórios de snapshot/scheduler ownership introduzidos pela Phase 7 foram removidos;
- CI #10301 passou os audits e falhou no Clippy porque `render_diagnostics.rs` ainda importava o nome removido `ChunkMeshTasks`; `1f1cdd2f4e3ba793a0e0f9e06a1f3be3c422b4d3` migrou esse último consumer para `PresentationScheduler`, sem mudança no diagnóstico;
- por erro de processo, esse corretivo foi publicado via Contents API antes do update do HANDOFF e portanto não ficou atômico com esta documentação; o histórico não foi reescrito/forçado e a exceção fica registrada aqui.

### Próximos cuts

1. auditar render-section identity/source stamps como base para o exit criterion de rebuild descartável;
2. separar/medir custo de meshing de render submission/assets e então atacar o spike de startup com evidência.

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