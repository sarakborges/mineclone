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
29. Streaming distingue **raio visível obrigatório**, **faixa warm/preload** e **raio de retenção/unload**. Trabalho faltando no raio visível sempre vence preload; preload compra antecedência; retenção evita destruir trabalho pronto cedo demais.
30. A zona preditiva distante não deve ficar visualmente exposta. Uma shell já pronta pode renderizar atrás da parte opaca da fog, mas chunks ainda em preparação permanecem ocultos.
31. **Retirement != unload imediato:** chunks aposentados permanecem elegíveis para reuso enquanto ainda estiverem dentro do raio de retenção. Reentrada no campo útil não deve exigir reconstruir mesh que acabou de ser descartado.

---

# Estado atual

Último HEAD de código publicado: `f5cb883765ad7bb0a63118f30a8289a9386197c5`  
Bloco: `Retain nearby chunks across movement`  
`VERSION = 0.14.68`

## Histórico recente relevante

- 0.14.31–0.14.55: ciclo de streaming/worldgen/lighting/perf com COW, queues, scratch reuse, Bevy hash collections, async mesh halo, metadata por índice e fim do micro-refactor genérico.
- `37959ac` / 0.14.56: prioridade crítica sobrevive ao dispatch async; generation/mesh distante podem ser preemptados e refileirados.
- `643a30c` / 0.14.57: configura `TaskPoolPlugin` explicitamente; feedback runtime: ganho perceptível, ainda não seamless.
- `7451d77` / 0.14.58: direção horizontal, preload à frente, prioridade de superfície em flight e anti-convoy inicial.
- `100f5bb` / 0.14.59: initial mesh antes de generation; generation limitada quando há mesh backlog.
- `a0921ca` / 0.14.60: anti-convoy passa a consultar readiness real dos `OnceLock`s.
- `52ec317` / 0.14.61: corrige gate de Clippy via `DesiredChunkSelection`; runtime ainda mostrava buracos em flight.
- `91fbf4d` / 0.14.62: content revision por chunk separada de lighting revision; lighting não invalida mais mesh inicial só por convergência.
- `ebbc019` / 0.14.63: preload base +2, corredor frontal até +8 e `visibility_band`; runtime: melhorou materialmente e o jogador já não alcança os buracos, embora ainda consiga vê-los.
- `b1061f8` / 0.14.64: render entities warm nascem ocultos fora do raio nominal.
- `5a58d04` / 0.14.65: `Added<ChunkRenderCoord>` promove imediatamente meshes que terminam já dentro do raio.
- `eadc7f5` / 0.14.66: corrige `private_interfaces` de `ChunkVisibilityState`; CI verde.
- `bccddfb` / 0.14.67: cria shell visual de +2 chunks além do raio nominal, mantendo a fog baseada no raio configurado, para que a promoção aconteça atrás da fog opaca; CI verde. Feedback runtime: **nenhuma diferença perceptível; chunks ainda aparecem renderizando atrás da fog**.
- `f5cb883` / 0.14.68: corrige churn de unload observado ao andar em círculos:
  - o usuário confirmou que chunks já vistos descarregavam e depois voltavam como buracos quando reentravam no raio de visão;
  - a fila `retired` deixa de significar “descarregar assim que possível” e passa a respeitar uma histerese horizontal de retenção;
  - `ChunkStreamingState::pop_retired_outside_horizontal_radius` só libera um retired para archive quando ele não está mais em `desired/retained` **e** já está fora do raio de retenção;
  - reentrada em `desired` mantém o retired na fila, mas impede unload; se sair novamente no futuro, o mesmo marcador pode voltar a ser elegível;
  - raio de retenção deriva da render distance: `R + max(ceil(R/2), 10)` chunks; exemplos: RD 4 -> 14, RD 12 -> 22, RD 24 -> 36;
  - decisão é horizontal-only para não destruir superfície/terreno útil só porque o jogador mudou altitude;
  - testes cobrem retenção dentro do raio, unload ao cruzar a borda e reentrada em selection.

## CI recente

- 0.14.61 / run `35047030986`: Clippy + `cargo check` success.
- 0.14.62 / run `35047686262`: Clippy failure apenas por getters runtime-dead; corrigido em 0.14.63.
- 0.14.66 / run `35049199522`: Clippy + `cargo check` success.
- 0.14.67 / run `35049716939`: Clippy + `cargo check` success.
- 0.14.68 / run `35050184571`: em validação no momento deste handoff.
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
- quando há mesh backlog, generation não monopoliza todo o pool async;
- mesh async valida content revision por chunk em vez de ser invalidado por churn de lighting;
- raio nominal tem precedência de fila; capacidade restante aquece a margem/previsão;
- shell visual pode existir atrás da fog; zona preditiva distante permanece oculta;
- entities recém-criados dentro da shell elegível recebem visibilidade no próprio frame;
- retired chunks próximos não são mais arquivados imediatamente; unload possui histerese baseada em render distance.

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
- lighting revision é separada da validade de conteúdo do mesh async.

---

# Ciclo ativo — prioridades runtime

## P0.1 — Chunk streaming: seamless time-to-visible + no unload churn

Sintomas/evidências confirmadas:

- FPS pode permanecer ~60 enquanto generation/rendering ficam atrás do movimento;
- 0.14.56–0.14.62 melhoraram throughput/prioridade, mas buracos continuaram;
- 0.14.63 fez a fronteira ficar fisicamente inalcançável em flight máximo, indicando throughput de antecipação suficiente para acompanhar o jogador;
- 0.14.64–0.14.67 tentaram separar warm/visible e esconder promoção atrás da fog, mas o usuário não percebeu melhora visual;
- nova evidência decisiva: **andar em círculos descarregava chunks já prontos; ao retornar, eles reapareciam como buracos vazios dentro do raio de visão**;
- isso identifica churn de unload/rebuild como gargalo concreto adicional, não apenas fog/promoção;
- requisito permanece: nenhum void/chunk ausente visível em movimento normal ou flight configurado.

Pipeline:

`selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility -> retention -> archive/unload`

### Gargalos tratados

1. prioridade perdida após seleção: 0.14.56;
2. pool async subdimensionado: 0.14.57;
3. worker convoy em caches frios: 0.14.58/0.14.60;
4. flight priorizando ar local em vez de superfície: 0.14.58;
5. lookahead direcional insuficiente: 0.14.58/0.14.63;
6. generation antes de mesh pronta: 0.14.59;
7. generation monopolizando pool com mesh backlog: 0.14.59;
8. lighting invalidando mesh inicial: 0.14.62;
9. preload curto/direção vencendo missing visible: 0.14.63;
10. warm parcialmente exposto: 0.14.64;
11. novo mesh dentro do raio esperando movimento para promoção: 0.14.65;
12. shell de promoção ainda perceptível atrás da fog: 0.14.67, sem melhora runtime confirmada;
13. **unload precoce destruindo chunks prontos durante trajetórias circulares/revisita**: 0.14.68.

### Horizontes atuais

- render distance nominal = banda de prioridade/visibilidade útil;
- base preload de selection continua +2 chunks;
- corredor frontal continua até +8 além da base quando existe movimento horizontal;
- shell visual 0.14.67 permite render até +2 além do nominal, atrás da fog calculada sobre o nominal;
- fog linear: início ~78% e fim ~98% do raio nominal;
- unload retention 0.14.68: `R + max(ceil(R/2), 10)`; default RD 12 -> 22 chunks;
- a pergunta do usuário sobre derivar **todos** os horizontes da render distance permanece válida: fog e retention já escalam; base preload/shell visual ainda têm offsets fixos. Não refatorar isso cegamente antes do runtime da 0.14.68, porque a evidência mais forte atual é churn de unload.

### Próximo diagnóstico após 0.14.68

1. repetir trajeto circular e revisitar exatamente a mesma área; chunks ainda dentro da retenção não devem perder render allocation;
2. testar flight máximo em linha reta e curvas longas;
3. se os buracos de revisita sumirem mas ainda houver pop-in frontal, medir first-missing coord por estágio (`pending -> generation -> ready -> mesh -> pool`) e distância ao jogador;
4. se ainda houver chunk pronto sendo destruído dentro da retenção, auditar `retire_chunk_render_allocation` e qualquer caminho alternativo de despawn;
5. depois, centralizar horizons de render/fog/preload/retention num modelo derivado de render distance + velocidade/backlog, separando distância estética de lead time preditivo;
6. só se a fronteira nominal ainda contiver gaps reais, implementar frontier readiness/contiguous promotion.

Não marcar P0.1 resolvido até runtime contínuo em flight máximo e revisita/círculos sem buracos visíveis.

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
4. adicionar readiness explícita de lighting apenas se a evidência runtime exigir.

Não marcar resolvido sem runtime com lamp junto e longe de fronteiras.

## P0.3 — Hydrology: continuidade de rivers/lakes/tunnels

Sintomas confirmados:

- rivers/lakes ainda podem produzir paredes/cortes retos;
- rivers podem nascer ou morrer sem origem/destino visualmente válidos;
- river/tunnel pode terminar abruptamente na interseção.

Causa concreta em `src/world/generation/density.rs`:

- `surface_carver_allowed = pass.region.hydrology.water_at(horizontal).is_none()` desliga o surface tunnel carver inteiro por coluna ao entrar em água;
- próximo fix deve compor/blendar o carver com hydrology, sem shaft/água quebrada.

Investigar também origem/destino, continuidade entre regiões, margens/topo, confluences e waterfall outlets.

## P1.1 — Mountains dominantes + regionais raros

Feedback confirmado:

- Mountains muito presentes;
- Witchwood, Enchanted Forest e Wasteland raros;
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

Montanhas usam distributions especiais; corrigir cobertura belt+peak, não só `weight`.

## P1.2 — Coast isolada no interior

- `coast.json` e `ocean.json` são `kind: hydrology` e não entram no selector regional comum;
- investigar hydrology overlay/identity, `ocean_strength`, coast blend thresholds e continentalness residual.

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
- dye foi reportado como fraco;
- Player HUD / target HUD permanecem fora deste ciclo até repriorização explícita.

---

# Refactor estrutural ainda relevante

## Per-chunk content revision implementada; lighting readiness continua condicional

Implementado em 0.14.62:

- `VoxelWorld` possui revision autoritativa por chunk para conteúdo voxel (blocks + fluids);
- insert/restore/block/fluid bumpam content revision;
- lighting não bumpa content revision;
- async mesh dependencies usam content revision e deixam lighting churn para background remesh;
- `chunk_mesh_revision` permanece separada para estado visual que inclui lighting.

Ainda pode servir ao P0.2, se evidência exigir:

- readiness explícita de lighting por chunk;
- direct-light seed async stale-safe;
- impedir shadow seam com lighting parcial sem bloquear primeira visibilidade.

---

# Ordem de execução no próximo `go`

1. **P0.1 seamless streaming** — validar 0.14.68 com círculos/revisita e flight máximo; confirmar que chunks próximos não são mais descarregados/reconstruídos.
2. Se revisita estiver resolvida mas houver pop-in frontal, instrumentar first-missing stage/distance e então centralizar horizons por render distance + lead time real.
3. **P0.2 lighting/shadows** — lamp latency + seam.
4. **P0.3 hydrology continuity** — river/tunnel gate binário, endpoints e margens/topo.
5. **P1.1 biome distribution** — reduzir Mountains, aumentar regionais e oak de Plains.
6. **P1.2 coast isolada** — corrigir overlay/identity hidrológico.
7. **P1.3 clouds** — corrigir altitude absoluta.
8. Retomar refactor estrutural conforme evidência dos P0.

---

# Performance direction

Meta não é apenas ~60 FPS; é mundo visualmente pronto antes de o jogador alcançá-lo e estável durante revisita.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados sem backlog visível;
- prioridade por visibilidade + direção, não só distância euclidiana;
- preencher primeiro banda útil e usar capacidade restante para prewarm;
- predictive lead deve depender de velocidade/latência; render/fog/retention devem respeitar render distance;
- não destruir render allocations recém-prontas enquanto ainda estão na vizinhança de retenção;
- reservar throughput para finalizar chunks já gerados antes de criar backlog novo;
- revision tracking por domínio;
- caches/metadata no owner correto;
- evitar worker convoy;
- stack/reuse para scratch limitado;
- evitar scans globais, allocations temporárias e writes idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative;
- **void cru nunca é fallback visual aceitável para streaming normal.**
