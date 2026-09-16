# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch de trabalho: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica e persistente do projeto. Exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`; feature branch só sob pedido explícito.
- Antes de escrever: buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Commits pequenos e coerentes; não misturar mudanças sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível; minor para feature compatível; major para breaking.
- Commit só de `HANDOFF.md` não sobe versão.
- Depois de mudança material em código, arquitetura, roadmap ou processo, atualizar este handoff.
- Runtime error/warning/feedback do usuário tem prioridade sobre roadmap.
- Corrigir warnings de Rust nos blocos tocados.
- CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` + `cargo check`.
- Não repetir que falta cargo check/run local; CI é o gate.
- `cargo test` manual só sob pedido explícito; fmt não é gate.
- Não declarar bug visual/gameplay resolvido sem evidência runtime.
- `go` / `continua` = executar próximo bloco sem confirmação desnecessária.
- Não gerar imagens sem pedido explícito.
- Root `VERSION` é a fonte operacional; `Cargo.toml` permanece intencionalmente em 0.10.16.

## Invariants arquiteturais relevantes

1. Cada fato de gameplay tem owner autoritativo.
2. Heavy generation/mesh/remesh fica fora da main thread; integração é budgetada.
3. Resultados async são revisionados; stale results são descartados/rescheduled.
4. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são primitives canônicos de fila deduplicada.
5. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
6. Prioridade de streaming deve sobreviver às fronteiras async; trabalho imediato pode preemptar apenas trabalho não imediato e a vítima volta à fila.
7. Mesh async valida revision de conteúdo por chunk, separada de lighting revision.
8. Neighbor ausente no snapshot inicial não invalida primeira aparição; boundary remesh corrige depois.
9. Streaming distingue: raio visível obrigatório, faixa warm/preload e raio de retenção/unload.
10. Trabalho faltando no raio visível sempre vence preload.
11. Retirement != unload imediato: chunks aposentados próximos permanecem reutilizáveis.
12. Visibilidade usa histerese: `show` e `hide` são limiares distintos.
13. A zona preditiva distante não deve ficar visualmente exposta.
14. Gameplay rendering usa uma stack explícita de câmeras; mundo, viewmodel e UI não compartilham implicitamente o mesmo estágio de composição.
15. O mundo 3D fica em HDR linear até a composição do viewmodel; tonemapping deve ocorrer uma única vez antes da UI.
16. UI gameplay é SDR e entra depois do 3D via alpha blend; HUD não pertence à câmera 3D da mão.
17. Invariant de produto: em velocidade normal configurada, inclusive flight máximo, o jogador não deve enxergar void/chunks ausentes ou silhuetas de chunks brotando no fundo.
18. Mudanças de camera/render pipeline têm blast radius global. Qualquer alteração de HDR/tonemapping exige validar mundo + viewmodel + HUD + overlays, não apenas fog.

---

# Estado atual

Último HEAD de código publicado: `1b16bee81ff609b7c4bc7e4667dd4a2e37831fff`  
`VERSION = 0.15.0`

Blocos mais recentes:

- `9302118` / 0.14.70 — terminal fog color alinhada ao sky color.
- `b028fbe` / 0.14.71 — experimento incompleto com `Hdr` apenas na gameplay/world camera; causou regressão grave.
- `57305f4` / 0.14.72 — liga `textures/sky/sun.png` e `textures/sky/moon.png` no `data/dimensions/overworld/sky.json`.
- `f864669` / 0.14.73 — reverte o experimento HDR incompleto após mundo preto + sobreposição/corrupção visual de HUD.
- `1b16bee` / 0.15.0 — migração HDR completa da stack gameplay:
  - câmera world: HDR, `Tonemapping::None`, `CameraOutputMode::Skip`, order 0;
  - câmera viewmodel: HDR, order 1, `clear_color=None`, faz o único tonemap/write 3D;
  - câmera UI dedicada: Camera2d SDR, order 2, clear transparente, `CameraOutputMode::Write` com alpha blending;
  - `IsDefaultUiCamera` saiu da câmera viewmodel e passou para a câmera UI dedicada;
  - ordens canônicas em `src/rendering/camera_stack.rs`;
  - CI run `35052641180`: Clippy + `cargo check` success.

## Arquitetura de câmera gameplay — 0.15.0

Stack:

`world HDR linear (order 0, Skip) -> viewmodel HDR (order 1, final 3D write/tonemap) -> UI SDR (order 2, alpha blend)`

Motivação:

- o experimento 0.14.71 ativou HDR somente no world camera enquanto a viewmodel camera permanecia SDR e ainda era `IsDefaultUiCamera`;
- Bevy 0.19 possui sharp edges reais ao misturar câmeras HDR/SDR no mesmo target sem composição explícita;
- o projeto já possuía duas câmeras 3D: world e viewmodel;
- HUD/textos eram renderizados pela câmera 3D da viewmodel porque ela carregava `IsDefaultUiCamera`;
- isso explica por que uma mudança aparentemente local na world camera afetou mundo e UI ao mesmo tempo.

Regras da stack nova:

1. world camera não faz tonemap nem escreve diretamente no target final;
2. viewmodel camera compartilha o domínio HDR, preserva o world color buffer e faz o único write/tonemap do 3D;
3. UI gameplay nunca é tonemapeada junto com o mundo;
4. UI camera limpa seu próprio intermediate para transparente e alpha-blenda no target final já composto;
5. world/viewmodel continuam `Msaa::Off`, evitando incompatibilidade de intermediate targets;
6. não alterar exposure/tonemapper artístico sem evidência runtime; 0.15.0 preserva o tonemapper default no estágio final da viewmodel.

Validação runtime obrigatória de 0.15.0:

1. mundo não pode ficar preto;
2. viewmodel/mão aparece uma única vez e com composição correta;
3. HUD/textos aparecem uma única vez, sem duplicação/sobreposição;
4. pause, inventory e settings in-world devem compor corretamente sobre gameplay;
5. underwater tint deve continuar funcionando como UI overlay;
6. sol/lua texturizados devem continuar visíveis;
7. só depois verificar se a silhueta de chunks brotando no fundo da fog foi eliminada/reduzida pelo caminho HDR coerente.

Não declarar a migração visualmente concluída até esse runtime.

## Diagnóstico atual de fog / streaming

Feedback runtime acumulado:

- 0.14.63: preload melhorou bastante; buracos continuavam visíveis, mas jogador não conseguia alcançá-los.
- 0.14.64–0.14.67: warm/visible split e shell atrás da fog não eliminaram o artefato.
- 0.14.68: corrigiu unload prematuro observado ao andar em círculos; chunks aposentados próximos deixaram de ser descartados imediatamente.
- 0.14.69: adicionou histerese de `Visibility`; ainda houve flicker.
- 0.14.70: `DistanceFog.color` foi alinhado a `sky_color`; runtime ainda mostrou chunks brotando como silhueta no fundo.
- Observação decisiva do usuário: **o que flicka aparece atrás da fog; parece silhueta de chunks surgindo no fundo**, e não mais simples toggle de rendering na borda.
- 0.14.71 tentou HDR de forma incompleta e quebrou o pipeline visual/UI.
- 0.15.0 substitui o workaround por uma composição HDR explícita de múltiplas câmeras.

Investigação concreta em Bevy 0.19.1:

- `DistanceFog` linear chega a opacidade total em `end` quando alpha=1.
- O terrain shader usa `main_pass_post_lighting_processing`, portanto a fog é aplicada pelo pipeline PBR.
- Em câmera não-HDR, o shader pode aplicar tonemapping/color processing no caminho do material enquanto `ClearColor` não atravessa necessariamente o mesmo caminho.
- Isso pode tornar fragmento 100% fogged diferente do fundo mesmo com a mesma cor numérica.
- Em 0.15.0 world + viewmodel permanecem no mesmo HDR intermediate e o tonemap é postergado até a composição 3D final; esse é agora o teste correto da hipótese de compositing.

Próxima validação runtime de P0.1:

1. validar primeiro a stack 0.15.0 inteira sem regressão visual/UI;
2. verificar a silhueta de chunks no fundo da fog em linha reta e movimento circular;
3. se persistir, não aumentar raios cegamente;
4. investigar fragment depth/far-background e, se necessário, uma cobertura de horizonte/far background no mesmo domínio HDR do terrain/fog;
5. confirmar se remesh/spawn substitui geometria atrás da fog e produz silhueta antes/depois de `fog end`.

Não marcar P0.1 resolvido sem runtime contínuo em flight máximo e movimento circular sem void/flicker/silhueta visível.

## Streaming atual

Pipeline:

`selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`

Horizontes atuais:

- render distance nominal = banda de prioridade/visibilidade útil;
- base preload all-direction = +2 chunks;
- corredor frontal = até +8 além da base quando há movimento horizontal;
- fog linear = start ~78%, end ~98% do raio nominal;
- visibilidade 0.14.69: show/hide derivados da render distance; exemplos RD 4 => 5/6, RD 12 => 14/16, RD 24 => 26/30;
- retention 0.14.68 = `R + max(ceil(R/2), 10)`; exemplos RD 4 => 14, RD 12 => 22, RD 24 => 36;
- retention e visibility são horizontal-only para não destruir superfície por mudança de altitude.

Gargalos tratados desde 0.14.56:

1. prioridade perdida após seleção;
2. task pool subdimensionado;
3. worker convoy em caches frios;
4. flight priorizando ar local em vez de superfície;
5. lookahead direcional insuficiente;
6. generation antes de mesh pronta;
7. generation monopolizando pool com mesh backlog;
8. lighting invalidando initial mesh;
9. preload curto/direção vencendo missing visible;
10. warm parcialmente exposto;
11. novo mesh dentro do raio esperando movimento para promoção;
12. unload precoce em revisita/círculos;
13. visibility ping-pong na borda;
14. fog/background passando por color-processing diferente;
15. composição world/viewmodel/UI implícita e incompatível com HDR.

---

# Celestial bodies

`src/rendering/celestial.rs` suporta textura data-driven:

- `CelestialBodyDefinition.texture: Option<String>`;
- `base_color_texture = definition.texture.as_ref().map(|texture| asset_server.load(texture.clone()))`;
- material é unlit, alpha blend, double-sided, fog disabled.

O problema do sol não era renderer: em `data/dimensions/overworld/sky.json`, `sun.texture` e `moon.texture` estavam `null` apesar de os arquivos existirem.

0.14.72 aponta para:

- `textures/sky/sun.png`
- `textures/sky/moon.png`

Validar runtime se textura aparece e alpha/orientação permanecem corretos.

---

# Próximas prioridades

## P0.1 — Streaming/fog seamless + validação HDR

Prioridade máxima: validar 0.15.0 e eliminar silhueta/void/flicker sem regredir world/viewmodel/UI.

## P0.2 — Lighting/shadows

Sintomas confirmados:

- lighting/shadow demora para estabilizar;
- `lamp` evidencia latência;
- aparecem seams/sombras falsas entre chunks durante convergência/carregamento.

Próximo trabalho:

1. seguir `lamp placement -> lighting queue -> changed_chunks -> remesh -> mesh visível`;
2. separar latência de propagation vs remesh vs integration;
3. impedir partial lighting de aparecer como seam;
4. readiness explícita apenas se evidência exigir.

## P0.3 — Hydrology continuity

Sintomas confirmados:

- rivers/lakes podem criar paredes/cortes retos;
- rivers podem nascer/morrer sem origem/destino válido;
- tunnel pode terminar abruptamente ao cruzar água.

Causa concreta conhecida em `src/world/generation/density.rs`:

`surface_carver_allowed = pass.region.hydrology.water_at(horizontal).is_none();`

Esse gate binário desliga o surface tunnel carver inteiro na coluna com água. Corrigir por composição/blend, sem shaft nem água quebrada. Depois revisar endpoints, continuidade entre regions, margens, confluences e waterfall outlets.

## P1.1 — Biome distribution

Feedback confirmado:

- Mountains muito presentes;
- Witchwood, Enchanted Forest e Wasteland raros;
- Plains precisa de leve aumento de oak.

Montanhas usam distributions especiais; ajustar cobertura belt/peak, não só weight.

## P1.2 — Coast isolada

Investigar hydrology overlay/identity, ocean strength, coast blend thresholds e continentalness residual.

## P1.3 — Clouds

Causa provável forte já identificada: cloud altitude absoluta ~34–48 enquanto Overworld seaLevel=90; X/Z seguem câmera, Y fica absoluto. Provavelmente estão abaixo do terreno.

## P2 histórico

- held block deve observar hotbar e esconder em slot vazio;
- ghost transparency deve ser percebida no bloco inteiro;
- dye ainda foi reportado como fraco;
- Player HUD / target HUD fora do ciclo atual salvo repriorização.

---

# Performance direction

Meta não é apenas ~60 FPS; é mundo visualmente pronto antes de o jogador alcançá-lo.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados sem backlog visível;
- prioridade por visibilidade e direção;
- preencher banda visível antes de expandir backlog;
- preload adaptativo/preditivo em vez de raio gigante indiscriminado;
- evitar churn de unload, visibility e remesh;
- reservar throughput para finalizar chunks já gerados;
- revision tracking por domínio;
- evitar worker convoy e scans globais desnecessários;
- natural hydrology permanece generation-authoritative;
- void cru, flicker e silhueta de chunk atrás da fog não são fallback visual aceitável.
