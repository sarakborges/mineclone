# HANDOFF — Asteria / Mineclone

> Handoff corrente. O histórico detalhado anterior ao Cut 16 está preservado em `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico ainda mais antigo em `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura em `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline: `develop@5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante estes cutovers internos.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar.
- Old saves/legacy compatibility não são prioridade.
- CI = audits + Clippy + Check; `cargo test` não roda automaticamente.
- Não avançar com gate vermelho ou em andamento.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues têm owner e bound explícitos;
- trabalho frame-sensitive é incremental/budgetado;
- generation não possui side effects de render/UI/ECS;
- presentation é derivada e descartável; `ChunkRenderPool` nunca é world truth;
- cada migration block termina com HANDOFF atualizado e CI verde;
- não reintroduzir hydrology legado.

## Fases

- Phase 1 concluída: core types/boundaries.
- Phase 2 concluída: authoritative storage/revisions/lookup/eviction ownership.
- Phase 3 concluída: biome/structure metadata independente de render/materialization.
- Phase 4 concluída: Streaming scheduler v2.
- Phase 5 concluída: Terrain generation v2.
- Phase 6 concluída: structures/connectors/feature planning.
- **Phase 7 em andamento: voxel presentation / meshing v2.**

## Phase 7 — estado consolidado

### Ownership/correctness já fechado

- initial meshing e remesh usam snapshots imutáveis e stale checks antes de publication;
- content/halo source e lighting source são stamps separados;
- `ChunkRenderPool` mantém source stamps section-aware por terrain/fluid meshlet;
- render section identity é explícita (`coord + kind + meshlet_index`);
- chunks vazios também recebem source stamp sem snapshot pesado;
- presentation scheduler é o único owner nominal do scheduling inicial;
- full presentation reset drena entities/meshes/stamps/accounting e avança `presentation_reset_revision`;
- streaming observa cada reset uma vez e requeueia presentation ausente a partir do `VoxelWorld` residente, sem regenerar world truth nem repetir side effects once-per-residency.

### Observabilidade fechada até aqui

- meshing async, publication main-thread e RenderApp são medidos separadamente;
- publication inicial/remesh apply são microsegundos na maior parte das janelas e não explicam o stall principal;
- RenderApp é decomposto em `ExtractCommands`, `PrepareAssets`, `PrepareMeshes`, views, queue, `Prepare`, `Render` e cleanup;
- Cut 16 subdivide `Prepare` nos sub-sets oficiais do Bevy 0.19.1.

### Cut 15 — world camera warm-up

Commit `47959b90c2e7f09774d1f24329dab48ff5a8bbae` (`Warm world camera before gameplay`), CI #10309 success.

- world camera nasce ativa ainda em Loading/Spawning, mantendo `CameraOutputMode::Skip` atrás do overlay;
- loading camera usa ordem explícita acima da world camera;
- pior frame de entrada caiu de ~272 ms para ~201 ms, mas `stage_prepare_max` permaneceu dominante (~133 ms);
- nenhum meshing, generation, budget ou world truth mudou.

### Cut 16 — timing interno de `RenderSystems::Prepare`

Código: commit `d3a8778e8e0b891e1413b40c126a4b518bce5fe2` (`Split render prepare stage timing`), CI #10310 success. Handoff/archive: `a9c75a9c03cdfcf292d5c64c9ea66c68b27f4639`, CI #10311 success.

- `render_prepare_diagnostics` mede `PrepareResources`, batch phases, write phase buffers, collect phase buffers, flush, bind groups, total e tail;
- lifecycle reset limpa essas métricas ao entrar em Loading e Gameplay;
- somente diagnóstico; não altera camera, assets, batching, budgets, presentation ou gameplay.

### Evidência após Cut 16

Gameplay `2026-09-30_04-17-04-401157000.txt` + vídeo `Gravando 2026-09-30 012214.mp4`:

- Loading: `render prepare total_max_us=8373`, `resources_max_us=1000`, `bind_groups_max_us=7891`;
- primeiro intervalo de Gameplay: `render prepare total_max_us=70971`, **`resources_max_us=67598`**, `bind_groups_max_us=3169`, `flush_max_us=1189`;
- no mesmo intervalo: `stage_prepare_max_us=70985`, `stage_render_max_us=50495`, `render work max_us=101426`, frame max `129088`, `main_work_max_us=7388`;
- portanto o cold-start restante está especificamente em **Bevy `RenderSystems::PrepareResources`**;
- existe um problema separado durante movimento: uma janela registrou frame max `102812 us` com **`main_work_max_us=102244`**, enquanto `stage_prepare_max_us=5166` e `stage_render_max_us=19341`. Esse stall é main-world e deve ser investigado em cut separado após o startup.

### Cut 17 — hipótese de prewarm de lighting PBR

Commit `0b215f029d7e107f58dd57342c9fc752eda4f803` (`Warm PBR lighting before gameplay`), CI #10312 success.

- experimento criou `DirectionalLight` e `PointLight` zero-intensity durante Loading para tentar pagar o cold-start de lighting atrás do overlay;
- as entidades eram separadas das luzes reais de Gameplay e não alteravam valores funcionais, shadows, world truth ou streaming;
- gameplay log `2026-09-30_05-09-17-966417700.txt` **refutou a hipótese**:
  - Loading ficou com `resources_max_us=1659`, `bind_groups_max_us=12197`;
  - primeiro intervalo de Gameplay ficou com `render prepare total_max_us=97863`, **`resources_max_us=93610`**, `stage_render_max_us=111069` e frame max `169143`;
  - depois do burst, `resources_max_us` volta a ~2–3 ms;
- conclusão: lighting warm-up não remove o cold-start de `PrepareResources`; não deve permanecer como complexidade especulativa.

### Audit pós-Cut 17 — clustering per-view

A leitura do Bevy 0.19.1 localizou uma operação com perfil compatível dentro de `PrepareResources`:

- `prepare_clusters_for_gpu_clustering` roda em `RenderSystems::PrepareResources` quando GPU clustering está habilitado;
- por view 3D ele cria `ViewClusterBindings` e `ViewGpuClusteringBuffers`, reserva cluster storage, lista inicial de até 65.536 índices e scratch buffers e escreve buffers GPU;
- a world camera já existe durante Loading por causa do Cut 15;
- a **viewmodel camera só nasce no primeiro Update de Gameplay**, portanto cria uma nova view 3D justamente na janela do cold-start;
- a viewmodel não necessita clustered lighting: o braço usa `StandardMaterial` `unlit=true`, e held-block usa `BlockModelMaterial` com display shading próprio.

### Cut 18 — experimento per-view e falha de runtime

Commit `74b8b1c1b0b32c933c40d8a626ae411b9c4f0d36` (`Skip clustering for viewmodel camera`), CI #10313 success.

- removeu corretamente o `LightingWarmupPlugin` refutado do Cut 17 e deletou `src/rendering/lighting_warmup.rs`;
- restaurou visibilidade mínima dos helpers de `sun_lighting` / `dynamic_lights`;
- tentou usar `ClusterConfig::None` exclusivamente na viewmodel camera para evitar clustering per-view;
- o binário compilou e passou Clippy/Check, mas o teste de runtime falhou antes de produzir benchmark válido:
  - Bevy 0.19.1 converte `ClusterConfig::None` em dimensões `UVec3::ZERO`;
  - com GPU clustering habilitado, `prepare_cluster_dummy_textures` ainda processa essa view e tenta criar `clustering dummy texture` com dimensão X zero;
  - wgpu rejeita a textura e o app encerra com `Validation RenderError`;
- portanto **não existe resultado de performance válido do Cut 18** e `ClusterConfig::None` não pode permanecer nesse backend;
- `ClusterConfig::Single` não será usado como substituição especulativa: ele ainda mantém infraestrutura de clustering per-view e não isola a causa do cold-start.

### Correção pós-Cut 18 — restaurar viewmodel válida

- `ClusterConfig::None` foi removido da viewmodel camera, retornando o runtime ao comportamento válido anterior ao experimento;
- a remoção do warm-up refutado do Cut 17 foi preservada;
- world camera, meshing, streaming, generation, presentation ownership e world truth não foram alterados;
- objetivo da correção foi somente restaurar runtime válido; não há claim de performance associado.

### Evidência pós-correção — hitch de movimento

Gameplay log válido pós-correção:

- startup voltou ao patamar saudável anterior ao experimento inválido, com `PrepareResources` máximo em ~65,1 ms e `main_work` baixo na entrada;
- durante movimento houve um frame de ~53,2 ms com `main_work` ~52,7 ms;
- outro hitch atingiu ~96,4 ms de frame com `main_work` ~96,0 ms;
- nas mesmas janelas o renderer permaneceu materialmente menor (`Prepare` ~4,5–5,1 ms; `Render` ~14,9–18,0 ms);
- publication e priority scans já medidos permanecem ordens de grandeza abaixo desses hitches;
- conclusão: o próximo diagnóstico deve localizar qual trabalho do **main schedule** concentra o custo antes de qualquer mudança funcional.

### Cut 19 — timing do núcleo main-world

Commit `dd198df25f08f6dd0b7a9d326ca790cd2c9a58aa` (`Measure main world stage timing`), CI #10317 success.

- novo `main_world_diagnostics` mede, por janela, count/avg/p95/p99/max em microssegundos;
- buckets instrumentados: `streaming`, `retirement`/eviction/warp reconciliation, `fluid`, `lighting`, `remesh`, `residency`, `visibility`, `generation_refill` e `deferred_mesh_retirement`;
- os marcadores foram inseridos nas chains já serializadas do `WorldPlugin`, sem mudar budgets, world truth, generation, presentation, meshing ou regras de gameplay;
- samples são resetados ao entrar em Loading/Gameplay e emitidos junto da cadência existente de render diagnostics;
- nenhum ganho de performance foi reivindicado; o objetivo era localizar o hitch com gameplay real.

### Evidência após Cut 19 — hitch localizado em streaming

Gameplay `2026-09-30_14-05-57-794802200.txt`:

- uma janela registrou frame max `95369 us`, `main_work_max_us=98400` e **`streaming_max_us=95019`**;
- outra registrou frame max `218414 us`, `main_work_max_us=212395` e **`streaming_max_us=208765`**;
- os demais buckets ficaram muito abaixo desses máximos nas mesmas janelas;
- portanto a observabilidade do Cut 19 localizou o hitch de movimento no bucket **streaming**; a próxima investigação de performance deve decompor `stream_chunks`, sem mexer nos demais subsistemas por hipótese.

### Cut 20 — enforce surface biome `size.max`

Commit `3525c290f13e5c361507f5097dd1c28569ffe98d` (`Enforce surface biome max size`), CI #10318 success.

Relato em gameplay: regiões de Plains ocupando a maior parte do terreno e ultrapassando o `size.max` autorado.

Root cause confirmado no código:

- `DimensionBiomeSize.max` era validado, escalado e armazenado para surface biomes, mas a geração normal usava apenas `size.min` para calcular o espaçamento global de sites;
- sites Voronoi vizinhos podiam selecionar o mesmo biome repetidamente e `sample_surface` compactava essas influências pelo mesmo biome index, fundindo a sequência em uma região visual/terrain contínua sem bound;
- em `asteria:overworld/plains`, o conteúdo autorado é `x/z min=120, max=420`, mas esse `max=420` não participava da seleção normal; ele só tinha efeito no caso especial de forced surface biome;
- o problema é anterior ao core rebuild: o primeiro gradual biome field já carregava/validava `max` sem aplicá-lo ao field normal.

Correção:

- surface selection passa a derivar uma região determinística candidate-specific cujo extent é o diâmetro correspondente aos `size.x.max` / `size.z.max` autorados;
- sites vizinhos com o mesmo raw biome continuam podendo se fundir dentro da mesma região;
- quando dois sites do mesmo biome compartilham uma aresta Voronoi mas pertencem a regiões máximas diferentes, um único lado vence por claim determinístico e o outro passa pelo fallback normal, interrompendo cadeias ilimitadas do mesmo biome;
- pesos, climate, distributions, `avoidNear`, `requireNear`, exclusive groups e conteúdo JSON não foram retunados para esconder o bug;
- regressões unitárias cobrem uso do `max`, vencedor único cross-region e preservação de merge dentro da mesma região.

Esta correção altera determinísticamente worldgen de surface biomes; validação visual limpa deve ser feita em **mundo novo**, não esperando que chunks já gerados de um save sejam reescritos.

### Cut 21 — diagnóstico direcionado dentro de streaming

Observação adicional sobre o mesmo gameplay pós-Cut 19:

- não existe nenhuma ocorrência de `slow streaming selection rebuild` no log inteiro;
- esse warning já existia com threshold de 8 ms exclusivamente ao redor de `rebuild_queue`, então o rebuild de seleção em si fica excluído como causa dos picos de ~95 ms e ~209 ms;
- `ChunkTaskQueue::poll_ready` e `poll_ready_by_key` usam `check_ready`, portanto polling de generation/mesh tasks não espera worker completar.

Instrumentação adicionada sem alterar scheduling, budgets ou world truth:

- `GenerationScheduler::sync_snapshot` avisa se refresh de snapshot passar de 8 ms;
- `GenerationScheduler::cancel_where` avisa se cancelamento de generation tasks passar de 8 ms;
- `PresentationScheduler::sync_snapshot` avisa se refresh de snapshot passar de 8 ms;
- `PresentationScheduler::cancel_where` e `cancel_farthest_where` avisam se cancelamento/preemption de mesh tasks passar de 8 ms;
- `PendingLightingUpdates::seed_chunk_direct_lighting` avisa se o initial direct-light seed passar de 8 ms;
- esse seed inicial ocorre dentro de `dispatch_initial_mesh_tasks` e, portanto, está contabilizado em `streaming`, não no bucket dinâmico `lighting` do Cut 19.

O Cut 21 é observabilidade direcionada apenas. Nenhum ganho de performance é reivindicado até novo gameplay log reproduzir os hitches.

### Cut 22 — separar `size.max` de adjacency autorada

Runtime pós-Cut 21 expôs uma regressão introduzida pelo Cut 20 durante bootstrap de mundo:

- panic em `surface biome site IVec2(3, 0) has no biome compatible with adjacency constraints`;
- o Cut 20 havia colocado a quebra de continuidade de `size.max` dentro de `adjacency_allows`, junto de `avoidNear`, `requireNear` e `exclusiveNeighborGroup`;
- em um site onde o raw biome perdia o claim cross-region e nenhum fallback permitido satisfazia simultaneamente essa regra nova, o domínio local ficava vazio e o loading panikava.

Correção do Cut 22:

- adjacency autorada foi separada em `authored_adjacency_allows`; `avoidNear`, `requireNear` e exclusive groups permanecem hard constraints;
- a continuidade de `size.max` agora é avaliada separadamente por `surface_size_allows`;
- seleção tenta primeiro raw/fallback que satisfaça tanto adjacency autorada quanto `size.max`;
- se `size.max` esvaziaria um domínio que ainda possui uma solução válida pelas constraints autoradas, o selector preserva essa solução em vez de panicar;
- se a própria adjacency autorada for impossível, o panic permanece, agora com mensagem explícita `authored adjacency constraints`, para não esconder conteúdo inconsistente;
- regressão unitária prova que uma fronteira de `size.max` pode rejeitar continuidade sem ser tratada como conflito de adjacency autorada;
- nenhum conteúdo JSON, climate, distribution, weight ou regra de exclusive group foi relaxado.

Consequência semântica deliberada: `size.max` continua limitando continuidade sempre que existe candidato compatível, mas adjacency autorada tem precedência quando os dois requisitos seriam localmente incompatíveis. Um bound absolutamente estrito em todos os casos exigiria assignment/região global em vez do selector local atual e não deve ser simulado com um panic.

## Próximos passos

1. passar audits + Clippy + Check do Cut 22;
2. criar mundo novo e confirmar que bootstrap não panika e que `size.max` continua interrompendo cadeias de surface biome quando existe fallback compatível;
3. coletar gameplay log reproduzindo movimento até ocorrer hitch;
4. correlacionar `streaming_max_us` com warnings `slow streaming ...` do Cut 21 e otimizar somente o hot path confirmado;
5. retomar a investigação do cold-start de `PrepareResources` sem repetir `ClusterConfig::None` nem o lighting warm-up refutado;
6. executar audit final da Phase 7 quando a dívida de performance estiver localizada/endereçada.

## Pacote deferido pós-refactor — Worldgen coherence

Planejamento registrado em `docs/post-refactor-worldgen-coherence.md`.

Implementar **somente após a conclusão da Phase 7 e fechamento do core rebuild**, sem misturar feature work ao cutover atual. O pacote cobre:

- ocean/coast margins com influência de altura estritamente lower-only, impedindo oceanos de erguer margens de qualquer mountain family;
- constraints declarativas entre volume biomes e surface biomes, com selectors por ID/family/tag;
- surface indicator opcional e determinístico para volume biomes via structure/structure group;
- rewrite da formação de floating islands como massas coerentes, grandes, irregulares e conectadas, com blending gradual e estratificação surface/subsurface/core;
- testes obrigatórios de determinismo, chunk seams, generation-order invariance, connectivity e regressões de biome relationships.

Ordem pós-refactor definida no documento: primitives de relações -> volume constraints -> surface indicators -> ocean lower-only -> floating islands -> integração/tuning.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não reescrever/force-push commits publicados; usar fast-forward;
- cada bloco coerente deve terminar com HANDOFF atualizado e gate verde;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
