# HANDOFF — Asteria / Mineclone

## 2026-10-01 — Attached Objects: chunk-batched presentation

- O storage/targeting de Attached Objects continua autoritativo no `VoxelWorld`; objetos individuais não precisam mais existir como Bevy Entity para interação, loot ou remoção.
- Removido o vínculo `WorldObjectKey -> Entity` do `WorldObjectStore`. A presentation agora mantém apenas batches por chunk, com contagem separada de objetos e batches para diagnóstico.
- `grass` (GLB/model) e `pebble`/`stick` (`extrudedSprite`) usam o mesmo pipeline de batching por chunk, agrupado por source mesh + material + flags de shadow.
- Cada batch é limitado a 256 instâncias para manter o tamanho de mesh previsível; alterações em Attached Objects invalidam/reconstroem somente o chunk cujo `chunk_object_revision` mudou.
- O resolver/cache de `extrudedSprite` foi reaproveitado pela presentation batched, sem duplicar geração de mesh/material.
- Targeting, transform, jitter, loot e interação continuam data-driven e independentes da existência de uma Entity por objeto.
- Esta mudança remove a arquitetura anterior de 1 Bevy Entity por Attached Object; **nenhum claim de ganho de FPS é feito sem log/runtime comparativo**.
- Commits principais: `5d6d745cc769eb7fe398a64530cd4a77f8dc6150`, `ffaf1fb46b928f05f1ae78086337bb66f5072157`, `4066093f4c98fb74757adb3f4276a0168ec8b5d1`, correções de gate `befd6e01ca6e212beb8588f2b08cc0c71f1e65ea`, `a8c97c2f9801447846b81a19b2d277cf1deeac7a`, `767b3b36f23155d75d51e80665fb504bccef5312`.
- CI Rust validation verde no HEAD `767b3b36f23155d75d51e80665fb504bccef5312` para push e PR (audits + Clippy + Check).

## 2026-10-01 — Log attack: locate, logging and streaming findings

- The provided runtime logs contain no Rust panic, ECS B000x error, or gameplay ERROR. The reported Floating Islands locate/warp failure was reproduced in the log sequence: `/locate biome asteria:overworld/floating_islands` returned `(-213, 219, 568)` before the volume-anchor validation fix, while the same run had no `warp.success` and later manual travel reached the area without an island. The locate result was therefore stale/invalid under the pre-fix volume selection behavior.
- `/locate biome asteria:overworld/volcano` started at `00:13:12` but had not completed by the next locate command at `00:13:20`. `PendingLocate` held only one task, so starting another locate could orphan the previous result and violate the command-feedback requirement. New overlapping locate commands are now rejected with explicit feedback instead of replacing the in-flight task.
- Surface biome locate was scanning every block in every chunk ring. Rare biome searches such as Volcano could therefore become extremely expensive. Locate now searches the deterministic surface-biome site lattice and validates the target at each site, reducing the search space from block-scale sampling to roughly tens of thousands of site samples across the full 32k-block radius.
- Runtime logs showed malformed diagnostic lines such as duplicated `[RUNTIME]` prefixes/timestamps, caused by concurrent appenders opening the same log file independently. Session logging now uses one mutex-protected persistent file handle, serializing writes and removing that race while avoiding per-event open/close churn.
- The logs also show real streaming hitches during sustained Creative flight: one 202 ms frame had `streaming_max_us=196704`, with ~3.7k retired chunks and ~2.4k pending chunks. This is a streaming-selection/retirement pressure issue rather than rendering preparation; it remains a separate performance target for the next pass.
- Runtime lighting also remains continuously backlogged during long flight (`propagation_pending=true` for the entire captured interval), processing hundreds of thousands of voxels per 5s diagnostic window with little visible change. This is another follow-up performance/correctness target, not treated as fixed by the current pass.
- The 00:10:41 new-world log selecting Plains as the random spawn biome is valid for that seed; the deterministic spawn roll lands inside Plains' configured weighted interval. It is not evidence of a random-spawn regression by itself.
- Warp search radius remains 32 blocks. The temporary 48-block expansion was reverted because the observed failure came from the pre-fix invalid locate result; increasing the search cube would have expanded worst-case search work 3.3x without addressing the root cause.

## 2026-10-01 — Biome exclusivity, minimum size and locate/warp correction

- Wasteland, Witchwood and Enchanted Forest now share `exclusiveNeighborGroup: "inland_biomes"`, so different members of that set cannot border one another.
- Fixed `exclusiveNeighborGroup` semantics so a biome can remain continuous with another site of the **same biome**; the group only blocks different biome IDs.
- Surface biome `size.min` is now enforced as a hard lower bound during site selection, including against different neighboring biome sites that define the candidate region boundary.
- `/locate biome` for volume biomes now verifies the anchor with the same resolved volume selection used by terrain generation. It no longer reports a geometric volume anchor when surface constraints (such as Floating Islands requiring Plains) reject it.
- Warp safe-position search radius increased from 32 to 48 blocks so current Floating Islands (`size.y.max = 36`) can be reached from their located anchor.
- Validation is green on `9a0a52cdff035794ba438cedfcef8ec8c610e082` (Clippy + Check + localization/content/GLB audits). An obsolete random-spawn test helper left from the previous spawn-weight change was also removed.

## 2026-10-01 — Independent random spawn weighting

- Corrected the previous interpretation: the five mountain surface biomes are **individual spawn choices**, not one family for random starting biome selection.
- Removed the `exclusiveNeighborGroup` family collapsing from random spawn selection.
- Added `DimensionBiome.spawnWeight`, independent from world-generation `weight`, so spawn balance can be tuned without changing biome distribution across the world.
- Random spawn now performs deterministic weighted selection using each biome's `spawnWeight`.
- Mountain variants currently use `spawnWeight: 0.5` individually; other surface biomes retain the default `1.0`. This makes the five mountain starts collectively less frequent while keeping each mountain biome independently selectable.
- World-generation `weight` values were not changed by this fix.
- Validation for commit `9726ef1deb27017053f975ee74a935eab6524603` was still running when this handoff entry was written.

## 2026-09-30 — Biome distribution, spawn and shoreline correction

- Random-biome spawn no longer counts every member of an `exclusiveNeighborGroup` as a separate top-level spawn option. Mountain variants now share one spawn-family slot, so the five mountain surface variants no longer make mountain starts disproportionately likely.
- Surface `size.max` enforcement was corrected to compare a candidate only against neighboring sites whose raw biome candidate is the same biome. The previous post-rebuild guard compared against unrelated neighboring biomes, causing the fallback path to reselect Plains/Ocean and allowing very long continuous regions.
- The surface adjacency checks now sample the real macro-climate field at neighboring sites instead of synthesizing unrelated climate values from the cell hash.
- The old ocean continentalness core was restored: when Ocean is fully inside its authored continentalness range, it is authoritative instead of competing with land candidates.
- Wasteland, Witchwood and Enchanted Forest no longer have hard mutual `avoidNear` exclusions in the overworld dimension. Those exclusions were suppressing the biomes almost completely because the rule was evaluated against every bordering raw site.
- Ocean shoreline margins are now explicitly forbidden from applying over any surface influence tagged `mountain`. Mountains, Gorge, Alps, Mountain Belt and Volcano carry the semantic `mountain` tag. This prevents a gradual mountain slope from passing the local `maxSlope` test block-by-block while still receiving ocean sand.
- Validation: CI Rust validation is green on `c3859d5b45ad2935329da8fb9e24e668f69e4da2` (Clippy + Check + content/localization/GLB audits).
- Size fallback now prioritizes `size.max` before authored adjacency when the two constraints have no common local solution, so the max-size bound cannot be bypassed by reselecting Plains/Ocean.
- Relevant commits: `c264683e384d1cd8a6c9c3504006a1033e5fa000`, `c8a62387a468c6d173fb91bea1a78035e0ce9450`, `c3859d5b45ad2935329da8fb9e24e668f69e4da2`.

## 2026-09-30 — 0.68.54 Geo embedded rock-cap correction

- Geo normal + large no longer use a crown/band sitting above the slime.
- Rock masses are independent, asymmetric and partially buried into the upper blob; side masses descend into the temples and the center stone sits lower on the forehead, matching the supplied reference.
- Pixel-art invariant preserved: all rock-detail faces remain cardinal/axis-aligned.
- Generator: `assets/models/creatures/slime_geo/generate_slime_geo.py`.


## 2026-09-30 — 0.68.53 Geo stone-crown rebuild

- Geo normal + large refeitos a partir do slime_blob e da referência do usuário.
- Corpo permanece uma bolota cinza limpa; identidade Geo fica numa coroa de pedras no topo.
- Dois espigões laterais maiores funcionam como chifres; pedras centrais menores fecham a diadema.
- Toda a coroa é pixel-art 3D estrita: caixas ortogonais e staircases, sem curvas nem faces diagonais lisas.
- Generator reproduzível: `assets/models/creatures/slime_geo/generate_slime_geo.py`.
- Validação: audit geral de GLBs + verificação específica de normals cardinais na `geo_stone_crown`.
- Face segue em `textures/creatures/slime_geo/face.png`; seis clips de animação preservados.


> Handoff corrente e operacional. Histórico anterior ao Cut 16: `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico antigo: `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura: `docs/asteria-core-rebuild.md`.
## 2026-09-30 — Loading screen telemetry alignment

- Loading screen phase rows now follow the active `WorldLoadingPhase` telemetry instead of the older one-off loading strings.
- Added presentation prewarm progress telemetry to `WorldLoadingState`: the final loading phase now reports `0/12` through `12/12` while Bevy/render-side presentation caches are warmed.
- Renamed the final loading-screen section from player spawning to **Presentation warm-up**, matching what actually consumes the visible loading time; player spawning itself is still performed at the start of that phase.
- Removed obsolete localization keys for the former terrain/chunks/assets/finalizing summary strings that no longer have consumers.
- Updated Portuguese and Spanish loading-phase labels that were still English and added the new presentation warm-up label in all three languages.
- Fluid settling remains a real loading phase but intentionally has no numeric counter because the current settling telemetry does not expose a stable total; its active-row highlight remains the indicator.
## 2026-09-30 — Comprehensive gameplay event logging

- Centralized gameplay event logging was added through `app::crash_log::log_gameplay_event`.
- Every gameplay event is emitted through Bevy logging under target `asteria::gameplay` and also appended to the active session log as `[RUNTIME ...] [EVENT] ...`.
- Logged interactions now include:
  - block mining start and block breaking (survival/creative);
  - block placement and layer placement;
  - world-object placement/removal/pickup/break requests and applied mutations;
  - tool uses and targets;
  - creature/player spawn, natural spawn, damage, death and death-timer despawn;
  - command submission, chat feedback, successful spawn/place/locate/warp/kill/modify flows and entity metadata modifications;
  - command-driven kill damage/death;
  - locate start/success/failure and warp request/success/failure;
  - world item spawn/drop/settle/pickup;
  - inventory slot clicks, creative item selection, sorting and discard;
  - target transitions for blocks/entities/world items/objects;
  - player game-mode changes;
  - world load bootstrap/completion, save success/failure, and authoritative chunk unloads.
- Event messages include contextual identifiers/positions/quantities/health/modes where available, so gameplay logs can be correlated without reconstructing state from screenshots.
- CI is green on `c6a2cf2779cf82dc194da0f36fc013315f6fda93`.
## 2026-09-30 — Floating island shape/material correction + spectator HUD cleanup

- Floating island sites now use `84..120` X/Z and `24..36` Y, reducing the near-touching footprint while making the landmass vertically thicker.
- Island lobes were tightened and rounded: the central mass is less elongated, secondary lobes are smaller and still overlap the core, and underside depth increased.
- Edge masking now tapers *before* the outer boundary instead of staying solid until the radius and dropping vertically; this removes the straight-wall/plate look.
- Top blend was tightened so the first generated solid voxel remains aligned with the authored surface layer; the `grass_block -> dirt -> stone` stack can now place grass on the actual top.
- Regression tests cover edge taper and top-layer depth; CI is green on `a845440585818262deba50bc833d7db9df00c9c8`.
- Floating Islands remain constrained by `surfaceConstraints.allow = plains`; no other surface biome is eligible.
- Spectator targeting visuals remain disabled, including target highlight/brush/artisan ghosts.
- Spectator player preview cameras are inactive; the local-player HUD remains hidden in spectator, including the character portrait preview.
- Fixed the Bevy 0.19 `Single` SystemParam lifetime errors introduced by the spectator patch.

## 2026-09-30 — 0.68.52 Dendro strict pixel-art geometry

- Dendro normal + large tiveram todos os detalhes vegetais refeitos como pixel art geométrica estrita.
- Folhas, frondes, caules, gavinhas, pétalas, sépalas e botão usam apenas caixas axis-aligned e caminhos ortogonais em degraus; não há curvas suaves nem prismas diagonais.
- Diagonais visuais são staircases de voxels; a gavinha agora é um hook/espiral quadrada.
- A flor large mantém pétalas laranja em camadas, mas cada lóbulo é um conjunto de terraços voxelados.
- Auditoria adicional valida que todas as normals dos GLBs são cardinais. Face 64x64 e seis clips de animação permanecem.

VERSION: `0.68.52`.

## 2026-09-30 — 0.68.51 Dendro reference rebuild + slime asset regression repair

- O merge do Asteria core rebuild em `develop` (`48197a30ed78cc6b3eadd5f2be7fcf0d2a9c4204`) trouxe de volta versões antigas dos assets de slime e sobrescreveu o patch visual aprovado logo antes; este bloco restaura explicitamente o estado aprovado de `aaad2601eccd99336994479261aa15c349cd0022` antes de aplicar o Dendro novo.
- Todos os `SlimeFace` aprovados permanecem 1:1 para texturas 64x64 e as nove faces elementais do patch do usuário são preservadas, incluindo a face própria do Electro.
- Electro continua como bolota amarelo/dourada + antena central; detalhes elétricos restantes ficam na textura, sem circuitos frontais em geometria.
- Dendro normal: bolota verde-clara limpa + broto, frondes de samambaia e gavinha apenas no topo.
- Dendro large: bolota verde-clara limpa + flor laranja larga, botão verde compacto, sépalas e folhagem no topo.
- Generator reproduzível: `assets/models/creatures/slime_dendro/generate_slime_dendro.py`; normal + large preservam os seis clips de animação existentes.
- Colliders físicos continuam do corpo blob. `SlimeFace` continua em `textures/creatures/slime_dendro/face.png`.
- Validação: face check 1:1 + audit de todos os GLBs antes do commit. QA visual final permanece in-game.

VERSION: `0.68.51`.


## 2026-09-30 — 0.68.55 Geo integrated shell correction

- Geo reference reread: dark stone is an integrated shell covering the upper half of the head and surrounding the eyes, not a crown/accessory above the blob.
- Side stone peaks lean outward through voxel stair-steps; no smooth/rotated diagonal geometry.
- Lower half remains pale and clean; forehead/temple stone masses are partially embedded in the blob.
- Geo normal + large regenerated; GLB audit + cardinal-normal audit passed.

VERSION: `0.68.55`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch operacional: `develop`.
- Não houve autorização para mover a continuidade do trabalho para uma branch de fase/rebuild separada; o trabalho corrente deve continuar em `develop`.
- Asteria core rebuild integrado em `develop` pelo merge `48197a30ed78cc6b3eadd5f2be7fcf0d2a9c4204`.
- `develop` contém o core rebuild integrado e os patches de slime subsequentes; versão de conteúdo atual `0.68.52`.
- `VERSION` atual em `0.68.63`; mudanças recentes de conteúdo/modelos de slime estão preservadas.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar.
- Old saves/legacy compatibility não são prioridade.
- CI obrigatório = audits + Clippy + Check; `cargo test` não é gate automático.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues possuem owner e bound explícitos;
- trabalho frame-sensitive precisa ser incremental/budgetado;
- generation não possui side effects de render/UI/ECS;
- presentation é derivada e descartável; `ChunkRenderPool` nunca é world truth;
- cada cut coerente termina com HANDOFF atualizado e CI verde;
- não reintroduzir Hydrology legado.

## Fases

- Phases 1–6 concluídas: core boundaries, authoritative storage, biome/structure metadata, Streaming scheduler v2, Terrain generation v2 e structures/connectors/feature planning.
- **Phase 7 em andamento: voxel presentation / meshing v2 + dívida de performance/correctness descoberta durante validação.**

## Sync de `develop` — Electro Slime 0.68.49

- O avanço paralelo de `develop` foi integrado por merge commit real, sem rebase/force-push e sem substituir código do rebuild.
- O delta de `develop` desde a baseline anterior é restrito a `assets/`, `data/creatures`, `HANDOFF.md` e `VERSION`; não há alteração `.rs` nesses seis commits.
- Electro normal + large foram refeitos a partir da linguagem do `slime_blob`, com paleta amarelo/dourada, antena elétrica e marcas discretas.
- Ambos usam exatamente `textures/creatures/slime_anemo/face.png`; a face Electro antiga foi removida.
- Generator reproduzível: `assets/models/creatures/slime_electro/generate_slime_electro.py`.
- Metadata/colliders acompanham os novos modelos; GLB audit e Khronos validator haviam passado em `develop` com zero erros/warnings.
- QA visual do Electro ainda deve conferir silhueta, face, detalhes e animações normal + large em gameplay.

## Estado recente

### Cut 23 — surface `size.min`

Commit `35313349c08c39dab6249afa35a56323117ce8ae`, CI #10321 success.

- jitter independente ±32% por site podia comprimir vizinhos até 36% do spacing nominal;
- surface sites agora usam jitter lateral determinístico por linha; domain warp continua owner da organicidade;
- validar worldgen em mundo novo.

### Cut 24 — limitar scans stale do generation frontier

Commit `75f476f54942f69ccd08df819b8b6ad5479099d8`, CI #10322 success.

Gameplay `2026-09-30_16-23-13-909518300.txt` reproduziu frame ~100.9 ms, `main_work` ~97.5 ms e `streaming` ~93.9 ms. `select_generation_wave` fazia priority scans O(n) e stale/already-owned entries escapavam de `budget.record(1)`. Agora cada `pop_pending_by_priority()` bem-sucedido consome uma unidade do budget de 1 ms / máximo 16 scans. Sem claim de ganho até log novo.

### Cut 25 — ocean/coast height influence lower-only

Commit `cc116cb8d477259bc0ccc6d954177b6b2d7805a0`, CI #10323 success.

- terrain influences possuem política `Blend` ou `LowerOnly`;
- `BiomeTerrain::Ocean` é `LowerOnly`; demais terrains continuam `Blend`;
- o resultado com ocean influence é limitado ao baseline não-oceânico que existiria sem a influência lower-only;
- oceano pode abaixar/truncar/soften coast, mas nunca elevar mountain/alps/mountain belt/volcano/gorge ou qualquer terrain futuro blended;
- região somente ocean preserva o shape oceânico;
- shoreline material/surfaceMargin identity não mudou.

## Pacote Worldgen coherence

### Cut 26 — volume biome constraints por surface biome

Commits publicados: `0c659fa3376fcc7f59594fe29dde97ca40b7ce22`, `4d84cae3783bdbfc20020ca5b7ee8ec424d8b472`, `c4b62f5bdf103a63807d0340234be5169c2e1bb9` e correção de gate `99cd6456af1f77b670f1e771d3b372cc46dac735`. O merge de `develop` em `12806bd94b08498d15ac92111b74e22863935b93` preservou o corte e passou CI #10341.

- `BiomeDefinition.tags`: tags semânticas opcionais para surface biomes;
- `BiomeDefinition.surfaceConstraints` opcional para volume biomes;
- selector data-driven suporta `ids` e `tags` com semântica OR dentro do selector;
- constraints suportam `allow` e `deny`; sem `allow` significa permitido salvo deny, e `deny` sempre vence;
- surface biomes não podem declarar `surfaceConstraints`; selectors vazios e tags vazias/duplicadas são rejeitados;
- `BiomeFieldEntry` carrega tags/constraints imutáveis para geração async;
- `volume_selection_in_region` preserva a API para planners e resolve a surface identity do anchor; o density pass usa `volume_selection_in_region_for_surface` para reutilizar `GenerationColumnSample.identity_surface_index` sem novo surface sample por voxel;
- volume biome sem constraints mantém comportamento irrestrito anterior;
- nenhum biome atual recebeu constraint inventada.

Regressões cobrem ID/tag matching, allow/deny + deny precedence, unrestricted behavior e filtro positivo/negativo.

### Cut 27 — volume biome surface indicators

Commit `5121da7ea2fb59d37b71c24ad12f38a1c722b9e6`, CI #10342 success.

- `VolumeStructurePlacementRules.mode` defaulta para `volume`, preservando conteúdo existente;
- novo modo `surface_indicator` continua sendo `BiomeStructure`, portanto structure groups, chance, priority, conflict groups, reserve-space e connectors usam o planner generalizado;
- identidade/chance continuam derivadas do volume site 3D original;
- o root é projetado no mesmo X/Z para o surface Y via `validated_structure_origin_y`;
- restrictions/ground-fit do root e children usam o surface biome efetivo da projeção;
- o volume site precisa continuar selecionado naquele X/Z, então `surfaceConstraints` também se aplicam;
- nenhuma indicator concreta foi inventada para conteúdo atual; o corte entrega a capacidade autorável.

### Cut 28 — floating islands como ilhas estratificadas

Implementação preparada em `tmp/cut28-floating-islands`:

- o shape deixou de ser um único blob radial; cada site agora tem um core central obrigatório + 3–5 lobes secundários determinísticos;
- os lobes secundários têm offset limitado e footprint mínimo que garantem overlap com o core; não são pedras independentes;
- união de masks usa smooth union `1 - (1-a)(1-b)`, produzindo transição gradual entre massas;
- cada lobe varia footprint X/Z, top offset e underside depth pelo seed; planar/detail noise continuam responsáveis pela irregularidade orgânica das bordas e do topo;
- topo permanece amplo e levemente ondulado; underside continua afunilando em direção às bordas;
- `VolumeBiomeSelection` carrega apenas o `vertical_radius` durante density sampling; o density pass converte a posição local em profundidade de superfície uma única vez e empacota só `u16` para o material pass;
- `surfaceLayers` passa a ser permitido em volume biome somente quando o density modifier é `floating_island`; demais volumes continuam proibidos de usá-lo;
- Floating Islands agora autoram `grass_block` depth 1 -> `dirt` depth 4 -> `stone`, mantendo `solidBlock: stone` como fallback;
- footprint autorado aumenta de `22..48` para `48..96` em X/Z e de `8..16` para `12..24` em Y;
- o novo `min` não aumenta o spacing global dos volume sites: Caverns já possuía `min=48` em X/Z e `min=18` em Y;
- o early-out de chunks altos agora considera `FloatingIsland` um solid volume modifier; antes o flag legado considerava somente `Solid`, o que podia descartar chunks de ilha acima do terrain local;
- nenhuma regra de surface terrain, structure ou streaming foi alterada.

Regressões do shape cobrem conexão core/lobe, topo amplo + underside afunilado e variação determinística por seed. O gate deve ser executado após consolidação.

## Correção operacional — continuidade em `develop`

- O handoff anterior continha uma regra de continuidade apontando para uma branch de Phase 7 separada. Essa regra não corresponde à instrução operacional atual e foi corrigida.
- `develop` é a branch de trabalho e fonte de continuidade deste projeto.
- Nenhuma mudança deve ser deslocada para outra branch por iniciativa própria.
- As correções de Floating Islands solicitadas nesta sessão (somente sobre Plains e maior espaçamento) ainda não devem ser descritas como integradas em `develop` até existirem commits efetivamente aplicados nesta branch.

## Cut 29 — presentation prewarm integrado em develop

O trabalho do branch de continuação foi integrado diretamente em `develop` após o PR #24 apresentar conflito com avanços posteriores de conteúdo. O merge automático do PR foi recusado pelo GitHub por conflitos; os dois arquivos de código do cut foram aplicados diretamente em `develop`.

- `InitialPresentationPrewarm` cria/player + prime da visibilidade inicial ainda sob o loading;
- mantém 12 frames completos de prewarm antes de solicitar Gameplay;
- estado de prewarm permanece local ao sistema de loading;
- worldgen, streaming, residency e authoritative world state não são alterados;
- o gate do branch de origem já havia passado audits + Clippy + Check (#10496/#10500);
- runtime ainda precisa ser validado em `develop`; não declarar ganho de performance sem log novo.

## Cut 30 — authoritative eviction no longer owns presentation retirement

Commit: `ceb7bb0a580c915ca42084a620269ed8dc4fe4d6`. CI Rust validation #10525: success.

- `evict_distant_chunks` deixou de receber `ChunkRenderer` e não toca mais `ChunkRenderPool`, entidades, mesh tasks ou remesh queue;
- render retirement continua exclusivamente em `retire_distant_chunk_meshes`, que roda antes da authoritative eviction no pipeline de Gameplay;
- authoritative eviction agora arquiva o chunk e publica apenas o trabalho de simulação/lighting decorrente da mudança de residency;
- removida a `ChunkEvictionPresentationRuntime`, eliminando a mistura explícita entre world eviction e presentation teardown;
- o helper de halo remesh permanece no caminho de retirement da apresentação, portanto a invalidação acontece enquanto a fonte ainda está disponível;
- nenhum comportamento de residency/streaming pretendido foi alterado.

## Cut 31 — presentation selection separated from authoritative streaming residency

Commit final: `2f6c8d9d8d7a81ee276b3c2f8d204fcf723df13b`. CI Rust validation: success.

- criada `ChunkPresentationSelection` como snapshot explícito da seleção de apresentação: center X/Z + hide radius + revision;
- `ChunkStreamingState` deixou de expor `retains_render_mesh`; streaming publica a seleção para a camada de presentation;
- initial meshing, remeshing e render retirement passaram a consultar `ChunkPresentationSelection`, não a inferir a retenção de mesh a partir da residency;
- `retire_distant_chunk_meshes` agora usa a revision própria da apresentação, permitindo que mudanças de seleção invalidem somente o backlog de retirement;
- halo remesh após retirement também usa a seleção de apresentação;
- dependências de retirement foram agrupadas em `ChunkRenderRetirementRuntime`, mantendo o boundary de presentation sem aumentar a assinatura do sistema;
- remesh contexts perderam a referência desnecessária ao streaming;
- lifecycle de `ChunkPresentationSelection` é resetado junto do Gameplay/Loading lifecycle;
- testes adicionados para revision estável, mudança de pose e independência de authoritative residency;
- mudanças paralelas de conteúdo do Hydro foram preservadas; não fazem parte deste cut.

## Floating Islands follow-up — integrado em develop

- Floating Islands agora aceitam somente `asteria:overworld/plains` como surface biome;
- footprint/lattice X/Z passou de `48..96` para `96..144`, afastando os sites;
- `grass_block` depth 1 -> `dirt` depth 4 -> `stone` continua autorado na biome;
- runtime visual ainda precisa confirmar a aplicação do grass na superfície.

## Cut 32 — comprehensive runtime/loading diagnostics

Commit final: `f7069a7eb186bb26e75c5a4b148ffe82efa3114e`. CI Rust validation: success.

- Loading now emits lifecycle + 0.5s progress events for Generating, Fluid Settling, Lighting, Meshing, Assets, Finalizing and Presentation Warm-up.
- Fluid settling logs explicit start/completion events; runtime fluid solver diagnostics are persisted to the session log instead of existing only as console `info!` output.
- Dynamic lighting now emits periodic runtime diagnostics with processed voxels, changed chunks/positions, dirty meshlets and remaining propagation work.
- Player movement now logs state transitions and periodic movement snapshots while moving: position, game mode, flight, swimming, grounded/running state, horizontal speed and vertical velocity.
- Existing render/streaming diagnostics already persist periodic frame, streaming, async generation/mesh/remesh, fluid-settling, mesh-pressure and asset-pressure diagnostics to the session log.
- Existing command/chat/warp gameplay events remain covered by `log_gameplay_event`; this cut fills the major missing movement/loading/lighting gaps.
- Logging is event/transition + periodic diagnostics, not per-frame raw spam, so the session log remains useful for postmortem analysis.
## Próximos passos

1. gameplay em mundo novo para validar Floating Islands: somente Plains, grass/dirt/stone e espaçamento;
2. coletar log pós-Cut 29 em `develop` e comparar com o baseline 0.68.59;
3. se o cold-start continuar, seguir para a decomposição de `PrepareResources`/`Render`;
4. retomar a dívida restante de voxel presentation / meshing v2.

## Regras de continuidade

- trabalhar diretamente em `develop`, salvo instrução explícita em contrário;
- não force-push/rewrite de commits publicados; usar fast-forward;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
- 0.68.56: Cryo Slime rebuilt from the supplied reference: integrated icy upper shell, tall central stepped crystal, outward diagonal voxel spikes, and smaller shards; normal + large regenerated with cardinal-only geometry.

- 0.68.57: Hydro Slime rebuilt from the reference as a clean turquoise blob with no horns. Normal and large use one integrated top water droplet, tapered only with cardinal voxel stair-steps; face remains texture-driven.

- 0.68.58: Pyro Slime rebuilt from the reference as a clean orange-gold blob with no horns. Normal and large now use an integrated asymmetric flame cluster on top, built only from cardinal voxel stair-steps; face remains texture-driven.

- 0.68.59: Hydro top reshaped as a broad continuation of the slime body tapering into a droplet point; Pyro crown-like tuft replaced by independent irregular flame tongues distributed across the top. Normal/large GLBs regenerated and cardinal pixel-art geometry revalidated.

- 0.68.60: Hydro top rebuilt as a shaded exposed-face voxel continuation of the body, broad at the root and tapering to a droplet point; its root uses the exact slime-top material. Pyro flame geometry kept, but all flame roots now begin in the exact top-surface material before transitioning to fire colors.

- 0.68.61: Hydro droplet vertically compressed while preserving its broad integrated root and existing shading, changing the silhouette from a tall spike to a shorter water-drop continuation. Pyro unchanged.

- 0.68.62: Hydro droplet reshaped from a cone/spike silhouette into a compact rounded drop: lower layers hold a bulbous width, taper begins later, and the point is short. Pyro unchanged.

- 0.68.63: Hydro top reshaped again from the in-game screenshot: removed the broad cap/mound silhouette. The droplet now has a narrow root buried into the slime, a small exposed bulb, and only a short pointed upper taper. Pyro unchanged.

- 0.68.64: Hydro corrected structurally: removed the separate HydroDroplet mesh/node entirely. The main hydro_blob_body profile itself now carries the water-drop silhouette and tapers continuously into a short apex point; Pyro unchanged.
