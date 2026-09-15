# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`
Branch de trabalho: `develop`
Stack: Rust + Bevy 0.19.0-dev

## Fonte canônica

Este arquivo, `HANDOFF.md` na raiz de `develop`, é a fonte canônica e persistente do handoff do projeto.

Atualizar o handoff significa atualizar **este arquivo**. Gerar, anexar ou enviar uma cópia `.txt`, markdown exportado ou download sem atualizar `HANDOFF.md` **não conta** como atualização do handoff. Cópias fora do repositório são apenas artefatos derivados para leitura/transferência e nunca substituem a fonte canônica.

Sempre que uma conversa produzir uma versão consolidada mais nova do handoff, consolidar primeiro aqui. Qualquer arquivo entregue ao usuário deve ser exportado a partir do conteúdo já consolidado nesta fonte.

## Regras de trabalho

- Trabalhar diretamente em `develop`.
- Não criar feature branch sem pedido explícito.
- Antes de alterar qualquer coisa, buscar o HEAD atual de `develop` e abrir os arquivos reais envolvidos.
- Fazer commits pequenos e coerentes.
- Não fazer mudanças arquiteturais não relacionadas ao problema/bloco atual.
- Não declarar bug como resolvido antes de confirmação em runtime quando a correção depender de comportamento visual/gameplay.
- Não gerar imagens a menos que o usuário peça explicitamente.
- Para assets binários enviados pelo usuário, usar exatamente os arquivos fornecidos; não recriar ou estilizar.

## Versionamento — obrigatório

O arquivo raiz `VERSION` é a fonte autoritativa da versão do projeto.

Todo bloco coerente deve subir a versão:

- `patch`: correções, refactors, otimizações internas e mudanças compatíveis sem nova feature relevante;
- `minor`: nova feature compatível;
- `major`: mudança incompatível/quebra deliberada de contrato.

O bump faz parte do próprio bloco. Não considerar um bloco fechado enquanto `VERSION` não tiver sido atualizado.

## Handoff — obrigatório

Atualizar `HANDOFF.md` sempre que houver mudança material no estado do projeto, arquitetura, roadmap, versão, HEAD relevante, regras de trabalho ou próximos passos.

Não acumular backlog histórico obsoleto. Bugs antigos só permanecem se ainda estiverem ativos ou se o usuário reportar regressão.

Antes de encerrar uma sequência longa, conferir se o handoff representa corretamente versão/HEAD de código, blocos concluídos, bloco atual, riscos conhecidos e próximo passo recomendado.

O campo de HEAD deste handoff deve rastrear o último commit de **código/version** relevante, não o commit do próprio `HANDOFF.md`, evitando autorreferência infinita.

## Validação / comunicação

- Não rodar `cargo check` como rotina do projeto.
- A validação prática é feita pelo usuário com `cargo run` quando necessário.
- Se o usuário enviar output de compilação/runtime, corrigir todos os errors e warnings relacionados antes de continuar refactors maiores.
- Não repetir em toda resposta que `cargo check` não foi executado ou que estamos “aguardando cargo run”.
- Só mencionar validação quando necessária para interpretar erro/warning, confirmar comportamento de runtime ou decidir próximo passo.
- Comunicação direta: menos narração, mais mudança concreta.

---

# Canon arquitetural

`ARCHITECTURE.md` é o canon arquitetural atual.

Princípios principais:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair por semelhança superficial.
3. Preferir `SystemParam`s coerentes a bags gigantes de resources; manter contextos mutáveis estreitos.
4. Usar availability/run conditions canônicas.
5. UI compartilhada pertence a `src/ui`.
6. Targeting tem um target autoritativo e consumidores change-driven.
7. Filas deduplicadas usam `DeduplicatedQueue<T>`; regras de voxel ficam em `VoxelUpdateQueue`.
8. `FrameWorkBudget` é o primitive canônico para orçamento por tempo/quantidade.
9. Cores internas usam HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
10. Remover módulos/helpers que só encaminham chamadas sem possuir invariant real.
11. Evitar rebuilds/scans globais por frame quando existe sinal de mudança ou metadata derivada no owner correto.
12. Pipeline assíncrono canônico: `generation task -> integrate chunk -> initial lighting seed -> halo snapshot -> mesh task -> spawn render entities`.
13. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
14. Não trocar corretude de world data por performance aparente; mover/stagear custo em vez disso.
15. Não criar nova abstração genérica acima de generation/mesh tasks quando o lifecycle comum já está em `ChunkTaskQueue` e os snapshots têm semânticas distintas.

---

# Estado atual do branch

Último HEAD de código/version confirmado antes desta gravação do handoff:

`64f5bd60559f5f335241a6e8b5204b98d387ac30`

Commit: `Reuse surface light across mesh faces`

`VERSION`: `0.12.53`

Sempre buscar HEAD/VERSION novamente antes de escrever, porque podem ter avançado.

---

# Estado do refactor

A auditoria arquitetural foi reiniciada a partir de `0.12.5`/`0.12.8` e segue ativa. O roadmap atual vem do canon + inspeção real do código; backlog antigo não deve ser seguido cegamente.

## Blocos concluídos

### 0.12.6 — streaming queues canônicas
- `ChunkStreamingState.pending`/`ready` -> `DeduplicatedQueue<IVec3>`.
- Membership O(1); `contains` e `From<Vec<T>>` adicionados.

### 0.12.7 — snapshots de registries
- `BiomeRegistry`/`StructureRegistry` clonáveis estruturalmente.
- Removidos rebuild helpers e `chunk_task_snapshots.rs`.

### 0.12.8 — lifecycle genérico de chunk tasks
- `ChunkTaskQueue<T>` centraliza `coord + revision + Task<T>`.
- Snapshots/limites continuam domain-specific.

### 0.12.9–0.12.12 — async bootstrap e budgets
- Generation/initial meshing off main thread.
- Polling sem vetores temporários.
- Integration budgetada por tempo/quantidade; asset/entity integration não fica ilimitada.

### 0.12.13–0.12.17 — UI change-driven
- Inventory/settings/seed/TPS/button animation/cosmic stars/scrollbar deixaram de produzir dirty writes/rebuilds desnecessários.

### 0.12.18–0.12.21 — targeting, biome, sky e HUD estáveis
- `TargetedBlock` e `CurrentBiome` só publicam mudanças semânticas.
- Environment/sky/celestial/shadows deixam de propagar mudanças falsas.
- HUD de tempo/coordenadas/dimensão/bioma só refaz texto quando entradas reais mudam.

### 0.12.22–0.12.24 — idle paths, unload/remesh e bootstrap budget
- Lighting/fluid idle paths reduzem execução sem trabalho.
- Unload limpa todas as filas de remesh.
- Stale async completions contam contra budget.

### 0.12.25–0.12.30 — metadata e hot-path hygiene
- `CurrentBiome` sem clone do snapshot anterior.
- `VoxelChunk` mantém occupancy/boundary metadata; `is_empty`/`has_fluid` e boundary checks O(1).
- `VoxelWorld` mantém índice vertical por XZ.
- Block edits idênticos viram no-op.
- Structure cache hit sem alocação de `String`.
- Remesh só cria `ChunkRenderContext` após achar trabalho.

### 0.12.31–0.12.33 — halo de mesh
- Leitura coesa block/fluid/light no halo.
- Snapshot guarda apenas a shell real: 1.736 posições, não cubo 18³.
- Capture itera diretamente a shell.

### 0.12.34–0.12.35 — lighting vertical
- Seed calcula apenas skylight necessário ao range do chunk.
- Direct skylight cache cresce lazy top-down até o menor Y consultado.

### 0.12.36 — chunk snapshots copy-on-write
- Buffers `blocks`/`fluids`/`light` em `Arc<[...]>`.
- `VoxelChunk::clone()` não deep-copia 4.096 entradas por canal; mutação preserva value semantics via COW.

### 0.12.37 e 0.12.41 — chunks vazios não ocupam mesh task
- Streaming e bootstrap integram chunks vazios diretamente no render pool depois de frontier/lighting necessários.
- Sem halo snapshot e sem ocupar slot async.

### 0.12.38 — fluid simulation gated por world tick
- `world_ticks_advanced` como run condition canônico.
- `process_fluid_updates` não entra em frames sem tick.

### 0.12.39 — concurrent cache factories deduplicadas
- `ConcurrentCache`/structure origin usam placeholder `OnceLock` por key.
- Mesma key não é recalculada simultaneamente por múltiplas generation tasks.

### 0.12.40 — setup SystemParam estreitado
- `WorldSetupRuntime` removido.
- `WorldSetupProgress` fica em `VoxelWorld + WorldLoadingState`; transition/fluid/tasks explícitos.

### 0.12.42 — streaming SystemParam dividido
- `ChunkStreamingRuntime` removido.
- Câmera/render distance explícitas.
- `ChunkStreamingWork` = world/state/tasks; `ChunkStreamingQueues` = remesh/fluid/lighting.

### 0.12.43 — fluid frontier usa metadata da face
- `boundary_has_fluid(direction)` evita scan de até 256 voxels quando a face relevante não contém fluido.

### 0.12.44 — wrapper de lighting remesh removido
- `refresh_chunk_lighting_mesh` removido; lighting reutiliza `refresh_chunk_geometry_mesh` diretamente.

### 0.12.45 — teardown de render allocation centralizado
- Refresh e unload compartilham `retire_chunk_render_allocation`.
- `ChunkRenderPool::take` volta a ser detalhe do owner.

### 0.12.46 — `DeduplicatedQueue` sem full retain scan
- `enqueue_front`/`remove` removem a única ocorrência diretamente em vez de `VecDeque::retain`.

### 0.12.47 — `VoxelRead::sample_at`
- `sample_at` é primitive coesa de block/fluid/light.
- `ChunkMeshSnapshot`, AO/lighting e fluid exposure reutilizam uma única amostra por posição quando precisam de múltiplos aspectos.

### 0.12.48 — AO/lighting usa 9 amostras únicas por face
- Quatro vértices compartilham center/4 sides/4 corners.
- 13 amostras por face -> 9, mantendo cálculo de AO/luz.

### 0.12.49 — fluid surface height compartilha neighborhood
- Quatro corners compartilham grades 3x3 no nível atual e acima.
- Até 32 leituras -> 18 por voxel de superfície.

### 0.12.50 — fluid solver usa leitura coesa
- Target amostrado uma vez; block+fluid passados juntos para `desired_fluid`.
- Support abaixo também usa uma amostra coesa.

### 0.12.51 — fluid frontier usa leitura coesa
- Spread target deixa de fazer loaded/solid/fluid como três consultas independentes.
- `sample_at` resolve tudo em uma leitura.

### 0.12.52 — handoff persistente no repositório
- `HANDOFF.md` passa a ser fonte canônica e persistente.
- `.txt`, anexos e downloads são derivados e não contam como atualização da fonte.
- Toda consolidação futura deve ser gravada aqui primeiro.
- HEAD do handoff rastreia último commit de código/version para evitar autorreferência.

### 0.12.53 — surface light reutilizada no meshing
- Terrain e fluid meshing deixam de consultar `VoxelRead::light_at` para o mesmo voxel-fonte a cada face exposta.
- A block light do voxel-fonte é lida uma vez do `VoxelChunk` já disponível e reutilizada por todas as faces daquele voxel.
- AO, amostras do neighborhood, block-light interpolation e escolha de diagonal continuam com a mesma semântica.
- A auditoria confirmou que um split ingênuo de “lighting-only attributes” não é seguro: block light participa de `should_flip_diagonal` quando AO empata, então mudança de iluminação pode alterar os índices da malha.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` e `ChunkMeshTasks` só porque ambos têm snapshot/revision; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não transformar unload em pipeline incremental com snapshot temporal de streaming sem evidência de hotspot real; isso adicionaria ownership cruzado/estado persistente para evitar scan ocasional.
- `ChunkContent`, `ChunkGeneration`, `CurrentDimensionContext`, `ChunkRenderer` e `ChunkUnloadRuntime` continuam coerentes enquanto representarem os concerns atuais.
- Não separar lighting remesh em simples atualização de atributos mantendo índices fixos: `should_flip_diagonal` também depende da block light e pode mudar a topologia indexada quando AO empata.
- Não reabrir bugs antigos automaticamente; só se permanecerem ativos ou houver regressão reportada.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Continuar procurando chamadas repetidas de `VoxelWorld`/`VoxelRead` para a mesma posição nos hot paths restantes e usar `sample_at` apenas quando múltiplos aspectos do mesmo voxel forem necessários.
2. Revisar `ChunkRenderPool::replace_*` e fallback para full refresh, procurando asset churn/rebuild evitável com invariant concreto.
3. Só reabrir uma separação específica de lighting/topology se existir representação que preserve também mudanças de diagonal/índices sem duplicar ownership da geometria.
4. Manter `DeduplicatedQueue` como owner de deduplicação/priority e atacar somente operações O(n) concretas em hot paths.
5. Só voltar a unload incremental, worldgen/cache ou task lifecycle se surgir evidência objetiva nova.
6. Rendering/HUD continuam change-driven; evitar micro-otimização por estética.

---

# Performance direction

Meta: ~60 FPS estáveis.

Direção adotada:

- heavy generation/mesh off main thread;
- integração budgetada;
- revision tracking para stale async work;
- caches no owner correto;
- metadata derivada mantida na fonte de mutação;
- evitar global per-frame scans;
- evitar temporários/alocações em hot paths;
- não substituir scans simples por cache externo com risco de staleness;
- preservar natural hydrology como geração autoritativa, sem reenfileirar água natural no solver dinâmico.

# Comunicação

- direta e focada em ação;
- não repetir caveats de `cargo check`/`cargo run`;
- não dizer “achamos a causa” sem evidência;
- quando houver bug real, corrigir e reportar de forma concisa;
- durante sequências longas, atualizar o usuário apenas em findings/blocos concluídos;
- atualizar `HANDOFF.md` depois de mudanças materiais.