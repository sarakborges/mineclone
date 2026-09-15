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
21. Scratch de cardinalidade estruturalmente limitada deve preferir stack/reuse a heap allocation por amostra, sem impor limites artificiais a conteúdo data-driven.
22. Escolhas de criação de mundo devem pertencer a `NewWorldConfig`; bootstrap consome essa configuração, não cria um segundo owner paralelo.
23. Spawn forçado por biome deve escolher a coluna inicial pelo biome autoritativo do surface field antes da geração, em vez de teletransportar o jogador para um segundo local depois do bootstrap.

---

# Estado atual do branch

Último HEAD de código/version confirmado antes desta gravação do handoff:

`f63062814eab6ccb17fc14d2477132bb158bc932`

Commit: `Fix spawn biome dropdown validation`

`VERSION`: `0.13.0`

Blocos/commits recentes relevantes:

- `c00effb1bcbff96d504365e269f79e90e989c8f4` — `Reduce surface biome sampling allocations` + `0.12.104`
- `7bbee70089fca506c545f1758dc064fc9cd8b17e` — `Reuse current biome identity buffers` + `0.12.105`
- `f0ee1c48beede24fde6a3a95483de5706a2db1a9` — `Fix surface biome sampling warning` + `0.12.106`
- `5a021f333ad756ae6fa89dad8682045aa15da504` — `Add gameplay hints and spawn biome selection` + bump `0.13.0`
- `e81e1803bbc212dd27e93986de1255ab41560ef2` — `Keep spawn biome search focus isolated`
- `f63062814eab6ccb17fc14d2477132bb158bc932` — `Fix spawn biome dropdown validation`

Sempre buscar HEAD/VERSION novamente antes de escrever código.

## CI atual

- `0.12.106` / run `34993918732`: **success**.
- `0.13.0` / HEAD `f63062814eab6ccb17fc14d2477132bb158bc932` / run `34996626194`: Clippy **success**, `cargo check` **success**, `cargo test` ainda em execução na última consulta antes desta gravação.

O bloco `0.13.0` só é considerado formalmente fechado quando `cargo test` desse HEAD também ficar verde.

---

# Estado consolidado do refactor/performance

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

## 0.12.80–0.12.86 — streaming/revisions/remesh async

- Unload backlog vem do delta de `desired`/`retained`; scan global de chunks carregados fica só no bootstrap.
- Feature caches e `surface_ranges` são podados por bootstrap/render-distance/generation-region, não por todo chunk atravessado.
- `VoxelWorld` mantém revisão de mesh por chunk residente.
- `ChunkMeshSnapshot` captura presença + revisão dos 27 chunks do cubo 3×3×3.
- Initial mesh/remesh async descarta resultado stale e recaptura halo atual.
- Background terrain/fluid/lighting remesh usa `ChunkRemeshTasks`/`ChunkTaskQueue` no `AsyncComputeTaskPool`.
- Apenas immediate geometry de edição/topologia continua síncrono para feedback do jogador.

## 0.12.87–0.12.95 — budgets/coalescência/scratch

- Dynamic lighting: máximo 4.096 voxels/frame, budget 2 ms, mínimo 256, checagem a cada 64.
- Fluid solver: máximo 512 updates/4 steps, budget 1 ms, mínimo 64; frontier congelada preservada.
- Restore de chunks arquivados: 1 ms / até 4 trabalhos por frame.
- Unload observa budget de 4 ms desde o primeiro chunk.
- `DeduplicatedQueue`/`VoxelUpdateQueue` suportam reserve; lighting bulk enqueue pré-aloca capacidade.
- Geometry/Lighting do mesmo coord são coalescidos quando produzem o mesmo terrain mesh; `Fluid` segue independente salvo full geometry supersedence.
- Fluid scheduling usa `Vec<f32>` indexado por `FluidId` em vez de HashMap por tick.
- Lighting changed-chunk scratch e emission-edit maps preservam capacidade entre frames sem manter caches derivados stale.

## 0.12.96–0.12.103 — CI e hot paths change-driven

- Bootstrap meshing consome `ChunkMeshTaskOutput.meshes` preservando dependencies.
- Strict Clippy ficou limpo de warnings estruturais; CI em push para `develop` virou gate obrigatório.
- Underwater HUD, target highlight, brush ghost e placement preview evitam mutações idempotentes.
- Held block separa rebuild estrutural/material de tint/orientation.
- Player idle evita reescritas de transform/state e collision stepping com delta zero.

## 0.12.104 — surface biome sampling com scratch fixo

- Neighborhood de 25 sites usa array em stack em vez de `Vec` temporário.
- Cache misses usam scratch fixo; pass intermediário de sites foi removido.
- Pesos regionais usam scratch compacto, preservando tie-breaking e ordem de influences.
- A alocação obrigatória restante no caminho é o `Vec<BiomeInfluence>` retornado por `BiomeFieldSample`.

## 0.12.105–0.12.106 — identidade de biome sem buffers descartáveis

- `track_current_biome`/identity path passou a reutilizar buffers do estado corrente em vez de reconstruir `String`/`Vec` temporários a cada atualização de posição.
- A implementação preserva blend contínuo e change detection: reuse de storage não transforma dado derivado em cache autoritativo.
- O warning restante do surface-biome sampling foi corrigido em `0.12.106`; o gate correspondente ficou verde.

---

# 0.13.0 — gameplay hints + Spawn Biome

## HUD/tooltips

- Crosshair mostra hint contextual para quebrar bloco e, quando há bloco selecionado para placement, o hint inclui quebrar/colocar.
- Player HUD mostra `Press E to open inventory.` abaixo do painel principal.
- Os novos hints respeitam `HudSettings` / `Display Tooltips` e reutilizam os owners visuais/textuais existentes em vez de criar estado paralelo.

## New World / Spawn Biome

- `NewWorldConfig` agora possui seleção opcional de spawn biome; `None` representa `Random` e preserva o comportamento anterior.
- New World → General ganhou `Spawn Biome` com dropdown pesquisável.
- A lista vem dos surface biomes registrados na dimensão default; nomes exibidos vêm da localização/definição do biome, enquanto a configuração armazena o biome ID.
- O dropdown isola foco de keyboard input de Seed/Ticks; abrir a busca reseta os outros inputs ativos.
- O filtro reutiliza labels normalizados armazenados nas opções; não cria a normalização de cada opção a cada frame.

## Bootstrap de spawn

- Para `Random`, a coluna inicial continua seguindo o comportamento normal.
- Para biome selecionado, o bootstrap procura uma coluna seca cujo `BiomeFieldSample.primary_id` seja exatamente o ID escolhido.
- A coluna encontrada passa a ser a `spawn_column` usada pelo mesmo pipeline de geração/carregamento existente.
- `safe_spawn_position` continua sendo o único owner do posicionamento físico final sobre a coluna gerada; não foi criado um segundo mecanismo de teleporte pós-bootstrap.
- Não houve alteração nas regras gerais de terrain generation; a feature só condiciona a escolha da coluna inicial.

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
- Surface sampling pode usar scratch fixo para o neighborhood de sites porque a cardinalidade é definida por `SITE_SEARCH_RADIUS`; isso não limita a quantidade data-driven de biomes no registry.
- Reuse de buffers de identidade de biome é scratch/state reuse, não autorização para persistir amostras derivadas stale do worldgen.
- `Spawn Biome` pertence a `NewWorldConfig`; não criar outro resource autoritativo para a mesma escolha.
- Spawn forçado deve permanecer integrado ao bootstrap via `spawn_column`; não adicionar teleporte corretivo posterior sem evidência de que o invariant atual falha.
- Solvers dinâmicos devem preservar a semântica da frontier ao ganhar budgets temporais.
- Archive compactado permanece preferível a guardar buffers COW brutos; restore caro é controlado por scheduling budget.
- Formatação de Rust não é requisito de CI; não reintroduzir format gate sem pedido explícito.
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos

Se nenhum error/warning/runtime report tiver prioridade:

1. Fechar o `cargo test` do HEAD `f63062814eab6ccb17fc14d2477132bb158bc932`; se falhar, corrigir todos os failures antes de continuar.
2. Fazer validação runtime da UI de `0.13.0`: posicionamento dos hints, toggle `Display Tooltips`, foco/keyboard do dropdown, busca/seleção e retorno a `Random`.
3. Validar runtime de `Spawn Biome` em seeds diferentes, confirmando que o player nasce no biome selecionado e que `Random` preserva o fluxo anterior.
4. Retomar inspeção objetiva de `Update`/`PostUpdate` por scans globais, builds síncronos, allocations temporárias e escritas redundantes em Components/Assets.
5. Revisar integração/spawn de mesh apenas se houver ganho estrutural sem introduzir lifecycle parcial por submesh; o caminho atual já substitui assets in-place quando topology/keys permitem.
6. Revisar custo do rebuild de seleção somente com ganho estrutural claro; não duplicar geração de volume só para eliminar o pequeno sort do raio local 3.
7. Manter `notify_loaded_chunk_neighbors` não-vazio conservador enquanto metadata atual não provar sobreposição voxel-a-voxel.

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
