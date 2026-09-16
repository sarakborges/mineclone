# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras

Este `HANDOFF.md` de `develop` é a fonte persistente canônica; anexos e exports são snapshots derivados.

- Trabalhar diretamente em `develop`, sem branch não solicitada. Antes de escrever verificar HEAD, `VERSION` raiz e arquivos reais.
- Commits pequenos e coerentes; cada bloco de código sobe `VERSION`: PATCH fix/refactor/otimização compatível, MINOR feature compatível, MAJOR mudança de contrato. Documentação isolada não requer bump.
- **Atualizar HANDOFF na mesma sessão de qualquer mudança material de código, arquitetura, roadmap, versão, processo ou feedback.** Não deixar snapshot canônico obsoleto.
- `go`/`continua` significa executar; erros/warnings reais e feedback de gameplay têm prioridade sobre roadmap. Corrigir warnings dos blocos tocados.
- CI canônico `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`; `fmt` não é gate. CI não executa `cargo test`. Não repetir narração de ausência de cargo local ou espera de `cargo run`; testes manuais somente quando usuário solicitar.
- Distinguir causa identificada, código publicado, CI confirmado e gameplay visual confirmado. CI verde não comprova rios conectados, iluminação visual nem FPS.
- Não gerar imagens sem pedido. `VERSION` raiz é versão operacional; versão `Cargo.toml` 0.10.16 é deliberadamente antiga.

## Canon e invariantes arquiteturais

1. `ARCHITECTURE.md` é o canon arquitetural. Responsabilidade com owner autoritativo, compartilhada corretamente entre gameplay/generation/rendering/HUD.
2. Pipeline async: selection/preload -> generation task -> integrate -> initial lighting -> halo mesh snapshot -> mesh task -> render allocation -> visibility -> retention -> archive/unload. Integração na main thread orçamentada; stale async descartado/reagendado por revisões.
3. Filas canônicas `DeduplicatedQueue<T>`/`VoxelUpdateQueue`; orçamento `FrameWorkBudget`; visible obrigatório supera preload, preempção devolve task não crítica à fila.
4. Mesh e remesh validam conteúdo/iluminação separadamente. `ChunkRemeshTasks::lighting_revisions` difere de `VoxelWorld::chunk_mesh_revisions`. Geometry, Lighting e Fluid remesh incorporam luz nos vértices e validam halo3×3×3; Terrain e Fluid são meshes distintas, nenhuma fila deve consumir trabalho da outra. Primeiro mesh preserva time-to-visible; investigar sem starvation.
5. Streaming separa visible/preload/retenção, retirement não implica unload imediato. Histerese sem revelar preload distante; luz interativa prioritária e revisão publicada após convergência. Iluminação direta inicial seed apenas uma vez por residência, nunca por retry de mesh.
6. Stack HDR: world camera order0 Skip, viewmodel HDR order1 tonemap final, UI SDR order2; preservar mundo/mão/HUD/overlays.
7. Fog readiness frontier até imóvel; esconder void/silhuetas sem perder paleta artística por bioma. Enquanto fundo for `ClearColor` plano, cor terminal de `DistanceFog` deve coincidir exatamente com o céu; não misturar paleta de fog em todo o sky para disfarçar costura. Nuvens world-space, vento independente jogador, seaLevel por dimensão.
8. Hot paths change-driven, sem alocações/scans/dirty writes no idle. Held viewmodel observa hotbar/visual/tint, não `PlacementOrientation`; R afeta exclusivamente bloco colocado.
9. Rios incidentes compartilham X/Z/Y do nó de drenagem/lago; terrain adjustment não altera endpoints; regiões vizinhas reproduzem mesmo segmento. Margens corrigidas em 0.15.12 são regressão proibida.
10. Objetivo caminhando, circulando e voo máximo: sem void, popping/flicker, chunks pretos/fog breathing; não esconder throughput insuficiente apenas mudando fog.
11. **Céu:** cada bioma é dono da própria paleta `skyColor` por fase. `CurrentBiome`/pesos e relógio determinam `EnvironmentVisualState.sky_color`; `SkyPlugin` escreve exatamente essa cor em `ClearColor`. `fogColor` é paleta independente. Ordenar rastreamento biome antes do cálculo visual no mesmo frame; jamais misturar uma porcentagem fixa de fogColor no céu.

---

# Estado em 2026-09-16

**Último código:** `93573d5a692f9029a55113e677e761e54fda41a2` (correção OnceLock), `1d8d096f688824c739934efe003bd312b6e660b1` (environment: remove blend artificial e ordena depois de track biome), `cb5222e1cf5bfdf90296e57e0038f08abf2761dd` (sky: regressão palette). Bumps: `.23` `eeaaacb2d2c838df64485cb241038caf06e26e54`; `.24` `3b5503e450b0836c61fd4b107ec2f5625621224c`. **VERSION = 0.15.24**; documento posterior não requer bump. Reconsultar HEAD/VERSION para próxima escrita.

**CI confirmado até 0.15.21 (Clippy + Check):** 0.15.3 run35054771262, .4 #2064, .5 #2066, .6 #2068, .7 #2070, .8 #2075, .9 #2078, .10 #2080, .11 #2082, .12 #2084, .13 run35109440316, .14 run35111512775, .15 run35112785390, .16 run35112964210, .17 run35113608338, .18 run35114063244, .19 run35116468021, .20 run35116860460, .21 run35118558910. **0.15.22 CI run35119214356 FAILED Clippy:** dois erros de parser em `src/voxel/chunk.rs:23/28` (`OnceLock<Arc<[Option<VoxelCell]>>`/FluidCell faltavam fechamento de `Option<...>`); Check foi pulado. Corrigidos exatamente os dois tokens no commit `.23` `93573d5` (compare mostra 2 linhas alteradas). **.23 run35120838368 e .24 run35121054358 em execução na última consulta**, NÃO declarar CI verde antes de confirmar, investigar logs se falharem. CI não roda unit tests nem gameplay.

## Prioridades — feedback 2026-09-16

| Item | Estado | Próximo passo |
| --- | --- | --- |
| P0.1 FPS/carregamento | Melhorou, ainda aberto | Profiling e otimizações medidas, cuidado custo remesh fluido .22 |
| P0.2 chunks escuros/sombras tardias | Aberto visualmente; .17 seed única CI verde, .21 Geometry freshness CI verde, .22 Fluid freshness publicou mas CI original falhou sintaxe corrigida .23 | CI .23/.24, first mesh stale, remesh e gameplay |
| P0.3 rios cortam túneis com parede reta | Aberto visualmente; carver .14 CI aprovado | Gameplay, density/carvers sem regredir margens |
| P0.4 rios sem ligação lago/oceano | Aberto visualmente; patches .15/.16/.19/.20 CI aprovado | End-to-end por seed e coordenadas, fluido físico entre chunks |
| P0.5 margens verticais de rios | **Corrigidas segundo usuário** após .12 | Preservar |
| P0.6 biomas regionais ausentes | ~3000 blocos quase só Plains/Mountains | Medir seleção efetiva vs semelhança visual |
| P0.7 sky/fog sem cor por bioma | **Sky sincronização corrigida no código .24, gameplay pendente.** Fog artístico segue sem aplicação efetiva por limitação do fundo plano | Confirmar CI .24 e sky em jogo; projetar sky gradient/fog terminal sem costura, sem transformar céu em média fixa |
| P1 antigo streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar |
| P2 antigo hotbar/ghost/dye/HUD | Usuário informou `2 feito` | Preservar |
| Bug R distorce held block | .18 CI aprovado, ainda sem confirmação visual | Testar R com orientável e escala |

Ordem: corrigir CI real primeiro, preservar FPS/P0.5, investigar P0.2/P0.4, depois P0.3/P0.6/P0.7 e R. Não encerrar bugs visuais só por CI.

## Histórico de código e decisões

- 0.14.70 `9302118`: fog terminal sky, silhuetas persistiram; .71 `b028fbe` HDR parcial quebrou mundo/HUD; .72 `57305f4` sol/lua texturizados; .73 `f864669` revert HDR parcial.
- .15.0 `1b16bee`: world HDR Skip, viewmodel tonemap final, UI SDR. .15.1 `4aeb119`: fog frontier/AABB -1 chunk, usuário confirmou fim dos chunks brotando na fog. .15.2 `7b16f8f`: remove temporal recovery. .15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns membership_revision, recriação câmera; CI verde.
- .15.4 `766d970`/`fbe301f`: lane de luz interativa preempta streaming/remesh após convergência; sombras ainda tardias. .15.5 `fa84d81`: remesh async halo3×3×3, tracker de revision separado.
- .15.6 `f8d244c`: proteção túnel sob água bed fade, barreira persistiu. .15.7 `5c99616`: mountain belt threshold .86->.90 width .22->.15, peaks spacing760->850 chance .48->.36 radius120–230->110–205; Wasteland weight1->1.30, Witchwood/Enchanted1->1.25, Plains oak chance .48->.54. Size/avoidNear não mudaram; biomas ainda visualmente ausentes.
- .15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda camera bound. .15.9 `b68ecb3`: world-space wind/tile pool180. .15.10 `eadb6ea`: três cubos transparentes -> uma mesh/nuvem (~2/3 entidades/draw calls menos); FPS melhor, ainda aberto.
- .15.11 `780df02`: wet ocean outlet exige floor ≥4 abaixo seaLevel; dry fringe não é outlet. .15.12 `68a26a3`: signed shore grading, faixa lateral, banco alto rebaixa, outer bank baixo eleva sem preencher canal; ocean density usa altura real. **Usuário confirmou margens corrigidas.**
- .15.13 `68e5062`: generation density carver primeiro, water_near lazy só com carve/cache inclusive None; testes scans0/1. FPS melhor segundo usuário, sem atribuição exclusiva.
- .15.14 `cc77c4f` bump `296960a`: túnel protege bed-3..water+3, fade4 acima/abaixo, testes limites/cache, margem preservada. CI35111512775 success, gameplay aberto.
- .15.15 `8528960`/fix `f03b5bc`, bump `2ab8466`: remove confluence_target que desviava afluente à corda reta enquanto tronco meandrava; todos terminam downstream X/Z, ranking upstream removido. CI35112785390 success.
- .15.16 `02f1b18` bump `e873159`: valid_lake_outlet impede saída seca de lago conectado pela entrada; testes. CI35112964210 success.
- .15.17 `68d1d6b`/`ca3bb60` bump `28f4f48`: initial_lighting_seeded uma vez/residência, retry mesh não apaga luz convergida, unload remove marca, restore reseeda; testes. CI35113608338 success, P0.2/FPS gameplay aberto.
- .15.18 `3b337bf` bump `363d205`: viewmodel ignora PlacementOrientation, não altera held no R, transform/escala fixos, hotbar/tint observados. CI35114063244 success, R runtime aberto.
- .15.19 `b04ad3f`/`391322c`/`bba6608`/`9044a40` bump `5fd8b56`: rios usam Y authoritative dos nós/lagos nos endpoints, remove queda artificial0.5, ajusta somente intermediários, testes segmento/lago/região. CI35116468021 success; possível seção elevada em vale abaixo de saída, gameplay aberto.
- .15.20 `a1d3fde` bump `754973c`: trace limit64 inconclusivo não cacheia false nos nós internos, cache completo somente destino/dead-end provado, testes. CI35116860460 success; rios gameplay aberto.
- .15.21 `fcbafa8` bump `d4b5cc7`: Geometry remesh captura revision halo light como Lighting; teste snapshot luz vizinho, CI35118558910 success. Fluid também incorpora luz, tratado .22.
- **.15.22** `685ab044`/`0edb5515`/`657804c6`/`ffd67c15`, bump `fdbf3e8`: filas Terrain e Fluid não se suprimem em enqueue/pop/coalesce, meshes são separadas. `enqueue_lighting_change(coord, &world)` enfileira Fluid para centro/vizinhos com fluido (contagem O(1)); Fluid remesh passa a validar halo de iluminação como Geometry/Lighting. Testes fila/invalidação. **CI35119214356 FALHOU em dois erros de sintaxe de genéricos no arquivo chunk.rs introduzidos no bloco; Check pulado.** Corrigido na .23, gameplay/FPS abertos.
- **.15.23** `93573d5a692f9029a55113e677e761e54fda41a2`, bump `eeaaacb2`: adiciona `>` faltante em `OnceLock<Arc<[Option<VoxelCell>]>>` e `OnceLock<Arc<[Option<FluidCell>]>>` sem mexer algoritmo; compare 2 linhas. CI run35120838368 pendente última consulta.
- **.15.24** `1d8d096f688824c739934efe003bd312b6e660b1`/`cb5222e1cf5bfdf90296e57e0038f08abf2761dd`, bump `3b5503e`: remove `horizon_color()` que fazia blend arbitrário 35% `sky_color`+`fog_color` e teste correspondente, pois usuário confirmou céu exclusivamente por bioma. `update_environment_visuals.after(track_current_biome)` em Update força amostra de bioma atual no mesmo frame. `sky.rs` mantém ClearColor EXATAMENTE `visuals.sky_color.to_color()`, inclui teste Bevy App alternando sky e fog separadamente para provar que mudar fog não muda sky. **CI run35121054358 pendente e gameplay sky/fog aberto.** Fog continua terminal = sky até haver gradiente de céu/implementação fog própria sem seam.

## Subsistemas / investigação direcionada

### Streaming, FPS, iluminação

Preload all-direction+2 e corredor frontal+8; fog start~78% end~98% do raio conforme readiness; histerese RD4=5/6 RD12=14/16 RD24=26/30; retention R+max(ceil(R/2),10): RD4=14 RD12=22 RD24=36. Geração/remesh assíncrona, integração budgetada. `hydrology/region/density.rs` cria Vec<VerticalDensityDelta> por coluna e testa lagos carve+shore; otimizar só com evidência sem alterar geometria. Cloud mesh .10, lazy water .13, seed única .17 são ganhos localizados. Medir FPS parado/andando, CPU worldgen/lighting/remesh vs GPU clouds.

Antes .17, `dispatch_initial_mesh_tasks` reseed a cada retry e `VoxelWorld::rebuild_chunk_light` apagava luz convergida. Seed única e unload remove marca. `lighting_updates.rs` budget2ms/4096 voxels; remesh 2 tasks/frame, 4 in-flight, 2 integrações. `ChunkMeshDependencies` inicial compara conteúdo apenas (time-to-visible), `VoxelWorld::chunk_mesh_revisions` inclui luz. Antes .21 Geometry podia sobrescrever Lighting convergida; .21 valida halo Geometry. Antes .22 Terrain removia fila Fluid e luz Fluid não atualizava; .22 separa filas e refresca fluid só quando existe água, versionando halo. **Primeira mesh ainda pode publicar luz velha**, pois `ChunkMeshTasks` só confere content. Candidata: publicar primeira mesh rápido e enfileirar relight se luz mudou desde snapshot (sem rejeitar primeira mesh/reseed), avaliar custo. P0.2/FPS abertos.

### Hidrologia

`generation/density.rs` combina hydrology density+surface carver; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` abre bed->waterLevel com headroom limitado. .14 protege leito/teto fade bounded lazy water_near; paredes persistindo: investigar cave connector/túnel/threshold sem mexer margem .12.

`river/selection.rs::build_flow_cache` source radius14 target8 trace64, keep_only_complete_downstream_paths wet ocean/lake; `river.rs::build_river_system` edge margin4, reachability, valid_lake_outlet. `river/path/confluence.rs` clipping bounds+RIVER_MAXIMUM_RADIUS. .15 ancora X/Z, .16 saída seca filtrada, .19 ancora Y e testa edge entre regiões, .20 cache trace-limit. **Falta teste end-to-end de conectividade física por seed:** seleção regional, confluence/lake levels, fonte de água rasterizada, ocean mouth e fluido entre chunks. `DrainageNetwork::is_wet_ocean` usa node point sample; `HydrologyRegion::water_at` macro5×5 interpolado, discrepância ainda HIPÓTESE não reproduzida. `river_height` threshold continentalness diferente de teste wet floor, conferir transição mar. Não prolongar rios arbitrariamente/regredir margens.

### Biomas, sky, fog e cor

`biome_field/selection.rs` suitability temperatura/umidade/continentalness/erosion, pesos elegíveis, vizinho dominante, avoidNear8, fallback raw; `surface.rs` Voronoi/Mountains belt+peak. `surface_site_spacing` maior mínimo regional; Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; pesos Plains1 Wasteland1.30 Witchwood/Enchanted1.25. Wasteland/Plains rolling e mesmos surfaceLayers grass/dirt/stone, tint/structures diferentes: semelhança NÃO é contagem real. Mountain belt `ridge=1-abs(fractal_noise)` favorece ruído perto de zero, amostra exploratória seed42 não mede bioma final; medir antes de mexer pesos.

`world/biome.rs::track_current_biome` amostra jogador e compõe surface/hydrology/volume; `CurrentBiomeVisuals` pesa registros de bioma. `rendering/environment.rs` interpola skyColor e fogColor por fase e influência; .24 força `after(track_current_biome)` para evitar identidade de frame anterior. `rendering/sky.rs` aplica SOMENTE sky_color ao ClearColor, teste mudança sky/fog independente. **`fog/color.rs` e `fog/attachment.rs` ainda usam sky_color no DistanceFog para esconder silhueta, fog_color artístico é calculado mas não aplicado; P0.7 aberto.** Fundo é plano; `DistanceFog.color=fog_color` geraria silhueta/costura. Solução precisa sky gradient com cor de horizonte coerente e zenith skyColor, ou composição customizada do fog que converja skyColor no limite: projetar com testes e sem quebra da stack HDR/celestials/stars/clouds. Não misturar 35% fogColor no ClearColor (tentativa abortada .24 por exigência do usuário).

### Coast, celestial e HUD

Coast identidade continentalness raw enquanto oceano físico macro5×5; material rejeita ocean bed acima seaLevel+3, identidade não testa altura; P1 concluído segundo usuário, reabrir só evidência. CelestialBodyDefinition sun/moon texturizados; nuvens world-space em Y relativo a seaLevel e vento independente. P2 informado concluído: hotbar vazio oculta held, ghost alpha uniforme, dye, HUD histórica. HUD planejada: player inferior esquerdo Yogg'Sara vida50/100 dentro barra/status acima; target acima crosshair com bloco à esquerda estilo inventory e nome/infos à direita shadow tooltip. R fix .18 runtime pendente.

---

# Próxima execução

1. Confirmar CI .23 run35120838368 e .24 run35121054358; corrigir qualquer erro real antes de novo bloco; .22 é FALHA confirmada mas sintaxe corrigida .23. `cargo test` não integra CI.
2. P0.7 sky: conferir transição entre biomas e relógio após ordem .24. Fog artística requer estratégia de gradiente/cor terminal, sem blending arbitrário do sky, e preservação de fog readiness e HDR. Não dizer resolvido sem gameplay.
3. P0.2: inicial mesh versus iluminação revisada, possível relight posterior sem starvation; confirmar gameplay .17/.21/.22 e medir FPS/throughput extra de fluid.
4. P0.4: end-to-end seed/coords em regiões vizinhas, edge Y/fluido por chunk, lago/confluence/ocean outlet, preservar .12/.15/.16/.19/.20.
5. P0.3 tunnel walls .14; P0.6 contagem final de biomas e mountain belt; R/viewmodel .18 e FPS gameplay.

Todo novo bloco de código sobe `VERSION` e atualiza HANDOFF na mesma sessão.