# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica persistente; exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`, salvo pedido explícito de branch. Antes de escrever, verificar HEAD, `VERSION` e arquivos reais envolvidos.
- Commits pequenos e coerentes, sem misturar causas independentes. Cada bloco de alteração de código sobe `VERSION` (PATCH fix/refactor compatível, MINOR feature compatível, MAJOR breaking); documentação isolada não sobe versão.
- Atualizar este HANDOFF na mesma sessão de qualquer alteração material de código, arquitetura, roadmap, processo ou feedback de runtime. **Não deixar o handoff defasado.**
- Feedback de gameplay/erro/warning tem prioridade sobre roadmap. `go`/`continua` = executar sem confirmação desnecessária.
- Corrigir warnings dos blocos tocados. CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`. Não repetir que faltou cargo local ou que está esperando `cargo run`; testes manuais cargo só sob pedido explícito, fmt não é gate.
- Compilação não comprova correção visual, hidrológica nem FPS; separar causa identificada, patch publicado e confirmação em gameplay.
- Sem gerar imagens sem pedido explícito. Root `VERSION` é operacional; versão de `Cargo.toml` permanece intencionalmente antiga (0.10.16).

## Invariantes arquiteturais

1. Cada fato de gameplay tem owner autoritativo; dados e decisões consistentes entre geração, rendering e HUD.
2. Generation, mesh e remesh pesados fora da main thread; integração com budget. Resultados async revisionados, stale descartado/reagendado.
3. Filas canônicas: `DeduplicatedQueue<T>` e `VoxelUpdateQueue`; budget `FrameWorkBudget`.
4. Prioridade atravessa async: visible mandatory vence warm/preload; preempção devolve tarefa não crítica à fila.
5. Mesh async valida revisão de conteúdo e iluminação separadamente; boundary remesh corrige halo recém-disponível. `ChunkRemeshTasks::lighting_revisions` não é `VoxelWorld::chunk_mesh_revisions`.
6. Streaming separa visible/preload/retention; retirement não é unload imediato; visibilidade usa histerese e não expõe preload distante.
7. HDR: world camera order 0 Skip -> viewmodel HDR order 1 tonemap final -> UI SDR order 2. Não alterar parcialmente sem validar mundo, mão, HUD e overlays.
8. Fog acompanha readiness do frontier mesmo parado. Terminal fog e céu precisam ocultar void/silhuetas sem descartar cores artísticas por bioma.
9. Hot paths change-driven; não gerar scans/dirty writes sem mudança.
10. Luz interativa vence streaming; convergência publica revisão. Seed inicial uma vez por residência, não por mesh retry.
11. Sky layers com seaLevel da dimensão; nuvens world-space com vento independente do jogador, tile recycling por câmera.
12. Held/viewmodel observa hotbar/visuals/posição para tint; não depende de `PlacementOrientation` nem gira a geometria por R. Orientação pertence ao bloco colocado.
13. Rios: geometria incidente usa posição e altitude canônicas do mesmo nó de drenagem ou water body; terreno só ajusta pontos internos sem abrir degrau no encontro. Qualquer alteração precisa preservar margem 0.15.12 e ser testada em fronteiras de região.
14. Produto: caminhada/voo máximo sem void, flicker, chunks escuros, popping ou fog breathing; não ocultar falta de throughput apenas com fog.

---

# Estado — 2026-09-16

Último commit **de código** `9044a40b1872e1be8f7ece92b2631a42ebbcf0e0` (regressão de fronteira); correções `b04ad3f` terrain, `391322c` curve, `bba6608` constantes; bump `5fd8b56805cf1432b26ea1166b64796c642a311c`; `VERSION = 0.15.19`. Este commit de handoff é documental sem bump. Antes de qualquer escrita posterior, reconsultar HEAD e VERSION.

**CI confirmado até 0.15.18:** 0.15.3 `5297a69` run35054771262 success; 0.15.4 `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; 0.15.13 `68e5062` run35109440316 success; 0.15.14 `a6f770f` run35111512775 success; 0.15.15 `f03b5bc` run35112785390 success; 0.15.16 `e873159` run35112964210 success; 0.15.17 `1fa516d` run35113608338 success; 0.15.18 `2ec5693` run35114063244 success (Clippy + Check). **0.15.19 run35116468021 em andamento na última checagem**; atualizar status quando houver conclusão. CI não roda `cargo test`, não comprova gameplay.

## Prioridades — feedback 2026-09-16

| Item | Estado | Próximo passo |
| --- | --- | --- |
| P0.1 FPS/carregamento | Melhorou, aberto | Profiling/otimizações mensuráveis, sem alterar budget às cegas |
| P0.2 chunks escuros/sombras tardias | Aberto visualmente; 0.15.17 CI aprovado | Seed única, convergência, revisão de halo/remesh e confirmação em gameplay |
| P0.3 rios cortam túneis com parede reta | Aberto visualmente; 0.15.14 CI aprovado | Persistindo, investigar density/carvers sem regredir margens |
| P0.4 rios sem conexão lago/oceano | Aberto visualmente; subcausas 0.15.15/16/19 tratadas em código | CI 0.15.19, confluências e travessia reais, continuidade física/regional, ocean outlet |
| P0.5 margens verticais dos rios | CORRIGIDO segundo usuário após 0.15.12 | Preservar integralmente |
| P0.6 biomas regionais ausentes | Aberto: ~3 mil blocos quase Plains/Mountains | Medir seleção por seed e separar frequência da semelhança visual |
| P0.7 sky/fog sem cor de bioma | Aberto, reconfirmado | Rastrear identidade->visuals->ClearColor e composição terminal do fog |
| P1 streaming/Coast/nuvens anterior | Usuário informou `1 feito` | Preservar; reabrir só com evidência |
| P2 hotbar/ghost/dye/HUD anterior | Usuário informou `2 feito` | Preservar |
| R distorce held block | Patch 0.15.18 CI aprovado, visual pendente | Testar R em blocos orientáveis/escala fixa |

**Ordem:** proteger FPS e P0.5; confirmar CI 0.15.19, validar P0.4 em gameplay e P0.2/P0.3, depois P0.6/P0.7. R ainda depende de feedback visual. Não declarar bug resolvido a partir de compile/testes unitários somente.

## Histórico de código relevante

- 0.14.70 `9302118`: fog terminal alinhada ao sky, silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial quebrou mundo/HUD. 0.14.72 `57305f4`: sol/lua texturizados. 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: world HDR Skip, viewmodel tonemap final, UI SDR. 0.15.1 `4aeb119`: fog frontier até primeira coluna ausente/AABB menos 1 chunk; usuário confirmou fim de chunks brotando dentro da fog. 0.15.2 `7b16f8f`: remove recovery temporal. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache de active_columns por membership_revision, câmera recriada; CI aprovado.
- 0.15.4 `766d970`/`fbe301f`: iluminação interativa preempta streaming e remesh após convergência; sombras ainda atrasam. 0.15.5 `fa84d81`: lighting remesh async valida halo3×3×3 e revisão própria separada.
- 0.15.6 `f8d244c`: túneis sob água fade pelo leito, usuário ainda viu barreiras. 0.15.7 `5c99616`: mountain belt threshold .86→.90 width .22→.15; peaks spacing760→850 chance .48→.36 radius120–230→110–205; Wasteland weight1→1.30, Witchwood/Enchanted1→1.25, oak Plains chance .48→.54. Size/avoidNear não mudaram; usuário ainda viu 3k blocos quase só Plains/Mountains.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda seguiam câmera. 0.15.9 `b68ecb3`: nuvens world-space, wind/tile pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes→1 mesh/nuvem (~2/3 entidades e draw calls a menos); FPS melhor, ainda aberto.
- 0.15.11 `780df02`: drenagem exige ocean bed ≥4 abaixo seaLevel, rejeita fringe seco; ocean seco não substitui river mouth. Rios soltos persistem. 0.15.12 `68a26a3`: signed shore grading, amostragem exata, faixa lateral maior, rebaixa bancos altos e eleva outer banks baixos sem preencher canal; ocean density usa altura exata. **Margens corrigidas segundo usuário.**
- 0.15.13 `68e5062`: `generation/density.rs` carver antes, `water_near` lazy só com carve, cache inclusive None por coluna; testes 0/1 scan, FPS melhor segundo usuário sem atribuição causal exclusiva.
- 0.15.14 `cc77c4f` + bump `296960a`: carver protege bed-3 até water+3 e faz fade4 superior/inferior; testes de limites/fade/cache, sem mudar margem. CI35111512775 aprovado; gameplay aberto.
- 0.15.15 `8528960` + fix `f03b5bc` + bump `2ab8466`: remove `confluence_target` que mandava afluente à reta entre nós enquanto rio principal meandrava; todas as arestas terminam no downstream horizontal autoritativo. Remove ranking/varredura upstream redundante e fix import DrainageNode. CI35112785390 aprovado.
- 0.15.16 `02f1b18` + bump `e873159`: `valid_lake_outlet` impede saída seca de lago considerado conectado por receber rio. Downstream exige oceano molhado/lago conectado/caminho real. Teste dead end vs próximo lago. CI35112964210 aprovado.
- 0.15.17 `68d1d6b` + `ca3bb60` + bump `28f4f48`: `ChunkStreamingState::initial_lighting_seeded` marca residência; primeiras passagens seed e relax, retries stale/preempted não apagam luz convergida, unload remove marca e restore reseeda. Testes once/retry/forget/reseed. CI35113608338 aprovado.
- 0.15.18 `3b337bf` + bump `363d205`: remove `PlacementOrientation` do held, não aplica rotation do placement na sincronização nem no spawn. Transform fixo rotação inversa do base viewmodel e escala HELD_BLOCK_SCALE. Observa hotbar, definições e tint; teste de transformação. CI35114063244 aprovado, runtime R pendente.
- **0.15.19** `b04ad3f`, `391322c`, `bba6608`, `9044a40`, bump `5fd8b56`: `river/path/curve.rs` usa `river_height(downstream)` ou altura exata do lago como endpoint, sem `min(start - 0.5)` que criava degrau a cada nó; `river/path/terrain.rs` só ajusta pontos internos, preserva entrada/saída e impede mergulho interno abaixo da altura de saída seguido de subida artificial; remove constante não utilizada `RIVER_MINIMUM_WATER_DROP`. Testes: pequena queda natural, duas edges sequenciais em mesmo X/Z/Y, entrada/saída de lago com Y idêntico, terrain anchors, perfil interno downhill, crossing de MESMO trecho em duas regiões com igual altura/strength. Run35116468021 estava em andamento. Ainda NÃO há confirmação de gameplay nem execução de cargo test. Atenção ao tradeoff: em vale muito mais baixo que o endpoint canônico, manter monotonicidade pode elevar segmento interno acima da topografia; investigar com dados/seed, não ajustar margens arbitrariamente.

## Subsistemas e diagnósticos

### Streaming / FPS

Pipeline `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction+2, corredor frontal+8 andando. Fog start~78% end~98% do raio recua conforme readiness. Histerese RD4=5/6 RD12=14/16 RD24=26/30, retention R+max(ceil(R/2),10), exemplos RD4=14 RD12=22 RD24=36. Async + integração budgetada; não elevar budgets de luz às cegas. Profiling parado/andando CPU worldgen/lighting/remesh vs GPU clouds. `hydrology/region/density.rs` monta Vec<VerticalDensityDelta> por coluna e varre lagos para carve+shore; otimizar apenas com evidência e preservando geometria. 0.15.10 cloud mesh, 0.15.13 lazy water, 0.15.17 seed única melhorias localizadas; FPS aberto.

### Iluminação

Antes 0.15.17, `dispatch_initial_mesh_tasks` reseed a cada retry de mesh stale, apagava luz convergida e invalidava snapshot por revision. Seed única por residência remove no unload; CI aprovado. `lighting_updates.rs`: budget2ms/4096 voxels, remesh 2 tasks/frame, 4 in-flight, 2 integrações/frame. Seed de `VoxelWorld` pode não atualizar `ChunkRemeshTasks::lighting_revisions` (tracker só recebe updates dinâmicos). Investigar halo, skylight, revisão, mesh persistente escura vs dados de luz escuros. P0.2 aberto.

### Hidrologia

`generation/density.rs` compõe hydrology density+surface carver; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` abre água bed→waterLevel e headroom limitado. 0.15.14 protege teto/leito com fade vertical bounded e lazy `water_near`. Persistindo parede, investigar cave connector, superfície de túnel e thresholds. P0.5 preservado.

`river/selection.rs` constrói flow_cache com source radius14, target radius8 e trace64; `keep_only_complete_downstream_paths` segue para wet ocean/lake. `river.rs::build_river_system` produz arestas com margin4, reachability e channels. `river/path/confluence.rs` recorta segmentos por bounds+RIVER_MAXIMUM_RADIUS. 0.15.15 uniformizou X/Z dos tributários, 0.15.16 removeu saída seca de lago; 0.15.19 uniformizou Y, preservou endpoints e incluiu teste fronteira compartilhada. **P0.4 aberto**: teste novo prova apenas identidade do MESMO trecho recriado em duas regiões, não garante seleção real de trechos nem conectividade de água/chunks. Investigar amostras regionais com seed real, lake/confluence water levels, physical water `region/water.rs`, ocean outlet: `DrainageNetwork::is_wet_ocean` calcula leito de sample pontual, mas `HydrologyRegion::water_at` calcula leito interpolando macro amostras 5×5; divergência pode produzir desembocadura anunciada molhada e água física seca. Verificar por seed antes de alterar sistema. `river_height` threshold continentalness vs drenagem wet ocean pode criar salto próximo ao mar. Sem prolongar rios arbitrariamente ou regredir margem.

### Biomas e cores

`biome_field/selection.rs`: suitability temperatura/humidade/continentalness/erosion, pesos de elegíveis, dominância/avoidNear em 8 vizinhos, fallback raw; `surface.rs`: Voronoi + Mountains belt/peak. `surface_site_spacing` deriva do maior mínimo regional, não raio individual. Overworld Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; pesos Plains1 Wasteland1.30 Witchwood/Enchanted1.25. Wasteland e Plains usam ambos rolling e grass/dirt/stone, apesar de visuals/tint/structures diferentes: semelhança visual NÃO prova frequência. Medir seed antes de trocar pesos.

`rendering/environment.rs` calcula sky_color/fog_color HSI por CurrentBiome e fase do dia se inputs mudam; `world/biome.rs` acompanha jogador e `world/biome/identity.rs` combina surface/hydrology/volume; `rendering/sky.rs` escreve ClearColor. `fog/color.rs` e `fog/attachment.rs` usam sky_color no DistanceFog para evitar silhuetas, ignorando `fog_color` artístico: confirmado em código. Não trocar simplesmente por fog_color sem resolver igualdade da cor terminal com fundo. Céu sem mudança demanda inspecionar identidade/current biome e visual update antes de mexer em fog.

### Coast, celestial, HUD e históricos

Coast identidade continentalness raw em `BiomeField::climate_at`, oceano físico usa macro sample interpolado5×5; material rejeita ocean bed acima seaLevel+3 mas identidade não testa elevação. P1 informado concluído, reabrir só com evidência. `rendering/celestial.rs` usa textura de CelestialBodyDefinition para sol/lua. Nuvens world-space, Y relativo seaLevel, wind independente do player. P2 usuário marcou concluído: held observa hotbar/slot vazio, ghost alpha uniforme, dye e HUD. HUD planejada player inferior esquerdo Yogg'Sara vida50/100 dentro da barra, status acima; target HUD acima crosshair com ícone de bloco à esquerda estilo inventory slot e textos direita com tooltip shadow. Bug R 0.15.18 CI aprovado, runtime pendente.

---

# Próxima execução

1. Consultar run35116468021 (último commit de código 0.15.19 `9044a40`) e corrigir warnings/errors se CI falhar; verificar status de run pós-handoff quando aplicável. `cargo test` NÃO é gate do CI.
2. P0.4: teste end-to-end por seed real da continuidade de seleção em regiões vizinhas, joins de lago/confluência e água física até oceano. Preservar 0.15.12/15/16/19. Se confluência ainda cortada, amostrar altura e fluid em coordenadas específicas; em margens não mexer.
3. P0.2: runtime seed única 0.15.17, revisões halo, skylight, remesh, FPS parado/andando.
4. P0.3: feedback visual da 0.15.14; se parede persistir, density/carver com tests e sem regressão margem.
5. P0.6: medir distribuição real por seed vs percepção. P0.7: CurrentBiome→EnvironmentVisualState→ClearColor e fog artístico terminal sem silhueta.
6. Confirmar R/viewmodel visual 0.15.18 e FPS. Não declarar bugs visuais resolvidos por CI.

Cada novo bloco coerente de código sobe VERSION e atualiza HANDOFF na mesma sessão.