# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`
Branch de trabalho: `develop`
Stack: Rust + Bevy 0.19.0-dev

## Fonte canônica

Este arquivo, `HANDOFF.md` na raiz de `develop`, é a fonte canônica e persistente do handoff do projeto.

Atualizar o handoff significa atualizar **este arquivo**. Cópias `.txt`, exports ou anexos são apenas artefatos derivados e não substituem esta fonte.

## Regras de trabalho

- Trabalhar diretamente em `develop`.
- Não criar feature branch sem pedido explícito.
- Antes de alterar código, buscar o HEAD atual de `develop` e abrir os arquivos reais envolvidos.
- Fazer commits pequenos e coerentes.
- Não fazer mudanças arquiteturais não relacionadas ao bloco atual.
- Não declarar bug visual/gameplay resolvido sem evidência de runtime quando a correção depender desse comportamento.
- Não gerar imagens a menos que o usuário peça explicitamente.
- Para assets binários enviados pelo usuário, usar exatamente os arquivos fornecidos.

## Versionamento — obrigatório

O arquivo raiz `VERSION` é a fonte autoritativa.

Todo bloco coerente deve subir versão:

- `patch`: fixes/refactors/otimizações internas compatíveis;
- `minor`: nova feature compatível;
- `major`: mudança incompatível/breaking.

O bump faz parte do bloco; não considerar o bloco fechado antes de atualizar `VERSION`.

## Handoff — obrigatório

Atualizar `HANDOFF.md` sempre que houver mudança material em estado, arquitetura, roadmap, versão, HEAD relevante, regras ou próximos passos.

Não acumular backlog histórico obsoleto. Bugs antigos só permanecem se ainda estiverem ativos ou se houver regressão reportada.

O HEAD registrado aqui deve apontar para o último commit de **código/version**, não para o commit do próprio handoff.

## Validação / comunicação

- Não rodar `cargo check` como rotina do projeto.
- Se o usuário enviar output de compilação/runtime, corrigir todos os errors e warnings relacionados antes de continuar refactors maiores.
- Não repetir em toda resposta que `cargo check` não foi executado ou que estamos esperando `cargo run`.
- Comunicação direta: menos narração, mais mudança concreta.

---

# Canon arquitetural

`ARCHITECTURE.md` é o canon arquitetural atual.

Princípios principais:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair por semelhança superficial.
3. Preferir `SystemParam`s coerentes a bags gigantes; manter contextos mutáveis estreitos.
4. Usar availability/run conditions canônicas.
5. UI compartilhada pertence a `src/ui`.
6. Targeting tem um target autoritativo e consumidores change-driven.
7. Filas deduplicadas usam `DeduplicatedQueue<T>`; regras de voxel ficam em `VoxelUpdateQueue`.
8. `FrameWorkBudget` é o primitive canônico para orçamento por tempo/quantidade.
9. Cores internas são HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
10. Remover helpers/módulos que só encaminham chamadas e não possuem invariant.
11. Evitar scans/rebuilds globais por frame quando existe sinal de mudança ou metadata no owner correto.
12. Pipeline async inicial canônico: generation task -> integrate chunk -> initial lighting seed -> halo snapshot -> mesh task -> spawn render entities.
13. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
14. Não trocar corretude do mundo por performance aparente; mover/stagear custo.
15. Não criar abstração genérica acima de generation/mesh tasks quando o lifecycle comum já está em `ChunkTaskQueue`.
16. Background remesh também é async; feedback imediato de edits/lighting pode continuar síncrono quando necessário.

---

# Estado atual do branch

Último HEAD de código/version confirmado antes desta gravação do handoff:

`74df7605481473f76fbe54aa4c970c4cc18a7e81`

Commit: `Move background chunk remesh off main thread`

`VERSION`: `0.12.84`

Sempre buscar HEAD/VERSION novamente antes de escrever código.

---

# Estado do refactor

A auditoria arquitetural/performance segue ativa. O roadmap vem do canon + inspeção real do código, não de backlog antigo seguido cegamente.

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
- Leituras block/fluid/light consolidadas em `sample_at`.
- `HANDOFF.md` virou fonte persistente canônica.

### 0.12.53–0.12.72 — meshing/render/lighting hot paths
- Surface block light reaproveitada por voxel-fonte no meshing.
- Lighting propagation reutiliza sample atual e seis luzes vizinhas.
- Terrain/fluid topology refresh preserva a metade não afetada da render allocation.
- Direct seed usa storage local do chunk e upper chunks resolvidos uma vez.
- `DeduplicatedQueue` usa generations/tombstones; miss de remesh é revision-cached.
- Full footprint Manhattan de emissão fica reservado a recoloração/intensidade não-zero -> não-zero.
- Empty chunk lighting usa shell + vizinhos externos.
- Fluid frontier usa metadata de dynamic fluid por boundary.
- `VoxelChunk::rebuild_light` faz um único COW do buffer.
- `ChunkMeshSnapshot::capture` resolve no máximo os 26 chunks vizinhos e mantém shell compacta de 1.736 samples.

### 0.12.73–0.12.79 — COW/batch/archive/samples locais
- `VoxelChunkContentMut` faz edits batch usando os mesmos helpers de metadata dos setters normais.
- Worldgen, structures e archive restore usam batch mutation.
- Archive encode/restore percorrem 4.096 slots uma vez, não block/fluid em passagens separadas.
- Halo usa um único `Box<[ShellSample]>`.
- Chunk vazio recebe direct seed por 256 colunas + cópia de layers.
- Direct skylight e mutações usam `sample_local` para leituras coesas block/fluid/light.

### 0.12.80 — unload dirigido pelo delta do streaming
- `ChunkUnloadState` não reconstrói mais uma lista global de chunks carregados a cada mudança de seleção.
- `ChunkStreamingState` mantém uma fila `retired` deduplicada.
- Cada rebuild promove para `retired` apenas a geração antiga que saiu tanto de `desired` quanto de `retained`.
- O scan global existe apenas no bootstrap para compatibilidade com chunks já residentes.

### 0.12.81–0.12.82 — poda de caches fora do passo de 1 chunk
- `WorldFeatureFields::retain_for_chunks` deixa de varrer seis caches em toda travessia de chunk.
- Feature caches são podados no bootstrap, mudança de render distance ou cruzamento de generation region.
- `surface_ranges.retain` segue a mesma cadência.
- `GENERATION_REGION_SIZE_CHUNKS = 8`; entre podas, retenção extra é apenas política de memória e não altera conteúdo gerado.

### 0.12.83 — revision tracking autoritativo para mesh snapshots
- `VoxelWorld` mantém revisão de mesh por chunk residente.
- Insert/restore/content edit/fluid edit/light edit/rebuild de light atualizam a revisão; unload remove a revisão residente.
- `ChunkMeshSnapshot` captura presença + revisão dos 27 chunks do cubo 3×3×3.
- `ChunkMeshDependencies::is_current` detecta tanto mutação de chunk existente quanto aparecimento/desaparecimento de vizinho.
- Initial mesh async descarta resultado stale e retorna o coord para `ready` para recapturar halo atual.

### 0.12.84 — background remesh fora da main thread
- Novo `ChunkRemeshTasks` reutiliza `ChunkTaskQueue` e `MeshContentSnapshot`.
- No máximo 4 remesh tasks ficam em voo; dispatch e integração são budgetados e limitados por frame.
- Background terrain/fluid remesh usa `ChunkMeshSnapshot` + `ChunkMeshDependencies` e roda no `AsyncComputeTaskPool`.
- Resultado stale por content revision ou por qualquer dependência do halo é re-enfileirado no mesmo tipo.
- Resultado de chunk descarregado ou sem render allocation é descartado.
- Aplicação de resultado continua usando partial refresh: terrain preserva fluid allocation; fluid preserva terrain allocation.
- `process_immediate_geometry_remesh` e `process_immediate_lighting_remesh` continuam síncronos para feedback imediato de gameplay.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` / `ChunkMeshTasks`; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não separar lighting remesh em atributos com índices fixos: block light participa de `should_flip_diagonal` e pode mudar topologia indexada.
- Não expor `VoxelWorld::chunk_mut` genericamente; mutações devem permanecer estreitas e ownership-aware.
- `DeduplicatedQueue` mantém FIFO/prioridade via generations/tombstones.
- Miss caching de remesh depende das revisions dos owners reais.
- Não remover full emission footprint de recoloração sem invalidation equivalente dos canais antigos.
- Empty-chunk lighting pode usar shell + vizinhos externos; chunks com conteúdo não podem ser reduzidos à shell sem frontier interna provada.
- Natural hydrology continua source/static; não reenfileirar água natural no solver dinâmico.
- Loading de chunk vazio só exige neighbor fluid remesh quando a face compartilhada possui fluido.
- Edits de medium/fluid não pertencem ao tracking de emissão de bloco.
- Não criar bitset de boundary enquanto contadores + early exit resolverem o hotspot.
- Rebuilds integrais de buffers COW devem obter mutable storage uma vez no owner.
- Batch content edit pertence a `VoxelChunk`; não criar builder externo com invariants duplicados.
- Halo de mesh deve resolver chunks vizinhos por shell, não voltar a lookup world-position por voxel.
- Unload backlog deve vir do delta do owner de seleção, não de scan global duplicado.
- Cache pruning não precisa acompanhar cada chunk do player; manter granularidade coerente com generation regions.
- Qualquer remesh async deve validar presença/revisão de todo o halo antes de aplicar resultado.
- Background remesh é async; caminhos immediate só devem ser movidos se houver evidência de que latência extra é aceitável.
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Auditar o novo `ChunkRemeshTasks` por oportunidades de coalescer geometry/fluid work enquanto uma task do mesmo coord já está em voo, sem perder a regra de supersedência de geometry.
2. Revisar se `process_immediate_lighting_remesh` pode gerar bursts síncronos durante streaming/lighting; só mover ou reduzir se houver forma de preservar feedback e convergência visual.
3. Revisar `ChunkMeshDependencies`/revisions para garantir que bumps em operações batch reflitam mudança real quando isso for relevante; não adicionar hashing ou scans de 4.096 voxels para evitar um bump barato.
4. Continuar inspeção objetiva de sistemas `Update`/`PostUpdate` por scans globais ou builds síncronos; não voltar a micro-otimização de helpers já enxutos.
5. Manter `notify_loaded_chunk_neighbors` não-vazio conservador enquanto metadata atual não provar sobreposição voxel-a-voxel.
6. Só voltar a unload incremental, collision/raycast ou task lifecycle se surgir evidência objetiva nova.

---

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh background fora da main thread;
- integração budgetada;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar scans globais por frame e allocations temporárias em hot paths;
- não trocar corretude por performance aparente;
- natural hydrology continua generation-authoritative.

# Comunicação

- direta e focada em ação;
- não repetir caveats de `cargo check`/`cargo run`;
- não dizer “achamos a causa” sem evidência;
- durante sequências longas, atualizar apenas findings/blocos concluídos;
- atualizar `HANDOFF.md` depois de mudanças materiais.
