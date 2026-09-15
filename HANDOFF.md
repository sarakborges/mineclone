# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`
Branch de trabalho: `develop`
Stack: Rust + Bevy 0.19.0-dev

## Fonte canônica

Este arquivo, `HANDOFF.md` na raiz de `develop`, é a fonte canônica e persistente do projeto. Cópias `.txt`, exports e anexos são derivados e não substituem este arquivo.

## Regras de trabalho

- Trabalhar diretamente em `develop`.
- Não criar feature branch sem pedido explícito.
- Antes de alterar código, buscar o HEAD atual de `develop` e abrir os arquivos reais envolvidos.
- Fazer commits pequenos e coerentes; não misturar mudanças arquiteturais sem relação com o bloco atual.
- Não declarar bug visual/gameplay resolvido sem evidência de runtime quando a correção depender desse comportamento.
- Não gerar imagens a menos que o usuário peça explicitamente.
- Para assets binários enviados pelo usuário, usar exatamente os arquivos fornecidos.
- Depois de mudança material, atualizar este handoff.

## Versionamento — obrigatório

O arquivo raiz `VERSION` é a fonte autoritativa.

Todo bloco coerente deve subir versão:

- `patch`: fixes/refactors/otimizações internas compatíveis;
- `minor`: nova feature compatível;
- `major`: mudança incompatível/breaking.

O bump faz parte do bloco; não considerar o bloco fechado antes de atualizar `VERSION`.

## Validação — obrigatória

O CI de Rust é parte obrigatória do fechamento de qualquer bloco de código.

- `.github/workflows/ci.yml` roda em `push` para `develop` e `main`, além de `pull_request`.
- Gates autoritativos: `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check` e `cargo test`.
- Não considerar bloco de código encerrado enquanto esses checks não estiverem verdes para o HEAD correspondente.
- Se o usuário enviar output de compilação/runtime, corrigir todos os errors e warnings relacionados antes de continuar refactors maiores.
- `cargo fmt`/`rustfmt` não é gate de CI.
- Não ficar em polling repetitivo de CI; consultar runs quando houver resultado concreto para agir.
- Comunicação direta: menos narração, mais mudança concreta.

## Handoff — obrigatório

Atualizar `HANDOFF.md` sempre que houver mudança material em estado, arquitetura, roadmap, versão, HEAD relevante, regras ou próximos passos.

Não acumular backlog histórico obsoleto. Bugs antigos só permanecem se ainda estiverem ativos ou se houver regressão reportada.

O HEAD registrado aqui deve apontar para o último commit de **código/version**, não para o commit do próprio handoff.

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
16. Terrain/fluid/lighting remesh de background usa pipeline async; apenas remesh de geometry imediato de edição do jogador permanece síncrono.
17. Solvers dinâmicos caros devem ter teto temporal e de quantidade quando o trabalho puder variar muito por frame.
18. Sistemas visuais e gameplay devem evitar reescrever Components/Assets com o mesmo valor; acesso mutável pode propagar change detection ou upload desnecessário.
19. Quando invariants permitirem, separar refresh estrutural de material/textura de refresh leve de tint/orientação/posição.
20. Movimento com delta zero não deve adquirir mutação de `Transform` nem executar collision stepping; estado ocioso deve permanecer realmente ocioso.

---

# Estado atual do branch

Último HEAD de código/version confirmado antes desta gravação do handoff:

`2cf90dd7f6286900a623ea29498637e754828503`

Commit: `Avoid idle player state mutations`

Blocos recentes:

- `3415295b19698c2acada803f36d16a65166ada62` — `Avoid redundant underwater HUD updates`
- `547ddf62879370fa8dc067e82f66d5cdc067cb23` — bump `0.12.99`
- `6d078797d8c5b566e2b013c9d0163de4d8088181` — `Avoid redundant targeting visual updates`
- `bf6cdf044f1327f259a874400f6637e763412013` — bump `0.12.100`
- `262bec7cf2fe5441e7659d002eb8b343f4e4e324` — `Make placement preview updates change driven`
- `4702a9d4b0e6ce09e6f45f7c629ff3d1966411f3` — bump `0.12.101`
- `c95975361b01c11f462298a7b89e48d2e6f24bbc` — `Separate held block material and tint refresh`
- `4a3220d443f6af296fc62d2a99ee6382c1790e21` — bump `0.12.102`
- `2cf90dd7f6286900a623ea29498637e754828503` — `Avoid idle player state mutations` + bump `0.12.103`

`VERSION`: `0.12.103`

Sempre buscar HEAD/VERSION novamente antes de escrever código.

## CI atual

Estado observado durante esta gravação:

- `0.12.102` / run `34990965664`: Clippy **success**, `cargo check` **success**, `cargo test` ainda em execução na última consulta.
- `0.12.103` / run `34992166745`: iniciado; preparação/dependências do runner em andamento na última consulta.

O bloco `0.12.103` ainda não está encerrado enquanto Clippy/check/test do HEAD correspondente não estiverem verdes.

---

# Estado do refactor

A auditoria arquitetural/performance segue ativa. O roadmap vem do canon + inspeção real do código, não de backlog antigo seguido cegamente.

## Base consolidada até 0.12.79

- Filas deduplicadas, snapshots clonáveis e lifecycle async de generation/mesh.
- Integração main-thread budgetada; UI/HUD/targeting/environment change-driven.
- Metadata de occupancy/boundaries no `VoxelChunk`; índice vertical no `VoxelWorld`.
- Halo de mesh reduzido à shell real; skylight vertical lazy.
- Chunk buffers COW com `Arc<[...]>`; chunks vazios pulam mesh task.
- Fluid solver gated por tick; contexts de setup/streaming estreitados; caches concorrentes deduplicados.
- Leituras block/fluid/light consolidadas em `sample_at`/`sample_local`.
- `VoxelChunkContentMut` faz edits batch usando os mesmos invariants dos setters.
- Worldgen, structures e archive restore usam batch mutation; archive encode/restore percorrem 4.096 slots uma vez.

## 0.12.80–0.12.82 — streaming deltas e cache pruning

- Unload backlog vem do delta de `desired`/`retained`; scan global de chunks carregados fica só no bootstrap.
- Feature caches e `surface_ranges` são podados no bootstrap, mudança de render distance ou cruzamento de generation region, não em todo chunk atravessado.
- `GENERATION_REGION_SIZE_CHUNKS = 8`; retenção extra entre podas é apenas política de memória.

## 0.12.83–0.12.86 — revision tracking e remesh async

- `VoxelWorld` mantém revisão de mesh por chunk residente.
- `ChunkMeshSnapshot` captura presença + revisão dos 27 chunks do cubo 3×3×3.
- Initial mesh/remesh async descarta resultado stale e recaptura halo atual.
- Background terrain/fluid/lighting remesh usa `ChunkRemeshTasks`/`ChunkTaskQueue` no `AsyncComputeTaskPool`.
- Terrain/fluid partial refresh preserva a outra metade da render allocation.
- Coord já em voo não causa head-of-line blocking; outros chunks continuam ocupando slots livres.
- Apenas immediate geometry de edição/topologia continua síncrono para feedback do jogador.

## 0.12.87–0.12.92 — budgets e alocação de hot paths

- Dynamic lighting: máximo 4.096 voxels/frame, budget 2 ms, mínimo 256, checagem a cada 64.
- Fluid solver: máximo 512 updates/4 steps, budget 1 ms, mínimo 64; frontier congelada preservada.
- Restore de chunks arquivados divide budget de dispatch da geração: 1 ms / até 4 trabalhos por frame.
- Streaming `pending` usa `sort_by_cached_key`.
- Unload observa budget de 4 ms desde o primeiro chunk.
- `DeduplicatedQueue`/`VoxelUpdateQueue` suportam reserve; lighting bulk enqueue pré-aloca capacidade.

## 0.12.93–0.12.95 — coalescência e scratch reuse

- Geometry/Lighting do mesmo coord são coalescidos quando produzem o mesmo terrain mesh; `Fluid` segue independente salvo full geometry supersedence.
- Fluid scheduling usa `Vec<f32>` indexado por `FluidId` em vez de HashMap por tick.
- Frontier de fluid reserva capacidade a partir de boundary dynamic-fluid metadata.
- Lighting changed-chunk scratch e emission-edit maps preservam capacidade entre frames sem manter caches derivados stale.

## 0.12.96–0.12.98 — validation/CI

- Bootstrap meshing foi corrigido para consumir `ChunkMeshTaskOutput.meshes` preservando dependencies.
- Strict Clippy ficou limpo de warnings estruturais; helpers mortos foram removidos/limitados a `cfg(test)`.
- CI passou a rodar em push para `develop`; formatação deixou de ser gate.
- Gates autoritativos são Clippy com warnings como erro, `cargo check` e `cargo test`.

## 0.12.99–0.12.102 — visual hot paths change-driven

- Underwater HUD mantém a consulta O(1) do voxel do olho, mas só altera visibility/tint quando necessário.
- Target highlight/brush ghost não reescrevem transform/visibility/material idênticos.
- Placement preview separa material estrutural de tint/posição e usa cache de target relevante.
- Held block separa rebuild de material/layers de orientação e tint; caminhar não reconstrói texturas da mão.

## 0.12.103 — player idle realmente ocioso

- `animate_viewmodel` usa estado local para não reescrever o transform base em todo frame ocioso; ao fim de uma animação/item-switch, o base é restaurado uma única vez.
- `walk` e `move_flying` só chamam `move_axis` para eixos com velocidade diferente de zero.
- Walking/flight velocity só são zeradas/escritas quando o valor realmente muda.
- `update_swimming_state` só altera `SwimmingState.active` quando o estado de submersão muda.
- Swimming só altera grounded/vertical velocity quando necessário e não chama collision stepping para delta vertical zero.
- Gravity, quando grounded com suporte e sem jump, retorna antes de integrar gravidade e antes do `move_axis`; deixa de simular uma microqueda+colisão a cada tick parado no chão.
- O ground support probe permanece por frame enquanto grounded para detectar remoção do suporte sem depender de cache derivado.

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
- Pedidos `Geometry` e `Lighting` do mesmo coord podem ser coalescidos porque produzem o mesmo terrain mesh; `Fluid` continua independente salvo quando full geometry já o supersede.
- `FluidId` pode ser usado como índice denso enquanto `FluidRegistry` mantiver IDs por posição em `definitions`; crescer o registry deve redimensionar scratch/state, não voltar a hashing por frame.
- Scratch containers podem preservar capacidade entre frames, mas caches derivados do conteúdo do mundo não devem sobreviver sem invalidation autoritativa.
- Componentes/Assets não devem ser mutavelmente acessados só para regravar o mesmo valor; isso pode propagar change detection ou asset upload desnecessário.
- Block model material/topology refresh deve ficar separado de tint/orientation refresh quando os inputs autoritativos permitem essa divisão.
- Grounded estável com suporte não precisa integrar gravidade só para colidir e zerar a mesma velocidade no mesmo tick; o probe de suporte é o invariant necessário nesse estado.
- Solvers dinâmicos devem preservar a semântica da frontier ao ganhar budgets temporais.
- Archive compactado permanece preferível a guardar buffers COW brutos; restore caro é controlado por scheduling budget.
- Formatação de Rust não é requisito de CI; não reintroduzir format gate sem pedido explícito.
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Fechar o gate de CI do HEAD `0.12.103`; corrigir qualquer falha de Clippy/check/test antes de considerar o bloco encerrado.
2. Continuar inspeção objetiva de `Update`/`PostUpdate` por scans globais, builds síncronos e escritas redundantes em Components/Assets.
3. Auditar camera/cursor/interaction e demais systems do player por change detection inútil, sem eliminar polling que seja necessário para input/foco.
4. Revisar integração/spawn de mesh apenas se houver ganho estrutural sem introduzir lifecycle parcial por submesh.
5. Revisar custo do rebuild de seleção somente com ganho estrutural claro; não duplicar geração de volume só para eliminar o pequeno sort do raio local 3.
6. Manter `notify_loaded_chunk_neighbors` não-vazio conservador enquanto metadata atual não provar sobreposição voxel-a-voxel.
7. Continuar procurando allocations/scratch descartados em hot paths quando a capacidade puder ser reutilizada sem manter dados derivados stale.

---

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh background fora da main thread;
- integração, restore, unload, lighting e fluid work budgetados;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar scans globais por frame, allocations temporárias e mutações idempotentes em hot paths;
- não trocar corretude por performance aparente;
- natural hydrology continua generation-authoritative.

# Comunicação

- direta e focada em ação;
- não repetir caveats de `cargo check`/`cargo run`;
- não dizer “achamos a causa” sem evidência;
- durante sequências longas, atualizar apenas findings/blocos concluídos;
- atualizar `HANDOFF.md` depois de mudanças materiais.