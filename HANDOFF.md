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
2. Pipeline async: selection/preload -> generation task -> integrate -> initial lighting -> halo mesh snapshot -> mesh task -> render allocation -> visibility -> retention -> archive/unload. Integração no main thread tem orçamento; stale async descartado/reagendado por revisions.
3. Filas canônicas `DeduplicatedQueue<T>`/`VoxelUpdateQueue`; budget `FrameWorkBudget`; prioridade visible mandatory supera preload, com preempção devolvendo task não crítica à fila.
4. Mesh e remesh validam revisões de conteúdo e iluminação separadamente. `ChunkRemeshTasks::lighting_revisions` não é `VoxelWorld::chunk_mesh_revisions`; boundary remesh corrige halo quando vizinho chega. **Geometry e Lighting remesh ambos publicam terrain mesh com luz incorporada: ambos validam halo de iluminação; Fluid remesh permanece separado.** Primeiro mesh tem política distinta para time-to-visible, investigar sem introduzir starvation.
5. Streaming separa visible, preload e retenção; retirement não é unload imediato. Histerese, sem revelar preload distante. Luz interativa tem prioridade; publicar revisão após convergência. Seed iluminação direta somente uma vez por residência de chunk, nunca em retry de mesh.
6. Stack HDR world camera order 0 Skip -> viewmodel HDR order 1 tonemap final -> UI SDR order 2. Validar mundo/mão/HUD/overlays em mudanças.
7. Fog readiness frontier até parado; cor terminal deve esconder void/silhuetas e ainda manter intenção artística por bioma. Nuvens world-space, wind independente jogador, seaLevel por dimensão.
8. Hot paths change-driven, evitar alocações/scans/dirty writes no idle. Held/viewmodel apenas observa hotbar/visuais/tint, sem `PlacementOrientation`; R só muda bloco colocado.
9. Rios: entradas e saídas incidentes devem usar X/Z/Y do mesmo nó de drenagem/lago; ajuste de terreno não pode mover endpoints. Região vizinha precisa reproduzir o mesmo trecho. Margens de 0.15.12 são regressão proibida.
10. Objetivo em caminhada, círculo e voo máximo: sem void, popping/flicker, chunks escuros ou fog breathing; não mascarar throughput insuficiente apenas com fog.

---

# Estado em 2026-09-16

**Último commit de código:** `fcbafa853ef2f6b3c2874fb212266884d73d240e` (`chunk_remesh_tasks.rs`: Geometry remesh valida iluminação do halo). Bump `d4b5cc728cab69966553ba814ea6abc8765a833d`; `VERSION = 0.15.21`. Commits posteriores de HANDOFF são documentais sem bump. Reconsultar HEAD/VERSION antes de alterações.

**CI Clippy + Check confirmado até 0.15.20:** .3 run35054771262; .4 #2064; .5 #2066; .6 #2068; .7 #2070; .8 #2075; .9 #2078; .10 #2080; .11 #2082; .12 #2084; .13 run35109440316; .14 run35111512775; .15 run35112785390; .16 run35112964210; .17 run35113608338; .18 run35114063244; .19 run35116468021 completed/success; .20 run35116860460 completed/success. **0.15.21 CI ainda não confirmado** na publicação deste handoff: consultar run do bump, corrigir erros/warnings concretos. CI não executa `cargo test` nem valida gameplay.

## Prioridades — feedback 2026-09-16

| Item | Estado | Próximo passo |
| --- | --- | --- |
| P0.1 FPS/carregamento | Melhorou, ainda aberto | Profiling/otimizações mensuráveis; não elevar budgets às cegas |
| P0.2 chunks escuros/sombras tardias | Aberto visualmente; seed única 0.15.17 CI aprovado e freshness de Geometry 0.15.21 publicada | Confirmar CI .21, examinar primeira mesh e gameplay, halo/revisions, FPS |
| P0.3 rios cortam túneis com parede reta | Aberto visualmente; carver 0.15.14 CI aprovado | Feedback, density/carvers sem regredir margem |
| P0.4 rios sem ligação com lago/oceano | Aberto visualmente; patches 0.15.15/.16/.19/.20 passaram CI | Conectividade física por seed, cross-region, ocean outlet, confluência |
| P0.5 margens verticais de rios | **Corrigidas segundo usuário** após 0.15.12 | Preservar |
| P0.6 biomas regionais ausentes | ~3000 blocos quase só Plains/Mountains | Medição por seed: seleção real vs semelhança visual |
| P0.7 sky/fog sem cor por bioma | Aberto, reconfirmado | CurrentBiome->EnvironmentVisualState->ClearColor, fog artístico terminal |
| P1 antigo streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar; reabrir apenas com evidência |
| P2 antigo hotbar/ghost/dye/HUD | Usuário informou `2 feito` | Preservar |
| Bug R distorce held block | 0.15.18 CI aprovado, sem confirmação visual | Pressionar R com bloco orientável, conferir escala/geometria |

Ordem: preservar FPS e P0.5; confirmar CI .21; P0.2 e conectividade P0.4; P0.3, P0.6/7, R. Não encerrar bug visual com CI.

## Histórico de código e decisões

- 0.14.70 `9302118`: fog terminal alinhada ao sky, silhuetas persistiram. 0.14.71 `b028fbe`: HDR parcial quebrou mundo/HUD. 0.14.72 `57305f4`: sol/lua texturizados. 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: world HDR Skip, viewmodel tonemap final, UI SDR. 0.15.1 `4aeb119`: fog frontier até primeira coluna ausente/AABB menos1 chunk; usuário confirmou fim dos chunks brotando dentro fog. 0.15.2 `7b16f8f`: remove temporal recovery. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns por membership_revision, recriação de câmera; CI verde.
- 0.15.4 `766d970`/`fbe301f`: lane de luz interativa preempta streaming e remesh após convergência; sombras ainda tardias. 0.15.5 `fa84d81`: remesh assíncrono valida halo3×3×3 e tracker independente de lighting revision.
- 0.15.6 `f8d244c`: proteção de túnel sob água fade pelo leito, barreira persistiu. 0.15.7 `5c99616`: mountain belt threshold .86->.90 width .22->.15, peaks spacing760->850 chance .48->.36 radius120–230->110–205; Wasteland weight1->1.30, Witchwood/Enchanted1->1.25, Plains oak chance .48->.54. Size e avoidNear não mudaram; usuário ainda viu biomas ausentes.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda camera-bound. 0.15.9 `b68ecb3`: world-space wind/tile pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes->1 mesh/nuvem (~2/3 entidades/draw calls a menos); FPS melhor, ainda aberto.
- 0.15.11 `780df02`: drenagem exige ocean bed ≥4 abaixo seaLevel; dry fringe não é outlet; ocean seco não sobrescreve river mouth. Rios soltos persistiram. 0.15.12 `68a26a3`: signed shore grading com altura exata, faixa lateral maior, rebaixa banco alto e eleva outer bank baixo sem preencher canal; ocean density usa altura real. **Usuário confirmou margens corrigidas.**
- 0.15.13 `68e5062`: `generation/density.rs` carver antes, `water_near` lazy apenas com carve e cache inclusive None/coluna, testes 0/1 scan; FPS melhor segundo usuário, não atribuir causa exclusiva.
- 0.15.14 `cc77c4f` bump `296960a`: proteção túnel entre bed-3 e water+3, fade4 acima/abaixo, testes de limites/fades/cache, preserva margem; CI35111512775 aprovado, gameplay aberto.
- 0.15.15 `8528960`, fix `f03b5bc`, bump `2ab8466`: removido `confluence_target` que desviava afluente para corda reta enquanto tronco meandrava. Todos terminam no mesmo downstream X/Z. Ranking/varredura upstream redundantes removidos, import corrigido; CI35112785390 aprovado.
- 0.15.16 `02f1b18` bump `e873159`: `valid_lake_outlet` impede saída seca do lago conectado apenas pela ENTRADA; downstream exige oceano molhado/lago conectado/caminho real, teste dead end vs próximo lago. CI35112964210 aprovado.
- 0.15.17 `68d1d6b` + `ca3bb60` bump `28f4f48`: `ChunkStreamingState::initial_lighting_seeded` marca primeira passagem de cada residência. Mesh stale/preempted retry não apaga luz propagada nem recria revisões; unload remove marca e restore reseeda. Teste once/retry/forget/reseed; CI35113608338 aprovado, P0.2/FPS gameplay aberto.
- 0.15.18 `3b337bf` bump `363d205`: remove `PlacementOrientation` do held viewmodel, não reaplica rotation do placement no spawn/sync, transform fixo `base_viewmodel_transform().rotation.inverse()`, escala HELD_BLOCK_SCALE; hotbar/tint/definições observados. Teste transformação, CI35114063244 aprovado, R runtime aberto.
- 0.15.19 `b04ad3f`, `391322c`, `bba6608`, `9044a40`, bump `5fd8b56`: `river/path/curve.rs` usa downstream authoritative `river_height` ou lake_level exato como endpoint; remove queda artificial `min(start - 0.5)`. `river/path/terrain.rs` ajusta APENAS pontos internos, preserva Y de endpoints, não mergulha abaixo da saída para depois subir; remove constante morta. Regressões relief0.1, dois trechos seguidos mesmo X/Z/Y, lago, endpoints e região vizinha com water height/strength iguais. CI35116468021 aprovado; `cargo test` não executado, gameplay aberto. Vale abaixo da altura final pode criar seção elevada, investigar por seed.
- 0.15.20 `a1d3fde` bump `754973c`: `river/selection.rs::drainage_reaches_water_destination` cache em trace-limit64 falso apenas na origem, não contamina nós internos; cache completo só em destino ou dead-end conclusivo. Helper `cache_reachability`, regressões de limite/destino/beco. CI35116860460 aprovado; gameplay aberto.
- **0.15.21** `fcbafa8` bump `d4b5cc7`: `chunk_remesh_tasks.rs`: Geometry e Lighting ambos reconstroem o mesmo terrain mesh com iluminação incorporada, mas só Lighting tinha `LightingRemeshDependencies` do halo 3×3×3. `ChunkRemeshDependencies::capture` agora associa revisão de luz a ambos; fluid-only conserva apenas content. Integração existente rejeita task stale/reagenda prioridade. Teste cria snapshot de VoxelWorld, verifica Geometry e Lighting invalidam com bump da luz de vizinho enquanto Fluid continua current. **CI pendente; `cargo test` não executado, gameplay P0.2 aberto.** Risco: retries sob luz mudando constantemente; acompanhar FPS.

## Subsistemas / investigação direcionada

### Streaming, FPS, iluminação

Pipeline async acima. Preload all-direction+2, corredor frontal+8; fog start~78% end~98% raio conforme readiness; histerese RD4=5/6 RD12=14/16 RD24=26/30; retention R+max(ceil(R/2),10): RD4=14 RD12=22 RD24=36. Geração/remesh async e integração budgetada. `hydrology/region/density.rs` constrói Vec<VerticalDensityDelta> por coluna e testa lagos carve+shore; otimizar com evidência sem alterar geometria. Cloud mesh .10, lazy water .13, seed única .17 são ganhos localizados; medir FPS parado/andando CPU worldgen/lighting/remesh versus GPU clouds.

Antes .17, `dispatch_initial_mesh_tasks` reseed a cada retry, `VoxelWorld::rebuild_chunk_light` apagava luz convergida e invalidava snapshots. Seed única, remove marca no unload. `lighting_updates.rs` budget2ms/4096 voxels; remesh 2 tasks/frame/4 in flight/2 integrações. `ChunkMeshDependencies` compara apenas content revisions (deliberado; não descarta primeira mesh por churn), `VoxelWorld::chunk_mesh_revisions` incorpora luz. Antes .21, Geometry remesh validava só content, podia sobrescrever remesh Lighting mais recente; .21 valida halo tracker para ambos. **Primeiro mesh ainda pode ser publicado com iluminação desatualizada** porque `ChunkMeshTasks` possui só `ChunkMeshDependencies`; investigar opção de publicar para time-to-visible e enfileirar remesh de luz quando snapshot foi superado, sem descartar/reseed/retry. Seed revision VoxelWorld pode não refletir `ChunkRemeshTasks::lighting_revisions` até dynamic updates. P0.2 não resolvido visualmente.

### Hidrologia

`generation/density.rs` compõe hydrology density+surface carver; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` abre água bed->waterLevel e headroom limitado. .14 protege leito/teto com fade vertical bounded, lazy water_near. Persistindo parede, cave connector/túnel/threshold, sem mexer P0.5.

`river/selection.rs::build_flow_cache` source radius14, target8, trace64; `keep_only_complete_downstream_paths` segue wet ocean/lake. `river.rs::build_river_system` arestas por região com margin4, reachability, valid_lake_outlet. `river/path/confluence.rs` recorta segmentos por bounds+RIVER_MAXIMUM_RADIUS. .15 ancora X/Z, .16 filtra saída seca, .19 ancora Y/testa mesmo edge entre regiões, .20 protege cache contra limite. **Não há teste end-to-end de conectividade física por seed**: seleção regional, confluence/lake levels, fonte de água rasterizada, ocean mouth, fluido entre chunks. `DrainageNetwork::is_wet_ocean` usa node point sample, `HydrologyRegion::water_at` usa macro 5×5 interpolado; discrepância é HIPÓTESE não reproduzida, não patch cego. `river_height` usa threshold continentalness diferente de wet floor; conferir mar. Não prolongar rios arbitrariamente, não regredir margens.

### Biomas e cor

`biome_field/selection.rs`: suitability de temperature/humidity/continentalness/erosion, pesos só elegíveis, vizinho dominante/avoidNear 8 vizinhos, fallback raw. `surface.rs`: Voronoi + Mountains belt/peak. `surface_site_spacing` do maior mínimo regional, não raio individual. Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; pesos Plains1 Wasteland1.30 Witchwood/Enchanted1.25. Wasteland e Plains usam rolling/mesmos surfaceLayers grass/dirt/stone, tint/structures diferentes; semelhança visual NÃO mede frequência. `mountain_belt` ridge `1-abs(fractal_noise)` favorece ruído perto de zero; seed42 amostragem exploratória não substitui contagem final nem justifica pesos.

`world/biome.rs` acompanha jogador e combina surface/hydrology/volume; `rendering/environment.rs` calcula sky_color e fog_color HSI por CurrentBiome/fase, `rendering/sky.rs` escreve ClearColor. `fog/color.rs`/`fog/attachment.rs` usam sky_color no DistanceFog para esconder silhueta, ignoram fog_color artístico (confirmado). Trocar diretamente fog_color causa mismatch terminal com fundo: projetar transição terminal compartilhada. Céu sem mudança exige rastrear identidade e updates.

### Coast, celestial e HUD

Coast identidade usa continentalness raw enquanto oceano físico usa macro interpolado5×5; material rejeita ocean bed acima seaLevel+3, identidade não testa altura. P1 concluído segundo usuário, reabrir com evidência. `rendering/celestial.rs` usa CelestialBodyDefinition sun/moon texturizados. Nuvens world-space Y relativo seaLevel/wind desacoplado. P2 informado concluído: hotbar vazio oculta held, ghost alpha uniforme, dye, HUD histórica. HUD planejada: player inferior esquerdo Yogg'Sara vida50/100 na barra/status acima; target acima crosshair, bloco esquerda como slot inventory e infos direita tooltip shadow. R fix .18 runtime pendente.

---

# Próxima execução

1. Confirmar CI 0.15.21, corrigir erros/warnings reais. CI não executa testes unitários/gameplay.
2. P0.2: analisar initial mesh vs lighting revisions e publicar eventual relight sem atrasar primeiro frame; validação visual .17/.21, FPS antes/depois.
3. P0.4: reproduzir end-to-end seed/coord em regiões vizinhas; amostrar edge Y/fluido por chunk, lake/confluence, ocean outlet. Preservar .12/.15/.16/.19/.20.
4. P0.3: feedback de tunnel walls após .14; corrigir density/carver sem regredir margem.
5. P0.6: medir seleção real versus semelhança visual e mountain belt; P0.7: CurrentBiome->EnvironmentVisualState->ClearColor e fog artística com terminal igual sky.
6. Confirmar R/viewmodel .18 e FPS. Não dar bug visual como resolvido por CI.

Todo bloco novo de código sobe `VERSION` e atualiza este HANDOFF na mesma sessão.
