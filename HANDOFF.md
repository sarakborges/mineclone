# HANDOFF — Asteria / Mineclone

> Handoff corrente e operacional. Histórico anterior ao Cut 16: `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico antigo: `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura: `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline de develop: `5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante os cutovers internos.
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

Commits publicados: `0c659fa3376fcc7f59594fe29dde97ca40b7ce22` + `4d84cae3783bdbfc20020ca5b7ee8ec424d8b472`; gate correction em andamento.

- `BiomeDefinition.tags`: tags semânticas opcionais para surface biomes (e reutilizáveis futuramente);
- `BiomeDefinition.surfaceConstraints` opcional para volume biomes;
- selector data-driven suporta `ids` e `tags` com semântica OR dentro do selector;
- constraints suportam `allow` e `deny`; sem `allow` significa permitido salvo deny, e `deny` sempre vence;
- surface biomes não podem declarar `surfaceConstraints`; selectors vazios e tags vazias/duplicadas são rejeitados;
- `BiomeFieldEntry` carrega tags/constraints imutáveis para geração async;
- `volume_selection_in_region` preserva a API para planners e resolve a surface identity do anchor; o density pass usa `volume_selection_in_region_for_surface` para reutilizar `GenerationColumnSample.identity_surface_index` sem novo surface sample por voxel;
- volume biome sem constraints mantém comportamento irrestrito anterior;
- nenhum biome atual recebeu constraint inventada; o corte entrega a capacidade genérica sem retuning arbitrário de conteúdo.

O primeiro gate do Cut 26 falhou somente porque o helper de teste `test_surface_entry` em `selection.rs` ainda construía `BiomeFieldEntry` sem os novos campos. A correção adiciona `tags: Vec::new()` e `surface_constraints: None`; nenhum comportamento runtime muda nesse follow-up.

Regressões cobrem ID/tag matching, allow/deny + deny precedence, unrestricted behavior e filtro positivo/negativo.

### Próximos workstreams

1. **Volume surface indicator:** metadata opcional + structure/structure group no surface usando planner generalizado.
2. **Floating islands rewrite:** ilhas grandes, irregulares e coerentes, smooth lobe transitions e grass -> dirt -> stone.

Princípios:

- metadata/data-driven para relações de conteúdo;
- sem hardcode de biome IDs quando primitive geral resolve;
- determinismo, chunk seams e generation-order invariance obrigatórios;
- Hydrology legado continua proibido;
- validar mudanças de worldgen em mundo novo.

## Próximos passos

1. fechar CI do Cut 26;
2. implementar surface indicator via structures;
3. reescrever floating islands e estratificação;
4. gameplay em mundo novo para validar pacote + novo log pós-Cut 24;
5. retomar cold-start de `PrepareResources` depois do pacote.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não force-push/rewrite de commits publicados; usar fast-forward;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
