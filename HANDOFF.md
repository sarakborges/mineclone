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
6. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são primitives canônicos de fila deduplicada.
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
26. Anti-convoy de worldgen deve observar readiness real dos caches compartilhados; não manter shadow state quando o próprio `OnceLock` já é a fonte autoritativa.
27. Validade de mesh async usa revision de **conteúdo por chunk** (blocks + fluids), separada de `chunk_mesh_revision`; mudanças de lighting não devem descartar geometria já construída.
28. Neighbor ausente no snapshot inicial pode aparecer enquanto o mesh está em flight; isso não invalida a primeira aparição, porque a correção de fronteira vem por remesh.
29. Streaming distingue **raio visível obrigatório**, **faixa warm/preload** e **raio de retenção/unload**. Trabalho faltando no raio visível sempre vence preload; preload compra antecedência; retenção evita destruir trabalho pronto cedo demais.
30. A zona preditiva distante não deve ficar visualmente exposta. Uma shell já pronta pode renderizar atrás da parte opaca da fog, mas chunks ainda em preparação permanecem ocultos.
31. **Retirement != unload imediato:** chunks aposentados permanecem elegíveis para reuso enquanto ainda estiverem dentro do raio de retenção. Reentrada no campo útil não deve exigir reconstruir mesh recém-descartado.
32. **Visibilidade usa histerese:** um chunk oculto entra apenas no raio de `show`; um chunk já visível só sai no raio maior de `hide`. Nunca usar o mesmo limiar para `Visible <-> Hidden`, pois movimento junto à fronteira causa flicker.

---

# Estado atual

Último HEAD de código publicado: `5701f00f27682d2ab4ea97c9dae2b5196f858b05`  
Bloco: `Add visibility hysteresis at render boundary`  
`VERSION = 0.14.69`

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
- `bccddfb` / 0.14.67: shell visual +2 além do nominal; fog continua baseada no raio configurado. Feedback runtime: nenhuma melhora perceptível; chunks ainda apareciam renderizando atrás da fog.
- `f5cb883` / 0.14.68: corrige churn de unload observado ao andar em círculos:
  - retired chunks só podem ser arquivados fora de um raio horizontal de retenção;
  - reentrada em `desired/retained` bloqueia unload e preserva render allocation/mesh;
  - retenção deriva da render distance: `R + max(ceil(R/2), 10)`; exemplos RD 4 -> 14, RD 12 -> 22, RD 24 -> 36;
  - decisão horizontal-only para não destruir superfície por mudança de altitude.
- `5701f00` / 0.14.69: corrige flicker visual de chunks distantes:
  - remove o único limiar binário de visibilidade;
  - `chunk_visibility_radii(render_distance)` calcula `show` e `hide` a partir da render distance;
  - a shell de entrada permanece limitada pela garantia atual de preload all-direction (+2), enquanto a banda de saída/histerese escala com a render distance;
  - exemplos: RD 4 => show 5/hide 6; RD 12 => show 14/hide 16; RD 24 => show 26/hide 30;
  - chunk `Hidden` só vira `Visible` dentro de `show`; chunk `Visible` permanece visível até ultrapassar `hide`;
  - isso impede `Visible -> Hidden -> Visible` ao orbitar ou cruzar repetidamente a borda do círculo.

## CI recente

- 0.14.61 / run `35047030986`: Clippy + `cargo check` success.
- 0.14.66 / run `35049199522`: Clippy + `cargo check` success.
- 0.14.67 / run `35049716939`: Clippy + `cargo check` success.
- 0.14.68 / run `35050184571`: superseded por 0.14.69 durante validação.
- 0.14.69 / run `35050560818`: em validação no momento deste handoff.
- Commits handoff-only não abrem Rust CI.

---

# Refactor/performance consolidado

## Streaming/render

- unload/rebuild reutilizam scratch;
- generation/mesh/remesh pesados ficam em tasks; integração main-thread é budgetada;
- `stream_chunks` e `ChunkRemeshQueue` evitam polling/dispatch vazio;
- empty chunks compartilham buffers por `Arc`/COW;
- pending sort reutiliza scratch e preserva empate via ordinal;
- halo de mesh materializa async a partir de clones COW + revisions;
- hot maps/sets sem semântica de ordem usam Bevy collections;
- chunks imediatos podem preemptar trabalho distante sem perder a vítima;
- task pools são dimensionados explicitamente para favorecer trabalho voxel async;
- streaming mantém direção horizontal recente, prefetch à frente e prioridade de superfície em flight;
- generation fria evita worker convoy em dependências compartilhadas;
- generation não monopoliza o pool quando há mesh backlog;
- mesh async valida content revision por chunk em vez de lighting churn;
- raio nominal tem precedência de fila; capacidade restante aquece margem/previsão;
- shell visual pronta pode existir atrás da fog; zona preditiva distante permanece oculta;
- retired chunks próximos não são arquivados imediatamente;
- visibility e unload possuem histerese separada para evitar churn visual e de recursos.

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

## P0.1 — Chunk streaming: seamless time-to-visible, sem unload churn e sem flicker

Evidência confirmada:

- FPS pode permanecer ~60 enquanto generation/rendering ficam atrás do movimento;
- 0.14.63 tornou a fronteira fisicamente inalcançável em flight máximo, indicando antecipação suficiente para acompanhar o jogador;
- 0.14.64–0.14.67 não removeram o artefato visual da borda;
- 0.14.68 identificou e corrigiu unload/rebuild prematuro ao andar em círculos;
- feedback seguinte: chunks ainda faziam **flicker a distância**;
- 0.14.69 trata o flicker como alternância binária de `Visibility` na borda e adiciona histerese `show/hide`.

Pipeline:

`selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`

Horizontes atuais:

- render distance nominal = prioridade/visibilidade útil;
- base preload all-direction = +2 chunks;
- corredor frontal = até +8 além da base quando existe movimento horizontal;
- fog linear = início ~78% e fim ~98% do raio nominal;
- show/hide visual derivado da render distance (0.14.69);
- unload retention (0.14.68) é maior que a banda de visibilidade, preservando meshes já vistos para revisitas curtas.

Próxima validação runtime:

1. andar em linha reta e observar a fronteira distante;
2. andar/voar em círculos na mesma área e observar se chunks continuam piscando;
3. revisitar rapidamente uma região já carregada e verificar se não volta a existir buraco vazio;
4. testar RD baixo/default/alto para verificar se a histerese continua natural.

Se o flicker persistir depois de 0.14.69, investigar antes de aumentar qualquer raio:

1. se o entity realmente alterna `Visibility` ou se o mesh está sendo substituído por remesh;
2. se a fog/material possui alpha/estado que torna a shell perceptível mesmo além do `fog end`;
3. se algum sistema de culling/despawn fora de `chunk_visibility` e `chunk_unloading` participa;
4. se boundary remesh recria entities e reseta `Visibility::Hidden`, causando flash apesar da histerese;
5. instrumentar `ChunkRenderCoord` com motivo de spawn/hide/show/retire por alguns frames se necessário.

Não marcar P0.1 resolvido até runtime contínuo em flight máximo e movimento circular sem buracos/flicker visíveis.

## P0.2 — Lighting/shadows: lamp latency + seam entre chunks

Sintomas confirmados:

- lighting/shadow demora perceptivelmente para estabilizar;
- `lamp` evidencia a latência;
- durante convergência/carregamento aparece linha/sombra falsa na fronteira entre chunks.

Próximo trabalho:

1. seguir `lamp placement -> lighting queue -> changed_chunks -> remesh -> mesh visível`;
2. separar latência de propagação vs remesh vs integration;
3. impedir lighting parcial de aparecer como seam;
4. readiness explícita de lighting apenas se evidência exigir.

## P0.3 — Hydrology: continuidade de rivers/lakes/tunnels

Sintomas confirmados:

- rivers/lakes podem produzir paredes/cortes retos;
- rivers podem nascer/morrer sem origem/destino visualmente válidos;
- tunnel pode terminar abruptamente ao cruzar água.

Causa concreta conhecida em `src/world/generation/density.rs`:

- `surface_carver_allowed = pass.region.hydrology.water_at(horizontal).is_none()` desliga o surface tunnel carver inteiro por coluna ao entrar em água;
- corrigir por composição/blend com hydrology, sem shaft/água quebrada.

Também investigar endpoints, continuidade entre regions, margens/topo, confluences e waterfall outlets.

## P1.1 — Mountains dominantes + regionais raros

Feedback confirmado:

- Mountains muito presentes;
- Witchwood, Enchanted Forest e Wasteland raros;
- Plains precisa de leve aumento de oak trees.

Valores conhecidos:

- regional weights: Plains/Wasteland 1.0; Witchwood/Enchanted 1.0; Mountains dimension weight 0.85;
- Plains/Wasteland size 120–420; Witchwood/Enchanted 140–460; Mountains metadata 90–260;
- Plains oak spacing 80, chance 0.48, jitter 24;
- mountain belt scale 0.0017, threshold 0.86, width 0.22, warpStrength 85;
- mountain peak spacing 760, chance 0.48, radius 120–230, warpStrength 42.

Montanhas usam distributions especiais; corrigir cobertura belt+peak, não apenas `weight`.

## P1.2 — Coast isolada no interior

- `coast.json` e `ocean.json` são hydrology e não entram no selector regional comum;
- investigar hydrology overlay/identity, `ocean_strength`, coast blend thresholds e continentalness residual.

## P1.3 — Clouds não renderizam

Causa provável forte:

- `src/rendering/sky_layers/clouds.rs` usa altitude absoluta ~34–48;
- Overworld `seaLevel = 90`;
- X/Z seguem câmera e Y permanece absoluto, então clouds ficam abaixo do terreno/jogador.

## P2 — Ghost/held block e outros

- held block observa apenas `PlayerHotbar.selected_slot()` + `item_at`;
- slot vazio deve esconder root/faces;
- placement preview deve limpar id/transform/visibility sem item;
- transparência do ghost deve ser percebida no bloco inteiro;
- dye foi reportado como fraco;
- Player HUD / target HUD fora do ciclo atual salvo repriorização explícita.

---

# Ordem de execução no próximo `go`

1. **P0.1 streaming** — validar 0.14.69; se flicker persistir, instrumentar motivo de visibility/remesh antes de alterar mais raios.
2. **P0.2 lighting/shadows** — lamp latency + seam.
3. **P0.3 hydrology continuity** — primeiro river/tunnel gate binário, depois endpoints/margens.
4. **P1.1 biome distribution** — reduzir Mountains, aumentar regionais e oak de Plains.
5. **P1.2 coast isolada**.
6. **P1.3 clouds**.

---

# Performance direction

Meta não é apenas ~60 FPS; é mundo visualmente pronto antes de o jogador alcançá-lo.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados sem backlog visível;
- prioridade baseada em visibilidade e direção;
- preencher primeiro a banda visível e usar capacidade restante para prewarm;
- preload adaptativo/preditivo em vez de raio gigante indiscriminado;
- evitar tanto churn de unload quanto churn de `Visibility`;
- reservar throughput para finalizar chunks gerados antes de criar backlog novo;
- revision tracking por domínio;
- evitar worker convoy em caches compartilhados;
- evitar scans globais, allocations temporárias e writes idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative;
- **void cru e flicker de chunk nunca são fallback visual aceitável para streaming normal.**
