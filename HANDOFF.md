# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico integral anterior foi preservado em `HANDOFF_ARCHIVE_2026-09-25.md`. Para continuidade normal, comece por este arquivo e por `docs/asteria-core-rebuild.md`.

## 2026-09-28 — Asteria Core rebuild / Phase 1 concluída, Phase 2 em andamento

- Branch ativa: `architecture/asteria-core-rebuild`.
- Baseline da reconstrução: `develop@5038934a97a51cdd8cdc94fc61314cb650d96d` (`0.68.48`).
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
- `ChunkContentRevision`: agora é nominal também no map/counter autoritativo de `VoxelWorld`, no cache de lighting, na verificação de fluid settling e nos dependency snapshots. O caminho de content revision não converte mais de/para `u64` cru.
- Object invalidation agora possui owner próprio (`ObjectRevisionState`). Scene revision e per-chunk object revision deixaram de ser campos soltos do `VoxelWorld`; internamente são domínios nominais distintos, mantendo `u64` apenas na API temporária consumida por `world_objects.rs`.

### Authoritative read / mutation / storage boundaries

- `ChunkMeshSnapshot` e `ChunkMeshDependencies` não dependem mais concretamente de todo o `VoxelWorld`; eles dependem da capability estreita `ChunkSnapshotSource`.
- Essa capability expõe somente leitura imutável de chunk + content revision. Mutation, archive/persistence, lighting, objects e demais APIs do container autoritativo não vazam para presentation por esse caminho.
- `VoxelWorld` é hoje um adapter/implementação dessa capability; Bevy `ResMut<VoxelWorld>` é desreferenciado explicitamente nos adapters de setup/streaming, em vez de contaminar o core trait com ECS wrapper types.
- `VoxelBlockMutation.chunk` e os resultados internos de `VoxelMutationRuntime::{add_layer,set_block,set_fluid}` usam `ChunkCoord`; `VoxelTopologyRuntime` mantém `IVec3` apenas como adapter externo onde ainda necessário.
- Os antigos `VoxelMutationRuntime::world()` / `VoxelTopologyRuntime::world()` foram removidos. Gameplay/tools/chat não recebem mais acesso ao container inteiro através do mutation facade.
- `VoxelRead` agora cobre leitura de sample/cell/fluid/loaded/solid/block-id; `VoxelTopologyRead` adiciona apenas object lookup. `VoxelTopologyReader` é um view restrito de leitura entregue por `runtime.read()`.
- Collision, raycast e block placement dependem de `VoxelRead`, não de `VoxelWorld`. Targeting, mining, Artisan's Kit, carpenter's axe, shears e chat placement foram migrados para o view restrito.
- Bevy `Res<VoxelWorld>` continua sendo explicitamente desreferenciado nos adapters que ainda recebem o resource diretamente; os traits core não conhecem `Res`/`ResMut`.
- `ChunkPersistenceState` possui o persistent set + archived payloads. `VoxelWorld` coordena archive/restore com resident state/revisions, mas não possui diretamente o set/map de persistência.
- `LoadedChunkColumnIndex` possui o índice vertical dos chunks residentes e as queries column-local/highest-Y.
- `ResidentChunkStore` possui o `HashMap<IVec3, VoxelChunk>` residente + o índice de colunas; insert/remove mantêm collection/index consistentes atomicamente.
- As semantics de persistence foram preservadas: chunk gerado e não modificado é descartado ao archive; chunk persistent é arquivado/restaurável; save serializa somente chunks persistent; generated-fluid settling não pode promover/alterar chunk já persistent pelo caminho derivado.
- `VoxelWorld` ainda coordena mutation, lighting e revision effects; o resident store não absorveu side effects laterais.
- Os testes de boundary de `streaming/meshing.rs` foram preservados. Durante o cutover uma regravação truncada removeu dois testes; isso foi detectado pelo diff, restaurado em `f20ae0a6`, e o checkpoint corrigido passou CI antes de continuar.

### Revision/dirty domains — diagnóstico Phase 2

- Object scene invalidation é um domínio real: `world_objects.rs` usa revision global para fast-path da cena e revision por chunk para atualização incremental. Esses valores foram agrupados em `ObjectRevisionState` e tipados internamente.
- `chunk_mesh_revisions` **não deve ganhar owner novo**. O getter é test-only e não existe leitura de produção desse counter; a apresentação real já invalida por `ChunkRemeshTasks`/`ChunkRemeshQueue`, inclusive lighting por meshlet em `lighting_updates.rs`.
- `lighting::propagation` ainda chama `commit_deferred_light_mesh_revisions()` por herança do mecanismo antigo, mas o trabalho efetivo de remesh vem dos changed positions -> meshlet masks -> remesh task revisions/queue. Próximo cut deve remover esse contador legado e renomear o setter de light que ainda menciona “deferred mesh revision”.
- `block_content_revision` ainda precisa ter seus consumidores/semantics confirmados antes de ser extraído ou removido.

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
- `05efa4c33cc6ae050a6ebe625a4ed7e58412d714` — mesh dependency content revisions deixam de usar `Option<u64>` cru; CI `36481297442` success.
- `f20ae0a60fe7f0ecf8bc78d9f92c0729ff9cf994` — `ChunkSnapshotSource` boundary finalizado com Bevy adapters e testes restaurados; CI `36482186553` success.
- `4555045f10293a60c500d75eed3361655d6d20b1` — `ChunkContentRevision` promovida para `voxel::revision`; CI `36483058849` success.
- `72ce79c768a08009d259e9e5ca7f7604edf4bb5b` — detailed mutation result passa a usar `ChunkCoord`; CI `36483446796` success.
- `5b6db802959255e635f75e2ea6a41e0da558b898` — mutation runtime internal chunk results tipados; CI `36483646178` success.
- `bb2afe5e8d4fc17a423b9d8545d74de48b10ba05` — full-world mutation escape hatch fechado e consumidores migrados para narrow read capabilities; CI `36485808944` success.
- `cb59a951256a91abf321e9b10750296c02aa9541` — `VoxelWorld::chunk_content_revision()` passa a expor `ChunkContentRevision` diretamente.
- `e498900fddd8dfa782fa24aabe9497305e31fbf0` — lighting content-revision cache tipado.
- `e5b36d4f79ea10cf2ed16467796e8763716af23a` — generated-fluid verification revisions tipadas; CI `36486908948` success.
- `576b770822ee8919b6cd7a1864308e159d4d3547` — content-revision map/counter autoritativos tipados e adapter `from_raw` removido; CI `36488836153` success.
- `fc011ac18d3f0de80fdd2551cf0ff6430377b99b` — `ChunkPersistenceState` passa a possuir persistent set + archived payloads; CI `36489367228` success.
- `ba0d53326c2beb1d4203d82f57443e9da7a543f4` — loaded chunk column index extraído; CI `36489860888` success.
- `59de357997883148ffcc5df581f916f4632683b7` — `ResidentChunkStore` passa a possuir resident chunks + column index; CI `36490280435` success.
- `3f072dd12db3f456b1060a890730f4457a7a847c` — scene + per-chunk object revisions agrupadas em `ObjectRevisionState`; CI `36491139143` success.
- `d240f50368cf76147638cf00039f5de8eb84eec6` — object scene/chunk revisions tipadas internamente mantendo adapter `u64`; CI `36491654054` success.

### Phase 1 — encerrada

Os exit criteria da Phase 1 estão atendidos para o cutover:

- identities/coordinate conversions relevantes têm tipos semânticos onde já existe owner conhecido;
- presentation/generation/mutation usam contracts estreitos em vez de receber o container inteiro;
- async boundaries possuem revisions nominais e stale-result contracts explícitos;
- authored definitions, deterministic metadata, mutable runtime data e presentation snapshots têm boundaries distintos;
- os novos core contracts não dependem de `App` Bevy e os adapters ECS ficam nas bordas.

Isso não significa que todo `IVec3`/`u64` legado já foi eliminado; significa que os tipos/boundaries necessários para migrar storage/simulation/presentation existem e não precisamos continuar expandindo abstrações sem consumidor real.

### Phase 2 — Authoritative chunk/world storage em andamento

Já concluído nesta phase:

- authoritative resident chunk store explícito;
- persistence/archive state separado;
- content revision nominal end-to-end;
- mutation gameplay facade sem escape hatch para `VoxelWorld` inteiro;
- object-scene/per-chunk object invalidation com owner explícito e revisions semanticamente distintas internamente.

Próximos cortes devem seguir as tarefas da Phase 2:

1. remover `chunk_mesh_revisions` legado em vez de criar owner artificial; presentation dirtiness já pertence ao remesh scheduler/queue;
2. confirmar callers/semantics de `block_content_revision` e então extrair, tipar ou remover conforme uso real;
3. centralizar cada mutation effect exatamente uma vez: content revision, persistence promotion, simulation/presentation dirtiness e object-scene effects;
4. tornar lookup semantics explícitas para resident / absent / known-but-not-resident onde o código realmente precisa dessa distinção;
5. definir/bound cache e eviction ownership sem acoplar isso aos render entities.

`VoxelWorld` ainda é um facade/coordenador útil e não deve ser explodido num megadiff. A meta agora é reduzir responsabilidades internas por owners concretos, preservando a API externa enquanto os callers migram.

## Baseline/runtime que precisa continuar preservado

- Baseline `0.68.48` possui os Electro GLBs semanticamente reparados e auditados por `tools/check_glb_assets.py`.
- Instrumentação de performance continua disponível: `frame_*`, `main_work_*` e `render work`.
- Fresh Phase 0 gameplay logs (stationary, movement/streaming e warp) continuam úteis para comparação de performance; não inventar métricas quando execução local real não estiver disponível.

## Continuidade imediata

1. Confirmar CI verde do último checkpoint antes de qualquer novo cutover.
2. Remover o mesh revision counter legado e o publish no-op de lighting, preservando `ChunkRemeshTasks`/`ChunkRemeshQueue` como owner real de presentation dirtiness.
3. Mapear callers e semantics de `block_content_revision` antes de tipar/extrair.
4. Não misturar object-scene, block-content e simulation/presentation dirtiness no mesmo commit.
5. Atualizar este handoff após cada bloco significativo e não avançar com CI vermelho/warnings.
