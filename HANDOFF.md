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
- **A atualização do handoff faz parte do próprio bloco de trabalho. Não encerrar um bloco material nem responder como concluído deixando o `HANDOFF.md` desatualizado.**
- Se o último bloco ainda estiver aguardando CI, registrar isso no handoff como pending; quando o CI fechar ou houver fix subsequente, atualizar o handoff novamente.
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Corrigir todos os warnings de Rust encontrados nos blocos tocados.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando o comportamento depender disso.
- Não gerar imagens sem pedido explícito.
- Comunicação direta: menos narração, mais mudança concreta; quando o usuário disser `go`/`continua`, executar o próximo bloco aplicável sem pedir confirmação desnecessária.
- Não ficar repetindo que `cargo check` não foi rodado, que está esperando `cargo run` ou equivalente. O CI canônico já valida Clippy + check; runtime do usuário só é citado quando realmente necessário para comportamento visual/gameplay.
- `cargo test` só é rodado manualmente sob pedido explícito do usuário.
- `cargo fmt`/`rustfmt` não é gate do projeto.
- Commit exclusivamente documental de `HANDOFF.md` não sobe `VERSION`.

## Versionamento

- Fonte operacional acordada para os blocos do projeto: arquivo raiz `VERSION`.
- Estado atual: `VERSION = 0.14.40`.
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

Último HEAD de **código** publicado neste bloco:

`da309b3a0bfa385afc2b15900358c46a301b456c`

Bloco: `Use Bevy hash collections in lighting`

`VERSION`: `0.14.40`

Observação operacional: houve commits documentais acidentais criando/removendo arquivos temporários vazios durante a atualização anterior; o estado final não mantém `noop` nem `HANDOFF.md.tmp`. Eles não alteram código nem `VERSION`.

Commits recentes relevantes:

- `e22c888` — `0.14.31`, `VoxelChunk::empty()` compartilha buffers vazios de blocks/fluids via `OnceLock<Arc<_>>`; light permanece exclusivo.
- `ba23847` — `0.14.32`, `DeduplicatedQueue` usa `HashMap::entry` para evitar lookups duplicados em enqueue/pop.
- `0aaefe8` — `0.14.33`, `DeduplicatedQueue` troca o `std::HashMap` por `bevy::platform::collections::HashMap`.
- `7b64f5b` — `0.14.34`, `VoxelWorld` usa `bevy::platform` para os `HashMap/HashSet` quentes; `BTreeSet` de colunas permanece `std` porque ordenação é semântica.
- `383ae33` — `0.14.35`, `LightingContext` usa `bevy::platform::collections::HashMap`.
- `e4ebe14` — `0.14.36`, propagation reutiliza o chunk central para ler vizinhos cardinais locais; só cruza de volta para `VoxelWorld` quando o vizinho sai do chunk.
- `ed45170` — `0.14.37`, `ChunkMeshSnapshot::capture` constrói o shell de 1736 samples diretamente com `Vec::with_capacity`, sem zerar/prefill o buffer inteiro antes de preenchê-lo.
- `d5915fa` — `0.14.38`, solver de lighting escreve luz imediatamente mas posterga o bump de `chunk_mesh_revision`, fazendo um bump por chunk alterado no fim do batch.
- `e54afd6` — `0.14.39`, remove `set_light_at` direto que ficou sem caller após o batching de revisions; mantém apenas a rota interna deferida usada pelo solver.
- `da309b3` — `0.14.40`, troca as hash collections quentes restantes de lighting para `bevy::platform`, incluindo pending emission edits, changed chunks da propagation e scratch local do dynamic lighting.

## CI recente

- `0.14.30` / run `35031098600`: Clippy **success**, `cargo check` **success**.
- `0.14.34` / run `35033347328`: Clippy **success**, `cargo check` **success**.
- `0.14.35` / run `35034048076`: Clippy **success**, `cargo check` **success**.
- `0.14.36` / run `35034426624`: Clippy **success**, `cargo check` **success**.
- `0.14.37` / run `35034814641`: Clippy **success**, `cargo check` **success**.
- `0.14.38` / run `35035219346`: **failure** de Clippy porque `set_light_at` ficou `dead_code` depois que o solver passou a usar a rota deferida.
- `0.14.39` / run `35035772978`: Clippy **success**, `cargo check` **success**; corrige a única falha de `0.14.38` sem `allow(dead_code)`.
- `0.14.40` / run `35036777258`: **pending/in progress** no momento desta atualização do handoff.

Falhas históricas que não devem ser reintroduzidas:

- `0.14.16/17`: `TimeHudText.presented_time` usava `u32` para dia; `WorldClock.day` é `u64`.
- `0.14.19/20`: `QueueRebuildScratch` privado apareceu na assinatura pública(super) de `stream_chunks` e elevou a função a 8 argumentos; resolvido com `ChunkStreamingSelection` SystemParam em `0.14.21`.
- `0.14.25`: structure support ainda chamava assinatura antiga de surface carver; corrigido em `0.14.26`.
- `0.14.38`: não manter API pública/interna órfã só para preservar shape anterior; remover ou ter caller real.

---

# Drafts preparados, mas NÃO publicados

Estes commits foram montados fora de `develop` antes desta atualização de handoff. Como o branch avançou, **não fazer fast-forward direto neles**; reconstruir/reencadear sobre o HEAD atual antes de publicar.

- draft `bd19088` — planejado como `0.14.41`: troca `desired/retained/surface_ranges` do streaming para `bevy::platform` e desacopla `WorldFeatureFields::retain_for_chunks` do tipo concreto de `HashSet`, aceitando iterador de coords.
- draft antigo `9c1a0e7` da seleção de streaming está descartado porque tinha incompatibilidade de tipo com `FeatureCaches::retain_for_chunks`.
- drafts anteriores de `0.14.36/37` que precederam a reconstrução corrigida também estão descartados; os commits canônicos são `e4ebe14` e `ed45170`.

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
- `VoxelChunk::empty()` compartilha storage vazio de blocks/fluids entre chunks com COW natural de `Arc::make_mut`; light continua storage próprio.
- `DeduplicatedQueue` evita hash lookup duplicado nos caminhos de enqueue/pop e usa o hasher do Bevy.
- `VoxelWorld` usa hash collections do Bevy para mapas/sets sem requisito de ordenação.

## Worldgen allocations

- Feature cache retention reutiliza quatro HashSets internos sob `Mutex` compartilhado pelas clones de `WorldFeatureFields`.
- `SurfaceCarverColumn` e `SurfaceMaterialColumn` reutilizam seus Vecs por coluna dentro de cada chunk.
- `BiomeField::sample_surface` tem limite estrutural derivado de `SITE_SEARCH_RADIUS`: 25 sites + macro biome = máximo 26 influências. O sample temporário usa `ArrayVec` e não aloca heap.
- `GenerationColumnSample` não usa array fixa de 26 porque isso inflaria os 256 samples persistentes; usa `SmallVec` inline para o caso comum e spill quando necessário.
- Structure support reutiliza a própria amostra de biome para `surface_height_from_sample`, removendo uma segunda amostragem completa.

## Lighting / mesh snapshot

- `seed_chunk_direct_lighting` pula upper chunks vazios antes do scan voxel a voxel; chunks não vazios mantêm a mesma semântica de skylight/dampening.
- `LightingContext` usa o hasher do Bevy.
- Pending emission edits, changed chunks da propagation e o scratch local de dynamic lighting usam `bevy::platform` hash collections.
- Propagation usa o próprio `VoxelChunk` para os vizinhos cardinais que permanecem dentro do chunk; lookup global só acontece quando cruza a borda.
- O early-return de medium opaco permanece antes das leituras cardinais, então a otimização não adiciona trabalho a voxel opaco.
- Mudanças de luz do solver continuam sendo aplicadas imediatamente; só o `chunk_mesh_revision` é consolidado por chunk ao fim do batch.
- `ChunkMeshSnapshot::capture` ainda materializa `SHELL_VOLUME = 1736` samples, mas não faz mais prefill + overwrite; cada slot é produzido uma vez em ordem compacta.
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
- `ChunkTaskQueue` tem poucos entries por design; trocar hasher ali não é prioridade sem profiling.
- A expansão inicial de 4096 seeds de lighting por chunk é um alvo real, mas mexe na ordem/latência de propagação. Não transformar em incremental sem preservar explicitamente a ordem de trabalho e a percepção de chunk pronto.

---

# Hot paths ainda abertos

Prioridade ligada ao relato de FPS caindo ao andar/carregar chunks:

1. **Após `0.14.40` ficar verde, reencadear e validar o draft de streaming selection hash collections** como próximo bloco (`0.14.41`), mantendo `retain_for_chunks` desacoplado do tipo concreto de set.
2. `ChunkRenderPool.active` ainda usa `std::HashMap`; é mapa grande e recebe `contains/get_mut/remove/insert` durante streaming/remesh. Candidato seguro seguinte se o diff ficar apenas em hasher.
3. `ChunkMeshSnapshot::capture` ainda faz 1736 amostragens síncronas por task inicial; o prefill já foi removido. Próximo ganho precisa reduzir lookups/amostragens ou mudar ownership com cuidado, não apenas trocar a forma de alocar.
4. Integração de chunk ainda expande 4096 posições para lighting no caminho main-thread. Investigar uma representação de seed por chunk/coluna ou expansão incremental que preserve semântica e prioridade.
5. Rebuild completo da seleção ao cruzar chunk ainda reconstrói/prioriza desired/pending; depois do hasher, só atacar algoritmo com profiling/invariant melhor.
6. `BiomeInfluence` nasce de índice de surface biome, mas alguns consumers ainda convertem ID de volta para índice; carregar o índice junto é otimização menor e só deve entrar depois dos hot paths de streaming.
7. Tratar `VERSION 0.14.x` vs `Cargo.toml 0.10.16` em bloco explícito separado; não misturar com runtime performance.

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

1. `0.14.40` está publicado em `develop`; CI run `35036777258` está pending/in progress nesta atualização.
2. Quando `0.14.40` ficar verde, recriar/reencadear `bd19088` sobre o HEAD real e publicar como `0.14.41`, incluindo a API genérica de retention para não acoplar `WorldFeatureFields` a um hasher específico.
3. Validar `0.14.41` no CI antes do bloco seguinte.
4. Depois da cadeia verde, auditar `ChunkRenderPool.active` para Bevy hash map e então voltar aos custos estruturais maiores: snapshot shell e seed expansion de lighting.
5. Atualizar este handoff ao final de cada bloco material, sem postergar para uma conversa futura.

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
