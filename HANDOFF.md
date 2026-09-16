# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica persistente; exports e anexos são snapshots derivados.

- Trabalhar diretamente em `develop`, salvo pedido explícito de branch. Antes de escrever, verificar HEAD, `VERSION` e arquivos reais envolvidos.
- Commits pequenos e coerentes, sem misturar causas independentes. Cada bloco de alteração de código sobe `VERSION` (PATCH para fix/refactor compatível, MINOR para feature compatível, MAJOR para breaking); documentação isolada não sobe versão.
- Atualizar este HANDOFF na mesma sessão de qualquer alteração material de código, arquitetura, roadmap, processo ou feedback de runtime. **Não deixar o handoff defasado.**
- Feedback de gameplay/erro/warning tem prioridade sobre roadmap. `go`/`continua` = executar sem confirmação desnecessária.
- Corrigir warnings dos blocos tocados. CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`. Não repetir que faltou cargo local ou que está esperando `cargo run`; testes manuais cargo só sob pedido explícito, fmt não é gate.
- Compilação não comprova correção visual, hidrológica nem FPS; separar causa identificada, patch publicado e confirmação em gameplay.
- Sem geração de imagens sem pedido explícito. Root `VERSION` é operacional; versão de `Cargo.toml` permanece intencionalmente antiga (0.10.16).

## Invariantes arquiteturais

1. Cada fato de gameplay tem owner autoritativo; dados e decisões devem ser consistentes entre geração, rendering e HUD.
2. Geração, mesh e remesh pesados fora da main thread; integração orçamentada. Resultados async revisionados; stale descartado e reagendado.
3. Filas deduplicadas canônicas: `DeduplicatedQueue<T>` e `VoxelUpdateQueue`; budget canônico `FrameWorkBudget`.
4. Prioridade de chunks atravessa fronteiras async: visible mandatory vence warm/preload; preempção devolve trabalho não crítico à fila.
5. Mesh async valida revisão de conteúdo e de iluminação separadamente; halo recém-disponível pode ser corrigido por boundary remesh. `ChunkRemeshTasks::lighting_revisions` é tracker próprio, distinto de `VoxelWorld::chunk_mesh_revisions`.
6. Streaming separa visible, preload e retenção; retirement não é descarte imediato. Visibilidade tem histerese e não expõe chunks de preload distantes.
7. Stack HDR: world camera order 0 Skip -> viewmodel HDR order 1 tonemap final -> UI SDR order 2; nunca alterar parcialmente sem validar mundo, mão, HUD, overlays.
8. Fog deve acompanhar readiness do frontier mesmo parado; terminal fog e fundo do céu precisam evitar silhuetas/void. Cores por bioma não podem ser descartadas para isso.
9. Hot paths devem ser change-driven; não fazer scans ou dirty writes quando nada mudou.
10. Luz interativa tem prioridade sobre streaming; publicação atrasada após convergência ainda precisa de revisão. Seed inicial e remesh devem concordar sobre freshness. Inicializar luz direta uma vez por residência de chunk, não a cada retry de mesh.
11. Sky layers usam seaLevel da dimensão; nuvens permanecem world-space, câmera apenas seleciona tile para reciclagem, vento não é acoplado ao player.
12. Held/viewmodel observa hotbar, definições visuais e posição para tint; não depende de `PlacementOrientation` nem muda geometria/Transform ao pressionar R. Rotação de placement deve afetar bloco colocado, não mão.
13. Produto: caminhada, círculos e voo máximo sem void, flicker, chunks inteiramente escuros, popping ou fog breathing; não mascarar throughput só com fog.

---

# Estado atual — 2026-09-16

Último commit **de código** `3b337bfa901706413f19c819d3058577a74afc33` (isola rotação do viewmodel); bump `363d2056dc9244eb7b0cbb19db1c87c231c587ee`; `VERSION = 0.15.18`. Este handoff é documentação, sem bump.

**CI confirmado até 0.15.18:** 0.15.3 `5297a69` run35054771262 success; 0.15.4 `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; 0.15.13 `68e5062` #2086/run35109440316 success; 0.15.14 `a6f770f` run35111512775 success; 0.15.15 `f03b5bc` run35112785390 success; 0.15.16 `e873159` run35112964210 success; **0.15.17 `1fa516d` run35113608338 success; 0.15.18 `2ec5693` run35114063244 success (Clippy + Check).** Runs CI validam build/warnings, não executam `cargo test` nem comprovam gameplay.

## Prioridades — feedback do usuário em 2026-09-16

| Item | Status | Próximo passo |
| --- | --- | --- |
| P0.1 FPS / carregamento | Melhorou, ainda pode melhorar | Profiling e otimizações mensuráveis, não marcar resolvido |
| P0.2 chunks escuros + sombras tardias | Aberto em gameplay; código 0.15.17 passou CI | Confirmar seed única, convergência, revisões de halo e remesh |
| P0.3 rios cortam túneis com paredes retas | Aberto em gameplay; patch matemático 0.15.14 passou CI | Se persistir, composição density/carvers |
| P0.4 rios sem conexão lago/oceano | Aberto em gameplay; subcausas 0.15.15/16 passaram CI | Continuidade entre regiões, confluência vertical, água física |
| P0.5 margens verticais dos rios | CORRIGIDO segundo usuário após 0.15.12 | Preservar |
| P0.6 biomas regionais ausentes | Aberto: ~3 mil blocos quase Plains/Mountains | Diagnosticar distribuição por seed, climate, seleção, overlay e apresentação |
| P0.7 sky/fog sem cor do bioma | Aberto, reconfirmado | Pipeline de cores/horizonte sem silhuetas |
| P1 anterior streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar, reabrir só com evidência |
| P2 anterior hotbar/ghost/dye/HUD histórico | Usuário informou `2 feito` | Preservar, bug R separado |
| NOVO R distorce held block | Patch 0.15.18 passou CI, **aguarda confirmação visual** | Testar R em blocos orientáveis e escala fixa |

**Ordem:** preservar FPS e P0.5; validar gameplay 0.15.14–18, investigar restante P0.4/P0.2 com testes/feedback; P0.6/P0.7 críticos; bug R corrigido em código, aberto em runtime até confirmação. Não marcar problemas visualmente resolvidos por CI.

## Histórico de código

- 0.14.70 `9302118`: terminal fog alinhada ao sky, silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial deixou mundo preto/HUD corrompido. 0.14.72 `57305f4`: sol/lua texturizados. 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: world HDR Skip, viewmodel final tonemap, UI SDR. 0.15.1 `4aeb119`: fog frontier até a primeira coluna ausente, AABB menos 1 chunk; usuário confirmou fim dos chunks brotando dentro da fog. 0.15.2 `7b16f8f`: remove recovery temporal. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns por membership_revision, recriação de câmera; CI success.
- 0.15.4 `766d970`/`fbe301f`: lane de iluminação interativa preempta streaming, remesh após convergência; sombras ainda atrasam. 0.15.5 `fa84d81`: lighting remesh async valida halo3×3×3 e tracker independente da revisão de conteúdo.
- 0.15.6 `f8d244c`: túneis sob água fade pelo leito, usuário ainda viu barreiras. 0.15.7 `5c99616`: mountain belt threshold .86→.90 width .22→.15; peaks spacing760→850 chance .48→.36 radius120–230→110–205; Wasteland wt1→1.30, Witchwood/Enchanted1→1.25, oak Plains chance .48→.54. Size/avoidNear não mudaram, 3k blocos quase Plains/Mountains.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda seguiam câmera. 0.15.9 `b68ecb3`: world-space com wind/tile pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes→1 mesh/nuvem (~2/3 entidades/draw calls a menos). FPS melhor, ainda aberto.
- 0.15.11 `780df02`: drenagem exige ocean bed pelo menos 4 abaixo seaLevel, segue fringe seco, ocean seco não substitui river mouth; rios soltos persistem. 0.15.12 `68a26a3`: shore grading assinado, amostragem exata, faixa lateral maior, rebaixa banco alto/eleva outer bank baixo sem preencher canal, ocean density usa altura exata; **margens corrigidas segundo usuário**.
- 0.15.13 `68e5062`: `generation/density.rs` carver antes, `water_near` lazy apenas com carve, cache inclusive None por coluna; testes 0/1 scan, FPS melhor segundo usuário sem atribuição exclusiva.
- 0.15.14 `cc77c4f` + bump `296960a`: `surface_carver_water_factor` protege bed-3 a water+3 com fade4 superior/inferior. Testes limites/fades/cache, sem mudar margem. CI35111512775 success; gameplay aberto.
- 0.15.15 `8528960` + fix `f03b5bc` + bump `2ab8466`: remove `confluence_target` que desviava afluentes à reta entre nós enquanto canal principal meandra; todos terminam no downstream autoritativo. Remove ranking/varredura upstream redundante, fix de import DrainageNode. CI35112785390 success; resolve subcausa horizontal, vertical/regiões ainda abertas.
- 0.15.16 `02f1b18` + bump `e873159`: `valid_lake_outlet` impede saída seca de lago conectado por ENTRADA; downstream precisa oceano molhado, lago conectado ou percurso válido. Preserva entrada/lago terminal; usa destination_cache e teste dead end versus próximo lago. CI35112964210 success; ainda pode existir rio desconectado entre regiões/água.
- 0.15.17 `68d1d6b` + `ca3bb60` + bump `28f4f48`: `ChunkStreamingState::initial_lighting_seeded` marca residência. Primeira passagem enfileira fluid frontier, seed direta e relax; retries mesh stale/preempted não apagam luz propagada nem recriam revisões, unload esquece marcador após archive para novo seed no restore. Teste once/retry/forget/reseed. CI35113608338 success; P0.2/FPS aguardam gameplay.
- 0.15.18 `3b337bf` + bump `363d205`: `player/viewmodel/model.rs` remove `PlacementOrientation` da seleção do held block e gatilho orientation_changed, não escreve rotação do Transform no sync; rotação fixa de apresentação = `base_viewmodel_transform().rotation.inverse()`, escala constante HELD_BLOCK_SCALE; continua observando hotbar, definições e tint. Teste unitário de transformação fixa. CI35114063244 success; R aguarda confirmação visual.

## Subsistemas e diagnósticos

### Streaming / FPS

Pipeline `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction+2 e corredor frontal+8 andando. Fog start~78% end~98% raio recua conforme readiness. Histerese RD4=5/6 RD12=14/16 RD24=26/30, retention R+max(ceil(R/2),10), exemplos RD4=14 RD12=22 RD24=36. Geração/remesh async, integração budgetada; não elevar budgets de luz cegamente. Profiling parado/andando para CPU worldgen/lighting/remesh versus GPU cloud transparency. `hydrology/region/density.rs` constrói Vec<VerticalDensityDelta> por coluna e testa lago em carve+shore; só otimizar por evidência preservando geometria. 0.15.10 nuvem, 0.15.13 lazy water, 0.15.17 seed única são melhorias localizadas; FPS ainda aberto.

### Iluminação

Antes, `dispatch_initial_mesh_tasks` reseed direto a cada mesh retry stale, `VoxelWorld::rebuild_chunk_light` apagava luz convergida e invalidava snapshot via revision. 0.15.17 marca seed única por residência, remove no unload; CI aprovado. `lighting_updates.rs` budget2ms/4096 voxels; remesh 2 tasks/frame, 4 in-flight e 2 integrações/frame. Seed bump no VoxelWorld não necessariamente bump em `ChunkRemeshTasks::lighting_revisions`, alimentado por atualizações dinâmicas. Investigar atualização de revisões, halo, propagação skylight, mesh persistente escura versus chunk de fato escuro. Não encerrar P0.2 sem gameplay.

### Hidrologia

`generation/density.rs` compõe hydrology density+surface carver. Antes 0.15.14 proteção vertical ilimitada acima bed-ROOF; agora limita água/roof com fades e water_near lazy. `density_sampling/hydrology.rs::enforce_hydrology_water_volume` limita água bed→waterLevel e headroom. Se barreira persistir, investigar cave connector vs surface tunnel, water_near borda e thresholds; não regredir P0.5.

`river/selection.rs::keep_only_complete_downstream_paths` escolhe caminhos a wet ocean/lake; `river.rs::build_river_system` cria edges por região com margem4, reachability e channels, `river/path/confluence.rs` insere segmentos que cruzam região, `spatial.rs` margem raio máximo. 0.15.15 ancora horizontalmente afluentes em nó comum; 0.15.16 valida saída seca de lago. Investigar flow cache, seleção entre regiões, clipping, convergência vertical no mesmo nó e preenchimento físico. `river_height` usa continentalness threshold para seaLevel enquanto outlet exige oceano submerso: investigar descontinuidade sem prolongar rios arbitrariamente. P0.4 aberto; margens P0.5 confirmadas corrigidas.

### Biomas e cores

`biome_field/selection.rs`: suitability temperatura/humidade/continentalness/erosion, pesos apenas elegíveis, restrição por vizinho dominante e avoidNear em 8 vizinhos, fallback raw. `surface.rs`: Voronoi + overlay independente Mountains belt/peak. `surface_site_spacing` deriva do maior mínimo regional, não de raio individual. Overworld: Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; Wasteland wt1.30, Witchwood/Enchanted1.25, Plains1.0. Wasteland e Plains têm terrain rolling e mesmos surfaceLayers grass/dirt/stone, embora visual/tint/estruturas distintos; isso pode dificultar identificar bioma, mas **não prova frequência real**. Não mudar peso nem alegar distribuição corrigida sem medição por seed e UI de bioma.

`rendering/environment.rs::update_environment_visuals` calcula HSI sky_color/fog_color por CurrentBiome e fase do dia se inputs mudam; `world/biome.rs::track_current_biome` acompanha player; `world/biome/identity.rs` combina surface/hydrology/volume; `rendering/sky.rs` escreve ClearColor em PostUpdate se visuals changed. `fog/color.rs` e `fog/attachment.rs` deliberadamente usam sky_color para DistanceFog, ignorando fog_color artístico, a fim de não expor silhuetas contra céu vazio. **Fato confirmado:** identidade artística de fog não chega à DistanceFog; céu inalterado requer investigar atualizações/identidades/visibilidade antes de substituir cores. DistanceFog tem cor terminal única, trocar diretamente por fog_color quebraria a igualdade com sky: projetar transição compatível ou render específico, não patch cego.

### Coast, celestial, HUD e históricos

Coast identidade usa continentalness raw `BiomeField::climate_at` enquanto oceano físico usa macro sample interpolado 5×5 `HydrologyRegion::macro_sample_at`; material rejeita ocean bed acima seaLevel+3 sem que identidade Coast verifique altura. P1 foi informado concluído, reabrir só evidência nova. `rendering/celestial.rs` lê textura CelestialBodyDefinition; Overworld sun/moon. Nuvens world-space Y seaLevel relativo, vento desacoplado do player, reciclagem de tile.

P2 histórico usuário marcou concluído: held block observa hotbar, slot vazio oculta, ghost alpha uniforme por bloco, dye, HUD planejada player inferior esquerdo Yogg'Sara vida50/100 dentro da barra e status acima; target HUD acima crosshair, bloco esquerda estilo slot inventory e texto direita tooltip shadow. Bug novo R recebeu fix isolado 0.15.18 CI success, validação runtime pendente. Nunca reaplicar placement orientation à mão.

---

# Próxima execução

1. **CI 0.15.14–18 Clippy+Check aprovado; gameplay permanece pendente.** Testes unitários foram acrescentados, mas CI não roda `cargo test`, portanto não registrar testes unitários como executados.
2. P0.4: testes determinísticos de continuidade entre seleção, edges e água em regiões vizinhas, outlets físicos e confluência vertical. Preservar margens 0.15.12, confluência horizontal 0.15.15, filtro lago 0.15.16.
3. P0.2: feedback de runtime de seed única 0.15.17, halo lighting revisions, propagação skylight, remesh e FPS antes/depois.
4. P0.3: feedback visual do patch 0.15.14. Persistindo parede, composição density/carvers sem regredir margens.
5. P0.6: medir seleção real por seed separando bioma selecionado de semelhança visual; P0.7: rastrear CurrentBiome→EnvironmentVisuals→ClearColor e compor fog artístico mantendo terminal compatível com céu.
6. Confirmar R/viewmodel visual após 0.15.18 e FPS; preservar P0.5 e P1/P2 históricos.

Todo bloco de código incrementa `VERSION` e atualiza este HANDOFF na mesma sessão. Feedback do usuário reordena prioridades imediatamente.
