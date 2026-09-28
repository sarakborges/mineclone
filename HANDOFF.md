# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico integral anterior foi preservado em `HANDOFF_ARCHIVE_2026-09-25.md`. Para continuidade normal, comece por este arquivo e por `docs/asteria-core-rebuild.md`.

## 2026-09-28 — Asteria Core rebuild / Phase 1 em andamento

- Branch ativa: `architecture/asteria-core-rebuild`.
- Baseline da reconstrução: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- Decisão arquitetural permanece: **Rust + Bevy**, com Bevy como host/framework e Asteria possuindo world core, metadata/generation, streaming, simulation boundaries e voxel presentation.
- Não é rewrite cego do jogo inteiro. Gameplay, content, assets, UI e sistemas válidos devem ser preservados/adaptados enquanto a fundação é substituída por cutovers explícitos.
- Hydrology legado continua removido e **não deve voltar**. Rivers/lakes/cave entrances e features longos futuros pertencem ao sistema generalizado de structures/connectors/structure groups + metadata. Dynamic fluid simulation continua separada.
- `VERSION` permanece `0.68.48`; estes commits são reconstrução arquitetural sem release de gameplay.

### Ownership já separado

O antigo `ChunkStreamingState` deixou de possuir diretamente vários estados que antes estavam misturados:

- logical residency (`desired` / `retained` / retirement);
- pending generation queue + critical/priority scan caches;
- ready/initial-presentation queue + scan cache;
- generation-wave lifecycle (targets, pending, prefetch, staged, settled publication);
- initial presentation activation state;
- mesh-pressure residency;
- streaming selection pose;
- surface/structure selection caches;
- streaming priority diagnostics;
- generation snapshot e presentation snapshot boundaries.

O facade `ChunkStreamingState` ainda coordena owners quando existe regra multi-owner; wrappers puramente delegadores foram removidos quando não agregavam invariante.

### Core identities / revisions concluídos até aqui

- `DimensionId`: `CurrentDimension` não armazena mais `String` arbitrária.
- `ChunkCoord`: identidade primitive-backed usada internamente em async queues/schedulers e vários owners de streaming/presentation.
- `TaskInputRevision`: revisão nominal do snapshot que originou trabalho async; stale results não usam `u64` cru nesse contract.
- `ResidencySelectionRevision`: owner de residency e caches retired/pending/ready armazenam revisão semântica; o `u64` restante é somente adapter temporário do facade legado.
- `GenerationRegionCoord`: volume-biome cache e retention scratch não confundem mais region identity com `IVec3` arbitrário.
- `WorldId`: `WorldSession` armazena identidade tipada; catálogo/snapshot/thumbnail recebem `&str` somente nas bordas existentes.
- `VoxelCoord`: conversão voxel -> chunk agora distingue semanticamente world voxel coordinate de `ChunkCoord`; APIs Bevy-facing continuam com adapters `IVec3` durante a migração.

### Cutovers verdes mais recentes

- `00c425ebcddedac2e7c5d907632e0723d35f8952` — async queue/schedulers com `ChunkCoord` interno; CI `36477699634` success.
- `5a85ee7388ae85c753f279fdd082cea35e3a54e0` — retired residency queue tipada; CI `36478092904` success.
- `0a574bafc0db7283b6458fef73a5007d6ac92c60` — pending queue/cache tipados; CI `36478253194` success.
- `fa3b494cc51bda18418d7a23f18a01c7ddbcd87a` — ready queue/cache tipados; CI `36478397317` success.
- `f158b9f3b361e7f44890447b76c1516c88326cef` — initial-presentation state tipado; CI `36478531501` success.
- `b9fa917c45de9c749982b615ca8a1f7f7e9f974b` — mesh-pressure state tipado; CI `36478719478` success.
- `e0cf88a44bd9993f5ed749f2b0761faa17170964` — generation-wave identity tipada; CI `36478958760` success.
- `3802282b8190a95c5c9b0b219cd0400ab0cfed53` — selection center usa `ChunkCoord` internamente; CI `36479165269` success.
- `324d6215a264a7d7c40fd34d600ea3de6d95684f` — residency-selection revision nominal nos owners/caches; CI `36479931215` success.
- `69d485a6b645655d09107c022ccd1f069c44cc6f` — generation-region cache identity tipada; CI `36480324199` success.
- `0dd65711aa13c6063f10aa943c70cb6826ff42f9` — active world session identity tipada (`WorldId`); CI `36480551060` success.
- `74b4af045278a76d5deb827b7854c0e7b249ad9f` — typed voxel-coordinate conversion core; CI `36480786611` success.

### Invariantes preservados

- Nenhum desses cutovers alterou priority order, render radius, retention radius, generation-wave batching, dynamic fluid settling, mesh-pressure policy ou selection shape.
- `IVec3` continua válido para aritmética espacial e adapters ainda não migrados; ele não deve permanecer como identidade interna quando o owner já sabe que o valor é especificamente chunk/region/voxel identity.
- Dynamic fluid settling continua deliberadamente separado do generation-wave identity owner.
- Async work continua revisionado e stale results continuam rejeitados.
- Cada bloco só avança após `Rust validation` verde, incluindo audits, Clippy `-D warnings` e `cargo check`.

### Phase 1 ainda não terminou

Ainda faltam os dois cortes mais importantes antes da Phase 2:

1. **authoritative chunk-content API**: criar/usar uma boundary estreita para leitura/mutação/revisions sem consumidores dependerem da implementação interna de `VoxelWorld`;
2. **chunk content / simulation / presentation revision semantics**: reduzir `u64` crus restantes em dependencies e preparar a centralização de mutation ownership.

`VoxelWorld` **não deve** ser convertido num megadiff. Hoje ele ainda mistura resident chunks, archived/persistent chunks, revision maps, block/fluid/object mutation e spatial access. O cutover deve separar essas responsabilidades de forma incremental, com behavior-preserving adapters.

`desired`/`retained` também permanecem `HashSet<IVec3>` por enquanto porque atravessam intensamente `streaming/selection.rs`; isso precisa ser um cutover próprio, não uma mudança colateral.

## Baseline/runtime que precisa continuar preservado

- Baseline `0.68.48` possui os Electro GLBs semanticamente reparados e auditados por `tools/check_glb_assets.py`.
- Instrumentação de performance continua disponível: `frame_*`, `main_work_*` e `render work`.
- Fresh Phase 0 gameplay logs (stationary, movement/streaming e warp) continuam úteis para comparação de performance; não inventar métricas quando execução local real não estiver disponível.

## Continuidade imediata

1. Confirmar CI verde do último checkpoint antes de qualquer novo cutover.
2. Continuar Phase 1 pela **authoritative chunk-content read/dependency boundary**, sem reescrever `VoxelWorld` inteiro.
3. Tipar content/dependency revisions de forma incremental e manter adapters explícitos para APIs legadas.
4. Depois fechar o contract de mutation ownership necessário para iniciar **Phase 2 — Authoritative chunk/world storage**.
5. Atualizar este handoff após cada bloco significativo e não avançar com CI vermelho/warnings.
