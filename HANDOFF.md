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

`cdaccb8a5273d1feb1ecae50d9ee8e184cd1d391`

Commit: `Make queue promotions amortized O(1)`

`VERSION`: `0.12.59`

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

### 0.12.53 — surface light reutilizada no meshing
- Terrain e fluid meshing deixam de consultar `VoxelRead::light_at` para o mesmo voxel-fonte a cada face exposta.
- A block light do voxel-fonte é lida uma vez do `VoxelChunk` já disponível e reutilizada por todas as faces daquele voxel.
- AO, neighborhood, block-light interpolation e escolha de diagonal mantêm a mesma semântica.
- Não separar lighting remesh em atualização ingênua de atributos: block light participa de `should_flip_diagonal` quando AO empata e pode mudar índices.

### 0.12.54 — samples reutilizados na propagação de iluminação
- Cada voxel retirado da `LightingQueue` resolve posição/chunk uma vez via `VoxelWorld::sample_at`.
- `cell`, `fluid` e luz atual são reutilizados para loaded-state, dampening, emissão e comparação.
- Em voxels não-opacos, seis luzes cardinais são lidas uma vez e compartilhadas entre skylight e block light: 12 leituras vizinhas -> 6.

### 0.12.55 — terrain refresh preserva meshes de fluido
- `refresh_chunk_geometry_mesh` constrói somente terrain.
- `replace_terrain_mesh_assets` substitui apenas o prefixo terrain; fluid permanece intacto.
- `mesh_bytes` recompõe terrain novo + `fluid_mesh_bytes` preservados.

### 0.12.56 — fluid topology refresh preserva terrain
- `ChunkRenderPool` centraliza o invariant do sufixo fluido.
- Mudança de `FluidId`s/handles destaca e recria apenas allocation fluida.
- Terrain entities/handles/keys/bytes permanecem intactos.

### 0.12.57 — terrain topology refresh preserva fluidos
- Terrain é prefixo da allocation; fluid permanece sufixo.
- Mudanças de `ChunkMeshKey`, quantidade de terrain meshes ou handles faltantes recriam só terrain.
- Spawn de terrain é compartilhado entre full spawn e rebuild parcial.

### 0.12.58 — seed direto de lighting usa storage local do chunk
- O scan acima do chunk continua lendo o `VoxelWorld` autoritativo e produz 256 níveis iniciais de skylight, um por coluna local.
- `VoxelWorld::rebuild_chunk_light` restringe a mutação ao canal de light e não expõe `&mut VoxelChunk`.
- Os 4.096 voxels internos deixam de fazer `sample_at(position)` + `set_light_at(position)`: 8.192 resoluções de posição/chunk eliminadas por seed.
- Dampening, emissão e ordem top-down permanecem iguais, sem buffer temporário de 4.096 luzes.

### 0.12.59 — promoção/remoção de fila amortizada O(1)
- `DeduplicatedQueue` troca `HashSet<T>` por `HashMap<T, generation>` e armazena `(value, generation)` no `VecDeque`.
- `enqueue_front` não procura/remove mais a ocorrência antiga: publica uma nova geração na frente e a anterior vira tombstone.
- `remove` invalida membership em O(1), sem `VecDeque::position + remove`.
- `pop` ignora tombstones; `len` representa apenas entradas ativas.
- Compactação periódica limita tombstones sem voltar ao full scan a cada promoção/remoção.
- `pop_where` preserva a ordem observável dos itens adiados e continua sendo O(n) por necessidade de predicate arbitrário; esse é o próximo ponto a investigar no domínio de remesh.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` e `ChunkMeshTasks`; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não transformar unload em pipeline incremental com snapshot temporal de streaming sem evidência de hotspot real.
- `ChunkContent`, `ChunkGeneration`, `CurrentDimensionContext`, `ChunkRenderer` e `ChunkUnloadRuntime` continuam coerentes enquanto representarem os concerns atuais.
- Não separar lighting remesh em simples atualização de atributos mantendo índices fixos: `should_flip_diagonal` também depende da block light.
- Não expor `VoxelWorld::chunk_mut` genericamente para otimizações bulk; mutações devem permanecer estreitas e ownership-aware.
- `DeduplicatedQueue` mantém FIFO/prioridade lógica através de generations/tombstones; não voltar a remoção linear por promoção sem benchmark/evidência.
- Não reabrir bugs antigos automaticamente; só se permanecerem ativos ou houver regressão reportada.

---

# Próximos passos da auditoria

Se nenhum error/warning/runtime report tiver prioridade:

1. Auditar `ChunkRemeshQueue::pop_renderable*`: `pop_where` pode reescanear em frames sucessivos filas compostas apenas por chunks ainda não presentes no `ChunkRenderPool`; buscar um sinal/revision que evite re-scan sem descartar trabalho adiado.
2. Continuar procurando chamadas repetidas de `VoxelWorld`/`VoxelRead` para a mesma posição nos hot paths restantes e usar `sample_at` apenas quando múltiplos aspectos do mesmo voxel forem necessários.
3. Revisar o custo de `enqueue_emission_edit_volumes`: expansão Manhattan de emissores pode inserir milhares de posições e deve continuar deduplicada; só otimizar se houver invariant que reduza trabalho sem perder relight em remoção/oclusão.
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
