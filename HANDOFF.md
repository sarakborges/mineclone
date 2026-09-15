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

`a19b8583a16d58422341ea5cea376790c98560ce`

Commit: `Relax empty chunks from lighting shell`

`VERSION`: `0.12.63`

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

### 0.12.36–0.12.52 — COW, async hygiene e leituras coesas
- Buffers de chunk usam `Arc<[...]>` com copy-on-write.
- Chunks vazios não ocupam mesh task; fluid simulation é gated por world tick.
- Concurrent caches deduplicam factories; setup/streaming contexts foram estreitados.
- Fluid frontier usa metadata de face.
- Lighting/remesh owners foram compartilhados e `VoxelRead::sample_at` virou primitive coesa.
- AO/lighting e fluid surface passaram a compartilhar neighborhoods.
- `HANDOFF.md` tornou-se fonte persistente canônica.

## Blocos recentes detalhados

### 0.12.53–0.12.58 — meshing, render refresh e lighting hot paths
- Surface block light é lida uma vez por voxel-fonte no meshing.
- Propagação de iluminação reutiliza sample atual e seis luzes vizinhas entre sky/block.
- Terrain/fluid topology refresh preserva a metade não afetada da render allocation.
- Direct seed usa storage local do chunk e elimina 8.192 resoluções de posição/chunk por seed.
- Lighting-only attribute split continua rejeitado porque block light pode alterar diagonal/índices.

### 0.12.59 — promoção/remoção de fila amortizada O(1)
- `DeduplicatedQueue` usa generations/tombstones em vez de `VecDeque::position + remove` em cada promoção.
- `remove` invalida membership em O(1); `pop` ignora tombstones; compactação periódica limita lixo.

### 0.12.60 — misses de renderabilidade não reescaneiam fila estável
- `DeduplicatedQueue` expõe revision lógica e `ChunkRenderPool` mantém `membership_revision`.
- `pop_renderable*` memoriza o par de revisions após um miss completo.
- Sem mudança nos owners, frames seguintes retornam em O(1) em vez de revarrer a fila inteira.

### 0.12.61 — full relight volume somente para mudança real de fonte
- O Manhattan footprint de raio 15 é mantido apenas para mudança de emissão HSI não-zero -> não-zero, preservando o fix histórico de recoloração/stale colored light.
- Placement, remoção e edits que não alteram emissão usam a fronteira incremental normal.
- `PendingLightingUpdates` preserva o primeiro estado anterior por posição para comparar emissão autoritativa antes/depois.

### 0.12.62 — mutação de bloco reaproveita a leitura anterior
- `VoxelWorld::set_block_at_with_previous` devolve `(chunk_coord, previous_cell)` da mesma leitura usada pela mutação.
- `VoxelMutationRuntime` deixa de fazer `cell_at` + `set_block_at` como duas resoluções separadas.
- O contrato público de `set_block_at` permanece intacto.

### 0.12.63 — chunks vazios relaxam lighting pela shell
- `VoxelChunk::is_empty()` confirma ausência tanto de blocks quanto de fluids.
- Depois do direct seed, um chunk vazio não precisa enfileirar seus 4.096 voxels internos: apenas a shell pode receber luz lateral/block light diretamente do exterior; mudanças na shell propagam para dentro.
- `LightingQueue::enqueue_chunk_boundary_voxels` enfileira os 1.352 voxels únicos da shell.
- Os 1.536 vizinhos externos continuam explicitamente enfileirados para que chunks já carregados possam receber skylight/block light do novo espaço mesmo se a shell interna não mudar.
- Por chunk vazio integrado: 5.632 posições potenciais -> 2.888, evitando 2.744 inserts antes da deduplicação.
- Chunks com conteúdo continuam usando relaxação completa.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` e `ChunkMeshTasks`; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não transformar unload em pipeline incremental com snapshot temporal de streaming sem evidência de hotspot real.
- `ChunkContent`, `ChunkGeneration`, `CurrentDimensionContext`, `ChunkRenderer` e `ChunkUnloadRuntime` continuam coerentes enquanto representarem os concerns atuais.
- Não separar lighting remesh em simples atualização de atributos mantendo índices fixos: `should_flip_diagonal` também depende da block light.
- Não expor `VoxelWorld::chunk_mut` genericamente para otimizações bulk; mutações devem permanecer estreitas e ownership-aware.
- `DeduplicatedQueue` mantém FIFO/prioridade lógica via generations/tombstones.
- Miss caching de remesh depende apenas de revisions dos owners reais: fila lógica + membership do render pool.
- Não remover o full emission footprint de recoloração sem mecanismo equivalente de invalidation de canais antigos.
- Empty-chunk lighting pode usar shell + vizinhos externos porque direct seed já cobre todo o interior e o chunk não possui medium/emitter interno.
- Não reabrir bugs antigos automaticamente; só se permanecerem ativos ou houver regressão reportada.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Auditar se a relaxação inicial de chunks **não vazios** pode usar metadata/invariants para evitar sempre enfileirar os 4.096 voxels, sem perder caves, emitters, fluid dampening ou lateral skylight. Não reduzir sem prova de corretude.
2. Continuar hot-path audit do streaming/integration: `sync_snapshot` já é change-driven e task polling é bounded a 8, então procurar custos maiores antes de micro-otimizar.
3. Collision/raycast/targeting atuais não mostraram lookup duplicado seguro: raycast faz uma leitura por voxel; swimming não pode reutilizar sample anterior porque há movimento horizontal entre sistemas.
4. Revisar outros predicate scans somente se recorrentes; não trocar estrutura por preferência.
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
