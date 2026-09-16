# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras

Este `HANDOFF.md` de `develop` é a fonte persistente canônica; anexos/exports são snapshots derivados.

- Trabalhar diretamente em `develop`, sem branch não solicitada. Antes de escrever verificar HEAD, `VERSION` raiz e arquivos reais.
- Commits pequenos/coerentes; cada bloco de código sobe `VERSION`: PATCH fix/refactor/otimização compatível, MINOR feature compatível, MAJOR mudança de contrato. Documento isolado não sobe versão.
- **Atualizar HANDOFF na mesma sessão de qualquer mudança material de código, arquitetura, roadmap, versão, processo ou feedback.** Não deixar canônico obsoleto.
- `go`/`continua` significa executar; erros/warnings reais e feedback gameplay têm prioridade sobre roadmap. Corrigir warnings dos blocos tocados.
- CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`; `fmt` não é gate. CI não executa `cargo test`. Não repetir ausência de cargo local ou espera de `cargo run`; testes manuais somente quando usuário solicitar.
- Diferenciar causa identificada, código publicado, CI confirmado e gameplay visual confirmado. CI verde não comprova rios conectados, iluminação visual nem FPS.
- Não gerar imagens sem pedido. `VERSION` raiz é operacional; versão de `Cargo.toml` 0.10.16 é deliberadamente antiga.

## Canon e invariantes arquiteturais

1. `ARCHITECTURE.md` é o canon. Responsabilidade com owner autoritativo, compartilhada corretamente entre gameplay/generation/rendering/HUD.
2. Pipeline async: selection/preload -> generation task -> integrate -> initial lighting -> halo mesh snapshot -> mesh task -> render allocation -> visibility -> retention -> archive/unload. Main thread orçamentada; stale async descartado/reagendado por revisões.
3. Filas canônicas `DeduplicatedQueue<T>`/`VoxelUpdateQueue`; `FrameWorkBudget`; visible obrigatório supera preload, preempção devolve task não crítica.
4. Mesh/remesh validam conteúdo/iluminação separadamente. `ChunkRemeshTasks::lighting_revisions` difere de `VoxelWorld::chunk_mesh_revisions`. Geometry, Lighting e Fluid remesh incorporam luz nos vértices e validam halo3×3×3; Terrain e Fluid são meshes distintas, uma fila não pode consumir trabalho da outra. Primeiro mesh preserva time-to-visible; investigar sem starvation. **Desde .28, ao integrar chunk novo (inclusive vazio), notificar remesh terreno de vizinho já renderizado com conteúdo em sua face, mesmo que a face do novo chunk seja vazia, pois luz direta do halo muda sem mudança posterior por relaxamento. Fluid independente.**
5. Streaming separa visible/preload/retenção; retirement não implica unload imediato. Histerese, sem revelar preload distante. Luz interativa prioritária, revisão publicada após convergência. Iluminação direta inicial seed uma vez por residência, nunca por retry de mesh.
6. Stack HDR: world camera order0 Skip, viewmodel HDR order1 tonemap final, UI SDR order2. Preservar mundo/mão/HUD/overlays.
7. Fog readiness frontier até imóvel; esconder void/silhuetas sem perder paleta artística por bioma. Enquanto fundo for `ClearColor` plano, terminal `DistanceFog.color` deve coincidir com céu; não misturar paleta de fog em todo sky para disfarçar costura. Nuvens world-space, vento independente, seaLevel por dimensão.
8. Hot paths change-driven, sem alocações/scans/dirty writes no idle. Held viewmodel observa hotbar/visual/tint e não `PlacementOrientation`; R só afeta bloco colocado. Biome .25 usa array oito vizinhos e único Vec sorteio; hidrologia .26 SmallVec inline 4 com spill, .27 compartilha distância irregular para água+margem e um único scan, .30 descarta corpos d'água fora do envelope máximo (inclui margem) antes do ruído irregular; .29 evita segundo lookup da definição de bloco por face na malha.
9. Rios incidentes compartilham X/Z/Y do nó drenagem/lago; terrain adjustment não altera endpoints; regiões vizinhas reproduzem mesmo segmento. Margens corrigidas em 0.15.12 são regressão proibida.
10. Objetivo caminhando/circulando/voo máximo: sem void, popping/flicker, chunks pretos ou fog breathing; não esconder throughput insuficiente apenas via fog.
11. **Céu:** bioma é dono de sua paleta `skyColor` por fase. `CurrentBiome`/pesos + relógio determinam `EnvironmentVisualState.sky_color`; `SkyPlugin` escreve exatamente essa cor no `ClearColor`. `fogColor` é paleta independente. Ordenar rastreamento biome antes da atualização visual no mesmo frame; jamais misturar porcentagem fixa de fogColor no céu.

---

# Estado em 2026-09-16

**Último código:** `.25` `ec38346a97e0d238f184aa32ed0a7c768ef0e3e4` biome selection, bump `87a27fe920a2d44405cb8eb02c4b4563c4f16d1b`; `.26` `d492c3685babc705d8a6e248a2e0a6ebf91decaa` hydrology density SmallVec, bump `25b262065a9a3a995ddd2c6ab427ed0f3f660c55`; `.27` `bbf9947655b04bc80689394352f8a71e632cb1bf` hydrology one-pass sample, bump `7fc04dc45b7fbe43bab796662ec4bb5ad8a96387`; `.28` `2f6e5928553f3ed06bfc9720183346b0273e5e32` neighbor lighting, follow-up `9b0e5444e16e3ced3f97514241d44ed9b329427b` restored assertion, bump `72eb67066781dbe4fb2a7fb8fdfeff373a714966`; `.29` `9acdb0ad1393408cd04949f933dcf92f7f5e7f44` geometry occlusion, `9f30340ab1ce273a7998c2066f49aa440e82f2ae` caller, bump `75731e9e3a06f61ff78d0466ca62a4f8fb497407`; **`.30` `9f43094d7874cdd5a690851826e7f9246367478e` hydrology culling, bump `0c1063f2eab07fdf631d2b158ca1d28f3100c57c`. VERSION = 0.15.30.** HEAD pode ser commit documental posterior: reconsultar antes de escrever. Precedent .24 sky `1d8d096` test `cb5222e`, bump `3b5503e`.

**CI:** versões .3–.21 Clippy+Check success; .22 run35119214356 failed OnceLock syntax, fixed .23; .23 run35120838368 failed dead-code sky mix, fixed .24; .24 run35121054358 SUCCESS; .25 run35122730177 SUCCESS job104884239808; .26 run35123049209 SUCCESS job104885298933; .27 run35123661593 SUCCESS job104887329235; .28 run35124362037 SUCCESS job104889650640; **.29 run35132844975 COMPLETED/SUCCESS Clippy+Check job104917811412; .30 run35133472565 em execução na última consulta, NÃO alegar aprovado antes de verificar.** CI NÃO executa unit tests nem gameplay; evitar afirmar FPS/visual comprovados. Não relatar .22/.23 como aprovados.

## Prioridades — feedback 2026-09-16

| Item | Estado | Próximo passo |
| --- | --- | --- |
| P0.1 FPS/carregamento | Melhorou, aberto; biome .25/hydrology .26/.27/.30 removem alocações/cálculos ou descartam lagos distantes; mesh .29 elimina lookup duplicado; FPS não medido. .28 adiciona remesh só borda com conteúdo | Medir andando/voando; throughput/custo Fluid .22 e Geometry .28 |
| P0.2 chunks escuros/sombras tardias | Aberto visualmente; seed única .17, Geometry freshness .21, Fluid freshness .22; .28 notifica terreno quando vizinho vazio/ar renderiza (CI aprovado) | Gameplay bordas e sombras, primeiro mesh stale e remesh |
| P0.3 rios cortam túneis com parede reta | Aberto visualmente; carver .14 CI aprovado | Investigar density/cave connector preservando margens |
| P0.4 rios sem ligação lago/oceano | Aberto visualmente; patches .15/.16/.19/.20 CI aprovados | Teste end-to-end seed/coords, fluido entre chunks |
| P0.5 margens verticais de rios | **Corrigidas segundo usuário** após .12 | Preservar |
| P0.6 biomas regionais ausentes | ~3000 blocos quase só Plains/Mountains; .25 NÃO muda seleção | Medir seleção final vs visual |
| P0.7 sky/fog sem cor por bioma | Sky sincronização .24 CI aprovado, gameplay pendente; fog artístico ainda não aplicado no renderer | Verificar céu e fog terminal sem costura, nunca misturar cor do céu |
| P1 antigo streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar |
| P2 antigo hotbar/ghost/dye/HUD | Usuário informou `2 feito` | Preservar |
| Bug R distorce held block | .18 CI aprovado, gameplay pendente | Testar R/escala orientável |

Preservar FPS e margens P0.5; resolver erros reais de CI; P0.2/P0.4/P0.3/P0.6/P0.7/R conforme evidência. Não encerrar bug visual só com CI.

## Histórico de código e decisões

- 0.14.70 `9302118`: fog terminal alinhada sky, silhuetas persistiram; .71 `b028fbe`: HDR parcial quebrou mundo/HUD; .72 `57305f4`: sol/lua texturizados; .73 `f864669`: revert HDR parcial.
- .15.0 `1b16bee`: world HDR Skip, viewmodel HDR order1 tonemap final, UI SDR. .15.1 `4aeb119`: fog frontier/AABB menos1 chunk, usuário confirmou fim chunks brotando na fog. .15.2 `7b16f8f`: remove temporal recovery. .15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns por membership_revision, recriação câmera; CI verde.
- .15.4 `766d970`/`fbe301f`: lane luz interativa preempta streaming/remesh após convergência; sombras tardias. .15.5 `fa84d81`: remesh async halo3×3×3, tracker separado.
- .15.6 `f8d244c`: túnel proteção sob água bed fade, barreira persistiu. .15.7 `5c99616`: mountain belt threshold .86->.90 width .22->.15, peaks spacing760->850 chance .48->.36 radius120–230->110–205; Wasteland weight1->1.30, Witchwood/Enchanted1->1.25, Plains oak chance .48->.54; size/avoidNear não alterados, biomas ainda visualmente ausentes.
- .15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda camera bound. .15.9 `b68ecb3`: world-space wind/tile pool180. .15.10 `eadb6ea`: três cubos transparentes -> uma mesh/nuvem (~2/3 entidades/draw calls menos); FPS melhor, ainda aberto.
- .15.11 `780df02`: wet ocean outlet exige floor ≥4 abaixo seaLevel; dry fringe não é outlet. .15.12 `68a26a3`: signed shore grading, faixa lateral, banco alto rebaixa, outer bank baixo eleva sem preencher canal; ocean density altura real. **Usuário confirmou margens corrigidas.**
- .15.13 `68e5062`: density carver primeiro, water_near lazy somente carve/cache inclusive None, scans0/1; FPS melhor segundo usuário sem atribuição exclusiva.
- .15.14 `cc77c4f` bump `296960a`: túnel protege bed-3..water+3 fade4, testes limites/cache, margem preservada, CI35111512775 success, gameplay aberto.
- .15.15 `8528960`/fix `f03b5bc`, bump `2ab8466`: remove confluence_target apontando afluente à corda reta enquanto tronco meandrava, terminam downstream X/Z; CI35112785390 success.
- .15.16 `02f1b18` bump `e873159`: valid_lake_outlet impede saída seca lago conectado pela entrada; CI35112964210 success.
- .15.17 `68d1d6b`/`ca3bb60` bump `28f4f48`: initial_lighting_seeded uma vez por residência, retry mesh não apaga luz convergida, unload remove marca e restore reseeda; CI35113608338 success, visual/FPS aberto.
- .15.18 `3b337bf` bump `363d205`: held viewmodel ignora PlacementOrientation/R, transforma/escala fixos; CI35114063244 success, runtime aberto.
- .15.19 `b04ad3f`/`391322c`/`bba6608`/`9044a40` bump `5fd8b56`: rios Y autoritativo nós/lagos endpoints, remove queda artificial0.5, ajusta só internos, testes segmento/lago/região. CI35116468021 success; risco seção elevada se vale abaixo saída, gameplay aberto.
- .15.20 `a1d3fde` bump `754973c`: trace-limit64 inconclusivo não cacheia false nós internos, só destino/dead end comprovado; CI35116860460 success, rios gameplay aberto.
- .15.21 `fcbafa8` bump `d4b5cc7`: Geometry remesh captura halo lighting como Lighting; CI35118558910 success.
- .15.22 `685ab044`/`0edb5515`/`657804c6`/`ffd67c15` bump `fdbf3e8`: Terrain/Fluid não se suprimem nas filas, `enqueue_lighting_change(coord,&world)` inclui fluid center/vizinhos com `has_fluid()` O(1), Fluid valida halo lighting. CI35119214356 falhou sintaxe OnceLock, corrigida .23.
- .15.23 `93573d5` bump `eeaaacb2`: fecha `>` faltantes em OnceLock de block/fluid; CI35120838368 falhou dead-code da tentativa sky mix, corrigido .24.
- .15.24 `1d8d096`/`cb5222e` bump `3b5503e`: remove helper/mistura 35% sky+fog; `update_environment_visuals.after(track_current_biome)` impede bioma do frame anterior; ClearColor exatamente sky_color e teste Bevy App (compilado, não executado). CI35121054358 Clippy+Check success; fog terminal continua sky, arte fog aberta.
- .15.25 `ec38346a97e0d238f184aa32ed0a7c768ef0e3e4` bump `87a27fe920a2d44405cb8eb02c4b4563c4f16d1b`: oito vizinhos `[usize;8]`, fallback raw count+nth, reuso único Vec ponderado, hash/critérios/pesos/ordem iguais, teste. CI35122730177 success, FPS não aferido.
- .15.26 `d492c3685babc705d8a6e248a2e0a6ebf91decaa` bump `25b262065a9a3a995ddd2c6ab427ed0f3f660c55`: `SmallVec<[VerticalDensityDelta;4]>` evita heap até 4, spill acima, teste seis deltas -21, CI35123049209 success.
- .15.27 `bbf9947655b04bc80689394352f8a71e632cb1bf` bump `7fc04dc45b7fbe43bab796662ec4bb5ad8a96387`: um scan `normalized_horizontal_distance` por corpo compartilhado carve+shore, desempate e soma iguais; teste três pontos, CI35123661593 success.
- .15.28 `2f6e5928553f3ed06bfc9720183346b0273e5e32`/`9b0e5444e16e3ced3f97514241d44ed9b329427b` bump `72eb67066781dbe4fb2a7fb8fdfeff373a714966`: notificação remesh Geometry para vizinho renderizado que possui conteúdo na face com novo chunk vazio/ar; Fluid independente; teste original aposentadoria preservado. CI35124362037 success, visual/FPS não confirmado.
- .15.29 `9acdb0ad1393408cd04949f933dcf92f7f5e7f44`/`9f30340ab1ce273a7998c2066f49aa440e82f2ae` bump `75731e9e3a06f61ff78d0466ca62a4f8fb497407`: `mesh/geometry.rs::is_face_exposed` recebe flag de transparência da definição já carregada pelo caller `mesh.rs`; remove lookup redundante source por face com vizinho sólido, sem alterar lookup/semântica vizinho. Teste `face_is_occluded` para opacos/transparentes. **CI35132844975 COMPLETED/SUCCESS Clippy+Check**, FPS não medido.
- **.15.30** `9f43094d7874cdd5a690851826e7f9246367478e` bump `0c1063f2eab07fdf631d2b158ca1d28f3100c57c`: `density_column_profile` filtra corpo d'água distante usando `maximum_horizontal_extent()*LAKE_SHORE_OUTER_DISTANCE` e teste confirma distância excluída possui strength0 e shore0. O máximo irregular 1.42 vezes maior raio e a largura externa da margem 1.18 garantem filtro conservador. Previne trig/ruído em lagos sem influência, preserva geometria/margem .12. **CI35133472565 em execução na última consulta; testes unitários não executados.**

## Subsistemas / investigação direcionada

### Streaming, FPS e iluminação

Preload all-direction+2, corredor frontal+8; fog start~78% end~98% raio conforme readiness; histerese RD4=5/6 RD12=14/16 RD24=26/30; retention R+max(ceil(R/2),10): RD4=14 RD12=22 RD24=36. Geração/remesh async, integração por orçamento. `hydrology/region/density.rs` .26 SmallVec inline4, .27 um scan/normalização por corpo e .30 filtro por envelope máximo que conserva faixa externa; Cloud mesh .10, lazy water .13, seed única .17, biome .25, mesh .29 são otimizações localizadas sem FPS medido; .28 acrescenta Geometry remesh ao chegar vizinho com conteúdo na face existente, medir throughput.

Antes .17 `dispatch_initial_mesh_tasks` reseed a cada retry e `VoxelWorld::rebuild_chunk_light` apagava luz convergida. Seed única e unload remove marca. `lighting_updates.rs` 2ms/4096 voxels; remesh 2 tasks/frame/4 inflight/2 integrações. `ChunkMeshDependencies` inicial compara só conteúdo para time-to-visible; `VoxelWorld::chunk_mesh_revisions` inclui luz. .21 Geometry valida halo lighting; .22 separa Terrain/Fluid filas e refresca fluid quando água, versionando halo. .28: novo chunk vazio já tem `VoxelLight` direto sem que `relax` altere células; antiga notificação só fluid do vizinho, nova também Geometry se há conteúdo na face. `face_lighting` amostra voxels do halo; diagonais/cantos ainda merecem investigação. **Primeira mesh pode publicar luz antiga durante processamento assíncrono**, `ChunkMeshTasks` valida só conteúdo; investigar replay de remesh e snapshots evitando starvation. P0.2/FPS seguem abertos sem gameplay.

### Hidrologia

`generation/density.rs` hydrology density + surface carver; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` abre bed->waterLevel headroom limitado. .14 protege leito/teto fade bounded e water_near lazy. Parede persistindo exige investigar cave connector/túnel/threshold sem mexer margem .12. .26/.27/.30 alteram armazenamento/redundância/culling conservador, não geometria.

`river/selection.rs::build_flow_cache` source radius14 target8 trace64; keep_only_complete_downstream_paths wet ocean/lake; `river.rs::build_river_system` edge margin4, reachability, valid_lake_outlet. `river/path/confluence.rs` bounds+RIVER_MAXIMUM_RADIUS. .15 ancora X/Z, .16 saída seca filtrada, .19 ancora Y e teste fronteira, .20 cache trace-limit. **Falta teste end-to-end de conectividade física por seed:** seleção regional, confluence/lake levels, fonte fluido rasterizada, ocean mouth, fluid entre chunks. `DrainageNetwork::is_wet_ocean` node point sample vs `HydrologyRegion::water_at` macro5×5 interpolado pode divergir — HIPÓTESE não reproduzida. `river_height` threshold continentalness difere teste wet floor; conferir transição mar e níveis de lagos. Não prolongar rios arbitrariamente nem regredir margens.

### Biomas, sky, fog e cor

`biome_field/selection.rs`: suitability temperatura/umidade/continentalness/erosion, pesos elegíveis, vizinho dominante/avoidNear8, fallback raw. .25 otimiza alocações SEM mudar pesos ou diversidade. `surface.rs` Voronoi/Mountains belt+peak; surface_site_spacing maior mínimo regional. Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; pesos Plains1 Wasteland1.30 Witchwood/Enchanted1.25. Wasteland/Plains rolling/mesmos surfaceLayers grass/dirt/stone com tint/structures diferentes: semelhança NÃO mede frequência. Mountain belt `ridge=1-abs(fractal_noise)` favorece ruído perto zero; amostra exploratória seed42 não mede bioma final.

`world/biome.rs::track_current_biome` amostra jogador, compõe surface/hydrology/volume; `CurrentBiomeVisuals` pesa registros. `rendering/environment.rs` interpola skyColor e fogColor por fase/influência, .24 força `after(track_current_biome)` para evitar cor do frame anterior. `sky.rs` aplica SOMENTE sky_color ao ClearColor; teste sky/fog independente, CI compilou teste mas não rodou. `fog/color.rs` e `fog/attachment.rs` usam sky_color no DistanceFog para esconder silhueta; **fog_color artístico é calculado mas não aplicado, P0.7 aberto**. Fundo plano: trocar terminal para fogColor gera costura. Precisa sky gradient com horizonte coerente e zênite skyColor, ou fog custom aproximando fogColor no meio e convergindo skyColor no limite, com testes, preservando HDR/celestials/stars/clouds. Não misturar fogColor no ClearColor.

### Coast, celestial e HUD

Coast identidade usa continentalness raw, oceano físico macro5×5; material rejeita ocean bed acima seaLevel+3, identidade não testa altura; P1 concluído segundo usuário, reabrir só evidência. Sol/lua CelestialBodyDefinition texturizados, nuvens world-space seaLevel e vento independente. P2 concluído segundo usuário: hotbar vazia oculta held, ghost alpha uniforme, dye, HUD histórica. HUD planejada: player inferior esquerdo Yogg'Sara vida50/100 dentro barra, status acima; target acima crosshair bloco esquerda estilo inventory, infos à direita shadow tooltip. R fix .18 gameplay pendente.

---

# Próxima execução

1. Confirmar CI `.30` run35133472565; corrigir qualquer erro/aviso concreto. Não confundir Clippy/Check com cargo test/gameplay.
2. P0.2 investigar iluminação de primeiro mesh/replay remesh, diagonais no halo e notificação .28; medir FPS/custo Geometry/Fluid.
3. P0.4 teste end-to-end seed/coords e água física entre regiões, lake/confluence/ocean outlet; preservar margens .12 e correções .15/.16/.19/.20.
4. P0.3 tunnel walls .14; P0.6 medir seleção final efetiva vs aparência, investigar distribuição/mountain belt sem mudanças cegas.
5. P0.7 sky runtime após .24, transição fog artística sem costura/HDR; R/viewmodel .18 e FPS gameplay.

Todo novo bloco de código sobe `VERSION` e atualiza HANDOFF na mesma sessão.
