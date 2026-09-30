# HANDOFF — Asteria / Mineclone

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


## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch: `develop` (Asteria core rebuild já integrado).
- Asteria core rebuild integrado em `develop` pelo merge `48197a30ed78cc6b3eadd5f2be7fcf0d2a9c4204`.
- `develop` contém o core rebuild integrado e os patches de slime subsequentes; versão de conteúdo atual `0.68.52`.
- `VERSION` em `0.68.52`; este bump cobre mudanças de conteúdo/modelos de slime.
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

## Próximos passos

1. consolidar e fechar CI do Cut 28;
2. gameplay em mundo novo para validar ocean/coast, constraints/indicator e floating islands;
3. coletar log pós-Cut 24 para confirmar se o hitch de generation frontier caiu;
4. retomar cold-start de `PrepareResources` depois do pacote.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não force-push/rewrite de commits publicados; usar fast-forward;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
