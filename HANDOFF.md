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
19. Fog de streaming é limitada pelo frontier real de render readiness; deve reagir à readiness mesmo com player parado.
20. Hot paths change-driven: sem scans caros nem dirty writes se os inputs autoritativos não mudaram, preservando reação imediata a mudanças reais.
21. Edits de lighting usam lane interativa deduplicada que preempta backlog e propaga a prioridade até a onda convergir; seu remesh/revisions não devem publicar estados intermediários dessa onda.
22. Freshness async é específica por domínio: content revision vale para geometry/fluid/lighting; lighting-remesh valida adicionalmente a mesh/light revision do halo, sem invalidar geometry por churn de luz.
23. Altitudes de sky layers devem usar o referencial data-driven da dimensão, não constantes absolutas do Overworld nem a altitude variável da câmera.

---

# Estado atual

Último HEAD de código publicado: `c22a1999ca68e18db4ee6318e5b1815e01b91d95`  
`VERSION = 0.15.8`

CI: 0.15.3 final (`5297a69`) run `35054771262` success; 0.15.4 corrigida (`fbe301f`) run #2064 success; 0.15.5 (`fa84d81`) run #2066 success; 0.15.6 (`f8d244c`) run #2068 success; 0.15.7 (`5c99616`) run #2070 success. CI #2075 do HEAD final 0.15.8 iniciado: confirmar conclusão antes de avançar.

## Blocos recentes

- `9302118` / 0.14.70 — terminal fog color alinhada ao sky color.
- `b028fbe` / 0.14.71 — experimento HDR parcial causou regressão grave.
- `57305f4` / 0.14.72 — texturas do sol e lua apontadas no JSON da sky.
- `f864669` / 0.14.73 — reverte HDR parcial depois de mundo preto/HUD corrompida.
- `1b16bee` / 0.15.0 — migração HDR completa: world order 0 Skip; viewmodel HDR order 1 final write/tonemap; UI Camera2d SDR order 2 alpha blend. CI run `35052641180` success; feedback runtime world/viewmodel/UI renderizados normalmente, mas fog ainda tinha chunk popping.
- `4aeb119` / 0.15.1 — fog limitada pela coluna ausente mais próxima de `ChunkRenderPool`, distância até AABB real menos guarda de 1 chunk. Feedback runtime: **chunks deixaram de brotar dentro da fog**.
- `7b16f8f` / 0.15.2 — remove recuperação temporal da fog; readiness passa a determinar start/end diretamente.
- `122d0ff` + `86da5c2` + `5297a69` / 0.15.3 — frontier change-driven: cache de `active_columns` invalidado por `membership_revision`, scan só se readiness/render distance/posição horizontal/camera entity mudam; edge case de recriação da câmera coberto. CI final `35054771262` success.
- `766d970` + `fbe301f` / 0.15.4 — lighting tem lane interativa deduplicada; prioridade de edit/lamp acompanha a propagação e preempta backlog de streaming; revisões/remesh alterados pela onda interativa só são publicados quando convergir. Fix de helper morto após Clippy. CI #2064 success.
- `fa84d81` / 0.15.5 — lighting-remesh async verifica também `chunk_mesh_revisions` autoritativas do halo 3×3×3; resultados capturados antes de nova luz não podem integrar. Geometry/fluid seguem content freshness para evitar starvation. CI #2066 success.
- `f8d244c` / 0.15.6 — remove gate binário que desativava surface tunnels em toda coluna com hydrology; compõe carve com blend vertical em relação ao bed/strength para manter teto/leito perto da água e continuidade em profundidade. CI #2068 success. Não declarar aparência de túnel corrigida sem runtime.
- `5c99616` / 0.15.7 — tuning data-driven de P1.1: mountain_belt threshold `0.86 -> 0.90`, width `0.22 -> 0.15`; mountain_peak spacing `760 -> 850`, chance `0.48 -> 0.36`, radius `120–230 -> 110–205`; Wasteland weight `1.0 -> 1.30`, Witchwood/Enchanted `1.0 -> 1.25`; Plains oak chance `0.48 -> 0.54` com spacing `80` preservado. Mantém mountain weight `0.85`, tamanho dos biomas e `avoidNear`. CI #2070 success; validar distribuição no runtime em novas regiões/seeds.
- `ebe0b7c` + `c22a199` / 0.15.8 — `src/rendering/sky_layers/clouds.rs` usa `CurrentDimensionContext`: Y = `dimension.sea_level + altitude_above_sea_level` (offset 34–48), X/Z continuam acompanhando câmera; quando definição da dimensão não existe, não posicionar nuvens incorretamente. `c22a199` apenas restaura formatação do match. CI #2075 em execução; runtime ainda pendente.

## Arquitetura de câmera gameplay — 0.15.x

`world HDR linear (order 0, Skip) -> viewmodel HDR (order 1, final 3D write/tonemap) -> UI SDR (order 2, alpha blend)`

O experimento 0.14.71 pôs `Hdr` apenas na world camera; viewmodel SDR ainda tinha `IsDefaultUiCamera`. Bevy 0.19 possui sharp edges ao misturar câmeras HDR/SDR no mesmo target sem composição explícita. Projeto já possuía world e viewmodel cameras; HUD/textos estavam associados à viewmodel por `IsDefaultUiCamera`. Regras atuais: world não faz tonemap/write final; viewmodel preserva world HDR e executa único tonemap/write 3D; UI entra SDR depois; UI limpa intermediate transparente, faz alpha blend; world/viewmodel continuam `Msaa::Off`; não mudar exposure/tonemapper artístico sem evidência runtime. 0.15.0 validou a recuperação visual da composição, não o streaming.

## Diagnóstico atual de fog / streaming

Histórico de feedback:

- 0.14.63: preload melhorou; buracos visíveis, porém não alcançáveis.
- 0.14.64–0.14.67: warm/visible e shell atrás da fog não eliminaram o artefato.
- 0.14.68: aposentadoria/retenção evita unload prematuro em círculos.
- 0.14.69: histerese de visibility, ainda houve flicker.
- 0.14.70: fog color alinhada com sky; silhuetas continuavam.
- 0.14.71 HDR parcial causou mundo preto/UI sobreposta; 0.14.73 revertido.
- 0.15.0: stack HDR correta mas chunk popping permanecia.
- Causa: visibility além do nominal não garantia generation+mesh antes de chunk entrar na faixa perceptível; render allocation atrasada podia estrear dentro da fog.
- 0.15.1: readiness real limita fog, feedback do usuário confirmou que parou o popping dentro dela.
- Novo feedback: fog podia permanecer fechada enquanto player parado.
- 0.15.2: sem `current_end` temporal, fog acompanha readiness diretamente.
- 0.15.3: membership revision invalida cache, posição horizontal/render distance/camera entity invalidam scan. Sem rebuild de `HashSet`/scan do disco/dirty write em frames estáveis.

`src/rendering/fog/distance.rs` mantém cache de colunas de `ChunkRenderPool::active_coords()`; procura coluna faltante mais próxima no raio nominal; mede distância horizontal até AABB física, recua `fog end` com 1 chunk de guarda, preserva largura aproximada da faixa linear e limita target a ~98% do radius. Sem recovery por `Time`. Readiness ou câmera recriada força atualização mesmo parado. Fallback visual de backlog, não substituto de throughput.

Próxima validação runtime de P0.1: player completamente parado enquanto chunks terminam; flight máximo; círculos; sem void/flicker/silhuetas, sem fog respirando demais. Se houver breathing, melhorar frontier/throughput em vez de fixar raio de fog maior. **Não marcar P0.1 resolvido antes dessa evidência.**

## Streaming atual

Pipeline:

`selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`

Horizontes: render distance nominal = priority/visibility útil; preload all-direction = +2 chunks; corredor frontal até +8 além da base sob movimento horizontal; fog nominal start ~78%/end ~98%, podendo recuar no frontier; visibility show/hide exemplos RD4=5/6, RD12=14/16, RD24=26/30; retention `R + max(ceil(R/2), 10)`, exemplos RD4=14, RD12=22, RD24=36. Retention e visibility horizontal-only.

Gargalos tratados desde 0.14.56: prioridade perdida depois da seleção; task pool pequeno; worker convoy/cache frio; flight selecionando ar em vez de superfície; lookahead insuficiente; geração antes de mesh; generation monopolizando pool; lighting invalidando mesh inicial; preload vencendo missing visible; warm exposto; mesh pronta esperando movimento para promoção; unload precoce em revisitas; visibility ping-pong; fog/background color mismatch; stack HDR implícita; fog fixa não refletindo frontier; recovery temporal; scan/write fog todo frame. Em 0.15.4/0.15.5 acrescentam-se lane interativa de lighting e freshness de lighting-remesh.

---

# Celestial bodies

`src/rendering/celestial.rs` suporta `CelestialBodyDefinition.texture: Option<String>`; asset path vira `base_color_texture`; material unlit/alpha blend/double-sided/fog disabled. 0.14.72 aponta o JSON de Overworld para `textures/sky/sun.png` e `textures/sky/moon.png`. Validar runtime alpha/orientação/texturas.

---

# Próximas prioridades

## P0.1 — Streaming/fog seamless

Prioridade de validação runtime de 0.15.3+ parada/flight máximo/círculos; não considerar encerrado apenas pelo CI. Se fog respirar sob backlog, otimizar throughput/priorização do frontier.

## P0.2 — Lighting/shadows

Sintomas: latência até estabilizar, lamp demorada, seams/sombras falsas entre chunks. 0.15.4 preempção da propagação interativa e publicação apenas após convergência; 0.15.5 rejeita async lighting-remesh stale via revision do halo. **Ainda precisa confirmar no runtime** lamp placement/removal, edit rápido repetido, chunk boundary, loading sob backlog. Se persistirem seams, distinguir propagation vs remesh scheduling vs mesh integration; readiness explícita só com evidência.

## P0.3 — Hydrology continuity

Sintomas: rivers/lakes com paredes/cortes retos, endpoints inválidos, tunnel acabando ao cruzar água. 0.15.6 substitui `water_at(horizontal).is_none()` em `generation/density.rs` por blend de carve sob água; fluid pass preenche ar apenas acima do bed level, mas validar resultado runtime. Continuar diagnóstico de continuidade entre regiões/edges, margens, confluences e waterfall outlets. `keep_only_complete_downstream_paths` já rejeita paths incompletos no trace e flow accumulation tem margem e boundary test; não mexer nessas partes sem prova do defeito.

## P1.1 — Biome distribution

Feedback: Mountains demais; Witchwood/Enchanted Forest/Wasteland raros; Plains pouco oak. 0.15.7 faz primeiro tuning data-driven. `sample_surface` combina regional Voronoi com macro overlay de mountain_belt/peak: Mountains não participa da competição de sites regionais. Outros quatro regionais não têm climate profile específico nos JSONs atuais; Wasteland tem `avoidNear` contra Witchwood/Enchanted, preservado. Validar várias seeds/áreas e distribuição efetiva antes de novos ajustes, mantendo biomas grandes conforme pedido anterior.

## P1.2 — Coast isolada

Investigação estática: `track_current_biome` usa continentalness direta de `BiomeField::climate_at(horizontal)` para identidade Coast/Ocean; água, density carve e material oceânicos usam continentalness de `HydrologyRegion::macro_sample_at` no grid interpolado 5×5. Há também um gate de material: `material_column_profile` omite ocean bed se floor projetado estiver acima de `sea_level + COAST_MAXIMUM_SURFACE_HEIGHT` (`+3` no momento), enquanto identidade Coast não considera esse floor. Esses critérios distintos podem deixar Coast visual em terreno seco/alto ou desalinhado da água. Investigar com caso reproduzível comparando continentalness raw/interpolada, floor, strength e coast blend; não mexer em thresholds ou substituir material sem confirmar. `hydrology_biome_weights`: surface/coast até strength 0.25; coast 0.25–0.55; coast/ocean 0.55–0.80; ocean após 0.80.

## P1.3 — Clouds

0.15.8 substitui altitude absoluta Y34–48 por `dimension.sea_level + (34–48)` usando `CurrentDimensionContext`. Com Overworld seaLevel=90, centros ficam Y124–138, sem seguir altitude Y da câmera; X/Z continuam acompanhando player. Confirmar CI #2075 e validar nuvens acima da superfície em runtime, inclusive dimensão com seaLevel diferente. Não declarar aparência correta só pela aritmética.

## P2 histórico

- held block deve observar hotbar e esconder em slot vazio;
- ghost transparency deve ser percebida no bloco inteiro;
- dye ainda foi reportado como fraco;
- Player HUD / target HUD fora do ciclo atual salvo repriorização.

---

# Performance direction

Meta não é só ~60 FPS: mundo visualmente pronto antes de ser alcançado. Heavy generation/mesh/remesh async; integração/restore/unload/lighting/fluid budgetados; prioridade por visibilidade e direção; banda visível antes de expandir backlog; preload preditivo em vez de raio indiscriminado; evitar churn de unload/visibility/remesh; throughput para chunks já gerados; revisions por domínio; evitar worker convoy, scans globais e dirty writes estáveis; hydrology natural continua generation-authoritative. Void cru, flicker e silhueta nascendo atrás da fog não são fallback aceitável.
