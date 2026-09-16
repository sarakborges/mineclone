# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch de trabalho: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras de trabalho

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica e persistente do projeto. Exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`; feature branch só sob pedido explícito.
- Antes de escrever, buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Commits pequenos e coerentes; não misturar mudanças sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível; minor para feature compatível; major para breaking.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- Commit exclusivamente documental de `HANDOFF.md` não sobe `VERSION`.
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Corrigir warnings de Rust nos blocos tocados.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando aplicável.
- Não gerar imagens sem pedido explícito.
- Quando o usuário disser `go`/`continua`, executar o próximo bloco aplicável sem confirmação desnecessária.
- Não repetir que falta `cargo check`/`cargo run`; CI canônico valida Clippy + check.
- `cargo test` só é rodado manualmente sob pedido explícito.
- `cargo fmt`/`rustfmt` não é gate do projeto.

## Versionamento e validação

- Fonte operacional: arquivo raiz `VERSION`.
- `src/app/version.rs` mantém `VERSION` separado de `Cargo.toml` para evitar invalidar fingerprints do Cargo em bumps frequentes.
- `[package].version = 0.10.16` permanece intencionalmente divergente enquanto essa política existir; não sincronizar silenciosamente.
- CI `.github/workflows/ci.yml` executa:
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo check`
- Push/PR exclusivamente de `HANDOFF.md` é ignorado via `paths-ignore`; código, `VERSION` e config continuam validando normalmente.

## Canon arquitetural

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair semelhança superficial.
3. Preferir `SystemParam`s estreitos/coerentes e availability/run conditions canônicas.
4. UI compartilhada pertence a `src/ui`.
5. Targeting tem um único target autoritativo e consumers change-driven.
6. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são os primitives canônicos de fila deduplicada.
7. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
8. Cores internas são HSI-first; visuals ponderados usam `CurrentBiomeVisuals`.
9. Evitar scans globais, allocations temporárias e dirty writes quando o owner já tem sinal/metadata.
10. Pipeline inicial: generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> spawn.
11. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
12. Terrain/fluid/lighting background remesh é async; immediate player-edit geometry permanece síncrono.
13. Não trocar corretude por performance aparente e não criar abstração genérica sobre generation/mesh task queues.
14. Sistemas visuais não devem reescrever Components/Assets com o mesmo valor.
15. Movimento com delta zero permanece ocioso.
16. Scratch recorrente/estruturalmente limitado prefere stack/reuse a heap repetida.
17. `NewWorldConfig` é owner das escolhas de criação de mundo; forced biome valida coluna inicial e safe-spawn final no mesmo biome.
18. Search/text-input compartilhado pertence a `src/ui`.
19. Preferências de HUD pertencem a `HudSettings`; targeting não conhece layout.
20. Queries mutáveis múltiplas sobre o mesmo Component precisam ser disjuntas via `Without<T>` ou `ParamSet`.
21. Child UI que obedece ao hide do parent usa `Visibility::Inherited`.
22. Revisions derivadas são separadas por domínio quando consumers têm dependências diferentes.
23. Prioridade de streaming deve sobreviver às fronteiras async; trabalho imediato pode preemptar apenas trabalho não imediato e a vítima volta à fila.
24. Worldgen/initial mesh/background remesh compartilham `AsyncComputeTaskPool`; quantidade de tasks in-flight não equivale a workers executando.
25. **Invariant de produto para streaming:** em velocidade normal configurada, inclusive flight, o jogador não deve enxergar void/chunks ainda ausentes. Throughput deve ser antecipado e priorizado para tornar o streaming visualmente seamless; não aceitar “60 FPS com mundo atrasado” como sucesso.
26. Anti-convoy de worldgen deve observar readiness real dos caches compartilhados; não manter shadow state de “cache aquecido” quando o próprio `OnceLock` já é a fonte autoritativa.
27. Validade de mesh async usa revision de **conteúdo por chunk** (blocks + fluids), separada de `chunk_mesh_revision`; mudanças de lighting não devem descartar geometria já construída e ficam responsáveis por seu próprio background remesh.
28. Neighbor ausente no snapshot inicial pode aparecer enquanto o mesh está em flight; isso não invalida a primeira aparição, porque o carregamento/visibilidade do neighbor já agenda a correção de fronteira por remesh.

---

# Estado atual

Último HEAD de código publicado: `91fbf4db862f156be068213df452de7d9e020c54`  
Bloco: `Separate chunk content revisions from lighting mesh churn`  
`VERSION = 0.14.62`

## Histórico recente relevante

- 0.14.31–0.14.50: ciclo de streaming/worldgen/lighting/perf com COW, queues, scratch reuse, Bevy hash collections, async mesh halo e eliminação de lookups repetidos.
- `cbce686` / 0.14.51: CI ignora commits exclusivamente de `HANDOFF.md`.
- `573898d` / 0.14.52: terrain/modifiers/seed derivados do `BiomeField` por índice.
- `b28b3c2` / 0.14.53: corrige call site esquecido da nova assinatura no bootstrap.
- `7a0283d` / 0.14.54: `BiomeFieldSample.primary_surface_index` + hydrology metadata por índice.
- `ee806d0` / 0.14.55: corrige initializer de teste; CI verde.
- `37959ac` / 0.14.56: prioridade do raio imediato sobrevive ao dispatch async; generation/mesh distante podem ser preemptados e refileirados; `ready` deixa de ser FIFO puro para chunks críticos.
- `643a30c` / 0.14.57: configura `TaskPoolPlugin` explicitamente: IO 10%/máx. 2 workers, async compute 50%/máx. 8, compute recebe o restante. Feedback runtime do usuário: **ganho perceptível, mas ainda não seamless**.
- `7451d77` / 0.14.58: streaming passa a antecipar movimento e atacar worker convoy/custo inútil durante flight:
  - mantém direção horizontal recente;
  - adiciona preload direcional de 2 chunks além do preload base;
  - chunks à frente ganham prioridade sobre lateral/traseira após os invariants locais;
  - quando o jogador está acima da faixa de superfície, terreno de superfície é priorizado antes de chunks de ar próximos, preservando o chunk atual como primeira prioridade;
  - generation evita despachar vários chunks da mesma região/coluna fria ao mesmo tempo;
  - dispatch pode examinar até 16 candidatos/frame para pular candidatos temporariamente bloqueados sem head-of-line blocking.
- `100f5bb` / 0.14.59: caminho `ready -> visible` ganha precedência:
  - initial mesh dispatch roda antes de novo generation dispatch no frame;
  - `ready` procura chunks à frente do movimento depois dos chunks críticos;
  - enquanto há mesh backlog, generation deixa de repor o pool acima de 4 tasks, reservando capacidade para converter chunks gerados em meshes visíveis.
- `a0921ca` / 0.14.60: anti-convoy deixa de inferir cache aquecido pelo primeiro chunk não vazio e passa a consultar diretamente `OnceLock::get()` em generation columns e prerequisites regionais (generation region + volume biomes + caves). Assim, dependência realmente fria continua com um único líder, mas cache que termina de inicializar libera paralelismo imediatamente mesmo antes de o chunk líder concluir todo o trabalho.
- `52ec317` / 0.14.61: corrige o gate do Clippy introduzido pelo novo preload; os parâmetros de `rebuild_desired_chunk_coords` passam por `DesiredChunkSelection` em vez de ultrapassar o limite de argumentos. Runtime subsequente do usuário: **buracos/void ainda aparecem em flight**.
- `91fbf4d` / 0.14.62: remove um bloqueio oculto entre `ready` e `visible`:
  - `VoxelWorld` passa a manter `chunk_content_revision` por chunk, atualizada por insert/restore e mutações de block/fluid, mas não por lighting;
  - `ChunkMeshDependencies` deixa de usar `chunk_mesh_revision` ampla e passa a validar conteúdo voxel por chunk;
  - propagation/relaxation de lighting pode alterar `chunk_mesh_revision` enquanto um mesh async está sendo construído sem tornar esse trabalho geométrico stale;
  - neighbor ausente durante capture pode carregar antes da integração sem obrigar descarte do mesh inicial; a correção de boundary fica com o remesh disparado pela visibilidade do neighbor;
  - mudanças reais de conteúdo em center/neighbor ainda invalidam o resultado async.

## CI recente

- 0.14.55 / run `35043364577`: Clippy + `cargo check` success.
- 0.14.56 / run `35045326563`: Clippy + `cargo check` success.
- 0.14.57: bloco executado em runtime pelo usuário com ganho observado.
- 0.14.58 / run `35046400211`: Clippy failure exclusivamente por `too_many_arguments` em `rebuild_desired_chunk_coords`; lógica compilável não foi validada pelo segundo gate porque `cargo check` foi skipped após Clippy.
- 0.14.59 / run `35046538650`: mesma falha herdada de Clippy; corrigida estruturalmente em 0.14.61, sem `#[allow]`.
- 0.14.60 / run `35046928557`: contém o lint herdado no selector e foi superseded pela correção estrutural.
- 0.14.61 / run `35047030986`: Clippy + `cargo check` success.
- 0.14.62 / run `35047686262`: em validação no momento deste handoff; é o gate canônico atual.
- Commits handoff-only não abrem Rust CI.

---

# Refactor/performance já consolidado

## Streaming/render

- unload/rebuild reutilizam scratch;
- generation/mesh/remesh pesados ficam em tasks; integração main-thread é budgetada;
- `stream_chunks` e `ChunkRemeshQueue` evitam polling/dispatch vazio;
- empty chunks compartilham buffers por `Arc`/COW;
- pending sort reutiliza scratch e preserva empate via ordinal;
- halo de mesh materializa async a partir de clones COW + revisions;
- mesh output mantém sort explícito;
- hot maps/sets sem semântica de ordem usam Bevy collections;
- chunks imediatos podem preemptar trabalho distante sem perder a vítima;
- Bevy task pools são dimensionados explicitamente para favorecer trabalho voxel async;
- streaming carrega direção horizontal recente, prefetch à frente e prioridade distinta para superfície quando o jogador está voando acima dela;
- generation fria evita worker convoy em dependências compartilhadas e consulta readiness diretamente no cache autoritativo;
- quando há mesh backlog, generation não deve monopolizar todo o pool async;
- mesh async valida content revision por chunk em vez de ser invalidado por churn de lighting.

## Worldgen/biome

- feature cache retention reutiliza scratch;
- `SurfaceCarverColumn`/`SurfaceMaterialColumn` reutilizam Vecs;
- `BiomeField::sample_surface` usa `ArrayVec`; generation columns usam `SmallVec`;
- `BiomeInfluence.surface_index` evita resolver o mesmo biome por string nos hot paths;
- terrain/hydrology metadata recorrente resolve pelo snapshot do `BiomeField`.

## Lighting

- direct seed pula upper chunks vazios;
- propagation usa leitura local de vizinhos quando possível;
- changed chunks consolidam mesh revision por batch;
- dynamic lighting só roda com trabalho;
- fluid frontier usa boundary metadata + scan limitado à face;
- lighting revision é separada da validade de conteúdo do mesh async; iluminação atualizada continua chegando por background remesh sem bloquear a primeira aparição do terreno.

O antigo roadmap de micro-refactor genérico terminou em 0.14.55. Nova evidência runtime reabriu o trabalho com foco em throughput, continuidade e regressões visuais reais.

---

# Ciclo ativo — prioridades runtime

## P0.1 — Chunk streaming: **seamless time-to-visible**

Sintoma confirmado:

- FPS pode permanecer ~60 enquanto generation/rendering ficam atrás do movimento;
- antes de 0.14.56 o jogador atravessava vários chunks vazios;
- 0.14.56–0.14.61 melhoraram throughput/prioridade, mas o usuário ainda confirma buracos em flight;
- requisito: **voar pelo mapa não pode expor chunks vazios/void**.

Pipeline atual:

`selection/prefetch -> generation dispatch/task -> generation integration -> initial lighting -> halo snapshot -> mesh dispatch/task -> mesh integration -> spawn`

### Gargalos já tratados

1. prioridade perdida depois da seleção: 0.14.56;
2. pool async subdimensionado pelo default do Bevy: 0.14.57;
3. workers desperdiçados aguardando a mesma região/coluna fria: 0.14.58, refinado com readiness real em 0.14.60;
4. flight priorizando ar local em vez de superfície visível: 0.14.58;
5. ausência de lookahead direcional: 0.14.58;
6. generation sendo despachada antes de mesh já pronta para avançar: 0.14.59;
7. generation monopolizando o pool enquanto há mesh backlog: 0.14.59;
8. mesh inicial sendo repetidamente descartado porque lighting relaxation alterava `chunk_mesh_revision` durante o trabalho async: 0.14.62.

### Nuances importantes

- generation/mesh tasks atuais são CPU-bound e os futures não têm pontos de `await`/yield internos; dropar o handle evita polling futuro, mas não interrompe magicamente CPU já em execução.
- `GENERATION_REGION_SIZE_CHUNKS = 8`; hydrology/volume/caves são compartilhados por região e usam caches `OnceLock`.
- readiness regional do anti-convoy exige generation region, volume biome region e cave region inicializados; `None` de caves ainda conta como cache inicializado porque o `OnceLock<Option<_>>` possui valor.
- flight speed atual = `WALK_SPEED 5 * FLY_SPEED_MULTIPLIER 5 = 25` blocos/s; chunk = 16 blocos, portanto ~1.56 chunks horizontais/s em velocidade máxima configurada.
- preload direcional adiciona 2 chunks à frente além do preload base, sem inflar todo o anel simetricamente.
- `chunk_content_revision` agora é o owner da validade geométrica voxel por chunk; `chunk_mesh_revision` continua cobrindo lighting/mesh e não deve voltar a ser usada como proxy de conteúdo.

### Próximo passo se 0.14.62 ainda mostrar void

Instrumentar apenas o necessário para localizar o novo limitante, medindo por estágio:

- pending generation;
- generation in-flight;
- generation completed awaiting integration;
- ready awaiting initial lighting/mesh dispatch;
- mesh in-flight;
- mesh completed awaiting integration;
- distância em chunks do primeiro buraco visível à frente.

Com base nisso, próximos candidatos estruturais, nesta ordem:

1. aumentar lookahead de forma **adaptativa à velocidade/backlog**, não como render distance artificial gigante;
2. separar generation de mesh/remesh em pools dedicados com orçamento total controlado, se a competição do pool compartilhado continuar sendo o limitante;
3. tornar cold regional prerequisites tasks explícitas/readiness explícita, se o custo de hydrology/volume/caves continuar dominando mesmo sem convoy;
4. mover initial direct-light seed para pipeline async stale-safe ou introduzir readiness explícita de lighting apenas se o seed síncrono aparecer como custo de main thread; ele não deve bloquear mesh por revision churn novamente;
5. se throughput físico ainda não puder acompanhar um movimento permitido pelo jogo, implementar estratégia visual coerente de far terrain/LOD/fog; **não aceitar void cru** e não congelar o jogador como solução padrão.

Não marcar P0.1 resolvido até runtime contínuo em flight máximo sem buracos visíveis.

## P0.2 — Lighting/shadows: lamp latency + seam entre chunks

Sintomas confirmados:

- lighting/shadow demora perceptivelmente para estabilizar;
- `lamp` evidencia a latência;
- durante convergência/carregamento aparece linha/sombra falsa na fronteira entre chunks.

Estado:

- dynamic lighting: budget de 2 ms / até 4096 voxels por frame;
- voxel edits entram com voxel + vizinhos prioritários;
- changed chunks entram em remesh conforme propagação altera o campo;
- initial/direct lighting seed continua síncrono entre generation integrada e mesh dispatch;
- desde 0.14.62 lighting pode convergir e solicitar remesh sem invalidar o primeiro mesh apenas por revision de luz.

Próximo trabalho:

1. seguir `lamp placement -> lighting queue -> changed_chunks -> remesh -> mesh visível`;
2. separar latência de propagação vs remesh vs mesh integration;
3. impedir lighting parcial de aparecer como seam;
4. adicionar readiness explícita de lighting apenas se a evidência runtime exigir; content revision já está separada.

Não marcar resolvido sem runtime com lamp junto e longe de fronteiras.

## P0.3 — Hydrology: continuidade de rivers/lakes/tunnels

Sintomas confirmados:

- rivers/lakes ainda podem produzir paredes/cortes retos;
- rivers podem nascer ou morrer sem origem/destino visualmente válidos;
- river/tunnel pode terminar abruptamente na interseção.

Causa concreta já identificada para river/tunnel em `src/world/generation/density.rs`:

- `surface_carver_allowed = pass.region.hydrology.water_at(horizontal).is_none()` desliga o surface tunnel carver inteiro por coluna ao entrar em água;
- o gate binário produz descontinuidade na borda de river/lake;
- próximo fix deve compor/blendar o carver com hydrology, sem reintroduzir shaft/água quebrada.

Investigar também:

- origem válida: spring/lake/upstream/confluence;
- destino válido: downstream contínuo até lake/ocean/confluence/trunk;
- continuidade entre hydrology regions;
- slope/blend vertical e horizontal de margens/topo;
- confluences e waterfall outlets.

Não marcar resolvido sem runtime de margem, nascente, foz, lago, confluence, waterfall e tunnel cruzando água.

## P1.1 — Mountains dominantes + regionais raros

Feedback confirmado:

- Mountains estão MUITO presentes;
- Witchwood, Enchanted Forest e Wasteland aparecem raramente;
- Plains precisa de leve aumento de oak trees.

Valores relevantes atuais:

- Plains/Wasteland regional weight: 1.0;
- Witchwood/Enchanted Forest regional weight: 1.0;
- Mountains dimension weight: 0.85;
- Plains/Wasteland size: 120–420;
- Witchwood/Enchanted Forest size: 140–460;
- Mountains metadata size: 90–260;
- Plains oak: spacing 80, chance 0.48, jitter 24;
- `mountain_belt`: scale 0.0017, threshold 0.86, width 0.22, warpStrength 85;
- `mountain_peak`: spacing 760, chance 0.48, radius 120–230, warpStrength 42.

Montanhas usam distributions especiais; sobrepresença deve ser corrigida na geometria/cobertura belt+peak, não só no `weight`.

## P1.2 — Coast isolada no interior

- `coast.json` e `ocean.json` são `kind: hydrology` e não entram no selector regional comum;
- coast isolada deve ser investigada no hydrology overlay/identity;
- conferir `ocean_strength`, coast blend thresholds e continentalness residual.

## P1.3 — Clouds não renderizam

Causa provável forte:

- `src/rendering/sky_layers/clouds.rs` usa altitude absoluta ~34–48;
- Overworld `seaLevel = 90`;
- X/Z seguem câmera, Y permanece absoluto, então clouds ficam abaixo do terreno/jogador.

Próximo fix: altitude coerente com dimensão/mundo, preservando density/color data-driven.

## P2 — Ghost/held block e outros

- held block observa apenas `PlayerHotbar.selected_slot()` + `item_at`;
- slot vazio deve esconder root/faces;
- placement preview deve limpar id/transform/visibility sem item;
- transparência do ghost deve ser percebida no bloco inteiro;
- dye foi reportado anteriormente como fraco;
- Player HUD / target HUD permanecem fora deste ciclo até repriorização explícita.

---

# Refactor estrutural ainda relevante

## Per-chunk content revision implementada; lighting readiness continua condicional

Implementado em 0.14.62:

- `VoxelWorld` possui revision autoritativa por chunk para conteúdo voxel (blocks + fluids);
- insert/restore/block/fluid bumpam content revision;
- lighting não bumpa content revision;
- async mesh dependencies usam content revision e deixam lighting churn para a fila de background remesh;
- `chunk_mesh_revision` permanece separada para estado visual que inclui lighting.

Ainda pode servir ao P0.2, se evidência exigir:

- readiness explícita de lighting por chunk;
- direct-light seed async stale-safe;
- impedir shadow seam com lighting parcial sem bloquear primeira visibilidade.

Lighting lazy expansion só é aceitável se preservar FIFO + dedup global + priority promotion da fila atual.

---

# Ordem de execução no próximo `go`

1. **P0.1 seamless streaming** — validar 0.14.62 em flight máximo; se ainda houver void, instrumentar backlog por estágio e atacar o estágio comprovado, começando por lookahead adaptativo/backlog e competição entre pools.
2. **P0.2 lighting/shadows** — lamp latency + seam; aproveitar a separação content/light feita em 0.14.62.
3. **P0.3 hydrology continuity** — primeiro river/tunnel gate binário, depois endpoints e margens/topo.
4. **P1.1 biome distribution** — reduzir Mountains materialmente, tornar regionais perceptíveis e aumentar levemente árvores de Plains.
5. **P1.2 coast isolada** — corrigir overlay/identity hidrológico.
6. **P1.3 clouds** — corrigir altitude absoluta incompatível com sea level/mundo.
7. Retomar refactor estrutural conforme evidência dos P0, evitando micro-churn sem efeito runtime.

---

# Performance direction

Meta não é apenas ~60 FPS; é mundo visualmente pronto antes de o jogador alcançá-lo.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados sem backlog visível;
- prioridade baseada em visibilidade e direção de movimento, não só distância euclidiana;
- preload adaptativo/preditivo em vez de render radius inflado indiscriminadamente;
- reservar throughput para finalizar chunks já gerados antes de criar backlog novo;
- revision tracking por domínio para stale async work;
- caches/metadata no owner correto;
- evitar worker convoy em caches compartilhados;
- stack/reuse para scratch limitado;
- evitar scans globais, allocations temporárias e writes idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative;
- **void cru nunca é o fallback visual aceitável para streaming normal.**
