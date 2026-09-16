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

---

# Estado atual

Último HEAD de código publicado: `100f5bb2efc93101cc086e5820992ee85a39d0aa`  
Bloco: `Reserve async capacity for visible chunk meshes`  
`VERSION = 0.14.59`

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
  - generation evita despachar vários chunks da mesma região/coluna fria até o primeiro chunk útil aquecer os caches compartilhados, evitando vários workers bloqueados no mesmo `OnceLock`;
  - dispatch pode examinar até 16 candidatos/frame para pular candidatos temporariamente bloqueados sem head-of-line blocking.
- `100f5bb` / 0.14.59: caminho `ready -> visible` ganha precedência:
  - initial mesh dispatch roda antes de novo generation dispatch no frame;
  - `ready` procura chunks à frente do movimento depois dos chunks críticos;
  - enquanto há mesh backlog, generation fica limitada a até 4 tasks in-flight pelo dispatcher, reservando capacidade do pool compartilhado para converter chunks já gerados em meshes visíveis.

## CI recente

- 0.14.55 / run `35043364577`: Clippy + `cargo check` success.
- 0.14.56 / run `35045326563`: Clippy + `cargo check` success.
- 0.14.57: validação Rust acionada e o bloco foi executado em runtime pelo usuário com ganho observado.
- 0.14.58 / run `35046400211`: em validação no momento deste handoff.
- 0.14.59 / run `35046538650`: em validação no momento deste handoff.
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
- streaming agora carrega uma noção de direção, prefetch à frente e prioridade distinta para superfície quando o jogador está voando acima dela;
- geração fria é serializada somente enquanto precisa aquecer dependências compartilhadas, evitando ocupar múltiplos workers esperando o mesmo cache;
- quando há mesh backlog, generation não pode monopolizar todo o pool async.

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
- fluid frontier usa boundary metadata + scan limitado à face.

O antigo roadmap de micro-refactor genérico terminou em 0.14.55. Nova evidência runtime reabriu o trabalho com foco em throughput, continuidade e regressões visuais reais.

---

# Ciclo ativo — prioridades runtime

## P0.1 — Chunk streaming: **seamless time-to-visible**

Sintoma confirmado:

- FPS pode permanecer ~60 enquanto generation/rendering ficam atrás do movimento;
- antes de 0.14.56 o jogador atravessava vários chunks vazios;
- 0.14.56 e principalmente 0.14.57 melhoraram o comportamento, mas o usuário ainda observa ganho insuficiente;
- requisito atualizado: **voar pelo mapa não pode expor chunks vazios/void**.

Pipeline atual:

`selection/prefetch -> generation dispatch/task -> generation integration -> initial lighting -> halo snapshot -> mesh dispatch/task -> mesh integration -> spawn`

### Gargalos já tratados

1. prioridade perdida depois da seleção: 0.14.56;
2. pool async subdimensionado pelo default do Bevy: 0.14.57;
3. workers desperdiçados aguardando a mesma região/coluna fria: 0.14.58;
4. flight priorizando ar local em vez de superfície visível: 0.14.58;
5. ausência de lookahead direcional: 0.14.58;
6. generation sendo despachada antes de mesh já pronta para avançar: 0.14.59;
7. generation monopolizando o pool enquanto há mesh backlog: 0.14.59.

### Nuances importantes

- generation/mesh tasks atuais são CPU-bound e os futures não têm pontos de `await`/yield internos; dropar o handle evita polling futuro, mas não interrompe magicamente CPU já em execução.
- `GENERATION_REGION_SIZE_CHUNKS = 8`; hydrology/volume/caves são compartilhados por região e usam caches `OnceLock`.
- flight speed atual = `WALK_SPEED 5 * FLY_SPEED_MULTIPLIER 5 = 25` blocos/s; chunk = 16 blocos, portanto ~1.56 chunks horizontais/s em velocidade máxima configurada.
- o preload direcional de 0.14.58 adiciona 2 chunks à frente além do preload base, sem transformar toda a seleção em um anel simétrico maior.

### Próximo passo se 0.14.59 ainda mostrar void

Instrumentar apenas o necessário para localizar o novo limitante, medindo por estágio:

- pending generation;
- generation in-flight;
- generation ready awaiting integration;
- ready awaiting initial lighting/mesh dispatch;
- mesh in-flight;
- mesh ready awaiting integration;
- distância em chunks do primeiro buraco visível à frente.

Com base nisso, próximos candidatos estruturais, nesta ordem:

1. separar generation de mesh/remesh em pools dedicados com orçamento total controlado, se a competição do pool compartilhado continuar sendo o limitante;
2. tornar cold regional prerequisites uma task explícita/readiness explícita, em vez de inferir aquecimento pela primeira geração útil;
3. mover initial direct-light seed para pipeline async stale-safe ou introduzir readiness de lighting, se `ready -> mesh` for o estágio saturado;
4. aumentar lookahead de forma **adaptativa à velocidade/backlog**, não como render distance artificial gigante;
5. se throughput físico ainda não puder acompanhar um movimento permitido pelo jogo, implementar uma estratégia visual de LOD/far terrain/fog coerente — não aceitar void cru como fallback e não congelar o jogador como solução padrão.

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
- initial/direct lighting seed continua síncrono entre generation integrada e mesh dispatch.

Próximo trabalho:

1. seguir `lamp placement -> lighting queue -> changed_chunks -> remesh -> mesh visível`;
2. separar latência de propagação vs remesh vs mesh integration;
3. impedir lighting parcial de aparecer como seam;
4. considerar content/light readiness + revisions por chunk se necessário.

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

## Candidate: per-chunk content/light readiness + revisions

Pode servir simultaneamente para:

- direct-light seed async stale-safe;
- impedir shadow seam com lighting parcial;
- saber quando chunk está realmente pronto para mesh/spawn;
- reduzir remesh/reprocessamento após edits.

Revisions atuais não são ideais:

- `chunk_mesh_revision` é ampla porque lighting também a altera;
- `block_content_revision` é global e não cobre fluid changes como revision por chunk.

Se P0.1/P0.2 confirmarem, introduzir owner explícito para revision/readiness de conteúdo por chunk (blocks + fluids) e estado de lighting, separado de mesh revision.

Lighting lazy expansion só é aceitável se preservar FIFO + dedup global + priority promotion da fila atual.

---

# Ordem de execução no próximo `go`

1. **P0.1 seamless streaming** — validar 0.14.58/0.14.59 em flight máximo; se ainda houver void, instrumentar backlog por estágio e atacar o estágio comprovado.
2. **P0.2 lighting/shadows** — lamp latency + seam; coordenar com initial lighting se compartilhar o gargalo.
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
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar worker convoy em caches compartilhados;
- stack/reuse para scratch limitado;
- evitar scans globais, allocations temporárias e writes idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative;
- **void cru nunca é o fallback visual aceitável para streaming normal.**