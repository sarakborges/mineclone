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

`b528e866906e1d4fad16a0f02a78e4ca58b56032`

Commit: `Remove obsolete lighting world wrappers`

`VERSION`: `0.12.66`

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

### 0.12.61 — full relight volume somente para mudança real de fonte
- Footprint Manhattan de raio 15 fica reservado a mudança HSI não-zero -> não-zero, preservando fix histórico de recoloração.
- Placement, remoção e edits sem mudança de emissão usam frontier incremental.

### 0.12.62 — mutação de bloco reaproveita leitura anterior
- `VoxelWorld::set_block_at_with_previous` devolve chunk + cell anterior da mesma leitura usada para mutar.
- `VoxelMutationRuntime` elimina um segundo lookup do mesmo voxel por edit.

### 0.12.63 — chunks vazios relaxam lighting pela shell
- Empty chunk usa shell interna de 1.352 voxels + 1.536 vizinhos externos.
- Relax queue potencial cai de 5.632 para 2.888 posições por empty chunk antes da deduplicação.
- Chunks com conteúdo continuam com full relaxation para preservar caves, emitters, fluid dampening e lateral skylight internos.

### 0.12.64 — direct seed resolve upper chunks uma vez
- O scan de skylight acima do chunk resolve cada upper chunk carregado uma vez, pula gaps verticais inteiros e lê `cell/fluid` diretamente do `VoxelChunk` local.
- Para cada upper chunk completo, até 4.096 lookups de mundo viram um lookup de chunk + leituras locais.

### 0.12.65 — direct skylight cache expande por chunk
- `LightingContext::DirectSkyColumn` passa a percorrer segmentos verticais por `VoxelChunk` em vez de resolver o mundo a cada Y.
- Chunks ausentes continuam produzindo um nível por world-Y com skylight inalterado, preservando `levels_from_top` e gaps.
- Em segmento vertical completo, até 16 lookups de mundo por coluna viram um lookup de chunk + leituras locais.

### 0.12.66 — wrappers world-position de lighting removidos
- `medium_dampening(world, position)` ficou obsoleto após os scans chunk-local de 0.12.64/65 e foi removido.
- `block_emission(world, position)` também não tinha mais consumidor após a migração para `block_emission_for_cell`.
- Permanecem apenas os primitives cell/sample-based realmente usados, evitando forwarding morto/dead code.

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
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Auditar integração de chunks além de meshing/lighting, especialmente fluid frontier e neighbor notifications, procurando trabalho feito mesmo quando metadata já prova que não há conteúdo relevante.
2. Procurar usos restantes de `cell_at`/`fluid_at`/`light_at` repetidos para a mesma posição em loops onde um chunk/sample já pode ser reutilizado.
3. Manter full relaxation em chunks com conteúdo até existir frontier interna correta para direct sky/emitter propagation.
4. Streaming selection/snapshots/task polling já estão bounded/change-driven; collision/raycast/targeting não mostraram lookup duplicado seguro.
5. Só voltar a unload incremental, worldgen/cache ou task lifecycle se surgir evidência objetiva nova.

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
