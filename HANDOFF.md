# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras

Este `HANDOFF.md` de `develop` é a fonte persistente canônica. Exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`, sem feature branch não solicitada. Antes de escrever: verificar HEAD, `VERSION` e os arquivos reais.
- Commits pequenos/coerentes. Todo bloco coerente de código sobe `VERSION`: PATCH fix/refactor/otimização compatível; MINOR feature compatível; MAJOR quebra de contrato. Documentação isolada não requer bump.
- **Atualizar este HANDOFF na mesma sessão de toda mudança material de código, arquitetura, roadmap, versão, processo ou feedback.** Não deixar obsoleto.
- `go`/`continua` = executar sem confirmação desnecessária; erros/warnings e feedback runtime do usuário prevalecem sobre roadmap. Corrigir warnings dos blocos tocados.
- CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`. Não repetir que falta cargo local ou que aguardamos `cargo run`; testes manuais apenas quando o usuário solicitar. `fmt` não é gate. CI **não executa** `cargo test`.
- Distinguir sempre causa identificada, código publicado, CI e confirmação de gameplay. CI verde não comprova correção visual, rios conectados, performance nem FPS.
- Sem geração de imagens sem pedido explícito. `VERSION` raiz é versão operacional; `Cargo.toml` permanece deliberadamente antigo (0.10.16).

## Canon e invariantes arquiteturais

1. `ARCHITECTURE.md` é o canon arquitetural. Cada fato tem owner autoritativo, compartilhado corretamente entre gameplay/generation/rendering/HUD.
2. Pipeline async: selection/preload -> generation task -> integrate -> initial lighting -> halo mesh snapshot -> mesh task -> render allocation -> visibility -> retention -> archive/unload. Integração orçamentada na main thread; stale async descartado/reagendado por revisions.
3. Filas canônicas `DeduplicatedQueue<T>`/`VoxelUpdateQueue`; budget `FrameWorkBudget`; visible mandatory supera preload, com preempção devolvendo task não crítica à fila.
4. Mesh e remesh validam revisões de conteúdo e iluminação separadamente. `ChunkRemeshTasks::lighting_revisions` não é `VoxelWorld::chunk_mesh_revisions`; boundary remesh corrige halo quando vizinho chega. **Geometry, Lighting e Fluid remesh TODOS incorporam luz em vértices e validam halo 3×3×3. Terrain e Fluid são meshes distintas, nenhuma delas substitui a fila da outra.** Primeiro mesh tem política distinta para time-to-visible, investigar sem introduzir starvation.
5. Streaming separa visible, preload e retenção; retirement não é unload imediato; histerese, sem revelar preload distante. Luz interativa tem prioridade, revisão publicada após convergência; seed iluminação direta somente uma vez por residência de chunk.
6. Stack HDR world camera order 0 Skip -> viewmodel HDR order 1 tonemap final -> UI SDR order 2. Validar mundo/mão/HUD/overlays em mudanças.
7. Fog readiness frontier até parado; cor terminal deve esconder void/silhuetas e manter intenção artística por bioma. Nuvens world-space, wind independente jogador, seaLevel por dimensão.
8. Hot paths change-driven, evitar alocações/scans/dirty writes no idle. Held/viewmodel observa hotbar/visuais/tint, sem `PlacementOrientation`; R só muda bloco colocado.
9. Rios: entradas e saídas incidentes usam X/Z/Y do mesmo nó de drenagem/lago; ajuste de terreno não move endpoints, região vizinha reproduz mesmo trecho. Margens de 0.15.12 são regressão proibida.
10. Objetivo em caminhada, círculo e voo máximo: sem void, popping/flicker, chunks escuros ou fog breathing; não mascarar throughput insuficiente apenas com fog.

---

# Estado em 2026-09-16

**Último commit de código:** `ffd67c15f268e93b15a346e02fadface23a10364` (`chunk_remesh_tasks.rs`: também valida fluid). Bloco .22: `685ab044f78edddafd0595feffdc2afd3b36bb14` (`chunk_remesh.rs`), `0edb55159d7e6665e1fa1be80e379b9bc1bce238` (`lighting_updates.rs`), `657804c65d7c06f4d7f9e32a9152216e0048aad2` (`voxel/chunk.rs`), `ffd67c15...` (`chunk_remesh_tasks.rs`). Bump `fdbf3e8dd7ef1dd1ca1df89634158389a1a988be`; `VERSION = 0.15.22`. Posteriores HANDOFF docs não exigem bump; reconsultar HEAD/VERSION antes de alterações.

**CI Clippy + Check confirmado até 0.15.21:** .3 run35054771262; .4 #2064; .5 #2066; .6 #2068; .7 #2070; .8 #2075; .9 #2078; .10 #2080; .11 #2082; .12 #2084; .13 run35109440316; .14 run35111512775; .15 run35112785390; .16 run35112964210; .17 run35113608338; .18 run35114063244; .19 run35116468021 success; .20 run35116860460 success; **.21 run35118558910 completed/success (Clippy + Check). .22 run35119214356 in progress na última consulta.** Corrigir warning/error se surgir. CI não executa `cargo test` nem gameplay.

## Prioridades — feedback 2026-09-16

| Item | Estado | Próximo passo |
| --- | --- | --- |
| P0.1 FPS/carregamento | Melhorou, ainda aberto | Profiling/otimizações mensuráveis; não elevar budgets às cegas, medir impacto remesh fluid .22 |
| P0.2 chunks escuros/sombras tardias | Aberto visualmente; .17 seed única CI aprovado, .21 freshness geometry CI aprovado, .22 fluid freshness pendente CI | Confirmar .22; initial mesh stale, remesh/fog, FPS |
| P0.3 rios cortam túneis com parede reta | Aberto visualmente; carver .14 CI aprovado | Gameplay, density/carvers sem regredir margens |
| P0.4 rios sem ligação lago/oceano | Aberto visualmente; .15/.16/.19/.20 CI aprovado | End-to-end por seed e coordenadas |
| P0.5 margens verticais de rios | **Corrigidas segundo usuário** após .12 | Preservar |
| P0.6 biomas regionais ausentes | ~3000 blocos quase só Plains/Mountains | Medir seleção real vs semelhança visual |
| P0.7 sky/fog sem cor por bioma | Aberto, reconfirmado | CurrentBiome->EnvironmentVisualState->ClearColor e fog artístico terminal |
| P1 antigo streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar |
| P2 antigo hotbar/ghost/dye/HUD | Usuário informou `2 feito` | Preservar |
| Bug R distorce held block | .18 CI aprovado sem confirmação visual | Testar R em bloco orientável e escala |

Prioridade preservar FPS e P0.5; confirmar .22; P0.2/P0.4, depois P0.3/P0.6/P0.7 e R. Não encerrar bug visual com CI.

## Histórico de código e decisões

- 0.14.70 `9302118`: fog terminal alinhada sky, silhuetas persistiram. .71 `b028fbe`: HDR parcial quebrou mundo/HUD. .72 `57305f4`: sol/lua texturizados. .73 `f864669`: revert HDR parcial.
- .15.0 `1b16bee`: world HDR Skip, viewmodel tonemap final, UI SDR. .15.1 `4aeb119`: fog frontier/AABB menos1 chunk; usuário confirmou fim dos chunks brotando na fog. .15.2 `7b16f8f`: remove temporal recovery. .15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns por membership_revision, recriação câmera; CI verde.
- .15.4 `766d970`/`fbe301f`: lane luz interativa preempta streaming/remesh após convergência; sombras tardias. .15.5 `fa84d81`: remesh async halo3×3×3, tracker revision separado.
- .15.6 `f8d244c`: proteção túnel sob água por bed fade, barreira persistiu. .15.7 `5c99616`: mountain belt threshold .86->.90 width .22->.15; peaks spacing760->850 chance .48->.36 radius120–230->110–205; Wasteland weight1->1.30, Witchwood/Enchanted1->1.25, Plains oak chance .48->.54. Size/avoidNear não mudaram, biomas ausentes visualmente.
- .15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 mas seguiam câmera. .15.9 `b68ecb3`: world-space wind/tile pool180. .15.10 `eadb6ea`: 3 cubos transparentes->1 mesh/nuvem (~2/3 entidades/draw calls menos); FPS melhor, ainda aberto.
- .15.11 `780df02`: wet ocean outlet exige floor ≥4 abaixo seaLevel; dry fringe não é outlet. .15.12 `68a26a3`: signed shore grading, largura lateral, banco alto rebaixa, outer bank baixo eleva sem preencher canal; ocean density altura real. **Usuário confirmou margens corrigidas.**
- .15.13 `68e5062`: generation density carver antes, water_near lazy apenas carve cache inclusive None/coluna, testes scans0/1; FPS melhor segundo usuário sem atribuição exclusiva.
- .15.14 `cc77c4f` bump `296960a`: proteção túnel bed-3..water+3 fade4 acima/abaixo, testes limites/cache, margens preservadas; CI35111512775 success, gameplay aberto.
- .15.15 `8528960` fix `f03b5bc`, bump `2ab8466`: remove confluence_target que apontava afluente à corda reta enquanto tronco meandrava; todos terminam downstream X/Z. Ranking/varredura upstream removidos; CI35112785390 success.
- .15.16 `02f1b18` bump `e873159`: valid_lake_outlet impede saída seca lago conectado pela entrada; teste; CI35112964210 success.
- .15.17 `68d1d6b`/`ca3bb60` bump `28f4f48`: initial_lighting_seeded por residência, retries mesh não apagam luz convergida, unload remove marca e restore reseeda; testes. CI35113608338 success, P0.2/FPS gameplay aberto.
- .15.18 `3b337bf` bump `363d205`: viewmodel não observa PlacementOrientation nem muta rotação ao R; transform fixo e escala, hotbar/tint continuam observados. CI35114063244 success, R runtime aberto.
- .15.19 `b04ad3f`/`391322c`/`bba6608`/`9044a40` bump `5fd8b56`: river path ancora Y do nó/lago nos endpoints, remove queda artificial 0.5; terrain ajusta só internos, teste trechos/lagos/fronteiras. CI35116468021 success; possibilidade vale abaixo saída criar seção elevada, gameplay aberto.
- .15.20 `a1d3fde` bump `754973c`: trace-limit64 inconclusivo não grava cache false para nós internos; cache integral apenas destino/dead-end conclusivo, testes. CI35116860460 success; rios gameplay aberto.
- .15.21 `fcbafa8` bump `d4b5cc7`: Geometry remesh passou a capturar revisão iluminação halo3×3×3 como Lighting; teste snapshot mudando luz de vizinho; **CI35118558910 success**. Investigação posterior mostrou Fluid também incorpora iluminação, tratado na .22.
- **.15.22** `685ab044`/`0edb5515`/`657804c6`/`ffd67c15`, bump `fdbf3e8`: `chunk_remesh.rs` não descarta mais fila Fluid ao enfileirar/pop Geometry, ImmediateGeometry ou coalescer Lighting+Geometry, pois `build_chunk_terrain_remeshes` e `build_chunk_fluid_remeshes` são distintos (`refresh.rs` troca assets separadamente). `enqueue_lighting_change(coord, &world)` enfileira também remesh Fluid para centro/vizinhos que `chunk.has_fluid()` indica por contador O(1), dedup. `lighting_updates.rs` passa world; `VoxelChunk::has_fluid` deixa de ser cfg(test); `chunk_remesh_tasks.rs` passa a validar lighting halo em **todos** Geometry/Lighting/Fluid porque fluid_mesh::build_fluid_meshes usa face_lighting/source_block_srgb. Testes: preservação de fila fluid entre Geometry/Lighting/Immediate, enqueue fluid só chunk com água, invalidação halo para todo remesh. **CI35119214356 em execução na última consulta; `cargo test` não executado; gameplay e FPS abertos.** Ações de Fluid poderão exigir tarefa adicional, mas são independentes e necessárias; medir budget/throughput.

## Subsistemas / investigação direcionada

### Streaming, FPS, iluminação

Pipeline async acima. Preload all-direction+2 e corredor frontal+8; fog start~78% end~98% do raio conforme readiness; histerese RD4=5/6 RD12=14/16 RD24=26/30; retention R+max(ceil(R/2),10): RD4=14 RD12=22 RD24=36. Geração/remesh async/integrada por budget. `hydrology/region/density.rs` Vec<VerticalDensityDelta> por coluna, lago carve+shore; otimizar só com evidência sem alterar geometria. Cloud mesh .10, lazy water .13, seed única .17 ganhos localizados; medir FPS parado/andando CPU worldgen/lighting/remesh versus GPU clouds.

Antes .17, `dispatch_initial_mesh_tasks` reseed a cada retry e `VoxelWorld::rebuild_chunk_light` apagava luz convergida. Seed única e unload remove marca. `lighting_updates.rs` budget2ms/4096 voxels; remesh 2 tasks/frame/4 in flight/2 integrações. `ChunkMeshDependencies` compara apenas content revisions, deliberado para time-to-visible; `VoxelWorld::chunk_mesh_revisions` inclui luz. Antes .21 Geometry podia sobrescrever mesh Lighting convergida; .21 valida halo Geometry. Antes .22 fila Terrain podia apagar solicitação Fluid sem renderizar água, e luz Fluid não era refrescada por iluminação; .22 separa filas e atualiza água apenas quando há fluid, com halo versionado. **Primeira mesh ainda pode publicar luz desatualizada** porque `ChunkMeshTasks` só verifica content, não luz. Solução candidata: publicar primeira mesh rápido e enfileirar um remesh de lighting quando a revisão de luz mudou desde captura (sem rejeitar mesh/reseed). Não alterar sem avaliar custo. P0.2/FPS permanecem abertos.

### Hidrologia

`generation/density.rs` compõe hydrology density+surface carver; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` abre água bed->waterLevel/headroom limitado. .14 protege leito/teto fade bounded, lazy water_near. Persistindo parede, cave connector/túnel/threshold sem mexer P0.5.

`river/selection.rs::build_flow_cache` source radius14 target8 trace64, keep_only_complete_downstream_paths wet ocean/lake; `river.rs::build_river_system` edge margin4, reachability, valid_lake_outlet. `river/path/confluence.rs` clipping bounds+RIVER_MAXIMUM_RADIUS. .15 ancora X/Z, .16 saída seca filtrada, .19 ancora Y/testa mesmo edge fronteiras, .20 cache trace-limit. **Falta teste end-to-end de conectividade física por seed:** seleção regional, confluence/lake levels, fonte de água rasterizada, ocean mouth e fluido entre chunks. `DrainageNetwork::is_wet_ocean` usa node point sample; `HydrologyRegion::water_at` usa macro 5×5 interpolado: discrepância é HIPÓTESE ainda não reproduzida. `river_height` threshold continentalness difere teste wet floor, conferir transição mar. Não prolongar rios arbitrariamente nem regredir margens.

### Biomas e cor

`biome_field/selection.rs`: suitability temperatura/humidade/continentalness/erosion, pesos elegíveis, vizinho dominante e avoidNear8, fallback raw. `surface.rs` Voronoi + Mountains belt/peak. `surface_site_spacing` maior mínimo regional. Plains120–420 Wasteland120–420 Witchwood/Enchanted140–460 Mountains macro .85; pesos Plains1 Wasteland1.30 Witchwood/Enchanted1.25. Wasteland/Plains rolling/mesmos surfaceLayers grass/dirt/stone com tint/structures distintos: aparência não mede frequência. Mountain belt ridge `1-abs(fractal_noise)` favorece ruído próximo zero, amostra exploratória seed42 não substitui contagem final.

`world/biome.rs` acompanha player, combina surface/hydrology/volume; `rendering/environment.rs` sky_color/fog_color HSI por CurrentBiome/fase, `rendering/sky.rs` ClearColor. `fog/color.rs`/`fog/attachment.rs` usam sky_color em DistanceFog para esconder silhueta, ignoram fog_color artístico (confirmado). Mudar só fog cria mismatch terminal: transição deve preservar cor terminal compartilhada. Céu sem mudança exige rastrear identidade/updates.

### Coast, celestial e HUD

Coast identidade continentalness raw enquanto oceano físico macro5×5; material rejeita ocean bed acima seaLevel+3, identidade não testa altura. P1 concluído segundo usuário, reabrir só evidência. `rendering/celestial.rs` CelestialBodyDefinition sun/moon texturizados; nuvens world-space seaLevel/wind independente. P2 informado concluído: hotbar vazio oculta held, ghost alpha uniforme, dye, HUD histórica. Planejado: player HUD inferior esquerdo Yogg'Sara vida50/100 na barra/status acima; target acima crosshair, bloco esquerda estilo inventory e texto direita shadow tooltip. R fix .18 runtime pendente.

---

# Próxima execução

1. Confirmar CI .22 run35119214356; corrigir erros/warnings se surgirem. CI não executa unit tests/gameplay.
2. P0.2: initial mesh vs iluminação revisada; publicar primeiro frame rápido + relight posterior se necessário, sem starvation; conferir gameplay .17/.21/.22 e FPS/throughput fluid.
3. P0.4: end-to-end seed/coord regiões vizinhas, edge Y e fluid por chunk, lake/confluence, ocean outlet. Preservar .12/.15/.16/.19/.20.
4. P0.3: gameplay tunnel walls .14, corrigir density/carver sem regredir margem.
5. P0.6 medir seleção real/mountain belt; P0.7 CurrentBiome->EnvironmentVisualState->ClearColor, fog artística terminal igual sky.
6. R/viewmodel .18 e FPS runtime. Não dar bug visual como resolvido por CI.

Todo bloco novo de código sobe `VERSION` e atualiza HANDOFF nesta mesma sessão.
