# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch de trabalho: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras de trabalho

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica e persistente do projeto. Exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`; feature branch só sob pedido explícito.
- Antes de escrever, buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Commits pequenos/coerentes; não misturar mudanças arquiteturais sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível; minor para feature compatível; major para breaking.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- A atualização do handoff faz parte do próprio bloco de trabalho. Não encerrar um bloco material deixando `HANDOFF.md` desatualizado.
- Se o último bloco ainda estiver aguardando CI, registrar isso como pending; quando o CI fechar ou houver fix subsequente, atualizar novamente.
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Corrigir todos os warnings de Rust encontrados nos blocos tocados.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando o comportamento depender disso.
- Não gerar imagens sem pedido explícito.
- Comunicação direta: quando o usuário disser `go`/`continua`, executar o próximo bloco aplicável sem confirmação desnecessária.
- Não ficar repetindo que `cargo check` não foi rodado ou que está esperando `cargo run`. O CI canônico valida Clippy + check; runtime do usuário só entra quando realmente necessário.
- `cargo test` só é rodado manualmente sob pedido explícito do usuário.
- `cargo fmt`/`rustfmt` não é gate do projeto.
- Commit exclusivamente documental de `HANDOFF.md` não sobe `VERSION`.

## Versionamento

- Fonte operacional acordada: arquivo raiz `VERSION`.
- Estado atual: `VERSION = 0.14.43`.
- `Cargo.toml` ainda declara `[package].version = 0.10.16`; essa divergência deve ser tratada em bloco explícito separado, não silenciosamente dentro de outro refactor.
- Até essa decisão, não inferir a versão do projeto pelo `Cargo.toml`.

## Validação

CI automático em `.github/workflows/ci.yml`:

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check`

Roda em push para `develop`/`main` e em pull requests.

## Canon arquitetural

`ARCHITECTURE.md` continua sendo o canon principal. Regras operacionais relevantes:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair semelhança superficial.
3. Preferir `SystemParam`s estreitos/coerentes e availability/run conditions canônicas.
4. UI compartilhada pertence a `src/ui`.
5. Targeting tem um único target autoritativo e consumidores change-driven.
6. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são os primitives de fila deduplicada.
7. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
8. Cores internas são HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
9. Evitar scans globais, allocations temporárias e dirty writes quando existe sinal/metadata no owner certo.
10. Pipeline inicial: generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> spawn.
11. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
12. Terrain/fluid/lighting background remesh é async; immediate geometry de edit do player permanece síncrono para feedback.
13. Não trocar corretude por performance aparente e não criar abstração genérica acima das generation/mesh task queues.
14. Sistemas visuais não devem reescrever Components/Assets com o mesmo valor; separar refresh estrutural de refresh leve quando os inputs diferirem.
15. Movimento com delta zero deve permanecer ocioso.
16. Scratch recorrente/estruturalmente limitado deve preferir stack/reuse a heap repetida sem impor limite artificial ao conteúdo.
17. `NewWorldConfig` é owner das escolhas de criação de mundo; forced biome valida coluna inicial e safe-spawn final no mesmo biome.
18. Search/text-input compartilhado pertence a `src/ui`; telas seguem owners de suas opções/filtros.
19. Preferências de HUD pertencem a `HudSettings`; targeting não conhece layout.
20. Queries mutáveis múltiplas sobre o mesmo Component precisam ser provadamente disjuntas via `Without<T>` ou `ParamSet`.
21. Child UI que deve obedecer ao hide do parent usa `Visibility::Inherited`; `Visible` só quando deve sobrescrever herança.
22. Revisions derivadas devem ser separadas por domínio quando consumers têm dependências diferentes; não usar revisão de mesh para invalidar lógica que depende só de blocos.

---

# Estado atual

Último HEAD de código publicado:

`8a961469cfc27d6aa4beb5070bcb07126b9988f4`

Bloco: `Move chunk mesh shell capture off main thread`

`VERSION`: `0.14.43`

Commits recentes relevantes:

- `e22c888` — `0.14.31`, `VoxelChunk::empty()` compartilha buffers vazios de blocks/fluids via `OnceLock<Arc<_>>`; light permanece exclusivo.
- `ba23847` — `0.14.32`, `DeduplicatedQueue` usa `HashMap::entry` para evitar lookups duplicados.
- `0aaefe8` — `0.14.33`, `DeduplicatedQueue` usa `bevy::platform::collections::HashMap`.
- `7b64f5b` — `0.14.34`, `VoxelWorld` usa hash collections do Bevy; `BTreeSet` de colunas permanece `std` porque ordenação é semântica.
- `383ae33` — `0.14.35`, `LightingContext` usa hash map do Bevy.
- `e4ebe14` — `0.14.36`, propagation reutiliza o chunk central para vizinhos cardinais locais.
- `ed45170` — `0.14.37`, `ChunkMeshSnapshot::capture` monta o shell de 1736 samples diretamente, sem prefill.
- `d5915fa` — `0.14.38`, solver de lighting consolida `chunk_mesh_revision` por chunk ao fim do batch.
- `e54afd6` — `0.14.39`, remove `set_light_at` órfão.
- `da309b3` — `0.14.40`, hash collections quentes restantes de lighting usam `bevy::platform`.
- `30c54ce` — `0.14.41`, `desired/retained/surface_ranges` do streaming usam `bevy::platform`; retention de feature caches aceita iterador de coords.
- `a6fc5c3` — `0.14.42`, `ChunkRenderPool.active` usa `bevy::platform::collections::HashMap`.
- `8a96146` — `0.14.43`, `ChunkMeshSnapshot::capture` passa a clonar os 26 vizinhos por `Arc`/COW e materializa os 1736 samples do shell dentro das tasks async de initial mesh/remesh; revisions continuam capturadas no main thread. Inclui teste de snapshot congelado após mutação posterior do world.

## CI recente

- `0.14.39` / run `35035772978`: Clippy **success**, `cargo check` **success**.
- `0.14.40` / run `35036777258`: Clippy **success**, `cargo check` **success**.
- `0.14.41` / run `35037142837`: Clippy **success**, `cargo check` **success**.
- `0.14.42` / run `35037660941`: Clippy **success**, `cargo check` **success**.
- `0.14.43` / run `35038065308`: **pending/in progress** nesta atualização.

Falhas históricas que não devem ser reintroduzidas:

- `0.14.16/17`: `TimeHudText.presented_time` usava `u32` para dia; `WorldClock.day` é `u64`.
- `0.14.19/20`: `QueueRebuildScratch` privado apareceu na assinatura pública(super) de `stream_chunks`; resolvido com `ChunkStreamingSelection`.
- `0.14.25`: structure support ainda chamava assinatura antiga de surface carver; corrigido em `0.14.26`.
- `0.14.38`: não manter API órfã só para preservar shape anterior.
- Draft `9c1a0e7`: não acoplar `retain_for_chunks` a `HashSet` com hasher específico; `0.14.41` resolve via iterador.

---

# Features consolidadas que continuam válidas

## Player HUD / hints

- Crosshair target hint contextual: break-only sem bloco selecionado; break/place com bloco selecionado.
- Crosshair hint não aparece com inventory ou pause e respeita `Display Tooltips`.
- Player HUD permanece com Inventory aberto e desaparece no pause.
- Inventory hint fechado: `Press E to open inventory, or ESC to pause game.`
- Inventory hint aberto: `Press E or ESC to close inventory.`
- `E` abre/fecha Inventory; `ESC` fecha Inventory quando aberto.
- Inventory hint é filho do `PlayerHudRoot` e usa `Visibility::Inherited` quando habilitado.

## Settings / HUD / shared UI

- `HUD` substituiu a antiga seção Miscellaneous.
- `Target Block Position` em `HudSettings`: `Center`, `Top-right`, `Hidden`.
- Target HUD separa layout de conteúdo/material; `TargetedBlock` continua owner do target.
- Shared search/text-input pertence a `src/ui` e é reutilizado por Creative Inventory e Spawn Biome.
- Settings/World Settings reutilizam scrollbar compartilhado.
- Spawn Biome é data-driven por `BiomeKind::Surface`.

## New World / Spawn Biome

- `NewWorldConfig` é owner único; `None` = Random.
- Forced spawn usa busca coarse-first por coluna seca e safe spawn final no mesmo biome.
- Caches pesados usados só na procura são podados antes da geração inicial.

## Worldgen balance já aplicado

- Plains tree chance `0.45 -> 0.48`.
- Witchwood tree chance `0.62 -> 0.66`.
- Enchanted Forest tree chance `0.62 -> 0.66`.
- Mountains weight `0.90 -> 0.85`.

---

# Refactor/performance consolidado

## Targeting / UI / ambiente

- Targeting usa `block_content_revision` separada da revisão de mesh e só raycasta quando inputs relevantes mudam.
- Highlight/placement preview usam snapshot derivado; `TargetedBlock` continua owner único.
- Estrelas, ambiente, hotbar, inventory, Settings, HUD e screen transition evitam writes idempotentes nos caminhos revisados.
- `process_dynamic_lighting` só roda quando `PendingLightingUpdates` possui trabalho.
- Diagnósticos de assets/mesh allocator só executam nos intervalos definidos.

## Streaming / render integration

- Unload recicla scratch; mesh integration evita buffers intermediários no replacement in-place.
- `QueueRebuildScratch` recicla `desired`, `pending`, `retired`; desired não cria Vec ordenado descartável.
- `ChunkStreamingSelection` encapsula settings + scratch.
- `stream_chunks` só coleta tasks quando existem pending e só despacha quando há capacidade/trabalho.
- `ChunkRemeshQueue` evita polling/dispatch vazio; immediate geometry permanece separado e síncrono.
- `VoxelChunk::empty()` compartilha storage vazio de blocks/fluids com COW natural de `Arc::make_mut`; light continua storage próprio.
- `DeduplicatedQueue`, `VoxelWorld`, streaming selection e `ChunkRenderPool.active` usam hash collections do Bevy onde não há requisito de ordenação.
- Retention de `WorldFeatureFields`/`FeatureCaches` recebe iterador de coords e faz uma passagem para horizontal chunks + generation regions.

## Worldgen allocations

- Feature cache retention reutiliza quatro HashSets internos sob `Mutex` compartilhado pelas clones de `WorldFeatureFields`.
- `SurfaceCarverColumn` e `SurfaceMaterialColumn` reutilizam seus Vecs por coluna.
- `BiomeField::sample_surface`: máximo estrutural 26 influências; temporário usa `ArrayVec`.
- `GenerationColumnSample` usa `SmallVec` inline no caso comum.
- Structure support reutiliza a própria amostra de biome para `surface_height_from_sample`.

## Lighting / mesh snapshot

- `seed_chunk_direct_lighting` pula upper chunks vazios antes do scan voxel a voxel.
- `LightingContext`, pending emission edits, changed chunks e scratch de dynamic lighting usam hash collections do Bevy.
- Propagation usa o próprio `VoxelChunk` para vizinhos cardinais locais e volta ao world lookup só ao cruzar borda.
- O early-return de medium opaco ocorre antes das leituras cardinais.
- Mudanças de luz são aplicadas imediatamente; `chunk_mesh_revision` é consolidado por chunk ao fim do batch.
- `ChunkMeshSnapshot::capture` não expande mais 1736 shell samples no main thread: captura clones COW dos vizinhos + revisions; shell é materializado no worker e os clones vizinhos são soltos em seguida.
- `enqueue_loaded_fluid_frontier` usa metadata O(1), escaneia só a face necessária e para ao localizar todos os fluids dinâmicos.

---

# Auditoria atual: alvos descartados sem nova evidência

- `walk`/flight já tratam delta zero como idle; camera look retorna sem delta/focus/grab.
- Viewmodel animation/held block já são guarded/cached.
- Dynamic held light e directional shadows já usam guards de mudança.
- Coordinate/name HUD já tem caches; underwater tint só recalcula quando inputs relevantes mudam.
- `track_current_biome` precisa amostrar continuamente durante movimento por causa de blends; não quantizar arbitrariamente.
- Fluid updates e dynamic lighting já têm scratch/budget/run conditions.
- `WorldTickClock` muda accumulator legitimamente por frame; micro-guard em `ticks_this_frame` não é ganho relevante.
- Celestial bodies são poucos; sem allocation hot path relevante.
- Clouds poderiam virar parent animado + child estático, mas o risco visual/lifecycle é maior; não fazer sem profiling.
- `notify_loaded_chunk_neighbors` deve continuar conservador até existir metadata que prove skip seguro.
- Structure rasterization não mostrou temp Vec hot evidente.
- Limites de tasks não equivalem a 20 threads concorrentes; reduzir arbitrariamente tende a reduzir throughput.
- Surface-range cache já aquece os quatro vizinhos cardinais usados pela seleção.
- `ChunkTaskQueue` tem poucos entries por design; trocar hasher ali não é prioridade.
- A expansão eager de 4096 lighting seeds continua um custo real, mas uma fila lazy precisa preservar FIFO, dedup global e priority promotion. Não criar uma segunda semântica de fila sem um invariant/projeto que preserve isso explicitamente.

---

# Hot paths / roadmap restante

Prioridade ligada ao relato de FPS caindo ao andar/carregar chunks:

1. **Próximo patch seguro (`0.14.44`)**: remover a alocação temporária escondida de `sort_by_cached_key` no rebuild da streaming selection. `QueueRebuildScratch.pending` deve armazenar `(coord, priority)` e ordenar in-place com chave já calculada, preservando exatamente o mesmo ordering tuple e a mesma seleção.
2. Reavaliar a expansão de 4096 lighting seeds apenas se for possível representar o trabalho incremental preservando FIFO + dedup + priority promotion do `DeduplicatedQueue`; não publicar uma versão semanticamente diferente apenas para espalhar custo.
3. `BiomeInfluence` nasce de índice de surface biome, mas alguns consumers convertem ID de volta para índice. Auditar todos os consumers e, se o shape puder ser estendido sem acoplamento ruim, carregar o índice junto para remover scans lineares repetidos.
4. Tratar `VERSION 0.14.x` vs `Cargo.toml 0.10.16` em bloco explícito separado quando os hot paths de runtime estiverem esgotados.
5. Fazer uma nova auditoria global do projeto depois desses itens. Se nenhum alvo sustentado por evidência/invariant restar, encerrar o roadmap em vez de inventar micro-otimizações.

---

# Bugs/produto fora do refactor atual

Só retomar quando o usuário priorizar ou quando runtime indicar regressão relacionada:

- ghost block / held block deve refletir a hotbar como observer; transparência é do bloco inteiro, não por face;
- hydrology river/lake margin e água escapando da margem já foram relatados;
- dye anteriormente estava fraco demais;
- iluminação/sombreamento teve regressões pós-09/09 e foi revertida; evitar mudanças visuais casuais no lighting;
- terrain generation ainda foi relatada como lenta em movimento mesmo após melhorias anteriores.

---

# Próximos passos

1. Aguardar CI de `0.14.43` (`35038065308`).
2. Se verde, publicar `0.14.44` removendo a allocation de `sort_by_cached_key` via scratch reutilizável de `(coord, priority)`.
3. Atualizar handoff e validar CI.
4. Auditar todos os consumers de `BiomeInfluence` e só então decidir se o índice deve virar dado carregado.
5. Reavaliar lighting seed expansion; se não houver desenho que preserve a fila canônica, registrar como alvo bloqueado por corretude.
6. Resolver explicitamente a divergência `VERSION` / `Cargo.toml` em bloco separado.
7. Fazer auditoria global final. Sem evidência nova, encerrar roadmap.

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados;
- reduzir custo indivisível dentro de um budget, não apenas adicionar budgets em volta;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- stack/reuse para scratch estruturalmente limitado;
- evitar scans globais, allocations temporárias e mutações idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative.
