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
16. Terrain/fluid/lighting remesh de background usa o pipeline async; apenas o remesh de geometry imediato de edição do jogador permanece síncrono.
17. Solvers dinâmicos caros devem ter teto temporal e de quantidade quando o trabalho puder variar muito por frame.

---

# Estado atual do branch

Último HEAD de código/version confirmado antes desta gravação do handoff:

`88d00f0330e450813d5034be19760860571b9a99`

Commit: `Bump version to 0.12.95`

Código do bloco 0.12.95:

- `25ca0c730a378379a2706ca509fc0ca8bf59fa21` — `Reuse lighting changed-chunk scratch storage`
- `2384a0aa2530df6b291e4e9fee585009ece12855` — `Retain lighting edit map capacity`
- `eddeace278a346e41fdadc9508fe643dc31b0a62` — `Reuse dynamic lighting chunk set`

`VERSION`: `0.12.95`

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

### 0.12.80–0.12.82 — streaming deltas e cache pruning
- Unload backlog vem do delta de `desired`/`retained`; scan global de chunks carregados fica só no bootstrap.
- Feature caches e `surface_ranges` são podados no bootstrap, mudança de render distance ou cruzamento de generation region, não em todo chunk atravessado.
- `GENERATION_REGION_SIZE_CHUNKS = 8`; retenção extra entre podas é apenas política de memória.

### 0.12.83 — revision tracking autoritativo para mesh snapshots
- `VoxelWorld` mantém revisão de mesh por chunk residente.
- Insert/restore/content edit/fluid edit/light edit/rebuild de light atualizam a revisão; unload remove a revisão residente.
- `ChunkMeshSnapshot` captura presença + revisão dos 27 chunks do cubo 3×3×3.
- `ChunkMeshDependencies::is_current` detecta mutação e aparecimento/desaparecimento de vizinho.
- Initial mesh async descarta resultado stale e recaptura halo atual.

### 0.12.84 — background remesh fora da main thread
- `ChunkRemeshTasks` reutiliza `ChunkTaskQueue` e `MeshContentSnapshot`.
- No máximo 4 remesh tasks ficam em voo; dispatch e integração são budgetados e limitados por frame.
- Terrain/fluid remesh usa `ChunkMeshSnapshot` + `ChunkMeshDependencies` no `AsyncComputeTaskPool`.
- Resultado stale por content revision ou qualquer dependência do halo é re-enfileirado no mesmo tipo.
- Resultado de chunk descarregado ou sem render allocation é descartado.
- Aplicação preserva partial refresh: terrain mantém fluid allocation; fluid mantém terrain allocation.

### 0.12.85 — async remesh sem head-of-line blocking
- Coord com task já em voo é deferido localmente em vez de causar `break` no dispatcher.
- Outros chunks continuam preenchendo slots async livres.
- Deferidos retornam à frente preservando a ordem relativa.

### 0.12.86 — lighting remesh também async
- `ChunkRemeshTaskKind` possui `Geometry`, `Lighting` e `Fluid`.
- Lighting remesh usa terrain build async e a mesma validação stale dos 27 chunks.
- Fila de lighting mantém prioridade própria; dispatcher atende Lighting -> Geometry -> Fluid.
- `process_immediate_lighting_remesh` foi removido.
- Apenas `process_immediate_geometry_remesh` continua síncrono para feedback imediato de edição/topologia.

### 0.12.87 — dynamic lighting com orçamento temporal
- Mantém teto de 4.096 voxels/frame.
- Orçamento temporal de 2 ms, mínimo 256 voxels e checagem a cada 64.
- `LightingContext` permanece vivo durante toda a chamada, preservando cache de direct-sky.
- O layer voxel recebe um predicate de orçamento; `FrameWorkBudget` continua no owner de world scheduling.

### 0.12.88 — fluid solver com orçamento temporal
- Mantém teto de 512 updates e 4 fluid steps/frame.
- Orçamento temporal de 1 ms, mínimo 64 updates.
- A fronteira do fluid step continua congelada no início do batch; voxels recém-enfileirados não são processados no mesmo step.

### 0.12.89 — restore de chunks arquivados budgetado
- Restore síncrono de chunk arquivado passa a contar junto com dispatch de nova generation task.
- `GENERATION_DISPATCH_BUDGET = 1 ms`; máximo de 4 trabalhos/frame.
- Backtracking não pode mais restaurar uma quantidade ilimitada de chunks de 4.096 slots no mesmo frame.

### 0.12.90 — prioridade de streaming calculada uma vez
- `pending.sort_by_cached_key` substitui `sort_by_key`.
- A chave com lookup em `surface_ranges` + sete campos é calculada uma vez por pending chunk, não repetidamente durante comparações.

### 0.12.91 — unload respeita orçamento desde o primeiro chunk
- `CHUNK_UNLOAD_BUDGET` continua 4 ms.
- O mínimo antes da checagem caiu de 8 chunks para 1.
- Chunks sujos que exigem archive encoding não podem mais obrigar oito operações antes de observar o budget.
- Ordem farthest-first e backlog do streaming permanecem inalterados.

### 0.12.92 — preallocation de filas voxel em bulk
- `DeduplicatedQueue` ganhou `reserve` e construção via `From<Vec<T>>` já nasce com capacidade compatível com o input.
- `VoxelUpdateQueue` expõe `reserve` para os owners de domínio.
- Bulk enqueue de lighting reserva previamente o volume completo, boundary voxels ou boundary neighbors conforme o caso.
- Evita crescimento incremental de `VecDeque`/`HashMap` durante seeds/relaxations grandes sem mudar a semântica deduplicada.

### 0.12.93 — coalescência de terrain remesh Geometry/Lighting
- `Geometry` e `Lighting` constroem o mesmo terrain mesh a partir do mesmo `ChunkMeshSnapshot`.
- Quando ambos estão pendentes para o mesmo coord, o dispatch de `Lighting` agora consome o pedido `Geometry` redundante.
- Se o pedido `Geometry` existia, mantém-se a regra anterior de que full terrain remesh supersede fluid-only remesh pendente.
- Um pedido puramente `Lighting` não cancela trabalho `Fluid` independente.
- Stale result continua re-enfileirado pelo kind da task; novas mudanças continuam entrando nas filas normais.

### 0.12.94 — scheduling de fluid sem HashMap por tick + frontier preallocation
- `FluidId` é índice contíguo no `FluidRegistry`; `PendingFluidUpdates::accumulated_steps` agora usa `Vec<f32>` indexado diretamente em vez de `HashMap<FluidId, f32>`.
- O mapa temporário de ready steps por tick foi substituído por `Local<Vec<usize>>`, reutilizado entre frames e indexado por `FluidId`.
- A semântica de `MAX_FLUID_STEPS_PER_FRAME`, descarte do excesso acumulado e frontier congelada permanece a mesma.
- Resume de fluid frontier usa `boundary_dynamic_fluid_count` para reservar previamente até 5 spread targets por fluido dinâmico antes de varrer a face.
- O solver continua deduplicando targets; a reserva só reduz crescimento incremental das estruturas internas.

### 0.12.95 — scratch de lighting reutilizado sem cache stale
- `relax_budgeted` recebe um `HashSet<IVec3>` externo para chunks alterados e limpa o conteúdo sem descartar a capacidade.
- `process_dynamic_lighting` mantém esse set como `Local<HashSet<IVec3>>` e usa `drain()` ao encaminhar remeshes, reutilizando os buckets entre frames com backlog.
- `emission_edit_previous_cells` usa `drain()` em vez de `mem::take`, preservando a capacidade do `HashMap` entre batches de edits.
- `LightingContext` continua sendo recriado por chamada; caches de direct-sky e highest-loaded-y não sobrevivem a mutações do mundo.
- A semântica de propagação, budget temporal/quantitativo e coalescência de remesh permanece inalterada.

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
- Qualquer mesh/remesh async deve validar presença/revisão de todo o halo antes de aplicar resultado.
- Lighting remesh pertence ao background async; immediate geometry permanece síncrono enquanto feedback do edit justificar.
- Pedidos `Geometry` e `Lighting` do mesmo coord podem ser coalescidos porque produzem o mesmo terrain mesh; `Fluid` continua independente salvo quando a regra de full geometry já o supersede.
- `FluidId` pode ser usado como índice denso enquanto `FluidRegistry` mantiver IDs por posição em `definitions`; crescer o registry deve redimensionar scratch/state, não voltar a hashing por frame.
- Scratch containers podem preservar capacidade entre frames, mas caches derivados do conteúdo do mundo não devem sobreviver sem invalidation autoritativa.
- Solvers dinâmicos devem preservar a semântica da frontier ao ganhar budgets temporais.
- Archive compactado permanece preferível a guardar buffers COW brutos; restore caro é controlado por scheduling budget.
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Continuar inspeção objetiva de `Update`/`PostUpdate` por scans globais ou builds síncronos; os principais builds de chunk/remesh, lighting e fluid já estão async/budgetados.
2. Revisar integração/spawn de mesh apenas se houver ganho estrutural sem introduzir lifecycle parcial por submesh; o caminho atual já substitui `Assets<Mesh>` in-place quando keys/topologia permanecem estáveis.
3. Revisar custo do rebuild de seleção somente com ganho estrutural claro; não duplicar geração de volume só para eliminar o pequeno sort do raio local 3.
4. Manter `notify_loaded_chunk_neighbors` não-vazio conservador enquanto metadata atual não provar sobreposição voxel-a-voxel.
5. Continuar procurando allocations/scratch descartados em hot paths quando a capacidade puder ser reutilizada sem manter dados derivados stale.
6. Só voltar a collision/raycast, UI ou task lifecycle se surgir evidência objetiva nova.

---

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh background fora da main thread;
- integração, restore, unload, lighting e fluid work budgetados;
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
