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
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Corrigir todos os warnings de Rust encontrados nos blocos tocados.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando o comportamento depender disso.
- Não gerar imagens sem pedido explícito.
- Comunicação direta: menos narração, mais mudança concreta; quando o usuário disser `go`/`continua`, executar o próximo bloco aplicável sem pedir confirmação desnecessária.
- Não ficar repetindo que `cargo check` não foi rodado, que está esperando `cargo run` ou equivalente. O CI canônico já valida Clippy + check; runtime do usuário só é citado quando realmente necessário para comportamento visual/gameplay.
- `cargo test` só é rodado manualmente sob pedido explícito do usuário.
- `cargo fmt`/`rustfmt` não é gate do projeto.

## Versionamento

- Fonte operacional acordada para os blocos do projeto: arquivo raiz `VERSION`.
- Estado atual: `VERSION = 0.14.30`.
- `Cargo.toml` ainda declara `[package].version = 0.10.16`; essa divergência foi detectada em 2026-09-15 e deve ser tratada como bloco explícito, não silenciosamente dentro de outro refactor.
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

HEAD de código validado antes deste handoff:

`85e00af33e5b4547f55e9cc7dc089ebe9ff77ab1`

Bloco: `Skip empty chunks in direct lighting seed`

`VERSION`: `0.14.30`

Commits recentes relevantes:

- `d6552a1` — `0.14.21`, encapsula render distance + scratch de seleção em `SystemParam`; corrige lints de `0.14.19`.
- `87bde0b` — `0.14.22`, evita coleta/dispatch do remesh background quando não há trabalho.
- `42730ae` — `0.14.23`, evita coleta/dispatch vazio no streaming quando generation/mesh queues estão ociosas.
- `420420d` — `0.14.24`, reutiliza HashSets de retention dos feature caches.
- `75c8000` — `0.14.25`, reutiliza buffers por coluna de surface carvers e surface materials; revelou call site antigo em structure support.
- `8c1915d` — `0.14.26`, corrige structure support para a nova API reutilizável de carvers.
- `1cee3d6` — `0.14.27`, `BiomeField::sample_surface` passa a usar `ArrayVec` stack-backed em vez de `Vec` por amostragem; structure support também usa scratch stack-backed.
- `2925a9a` — `0.14.28`, structure support reutiliza a amostra de biome já calculada para a altura de superfície.
- `5f6722a` — `0.14.29`, `GenerationColumnSample.surface_influences` usa `SmallVec<[(usize, f32); 4]>`, mantendo spill para heap sem limite artificial.
- `85e00af` — `0.14.30`, direct lighting seed ignora upper chunks vazios antes do scan de voxels.

## CI recente

- `0.14.21` / run `35026065652`: Clippy **success**, `cargo check` **success**.
- `0.14.22` / run `35027171366`: Clippy **success**, `cargo check` **success**.
- `0.14.23` / run `35027716003`: Clippy **success**, `cargo check` **success**.
- `0.14.24` / run `35028227550`: Clippy **success**, `cargo check` **success**.
- `0.14.25` / run `35028710877`: **failure** de compilação porque `structures/support.rs` ainda chamava a assinatura antiga de `resolve_surface_carver_column`.
- `0.14.26`: correção desse call site; CI verde antes do próximo bloco.
- `0.14.27` / run `35029822534`: Clippy **success**, `cargo check` **success**.
- `0.14.28` / run `35030176206`: Clippy **success**, `cargo check` **success**.
- `0.14.29` / run `35030633441`: Clippy **success**, `cargo check` **success**.
- `0.14.30` / run `35031098600`: Clippy **success**, `cargo check` **success**.

Falhas históricas que não devem ser reintroduzidas:

- `0.14.16/17`: `TimeHudText.presented_time` usava `u32` para dia; `WorldClock.day` é `u64`.
- `0.14.19/20`: `QueueRebuildScratch` privado apareceu na assinatura pública(super) de `stream_chunks` e elevou a função a 8 argumentos; resolvido com `ChunkStreamingSelection` SystemParam em `0.14.21`.

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

- Unload recicla scratch; mesh integration evita buffers intermediários no caminho de replacement in-place.
- `QueueRebuildScratch` recicla `desired`, `pending`, `retired`; o desired set não cria mais um Vec ordenado que era descartado.
- `ChunkStreamingSelection` encapsula settings + scratch sem poluir assinatura do sistema.
- `stream_chunks` sincroniza snapshots de generation/mesh, mas só coleta quando há tasks pending e só despacha quando há capacidade/trabalho.
- `ChunkRemeshQueue` evita polling/dispatch vazio do background remesh; immediate geometry permanece separado e síncrono.

## Worldgen allocations

- Feature cache retention reutiliza quatro HashSets internos sob `Mutex` compartilhado pelas clones de `WorldFeatureFields`.
- `SurfaceCarverColumn` e `SurfaceMaterialColumn` reutilizam seus Vecs por coluna dentro de cada chunk.
- `BiomeField::sample_surface` tem limite estrutural derivado de `SITE_SEARCH_RADIUS`: 25 sites + macro biome = máximo 26 influências. O sample temporário usa `ArrayVec` e não aloca heap.
- `GenerationColumnSample` não usa array fixa de 26 porque isso inflaria os 256 samples persistentes; usa `SmallVec` inline para o caso comum e spill quando necessário.
- Structure support reutiliza a própria amostra de biome para `surface_height_from_sample`, removendo uma segunda amostragem completa.

## Initial lighting / streaming main-thread

- `seed_chunk_direct_lighting` precisa considerar chunks superiores para skylight.
- Antes de `0.14.30`, mesmo upper chunk completamente vazio fazia scan de até 4096 voxels; agora `VoxelChunk::is_empty()` O(1) pula esse scan.
- Chunks não vazios continuam com a mesma semântica de dampening/skylight.
- `enqueue_loaded_fluid_frontier` já usa `boundary_dynamic_fluid_count` O(1), escaneia no máximo a face necessária e para ao localizar todos os fluids dinâmicos; não refatorar sem nova evidência.

---

# Auditoria atual: o que já foi descartado

Não repetir estes alvos sem profiling/evidência nova:

- `walk`/flight já tratam delta zero como idle e evitam writes desnecessários.
- Camera look retorna sem delta/focus/grab.
- Viewmodel animation/held block já são guardados/cached.
- Dynamic held light e directional shadows já usam guards de mudança.
- Coordinate/name HUD já tem caches; underwater tint só recalcula visual quando inputs relevantes mudam.
- `track_current_biome` precisa amostrar continuamente durante movimento por causa de blends; não quantizar arbitrariamente.
- Fluid updates e dynamic lighting já possuem scratch/budget/run conditions.
- `WorldTickClock` muda accumulator legitimamente por frame; micro-guard em `ticks_this_frame` não é ganho relevante.
- Celestial bodies são poucos; sem allocation hot path relevante.
- Clouds poderiam ser reestruturadas em parent animado + child estático, mas o risco visual/lifecycle é maior; não fazer sem profiling.
- `notify_loaded_chunk_neighbors` deve continuar conservador até existir metadata que prove um skip seguro.
- Structure rasterization revisada não mostrou temp Vec hot evidente.
- Limites de tasks `generation/initial mesh/remesh` não equivalem a 20 threads concorrentes; reduzir os limites arbitrariamente tende a reduzir throughput sem provar ganho de FPS.
- Surface-range cache já aquece os quatro vizinhos cardinais usados pela seleção, então um warmup cardinal adicional seria redundante.

---

# Hot paths ainda abertos

Prioridade ligada ao relato de FPS caindo ao andar/carregar chunks:

1. **Main thread entre generation e mesh:** `initial lighting -> fluid frontier -> ChunkMeshSnapshot::capture` ainda acontece antes de cada initial mesh task.
2. `ChunkMeshSnapshot::capture` clona o center chunk via Arc/COW, mas materializa um shell compacto de 1736 `ShellSample`s por task; investigar reuse/layout/metadata sem mover incorretamente leitura mutável para worker.
3. `VoxelChunk::empty()` ainda aloca blocks, fluids e light. Candidato seguro: compartilhar apenas buffers vazios de blocks/fluids via `OnceLock<Arc<[...]>>` + COW; manter light exclusivo porque initial lighting o reescreve.
4. `BiomeInfluence` nasce de um índice de surface biome, mas generation columns/structure support convertem ID de volta para índice; carregar o índice junto pode remover buscas lineares por string. Auditar todos os consumers antes para não criar API desnecessária/dead code.
5. Rebuild completo da seleção ao cruzar chunk ainda reconstrói/prioriza o conjunto desejado; só atacar com profiling/invariant melhor, pois o cache de `surface_range` já evita recomputações óbvias.

---

# Bugs/produto fora do refactor atual

Itens históricos conhecidos, só retomar quando o usuário priorizar ou quando o runtime indicar regressão relacionada:

- ghost block / held block deve refletir a hotbar como observer; transparência é do bloco inteiro, não por face;
- hydrology river/lake margin e água escapando da margem já foram relatados;
- dye anteriormente estava fraco demais;
- iluminação/sombreamento teve regressões pós-09/09 e foi revertida; evitar mudanças visuais casuais no lighting sem necessidade;
- terrain generation ainda foi relatada como lenta em movimento mesmo após melhorias anteriores.

---

# Próximos passos

Se nenhum runtime error/warning tiver prioridade:

1. Partir do HEAD real após este handoff e manter `VERSION = 0.14.30` até o próximo bloco de código.
2. Investigar e, se seguro, implementar shared COW de blocks/fluids vazios em `VoxelChunk::empty()` como próximo patch de performance; light permanece exclusivo.
3. Em seguida, medir estaticamente opções para reduzir custo de `ChunkMeshSnapshot::capture` sem quebrar revision/dependency semantics.
4. Auditar a possibilidade de carregar `surface_biome_index` junto de `BiomeInfluence` e eliminar ID->index lookups apenas se todos os consumers continuarem coerentes.
5. Tratar a divergência `VERSION 0.14.x` vs `Cargo.toml 0.10.16` em bloco explícito separado; não misturar com refactor de runtime.
6. Continuar atualizando este handoff após cada bloco material.

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados;
- reduzir custo indivisível dentro de um budget, não apenas adicionar mais budgets em volta;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- stack/reuse para scratch estruturalmente limitado;
- evitar scans globais, allocations temporárias e mutações idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative.
