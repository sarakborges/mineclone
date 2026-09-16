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
- Depois de mudança material em código, arquitetura, roadmap ou processo, atualizar este handoff na mesma sessão de trabalho. Não acumular versões sem atualização.
- Runtime error/warning/feedback do usuário tem prioridade sobre roadmap. `go` / `continua` = executar próximo bloco sem confirmação desnecessária.
- Corrigir warnings Rust dos blocos tocados. CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` + `cargo check`.
- Não repetir que falta cargo check/run local; CI é gate de compilação. `cargo test` manual só sob pedido explícito; fmt não é gate.
- Não declarar bug visual/gameplay ou recuperação de FPS resolvido sem evidência runtime. Registrar claramente hipóteses vs causas confirmadas.
- Não gerar imagens sem pedido explícito.
- Root `VERSION` é a fonte operacional; `Cargo.toml` permanece intencionalmente em 0.10.16.

## Invariants arquiteturais relevantes

1. Cada fato de gameplay tem owner autoritativo.
2. Heavy generation/mesh/remesh fora da main thread; integração budgetada.
3. Resultados async revisionados; stale descartado e rescheduled.
4. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são primitives canônicos de fila deduplicada.
5. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
6. Prioridade streaming atravessa fronteiras async; trabalho imediato só preempta não imediato e vítima volta à fila.
7. Mesh async valida revision de conteúdo separada de lighting; um halo novo que estava ausente inicialmente não invalida first-visible work, boundary remesh corrige depois.
8. Streaming separa visible mandatory, warm/preload e retention/unload. Trabalho faltante visível vence preload.
9. Retirement != unload imediato; chunks aposentados próximos seguem reutilizáveis.
10. Visibility tem histerese show/hide, não mostrar zona preditiva distante.
11. Gameplay rendering é stack explícita de câmeras HDR world/viewmodel + SDR UI; tonemap uma vez antes da UI, que não pertence à câmera 3D da mão.
12. Invariant de produto: flight máximo, caminhada/círculos/parado sem void/chunk popping/silhuetas à distância.
13. HDR/tonemap exige validação mundo+viewmodel+HUD+overlays; mudança isolada pode quebrar a stack.
14. Fog de streaming limitada pelo frontier real de readiness, reage à readiness parado.
15. Hot paths change-driven, sem scans/dirty writes quando inputs autoritativos não mudam.
16. Lighting de edits usa lane interativa deduplicada, preempta backlog e publica revisões/remesh depois da convergência; feedback atual informa que essa política ainda gera latência perceptível e precisa ser refinada com controle de custo.
17. Async lighting-remesh valida halo de content e revisão própria de lighting; geometry/fluid não devem ser invalidados apenas por churn de luz.
18. Sky layers usam seaLevel data-driven da dimensão; nuvens têm coordenadas reais de mundo e câmera só escolhe qual tile distante reciclar, nunca soma posição do player continuamente.
19. Fog terminal e clear sky devem concordar para não expor chunk frontier; restaurar paleta artística de `fogColor` sem quebrar essa continuidade exige estratégia explícita, não simplesmente trocar uma das duas cores.

---

# Estado atual — 2026-09-16

Último HEAD **de código** publicado: `68a26a333365a161b0e75b7efd6f702cae31bb2c`  
`VERSION = 0.15.12`  
Branch `develop`. Este commit do HANDOFF é somente documentação e não sobe `VERSION`.

CI: 0.15.3 final (`5297a69`) run `35054771262` success; 0.15.4 fix (`fbe301f`) #2064 success; 0.15.5 (`fa84d81`) #2066 success; 0.15.6 (`f8d244c`) #2068 success; 0.15.7 (`5c99616`) #2070 success; 0.15.8 final (`c22a199`) #2075 success; 0.15.9 (`b68ecb3`) #2078 success; 0.15.10 (`eadb6ea`) #2080 success; 0.15.11 (`780df02`) #2082 success; **0.15.12 (`68a26a3`) #2084 está em execução na última consulta**. Conferir resultado, corrigir falha/warnings antes do próximo bloco.

## Blocos recentes

- `9302118` / 0.14.70 — terminal fog color alinhada ao sky color; feedback então ainda tinha silhuetas.
- `b028fbe` / 0.14.71 — experimento HDR parcial causou mundo preto/HUD corrompida; `57305f4` / 0.14.72 apontou texturas do sol/lua; `f864669` / 0.14.73 reverteu HDR parcial.
- `1b16bee` / 0.15.0 — HDR stack explícita: world order 0 Skip; viewmodel HDR order 1 final write/tonemap; UI SDR order 2 alpha blend. CI `35052641180` success; feedback runtime composição normalizada, mas fog ainda tinha chunk popping.
- `4aeb119` / 0.15.1 — fog limitada pela coluna ausente mais próxima de `ChunkRenderPool`, distância até AABB real menos guarda de 1 chunk. Usuário confirmou que chunks pararam de brotar dentro da fog.
- `7b16f8f` / 0.15.2 — remove recuperação temporal da fog; readiness determina start/end diretamente.
- `122d0ff` + `86da5c2` + `5297a69` / 0.15.3 — frontier change-driven, cache `active_columns` invalidado por `membership_revision`; scan só quando readiness/render distance/posição horizontal/camera entity mudam, cobre recriação de câmera. CI `35054771262` success.
- `766d970` + `fbe301f` / 0.15.4 — lighting: lane interativa deduplicada, preempção de streaming, prioridade acompanha propagação; mudanças da onda publicadas após convergir. CI #2064 success.
- `fa84d81` / 0.15.5 — lighting-remesh async verifica revisão do halo 3×3×3 além do content. Implementação mantém tracker próprio em `ChunkRemeshTasks`, separado do `VoxelWorld::chunk_mesh_revisions`; **não o chamar de tracker único/autoritativo**. Seed inicial de luz não faz bump nesse tracker automaticamente: ponto a investigar com chunks escuros. CI #2066 success.
- `f8d244c` / 0.15.6 — remove gate binário que desativava surface tunnels em coluna molhada, usa blend vertical abaixo de bed; pode adicionar `water_near` em todo column hot path, investigar FPS. CI #2068 success; aparência ainda não comprovada.
- `5c99616` / 0.15.7 — P1.1 tuning data-driven: mountain_belt threshold `0.86 -> 0.90`, width `0.22 -> 0.15`; mountain_peak spacing `760 -> 850`, chance `0.48 -> 0.36`, radius `120–230 -> 110–205`; Wasteland weight `1.0 -> 1.30`, Witchwood/Enchanted `1.0 -> 1.25`; Plains oak chance `0.48 -> 0.54`, spacing80 mantido. Evitar novos pesos sem amostragem/runtime. CI #2070 success.
- `ebe0b7c` + `c22a199` / 0.15.8 — nuvens Y=`seaLevel + 34..48` em vez de Y absoluto abaixo de Overworld, mas X/Z ainda seguiam a câmera; feedback runtime: posição grudada na câmera, esquisito. CI #2075 success.
- `b68ecb3` / 0.15.9 — nuvens ancoradas em world coordinates e vento global, câmera apenas escolhe tile do pool por múltiplos de 180, testes de helper incluídos. CI #2078 success; ainda requer feedback visual sobre teleport nos boundaries/tile recycling.
- `eadb6ea` / 0.15.10 — performance: uma mesh transparente por nuvem em vez de três cubos transparentes sobrepostos; elimina 2/3 de entidades/draws de cloud pool, mantém densidade data-driven, world coordinates. CI #2080 success; **não concluir FPS recuperado até medições runtime**.
- `780df02` / 0.15.11 — rios só consideram destino oceânico quando o bed efetivo calculado fica pelo menos 4 blocos abaixo do seaLevel, não somente pela continentalness; flow tracing/selection avança pela faixa oceânica ainda seca e ocean outlet prioriza destino molhado; ocean water_at seco não suplanta a água de uma foz. Testes de predicado de ocean wet adicionados. CI #2082 success. Verificar conectividade entre regiões, largura da foz e seeds em runtime.
- `68a26a3` / 0.15.12 — margens: mesmo graph scan com margin alargada, separa força do núcleo real e faixa externa; grade signed até altura da água+offset sobre coluna de terreno exata, rebaixa terra alta e eleva somente banco externo baixo, sem preencher interior da água; transição com smoothstep e testes de grading. Ocean density interpola a partir de altura exata sem salto na entrada da faixa oceânica. CI #2084 em execução; validar rios/lagos visualmente e perfil de FPS.

## Arquitetura de câmera e fog

`world HDR linear (order 0, Skip) -> viewmodel HDR (order 1, final 3D write/tonemap) -> UI SDR (order 2, alpha blend)`

0.14.71 aplicou HDR só à world camera, mas viewmodel SDR mantinha `IsDefaultUiCamera`, causando regressão em composição. Stack 0.15.0 separa UI; world e viewmodel usam `Msaa::Off`; não alterar exposure/tonemap artístico sem evidência. Fog em `src/rendering/fog/distance.rs`: cache de colunas do `ChunkRenderPool`; distância à AABB física da coluna faltante; recua fog end 1 chunk, start acompanha preservando largura nominal, target ~98% do render radius, readiness ou câmera recriada força update parado. Fallback visual, não substituto de throughput.

**Nova regressão observada 2026-09-16:** usuário relata que os biomas perderam suas cores de fog/sky. `EnvironmentVisualState` ainda calcula `sky_color` e `fog_color` por bioma/fase, mas `rendering/fog/color.rs` e `fog/attachment.rs` usam só `visuals.sky_color`, ignorando `fog_color` desde alinhamento antigo; `SkyPlugin` atualiza `ClearColor` com sky_color. Verificar também se a identidade CurrentBiome e registro de visuals refletem bioma correto, e composição HDR em runtime; não afirmar que o sky perdeu cor por esse único fato. **Não trocar fog_color isoladamente sem resolver terminal sky mismatch**, pois isso reintroduz silhuetas do streaming. Uma solução consistente de horizonte/gradiente deve restaurar identidade visual preservando o frontier oculto.

P0.1 runtime pendente: player parado enquanto chunks terminam, flight máximo, círculos, sem void/flicker/silhuetas/fog breathing. Feedback 0.15.1 parou popping dentro da fog, mas versões posteriores sem validação completa.

## Streaming atual e gargalos

`selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`

Horizontes: render distance nominal=visible/priority; preload all-direction +2 chunks, corredor frontal até +8 em movimento horizontal; fog nominal start~78%/end~98%, podendo recuar no frontier; visibility RD4=5/6, RD12=14/16, RD24=26/30; retention `R + max(ceil(R/2),10)` com RD4=14 RD12=22 RD24=36; retention/visibility horizontal-only.

Gargalos tratados desde 0.14.56: prioridade perdida entre seleção e task; task pool/worker convoy/cache frio; flight selecionando ar vs superfície; lookahead; geração monopolizando mesh; initial light invalidando mesh; preload vencendo visible; warm exposto; mesh pronta esperando movimento; unload precoce; visibility ping-pong; background fog mismatch; HDR partial; fixed fog; temporal recovery; scan/dirty write a cada frame; interatividade de luz, async lighting stale.

**Performance nova regressão observada:** FPS caiu drasticamente após blocos 0.15.6–0.15.9 (não atribuir causalidade sem medidas); 0.15.10 reduz cloud alpha overdraw agora acima do terreno, porém FPS ainda não foi reavaliado. Inspecionar hot path `generation/density.rs`: `water_near` executa em toda coluna mesmo quando não há surface-carver candidato; `SurfaceCarverColumn` pode ser vazio, propagar fast path antes de scan hídrico. `hydrology/region/density.rs` cria `Vec` de water bodies por coluna; avaliar SmallVec sem modificar sem necessidade. Comparar FPS parado vs andando e log `render assets`/profiling se disponível; priorizar correção compatível sem elevar budget de iluminação arbitrariamente. Novos cálculos de 0.15.11/0.15.12 também exigem atenção ao custo de worldgen.

---

# Celestial bodies

`src/rendering/celestial.rs` suporta `CelestialBodyDefinition.texture: Option<String>` como `base_color_texture`; material unlit/alpha blend/double-sided/fog disabled. Overworld sky aponta `textures/sky/sun.png`/`moon.png` desde 0.14.72. Validar alpha/orientação/texturas no jogo.

---

# Próximas prioridades (feedback runtime governa ordem)

## P0.1 — Regressão grave de FPS

**Usuário informou queda drástica de FPS em 2026-09-16.** Checar CI da 0.15.12 e feedback/medidas da 0.15.10, distinguir GPU cloud alpha overdraw de CPU worldgen/lighting/remesh. Não considerar resolvido pela redução de draw calls. Investigar/otimizar `water_near` desnecessário em `generation/density.rs` após 0.15.6 e allocations de hydrology, sem voltar ao gate binário que interrompe túneis.

## P0.2 — Chunks totalmente escuros + sombras atrasadas

Feedback 2026-09-16: sombras melhoraram, mas atualização **não é imediata**; chunks **inteiramente escuros ainda existem**. 0.15.4 prioriza propagação edit mas segura publicação de remesh até convergir, `lighting_updates.rs` usa budget 2ms/até4096 voxels; `chunk_remesh.rs` despacha até2 tasks/frame, 4 in-flight, integra até2/frame; primeira mesh pode capturar luz apenas seeded e preceder convergência dos vizinhos. 0.15.5 tem tracker `lighting_revisions` separado que só recebe bump em `process_dynamic_lighting` quando `changed_chunks` publicado; `seed_chunk_direct_lighting` atualiza `VoxelWorld` revision mas não esse tracker: hipótese de freshness incoerente que precisa de correção/teste. Investigar priorização no remesh/integração, dark chunk que permanece vs atraso transitório; evitar simplesmente aumentar budgets e reduzir FPS. **Não marcar P0.2 corrigido pelo CI.**

## P0.3 — River/lake morphology e conectividade

Feedback 2026-09-16: margens ainda formavam corte seco vertical, rios ainda existiam sem ligação a outros corpos d'água. Causa verificada: antiga `shore_density_delta` fazia `(target_surface - surface_elevation).max(0)`, não rebaixava cliffs, faixa de river bank só raio0.75..1.0 (~1–3 blocos), carve parava em `water_level+1.5` deixando teto alto intacto. 0.15.12 introduz grade assinado sobre interior alto e faixa lateral maior, com altura real da coluna. `keep_only_complete_downstream_paths` antiga aceitava primeiro nó sob limiar de continentalness, que podia estar acima do seaLevel e sem água; 0.15.11 usa wet outlet. Validar margem, confluências, outlets, conexão entre regiões e water fill. Se houver trecho solto mesmo com outlet wet, investigar seleção flow-cache local de regiões vs path/edge graphs. Não declarar hidrologia concluída sem gameplay.

## P1.1 — Biome distribution

Feedback: Mountains demais; Witchwood/Enchanted/Wasteland raros; Plains pouco oak. Tuning 0.15.7 altera cobertura mountain belt/peak e pesos sem mexer no tamanho/avoidNear; regionais compartilham sites Voronoi e Mountains é macro overlay separado. Validar múltiplas seeds/áreas e densidade oak no runtime, não repetir tuning às cegas.

## P1.2 — Coast isolada

`track_current_biome` obtém hydrology overlay com continentalness raw de `BiomeField::climate_at`; oceano físico usa `HydrologyRegion::macro_sample_at` interpolado 5×5. Material oceânico rejeita floor acima de `seaLevel+3`, identidade Coast não. 0.15.11 rejeita destinos oceânicos fisicamente secos na drenagem, mas identidade visual continua com critérios diferentes; investigar caso reproduzível raw/interpolado/elevation/strength/coast blend, alinhar owner da identidade sem gerar oceanos falsos.

## P1.3 — Clouds

0.15.8 altura relativa ao seaLevel (Overworld 90 -> centros Y124–138), 0.15.9 world-space/tile recycling, 0.15.10 uma mesh por nuvem reduz custo. Conferir visual em diferentes dimensões, tile transitions/teleports, densidade e FPS.

## P2 histórico

- held block observa hotbar e esconde slot vazio;
- ghost block transparência uniforme por bloco, não faces individualmente;
- dye reportado fraco;
- Player HUD planejada canto inferior esquerdo, nome placeholder Yogg'Sara, vida 50/100 dentro da barra, status acima; Target HUD acima da crosshair, bloco à esquerda em slot de inventory, textos à direita com tooltip shadow, entidades futuramente. Não gerar imagens sem pedido.

---

# Performance direction

Meta não só ~60 FPS, mas mundo pronto antes de ser alcançado. Heavy generation/mesh/remesh async; integração/restore/unload/lighting/fluid budgetados; prioridade por visibilidade e direção; visible antes do backlog; preload preditivo; evitar unload/visibility/remesh churn e scans/dirty writes; hydrology autoritativa na generation. Void cru, flicker, silhueta na fog e chunks totalmente escuros não são fallbacks aceitáveis. Alterações visuais devem preservar identidade de biomas e pipeline HDR completo.