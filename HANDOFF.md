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

`234adae1e0084b6416fc1fd978b588f59dc689f9`

Commit: `Separate fluid lighting invalidation from block edits`

`VERSION`: `0.12.69`

Sempre buscar HEAD/VERSION novamente antes de escrever, porque podem ter avançado.

---

# Estado do refactor

A auditoria arquitetural segue ativa; o roadmap vem do canon + inspeção real do código, não de backlog antigo seguido cegamente.

## Resumo histórico

### 0.12.6–0.12.24
- Filas deduplicadas, snapshots clonáveis e lifecycle async de generation/mesh.
- Integração main-thread budgetada.
- UI/HUD/targeting/environment passaram a ser change-driven.
- Lighting/fluid idle paths e unload/remesh foram reduzidos/budgetados.

### 0.12.25–0.12.52
- Metadata de occupancy/boundaries no `VoxelChunk`; índice vertical no `VoxelWorld`.
- Halo de mesh reduzido à shell real; skylight vertical lazy.
- Chunk buffers COW com `Arc<[...]>`.
- Chunks vazios pulam mesh task; fluid solver gated por tick.
- Contexts de setup/streaming estreitados; caches concorrentes deduplicados.
- Leituras block/fluid/light consolidadas em `sample_at`; AO/fluid neighborhoods compartilhados.
- `HANDOFF.md` virou fonte persistente canônica.

## Blocos recentes detalhados

### 0.12.53–0.12.58 — meshing/render/lighting hot paths
- Surface block light é lida uma vez por voxel-fonte no meshing.
- Propagação reutiliza sample atual e seis luzes vizinhas entre sky/block.
- Terrain/fluid topology refresh preserva a metade não afetada da render allocation.
- Direct seed usa storage local do chunk e elimina 8.192 resoluções de posição/chunk por seed.
- Lighting-only attribute split continua rejeitado porque block light pode alterar diagonal/índices.

### 0.12.59–0.12.60 — filas/remesh
- `DeduplicatedQueue` usa generations/tombstones; promotion/remove deixam de remover linearmente no deque.
- Revisions lógicas da fila + membership revision do render pool cacheiam misses de `pop_renderable*`, evitando re-scan por frame sem mudança.

### 0.12.61–0.12.63 — edits/lighting incremental
- Full footprint Manhattan de emissão fica reservado a mudança HSI não-zero -> não-zero.
- `VoxelWorld::set_block_at_with_previous` reaproveita a leitura anterior do voxel.
- Chunks vazios relaxam lighting pela shell interna + vizinhos externos, reduzindo a fila potencial de 5.632 para 2.888 posições.

### 0.12.64–0.12.66 — skylight chunk-local
- Direct seed resolve upper chunks uma vez e lê storage local top-down.
- `DirectSkyColumn` expande o cache lazy por segmentos de chunk mantendo um valor por world-Y inclusive em gaps.
- Wrappers world-position de dampening/emissão que ficaram obsoletos foram removidos.

### 0.12.67 — metadata de fluido dinâmico por boundary
- `VoxelChunk` mantém `boundary_dynamic_fluid_counts` no owner de occupancy/boundary metadata.
- `set_fluid` acompanha transições source ↔ dynamic mesmo sem mudança de occupancy.
- Fluid frontier rejeita em O(1) faces sem fluido dinâmico; margens contendo apenas água natural/source deixam de escanear até 256 voxels.

### 0.12.68 — empty-neighbor fluid remesh filtrado por boundary
- Quando um chunk vazio carrega, fluid remesh do vizinho só é enfileirado se a face compartilhada do vizinho realmente contém fluido.
- A invalidação necessária de `halo ausente -> ar carregado` é preservada, mas vizinhos sem água deixam de sofrer remesh inútil.

### 0.12.69 — invalidação de lighting por fluido separada de block edit
- Corrigida chamada stale do fluid solver à assinatura antiga de `PendingLightingUpdates::enqueue_voxel_edit`.
- `enqueue_medium_edit` invalida apenas a lighting frontier para mudança de meio/fluid.
- Tracking de `previous_cell` e full emission footprint permanecem exclusivos de edits de bloco, evitando misturar mudança de fluido com recoloração/emissão.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` e `ChunkMeshTasks`; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não transformar unload em pipeline incremental sem evidência de hotspot real.
- Não separar lighting remesh em atributos com índices fixos: block light também participa de `should_flip_diagonal`.
- Não expor `VoxelWorld::chunk_mut` genericamente; mutações devem permanecer estreitas.
- `DeduplicatedQueue` mantém FIFO/prioridade via generations/tombstones.
- Miss caching de remesh depende apenas de revisions dos owners reais.
- Não remover full emission footprint de recoloração sem invalidation equivalente de canais antigos.
- Empty-chunk lighting pode usar shell + vizinhos externos; chunks com conteúdo não podem ser reduzidos à shell sem frontier interna provada.
- Direct skylight caches devem preservar um valor por world-Y, mesmo em gaps verticais sem chunk carregado.
- Natural hydrology continua source/static; metadata de boundary deve distinguir source de fluido dinâmico para não reativar água natural no solver.
- Loading de chunk vazio só exige neighbor fluid remesh quando a face compartilhada possui fluido; halo ausente e ar carregado diferem no fluid mesher.
- Edits de meio/fluid não pertencem ao tracking de mudança de emissão de bloco.
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Continuar a auditoria de `enqueue_loaded_fluid_frontier`: após a rejeição O(1) por face, verificar se o scan de 256 voxels quando há fluido dinâmico pode usar metadata mais precisa sem inflar estado por chunk.
2. Revisar `notify_loaded_chunk_neighbors` do caminho não-vazio para invalidações conservadoras que possam ser filtradas com metadata existente, sem criar bitsets/caches externos sem evidência.
3. Procurar usos restantes de `cell_at`/`fluid_at`/`light_at` repetidos para a mesma posição em loops onde um chunk/sample já pode ser reutilizado.
4. Manter full relaxation em chunks com conteúdo até existir frontier interna correta para direct sky/emitter propagation.
5. Streaming selection/snapshots/task polling já estão bounded/change-driven; collision/raycast/targeting não mostraram lookup duplicado seguro.
6. Só voltar a unload incremental, worldgen/cache ou task lifecycle se surgir evidência objetiva nova.

---

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh off main thread;
- integração budgetada;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar global per-frame scans e temporários/alocações em hot paths;
- não trocar corretude por performance aparente;
- natural hydrology continua generation-authoritative, não reenfileirada no fluid solver dinâmico.

# Comunicação

- direta e focada em ação;
- não repetir caveats de `cargo check`/`cargo run`;
- não dizer “achamos a causa” sem evidência;
- durante sequências longas, atualizar o usuário apenas em findings/blocos concluídos;
- atualizar `HANDOFF.md` depois de mudanças materiais.
