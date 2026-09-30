# HANDOFF — Asteria / Mineclone

> Handoff corrente e operacional. Histórico anterior ao Cut 16: `HANDOFF_ARCHIVE_2026-09-30_PRE_CUT16.md`; histórico antigo: `HANDOFF_ARCHIVE_2026-09-25.md`; decisões de arquitetura: `docs/asteria-core-rebuild.md`.

## Estado atual — 2026-09-30

- Repo: `sarakborges/mineclone`.
- Branch: `architecture/asteria-core-rebuild`.
- PR draft: #22 `Asteria core rebuild` -> `develop`.
- Baseline de develop: `5038934a97a51cddcd8cdc94fc61314cb650d96d` (`0.68.48`).
- `VERSION` permanece `0.68.48` durante os cutovers internos.
- Rust + Bevy permanecem; o rebuild troca boundaries/ownership, não a stack.
- Hydrology legado foi removido deliberadamente e não deve voltar.
- Old saves/legacy compatibility não são prioridade.
- CI obrigatório = audits + Clippy + Check; `cargo test` não é gate automático.
- Não declarar ganho de performance sem gameplay log real.

## Invariantes

- authoritative world != resident world != rendered presentation;
- jobs async usam inputs imutáveis e publication stale é rejeitada;
- caches/queues possuem owner e bound explícitos;
- trabalho frame-sensitive precisa ser incremental/budgetado;
- generation não possui side effects de render/UI/ECS;
- presentation é derivada e descartável; `ChunkRenderPool` nunca é world truth;
- cada cut coerente termina com HANDOFF atualizado e CI verde;
- não reintroduzir Hydrology legado.

## Fases

- Phases 1–6 concluídas: core boundaries, authoritative storage, biome/structure metadata, Streaming scheduler v2, Terrain generation v2 e structures/connectors/feature planning.
- **Phase 7 em andamento: voxel presentation / meshing v2 + dívida de performance/correctness descoberta durante validação.**

## Phase 7 — estado consolidado

### Presentation/render

- initial meshing e remesh usam snapshots imutáveis + stale checks;
- content/halo e lighting possuem stamps separados, section-aware;
- full presentation reset é descartável e reconstruído do `VoxelWorld` residente;
- publication main-thread é barata na maior parte das janelas e não explica os grandes hitches;
- RenderApp está instrumentado por stages e `Prepare` pelos sub-sets oficiais do Bevy 0.19.1.

### Cold-start conhecido

- Cut 15 aqueceu a world camera ainda em Loading e reduziu parcialmente o primeiro frame;
- Cut 16 localizou o cold-start remanescente em `RenderSystems::PrepareResources`;
- lighting warm-up do Cut 17 foi refutado e removido;
- `ClusterConfig::None` na viewmodel do Cut 18 é inválido nesse backend/wgpu e foi revertido;
- cold-start de `PrepareResources` segue separado do hitch de movimento e deve ser retomado depois do pacote atual.

### Streaming/hitch

- Cut 19 instrumentou main-world stages e localizou os hitches de movimento em `streaming`;
- Cut 21 adicionou warnings direcionados para rebuild, scheduler snapshots/cancelamentos e direct-light seed;
- `ChunkTaskQueue` usa `check_ready`, portanto polling não espera worker terminar.

### Surface biome size correctness

- Cut 20 (`3525c290f13e5c361507f5097dd1c28569ffe98d`) passou a aplicar `size.max` à continuidade normal de surface biomes;
- Cut 22 (`7ce183d8491ee43b516ed060b84f7ce2f060d7cd`) separou `size.max` das hard constraints de adjacency para não transformar ausência de fallback em panic indevido;
- Cut 23 (`35313349c08c39dab6249afa35a56323117ce8ae`, CI #10321 success) corrigiu `size.min`: jitter independente ±32% por site podia comprimir sites vizinhos até 36% do spacing nominal. Surface sites agora usam jitter lateral determinístico por linha; domain warp continua owner da organicidade. Validar em mundo novo.

## Cut 24 — limitar scans stale do generation frontier

Gameplay `2026-09-30_16-23-13-909518300.txt` reproduziu novamente o hitch com evidência mais precisa:

- frame máximo ~100.9 ms;
- `main_work_max_us` ~97.5 ms;
- `streaming_max_us` ~93.9 ms;
- retirement, fluid, lighting, remesh, residency, visibility, generation refill e deferred mesh retirement permaneceram muito menores na mesma janela;
- nenhum warning `slow streaming ...` do Cut 21 disparou no arquivo inteiro.

Root cause confirmado em `streaming/generation.rs`:

- `select_generation_wave` usa `pop_pending_by_priority`, que faz o scan caro da pending frontier;
- o `FrameWorkBudget` de generation dispatch é 1 ms / máximo 16 itens;
- entradas já cobertas por render pool, ready queue, mesh task, generation task ou generated-unpublished eram descartadas com `continue` **sem** `budget.record(1)`;
- com milhares de pending entries, o loop podia realizar uma sequência arbitrária de priority scans O(n) no mesmo Update sem atingir o item cap nem observar novamente o deadline.

Correção:

- cada `pop_pending_by_priority()` bem-sucedido agora registra imediatamente uma unidade de budget, antes de qualquer stale/ownership filter;
- cada scan passa a custar budget exatamente uma vez, inclusive quando o coord é descartado;
- o loop volta a respeitar o limite de 16 scans e o deadline global/frame budget;
- nenhuma prioridade, wave size, worker limit, world truth ou conteúdo foi alterado.

Ainda não há claim de ganho até gameplay pós-Cut 24 confirmar que o pico de ~94 ms desapareceu/reduziu como esperado.

## Pacote Worldgen coherence — execução autorizada agora

O pacote antes deferido em `docs/post-refactor-worldgen-coherence.md` foi explicitamente promovido pelo usuário para implementação imediata após o Cut 24. Executar em cortes independentes, sempre com CI verde entre eles:

1. **Ocean margin lower-only:** margens oceânicas nunca podem elevar o terreno do vizinho; isso deve valer inclusive contra mountains/alps/mountain belts/volcano/gorge e qualquer futura mountain family.
2. **Volume → surface constraints:** volume biomes podem declarar em quais surface biomes podem existir; seleção deve permanecer determinística e chunk-order invariant.
3. **Volume surface indicator:** volume biome pode declarar structure/structure group opcional no surface biome para indicar sua presença; usar o pipeline generalizado de structures, não side effect ad-hoc.
4. **Floating islands rewrite:** ilhas grandes, irregulares e coerentes, com transição gradual entre lobes/pedaços e estratificação grass -> dirt -> stone em vez de blobs aleatórios de stone.

Princípios do pacote:

- metadata/data-driven sempre que a regra é de conteúdo;
- não hardcodar IDs de bioma quando uma primitive geral resolve;
- determinismo, chunk seams e generation-order invariance obrigatórios;
- Hydrology legado continua proibido;
- mudanças de worldgen devem ser validadas em mundo novo.

## Próximos passos

1. fechar CI do Cut 24;
2. implementar ocean margin lower-only;
3. implementar constraints volume→surface;
4. implementar surface indicator via structures;
5. reescrever floating islands e estratificação;
6. gameplay em mundo novo para validar o pacote e coletar novo log do hitch;
7. retomar cold-start de `PrepareResources` depois do pacote.

## Regras de continuidade

- trabalhar em `architecture/asteria-core-rebuild`, nunca direto em `develop`;
- não force-push/rewrite de commits publicados; usar fast-forward;
- corrigir root cause de Clippy/Check, nunca esconder warning com `allow`;
- preservar gameplay/content/UI/assets válidos durante o rebuild;
- não inventar performance claims sem logs reais.
