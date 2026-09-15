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
- Só mencionar validação quando necessária para interpretar error/warning, confirmar comportamento de runtime ou decidir próximo passo.
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

`5e3b8455b9e574e5b2de8698c5b92fe7564d544e`

Commit: `Return prior cell from block mutation`

`VERSION`: `0.12.62`

Sempre buscar HEAD/VERSION novamente antes de escrever, porque podem ter avançado.

---

# Estado do refactor

A auditoria arquitetural foi reiniciada a partir de `0.12.5`/`0.12.8` e segue ativa. O roadmap atual vem do canon + inspeção real do código; backlog antigo não deve ser seguido cegamente.

## Blocos concluídos — resumo histórico

### 0.12.6–0.12.12 — filas, snapshots e async bootstrap
- Streaming passou a usar `DeduplicatedQueue<IVec3>`.
- Registries usados em generation tasks ficaram clonáveis estruturalmente.
- `ChunkTaskQueue<T>` centraliza lifecycle de tasks com coord/revision.
- Generation e initial meshing foram movidos para tasks; integração main-thread passou a ser budgetada.

### 0.12.13–0.12.24 — UI/HUD change-driven e budgets de runtime
- Inventory/settings/seed/TPS/buttons/cosmic stars/scrollbar deixaram de produzir dirty writes/rebuilds desnecessários.
- Targeting, biome, sky, celestial, shadows e HUDs passaram a publicar apenas mudanças semânticas.
- Lighting/fluid idle paths foram reduzidos; unload limpa filas de remesh; stale async completions contam no budget.

### 0.12.25–0.12.35 — metadata, halo e lighting vertical
- `VoxelChunk` mantém occupancy/boundary metadata e `VoxelWorld` mantém índice vertical por coluna XZ.
- Block edits idênticos viram no-op; cache hits e remesh hot paths perderam alocações/scans evitáveis.
- Halo de mesh passou a capturar somente shell real de 1.736 posições.
- Skylight seed e direct skylight cache passaram a trabalhar apenas no range vertical necessário.

### 0.12.36–0.12.43 — snapshots COW, fluid tick e contexts estreitos
- Buffers de chunk usam `Arc<[...]>` com copy-on-write.
- Chunks vazios não ocupam mesh task.
- Fluid simulation é gated por world tick.
- Concurrent caches deduplicam factories por key.
- Setup/streaming `SystemParam`s foram estreitados.
- Fluid frontier usa metadata de face para evitar scans de boundary desnecessários.

### 0.12.44–0.12.52 — owners compartilhados e leituras coesas
- Lighting remesh reutiliza geometry refresh diretamente.
- Teardown de render allocation foi centralizado.
- `DeduplicatedQueue` deixou de usar full `retain` para remoção/promoção.
- `VoxelRead::sample_at` virou primitive coesa block/fluid/light.
- AO/lighting passou de 13 para 9 amostras únicas por face.
- Fluid surface height compartilha neighborhood; solver/frontier usam leituras coesas.
- `HANDOFF.md` tornou-se a fonte persistente canônica do estado do projeto.

## Blocos recentes detalhados

### 0.12.53–0.12.58 — meshing, render refresh e lighting hot paths
- Surface block light é lida uma vez por voxel-fonte no meshing.
- Propagação de iluminação reutiliza sample atual e seis luzes vizinhas entre sky/block.
- Terrain/fluid topology refresh preserva a metade não afetada da render allocation.
- Direct seed usa storage local do chunk e elimina 8.192 resoluções de posição/chunk por seed.
- Lighting-only attribute split continua rejeitado porque block light pode alterar a diagonal/índices.

### 0.12.59 — promoção/remoção de fila amortizada O(1)
- `DeduplicatedQueue` usa `HashMap<T, generation>` e `(value, generation)` no `VecDeque`.
- Promoção publica nova geração; ocorrência anterior vira tombstone.
- `remove` invalida membership em O(1); `pop` ignora tombstones.
- Compactação periódica limita lixo sem full scan por operação.

### 0.12.60 — misses de renderabilidade não reescaneiam fila estável
- `DeduplicatedQueue` expõe revision lógica; `ChunkRenderPool` mantém `membership_revision`.
- `pop_renderable*` memoriza `(queue_revision, pool_membership_revision)` após miss completo.
- Enquanto fila e membership não mudarem, frames seguintes não revarrem a fila.
- Qualquer mudança real nos owners invalida naturalmente o miss, sem descartar trabalho adiado.

### 0.12.61 — full relight volume somente para mudança real de fonte
- O histórico confirmou que o Manhattan footprint de raio 15 foi introduzido para corrigir recoloração de fontes: canais antigos podem se sustentar mutuamente no campo incremental.
- `PendingLightingUpdates` preserva o primeiro `VoxelCell` anterior por posição até processar os edits.
- A emissão HSI anterior/atual é comparada via `block_emission_for_cell`.
- O footprint de até 4.991 posições só é enfileirado em mudança não-zero -> não-zero da emissão.
- Placement, remoção e edits que não alteram emissão usam somente a fronteira incremental normal.
- O fix histórico de recoloração permanece preservado.

### 0.12.62 — mutação de bloco reaproveita a leitura anterior
- `VoxelWorld::set_block_at_with_previous` devolve `(chunk_coord, previous_cell)` usando a leitura que a própria mutação já precisa fazer.
- `VoxelMutationRuntime` deixa de fazer `cell_at(world_position)` antes de `set_block_at`, removendo uma segunda resolução de posição/chunk por edit.
- `set_block_at` público preserva seu contrato existente e delega à operação detalhada.
- Teste cobre retorno correto do voxel anterior.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` e `ChunkMeshTasks`; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não transformar unload em pipeline incremental com snapshot temporal de streaming sem evidência de hotspot real.
- `ChunkContent`, `ChunkGeneration`, `CurrentDimensionContext`, `ChunkRenderer` e `ChunkUnloadRuntime` continuam coerentes enquanto representarem os concerns atuais.
- Não separar lighting remesh em simples atualização de atributos mantendo índices fixos: `should_flip_diagonal` também depende da block light.
- Não expor `VoxelWorld::chunk_mut` genericamente para otimizações bulk; mutações devem permanecer estreitas e ownership-aware.
- `DeduplicatedQueue` mantém FIFO/prioridade lógica através de generations/tombstones; não voltar a remoção linear por promoção sem benchmark/evidência.
- Miss caching de remesh depende somente de revisions dos owners reais: fila lógica + membership do render pool; não criar mirrors booleanos de “renderable”.
- Não remover o full emission footprint de recoloração sem substituir o mecanismo de invalidation de canais antigos; ele corrige stale colored light real.
- Não reabrir bugs antigos automaticamente; só se permanecerem ativos ou houver regressão reportada.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Continuar procurando chamadas repetidas de `VoxelWorld`/`VoxelRead` para a mesma posição nos hot paths restantes; priorizar raycast/collision/targeting e solver paths recorrentes, mas só consolidar quando múltiplos aspectos do mesmo voxel são realmente usados.
2. Revisar outros `pop_where`/predicate scans somente se ocorrerem em caminho recorrente; não trocar estrutura por preferência.
3. Inspecionar hot paths de player/world collision para scans ou lookups repetidos por frame antes de voltar a worldgen/cache.
4. Só reabrir separação específica de lighting/topology se existir representação que preserve mudanças de diagonal/índices sem duplicar ownership da geometria.
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
