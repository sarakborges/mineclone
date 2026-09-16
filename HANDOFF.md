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
4. Mesh/remesh validam conteúdo/iluminação separadamente. `ChunkRemeshTasks::lighting_revisions` difere de `VoxelWorld::chunk_mesh_revisions`. Geometry, Lighting e Fluid remesh incorporam luz nos vértices e validam halo3×3×3; Terrain e Fluid são meshes distintas, uma fila não pode consumir trabalho da outra. Primeiro mesh preserva time-to-visible; investigar sem starvation.
5. Streaming separa visible/preload/retenção; retirement não implica unload imediato. Histerese, sem revelar preload distante. Luz interativa prioritária, revisão publicada após convergência. Iluminação direta inicial seed uma vez por residência, nunca por retry de mesh.
6. Stack HDR: world camera order0 Skip, viewmodel HDR order1 tonemap final, UI SDR order2. Preservar mundo/mão/HUD/overlays.
7. Fog readiness frontier até imóvel; esconder void/silhuetas sem perder paleta artística por bioma. Enquanto fundo for `ClearColor` plano, terminal `DistanceFog.color` deve coincidir com céu; não misturar paleta de fog em todo sky para disfarçar costura. Nuvens world-space, vento independente, seaLevel por dimensão.
8. Hot paths change-driven, sem alocações/scans/dirty writes no idle. Held viewmodel observa hotbar/visual/tint e não `PlacementOrientation`; R só afeta bloco colocado.
9. Rios incidentes compartilham X/Z/Y do nó drenagem/lago; terrain adjustment não altera endpoints; regiões vizinhas reproduzem mesmo segmento. Margens corrigidas em 0.15.12 são regressão proibida.
10. Objetivo caminhando/circulando/voo máximo: sem void, popping/flicker, chunks pretos ou fog breathing; não esconder throughput insuficiente apenas via fog.
11. **Céu:** bioma é dono de sua paleta `skyColor` por fase. `CurrentBiome`/pesos + relógio determinam `EnvironmentVisualState.sky_color`; `SkyPlugin` escreve exatamente essa cor no `ClearColor`. `fogColor` é paleta independente. Ordenar rastreamento biome antes da atualização visual no mesmo frame; jamais misturar porcentagem fixa de fogColor no céu.

---

# Estado em 2026-09-16

**Último código:** `93573d5a692f9029a55113e677e761e54fda41a2` (OnceLock), `1d8d096f688824c739934efe003bd312b6e660b1` (environment sem mistura e ordenado após biome), `cb5222e1cf5bfdf90296e57e0038f08abf2761dd` (teste sky). Bumps `.23` `eeaaacb2d2c838df64485cb241038caf06e26e54`, `.24` `3b5503e450b0836c61fd4b107ec2f5625621224c`. **VERSION = 0.15.24**. Commits documentais posteriores não exigem bump; consultar HEAD/VERSION antes de nova escrita.

**CI Clippy + Check APROVADO até 0.15.21, e novamente na 0.15.24:** .3 run35054771262, .4 #2064, .5 #2066, .6 #2068, .7 #2070, .8 #2075, .9 #2078, .10 #2080, .11 #2082, .12 #2084, .13 run35109440316, .14 run35111512775, .15 run35112785390, .16 run35112964210, .17 run35113608338, .18 run35114063244, .19 run35116468021, .20 run35116860460, .21 run35118558910. **.22 run35119214356 FALHOU Clippy** por dois erros de sintaxe `OnceLock<Arc<[Option<...>]>>` em `src/voxel/chunk.rs`, Check pulado; corrigidos .23. **.23 run35120838368 FALHOU Clippy** por dois warnings `-D dead-code` em `src/rendering/environment.rs`: `HORIZON_FOG_COLOR_WEIGHT` e `horizon_color()` não eram usados; Check pulado. Ambos REMOVIDOS na .24. **.24 run35121054358 COMPLETED/SUCCESS, Clippy success e Check success**, confirmado pelo job `104878634447`. CI não executa unit tests (`cargo test`) nem confirma aparência em gameplay. Não voltar a relatar .22/.23 como pendentes ou aprovados.

## Prioridades — feedback 2026-09-16

| Item | Estado | Próximo passo |
| --- | --- | --- |
| P0.1 FPS/carregamento | Melhorou, ainda aberto | Profiling/otimizações medidas, atenção ao custo Fluid remesh .22 |
| P0.2 chunks escuros/sombras tardias | Aberto visualmente; seed única .17, Geometry freshness .21, Fluid freshness .22 compilam no CI .24 | First mesh stale/remesh, FPS e gameplay |
| P0.3 rios cortam túneis com parede reta | Aberto visualmente; carver .14 CI aprovado | Gameplay e density/carvers sem regredir margens |
| P0.4 rios sem ligação lago/oceano | Aberto visualmente; patches .15/.16/.19/.20 CI aprovados | End-to-end por seed/coords, fluido físico entre chunks |
| P0.5 margens verticais de rios | **Corrigidas segundo usuário** após .12 | Preservar |
| P0.6 biomas regionais ausentes | ~3000 blocos quase só Plains/Mountains | Medir seleção efetiva vs semelhança visual |
| P0.7 sky/fog sem cor por bioma | Sky sincronização .24 **CI aprovado, gameplay pendente**. Fog artístico ainda não aplicado efetivamente devido fundo plano | Verificar sky no jogo, desenhar transição fog que convirja sky sem costura, sem misturar a cor do céu |
| P1 antigo streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar |
| P2 antigo hotbar/ghost/dye/HUD | Usuário informou `2 feito` | Preservar |
| Bug R distorce held block | .18 CI aprovado, sem confirmação visual | Testar R em bloco orientável e escala |

Prioridade preservar FPS e margens P0.5; P0.2/P0.4 depois P0.3/P0.6/P0.7 e R. Não encerrar bug visual só com CI.

## Histórico de código e decisões

- 0.14.70 `9302118`: fog terminal alinhada sky, silhuetas persistiram; .71 `b028fbe`: HDR parcial quebrou mundo/HUD; .72 `57305f4`: sol/lua texturizados; .73 `f864669`: revert HDR parcial.
- .15.0 `1b16bee`: world HDR Skip, viewmodel tonemap final, UI SDR. .15.1 `4aeb119`: fog frontier/AABB menos1 chunk, usuário confirmou fim chunks brotando na fog. .15.2 `7b16f8f`: remove temporal recovery. .15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns por membership_revision, recriação câmera; CI verde.
- .15.4 `766d970`/`fbe301f`: lane luz interativa preempta streaming/remesh após convergência; sombras tardias. .15.5 `fa84d81`: remesh async halo3×3×3, tracker separado.
- .15.6 `f8d244c`: túnel proteção sob água bed fade, barreira persistiu. .15.7 `5c99616`: mountain belt threshold .86->.90 width .22->.15, peaks spacing760->850 chance .48->.36 radius120–230->110–205; Wasteland weight1->1.30, Witchwood/Enchanted1->1.25, Plains oak chance .48->.54; size/avoidNear não alterados, biomas ainda visualmente ausentes.
- .15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda camera bound. .15.9 `b68ecb3`: world-space wind/tile pool180. .15.10 `eadb6ea`: três cubos transparentes -> uma mesh/nuvem (~2/3 entidades/draw calls menos); FPS melhor, ainda aberto.
- .15.11 `780df02`: wet ocean outlet exige floor ≥4 abaixo seaLevel; dry fringe não é outlet. .15.12 `68a26a3`: signed shore grading, faixa lateral, banco alto rebaixa, outer bank baixo eleva sem preencher canal; ocean density altura real. **Usuário confirmou margens corrigidas.**
- .15.13 `68e5062`: density carver primeiro, water_near lazy somente carve/cache inclusive None, scans0/1; FPS melhor segundo usuário sem atribuição exclusiva.
- .15.14 `cc77c4f` bump `296960a`: túnel protege bed-3..water+3 fade4, testes limites/cache, margem preservada, CI35111512775 success, gameplay aberto.
- .15.15 `8528960`/fix `f03b5bc`, bump `2ab8466`: remove confluence_target apontando afluente à corda reta enquanto tronco meandrava, terminam downstream X/Z, ranking upstream removido. CI35112785390 success.
- .15.16 `02f1b18` bump `e873159`: valid_lake_outlet impede saída seca lago conectado pela entrada, testes, CI35112964210 success.
- .15.17 `68d1d6b`/`ca3bb60` bump `28f4f48`: initial_lighting_seeded uma vez por residência, retry mesh não apaga luz convergida, unload remove marca e restore reseeda, testes; CI35113608338 success, visual/FPS aberto.
- .15.18 `3b337bf` bump `363d205`: held viewmodel ignora PlacementOrientation/R, transforma/escala fixos, hotbar/tint continuam, CI35114063244 success, runtime aberto.
- .15.19 `b04ad3f`/`391322c`/`bba6608`/`9044a40` bump `5fd8b56`: rios Y autoritativo nós/lagos endpoints, remove queda artificial0.5, ajusta só internos, testes segmento/lago/região. CI35116468021 success; risco seção elevada se vale abaixo saída, gameplay aberto.
- .15.20 `a1d3fde` bump `754973c`: trace-limit64 inconclusivo não cacheia false nós internos, só destino/dead end comprovado, testes; CI35116860460 success, rios gameplay aberto.
- .15.21 `fcbafa8` bump `d4b5cc7`: Geometry remesh captura halo lighting como Lighting; teste luz vizinho, CI35118558910 success.
- **.15.22** `685ab044`/`0edb5515`/`657804c6`/`ffd67c15` bump `fdbf3e8`: Terrain/Fluid não se suprimem nas filas enqueue/pop/coalesce; `enqueue_lighting_change(coord,&world)` inclui fluid center/vizinhos com `has_fluid()` O(1); Fluid também valida halo lighting. Testes filas/revisões; CI35119214356 FALHOU sintaxe OnceLock, corrigida .23. Gameplay/FPS abertos.
- **.15.23** `93573d5a692f9029a55113e677e761e54fda41a2` bump `eeaaacb2`: fecha `>` faltantes em `OnceLock<Arc<[Option<VoxelCell>]>>`/Fluid, compare confirmou só duas linhas; CI35120838368 FALHOU em dois avisos dead code do horizon helper introduzido antes e ainda não usado; Check pulou. Ambos removidos na .24.
- **.15.24** `1d8d096f688824c739934efe003bd312b6e660b1`/`cb5222e1cf5bfdf90296e57e0038f08abf2761dd` bump `3b5503e`: remove helper/mistura arbitrária 35% sky+fog e teste velho, usuário determinou sky exclusivamente por bioma. `update_environment_visuals.after(track_current_biome)` Update garante sample bioma mesmo frame; sky.rs mantém ClearColor EXATO `sky_color` e teste Bevy App alterando sky/fog separadamente confirma intenção em código (teste **não executado** no CI). **CI35121054358 PASSOU Clippy + Check**; gameplay sky/fog aberto. Fog terminal ainda sky até implementar transição sem seam.

## Subsistemas / investigação direcionada

### Streaming, FPS e iluminação

Preload all-direction+2, corredor frontal+8; fog start~78% end~98% raio conforme readiness; histerese RD4=5/6 RD12=14/16 RD24=26/30; retention R+max(ceil(R/2),10): RD4=14 RD12=22 RD24=36. Geração/remesh async, integração por orçamento. `hydrology/region/density.rs` cria Vec<VerticalDensityDelta> por coluna e testa lagos carve+shore; otimizar só com evidência sem alterar geometria. Cloud mesh .10, lazy water .13, seed única .17 são ganhos localizados; medir FPS parado/andando CPU worldgen/lighting/remesh vs GPU clouds.

Antes .17 `dispatch_initial_mesh_tasks` reseed a cada retry e `VoxelWorld::rebuild_chunk_light` apagava luz convergida. Seed única e unload remove marca. `lighting_updates.rs` 2ms/4096 voxels, remesh 2 tasks/frame, 4 inflight, 2 integrações. `ChunkMeshDependencies` inicial compara só conteúdo para time-to-visible; `VoxelWorld::chunk_mesh_revisions` inclui luz. .21 Geometry valida halo lighting e não pode sobrescrever Lighting mais nova; .22 separa fila Terrain/Fluid e refresca fluid apenas quando existe água, versionando halo. **Primeira mesh ainda pode publicar luz velha**, pois ChunkMeshTasks compara só conteúdo. Candidata: publicar primeira mesh rápido, depois relight se luz mudou desde captura (sem rejeitar primeira mesh ou reseed), avaliar custo. P0.2/FPS abertos.

### Hidrologia

`generation/density.rs` hydrology density + surface carver; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` abre bed->waterLevel headroom limitado. .14 protege leito/teto fade bounded e water_near lazy. Parede persistindo exige investigar cave connector/túnel/threshold sem mexer margem .12.

`river/selection.rs::build_flow_cache` source radius14 target8 trace64; keep_only_complete_downstream_paths wet ocean/lake; `river.rs::build_river_system` edge margin4, reachability, valid_lake_outlet. `river/path/confluence.rs` bounds+RIVER_MAXIMUM_RADIUS. .15 ancora X/Z, .16 saída seca filtrada, .19 ancora Y e teste fronteira, .20 cache trace-limit. **Falta teste end-to-end de conectividade física por seed:** seleção regional, confluence/lake levels, fonte fluido rasterizada, ocean mouth, fluid entre chunks. `DrainageNetwork::is_wet_ocean` node point sample vs `HydrologyRegion::water_at` macro5×5 interpolado pode divergir — HIPÓTESE não reproduzida. `river_height` threshold continentalness difere teste wet floor, conferir transição mar. Não prolongar rios arbitrariamente nem regredir margens.

### Biomas, sky, fog e cor

`biome_field/selection.rs`: suitability temperatura/umidade/continentalness/erosion, pesos só elegíveis, vizinho dominante/avoidNear8, fallback raw. `surface.rs` Voronoi/Mountains belt+peak; surface_site_spacing maior mínimo regional. Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; pesos Plains1 Wasteland1.30 Witchwood/Enchanted1.25. Wasteland/Plains rolling/mesmos surfaceLayers grass/dirt/stone com tint/structures diferentes: semelhança NÃO mede frequência. Mountain belt `ridge=1-abs(fractal_noise)` favorece ruído perto zero; amostra exploratória seed42 não mede bioma final.

`world/biome.rs::track_current_biome` amostra jogador, compõe surface/hydrology/volume; `CurrentBiomeVisuals` pesa registros. `rendering/environment.rs` interpola skyColor e fogColor por fase/influência, .24 força `after(track_current_biome)` para evitar cor do frame anterior. `sky.rs` aplica SOMENTE sky_color ao ClearColor; teste sky/fog independente, CI compilou teste mas não rodou. `fog/color.rs` e `fog/attachment.rs` usam sky_color no DistanceFog para esconder silhueta; **fog_color artístico é calculado mas não aplicado, P0.7 aberto**. Fundo plano: trocar terminal para fogColor gera costura. Precisa sky gradient com horizonte coerente e zênite skyColor, ou fog custom com aproximação fogColor no meio e convergência skyColor no limite, com testes, mantendo HDR, celestials/stars/clouds. Não misturar 35% fogColor no ClearColor (tentativa abortada .24 por exigência usuário).

### Coast, celestial e HUD

Coast identidade usa continentalness raw, oceano físico macro5×5; material rejeita ocean bed acima seaLevel+3, identidade não testa altura; P1 concluído segundo usuário, reabrir só evidência. Sol/lua CelestialBodyDefinition texturizados, nuvens world-space seaLevel e vento independente. P2 concluído segundo usuário: hotbar vazia oculta held, ghost alpha uniforme, dye, HUD histórica. HUD planejada: player inferior esquerdo Yogg'Sara vida50/100 dentro barra, status acima; target acima crosshair bloco esquerda estilo inventory, infos à direita shadow tooltip. R fix .18 gameplay pendente.

---

# Próxima execução

1. CI mais recente .24 run35121054358 já confirmado **Clippy + Check verde**; .22 e .23 falharam com causas e reparos rastreados. Não repetir consulta sem motivo. `cargo test` não é executado.
2. P0.7 sky: conferir transição entre biomas/relógio após ordem .24. Fog artística requer gradiente ou convergência terminal com sky sem misturar sky global e sem regredir fog readiness/HDR. Não dizer resolvido sem gameplay.
3. P0.2: primeira mesh versus iluminação revisada, relight posterior sem starvation, conferir .17/.21/.22 e medir FPS/throughput Fluid.
4. P0.4: end-to-end seed/coords regiões vizinhas, edge Y e fluido por chunk, lake/confluence/ocean outlet; preservar .12/.15/.16/.19/.20.
5. P0.3 tunnel walls .14; P0.6 medir seleção final biomas/mountain belt; R/viewmodel .18 e FPS gameplay.

Todo novo bloco de código sobe `VERSION` e atualiza HANDOFF na mesma sessão.
