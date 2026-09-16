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
10. Luz interativa tem prioridade sobre streaming; publicação atrasada após convergência ainda precisa de revisão. Seed inicial e remesh devem concordar sobre freshness.
11. Sky layers usam seaLevel da dimensão; nuvens permanecem world-space, câmera apenas seleciona tile para reciclagem, vento não é acoplado ao player.
12. Block held/viewmodel deve observar hotbar e manter transform geométrico próprio. Rotação de placement acionada por R não deve distorcer nem reescalar o modelo da mão.
13. Produto: caminhada, círculos e voo máximo sem void, flicker, chunks inteiramente escuros, popping ou fog breathing; não mascarar throughput só com fog.

---

# Estado atual — 2026-09-16

Último commit **de código** publicado: `8528960c6b9f701f63732b2f6aa0b6b8d4da7961` (confluências ancoradas). Bump `2ab846633a046b7ece6098d014db7e52227e73dd`: `VERSION = 0.15.15`. Este commit apenas documenta e não sobe VERSION.

CI: 0.15.3 `5297a69` run `35054771262` success; 0.15.4 fix `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 final `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; 0.15.13 `68e5062` #2086 / run `35109440316` success. **0.15.14** run `35111512775`, Clippy + Check success. **0.15.15:** CI e gameplay ainda não confirmados neste registro.

## Estado de prioridades — confirmação mais recente do usuário

Numeração abaixo referencia a listagem consolidada anterior. **Feedback de runtime 2026-09-16 é autoritativo para status.**

| Item | Status | Próximo passo |
| --- | --- | --- |
| P0.1 FPS / carregamento | **Melhorou, ainda pode melhorar** | Continuar profiling e otimizações mensuráveis; não marcar resolvido |
| P0.2 chunks totalmente escuros + atualização tardia de sombras | **Aberto** | Revisão seed→propagação→remesh→integração, com atenção a revisões distintas |
| P0.3 rios cortam túneis com paredes/barreiras retas | **Aberto em gameplay; correção localizada 0.15.14 sem confirmação visual** | Verificar paredes; se persistirem, composição density e carvers |
| P0.4 rios param no meio do nada, sem conexão lago/oceano | **Aberto em gameplay; patch geométrico localizado 0.15.15, não resolve comprovadamente todo o grafo** | Validar confluências e continuar investigação região/seleção/água física |
| P0.5 margens dos rios fazem corte vertical | **CORRIGIDO conforme confirmação explícita do usuário** após 0.15.12 | Preservar correção |
| P0.6 biomas regionais quase não aparecem | **Aberto: 3 mil blocos, quase só Plains e Mountains** | Diagnosticar seleção espacial/clima/proximidade + overlay macro; não só pesos |
| P0.7 céu e fog não mudam conforme o bioma | **Aberto, usuário reconfirmou** | Restaurar identidade sky/fog em toda cadeia sem reintroduzir silhuetas no frontier |
| P1 anterior (streaming/Coast/nuvens) | **Usuário informou "1 feito"** | Preservar invariantes; reabrir só por nova evidência |
| P2 anterior (hotbar/ghost/dye/HUD histórico) | **Usuário informou "2 feito"** | Preservar; novo bug R é independente |
| NOVO: R / held block | **Aberto: R distorce modelo na mão** | Separar orientação de placement e geometria/Transform do viewmodel |

**Ordem atual:** evitar regressão de FPS; patches 0.15.14 (P0.3) e 0.15.15 (subcausa P0.4) aguardam confirmação visual; investigar restante de P0.4 e P0.2 em blocos independentes; P0.6/P0.7 são críticos; bug R após bloqueadores. P0.5 e itens P1/P2 anteriores concluídos por feedback, sem estender esse status aos novos bugs.

## Histórico dos blocos de código

- 0.14.70 `9302118`: terminal fog alinhada ao sky mas silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial deixou mundo preto/HUD corrompido; 0.14.72 `57305f4`: texturas sol/lua; 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: stack HDR explícita (world Skip, viewmodel final tonemap, UI SDR). 0.15.1 `4aeb119`: frontier fog limitada à coluna ausente mais próxima, AABB real menos 1 chunk, usuário confirmou fim de chunks brotando dentro da fog. 0.15.2 `7b16f8f`: remove recovery temporal. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache `active_columns` por `membership_revision`, recriação de câmera. CI success.
- 0.15.4 `766d970`/`fbe301f`: lighting lane interativa deduplicada preempta streaming, propagação priorizada, remesh publicado após convergência. CI success; sombras ainda atrasam. 0.15.5 `fa84d81`: remesh lighting async valida halo 3×3×3 de revisões distinto das revisões de conteúdo; seed inicial ainda pode divergir.
- 0.15.6 `f8d244c`: surface tunnels sob água passam a fade pelo leito; ainda paredes retas; corrigido matematicamente no 0.15.14 mas não validado visualmente.
- 0.15.7 `5c99616`: mountain belt threshold .86→.90, width .22→.15; mountain peaks spacing760→850, chance .48→.36, radius120–230→110–205; pesos Wasteland1→1.30 e Witchwood/Enchanted1→1.25; oak Plains chance .48→.54; size/avoidNear inalterados. 3k blocos quase só Plains/Mountains: mudar peso não bastou.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens Y=seaLevel+34..48, ainda seguiam câmera. 0.15.9 `b68ecb3`: world-space + wind/tile pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes→1 mesh/nuvem, ~2/3 menos entidades/draw calls. FPS melhor, mas não encerrado.
- 0.15.11 `780df02`: drainage exige oceano fisicamente submerso (bed 4 abaixo seaLevel), segue fringe seco, ocean water_at seco não substitui mouth; CI success, rios soltos persistem.
- 0.15.12 `68a26a3`: shore grading assinado para rios/lagos, altura exata, faixa lateral maior, rebaixa banco alto e eleva outer bank baixo sem preencher canal, ocean density usa altura exata; CI success; margens verticais **confirmadas corrigidas**.
- 0.15.13 `68e5062`: `generation/density.rs` carver antes, `water_near` lazy somente em voxel realmente carvado, cache inclusive None por coluna; teste zero scans sem carve, um scan entre voxels; CI success; FPS melhor segundo usuário, sem atribuição causal exclusiva.
- 0.15.14 `cc77c4f` + bump `296960a`: `surface_carver_water_factor` protege `bed_level - 3` a `water_level + 3`, fades inferior/superior de 4 blocos, volta gradualmente a permitir túnel acima da água. Testes unitários de limites/fades e cache lazy preservados; sem novo scan ou mudança de shoreline; CI #2093 run `35111512775` success, gameplay sem confirmação.
- 0.15.15 `8528960` + bump `2ab8466`: em `hydrology/river.rs`, remove `confluence_target` que redirecionava afluentes para um ponto de interpolação em linha reta entre dois nós embora a calha principal use meandros. Cada afluente agora termina no `downstream` autoritativo que também inicia a aresta seguinte; remove varredura de upstreams/offset/rank redundante, reduz custo por confluência. **Corrige inconsistência horizontal geométrica identificada, mas geometria vertical, seleção regional, destinos lógicos, lagos e água física continuam sob investigação.** CI e gameplay a conferir.

## Subsistemas e diagnósticos a preservar

### Streaming / FPS

Pipeline: `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction +2 chunks, corredor frontal +8 andando, fog nominal start ~78% / end ~98% do raio mas recua conforme readiness. Histerese RD4=5/6, RD12=14/16, RD24=26/30; retention `R + max(ceil(R/2),10)` (RD4=14 RD12=22 RD24=36). Geração/remesh async e integração budgetada. Não subir budgets de iluminação às cegas. Perfil parado/andando: worldgen/lighting/remesh CPU versus cloud transparency GPU. `hydrology/region/density.rs` ainda aloca `Vec<VerticalDensityDelta>` por coluna e verifica lago em carve e shore; otimizar só com evidência e preservar a geometria.

### Iluminação

`world/streaming.rs::dispatch_initial_mesh_tasks` chama `seed_chunk_direct_lighting` e enfileira relaxation, mas pode gerar snapshot/mesh antes de luz entre vizinhos convergir. `lighting_updates.rs` tem budget 2 ms/4096 voxels, remesh até 2 tasks/frame, 4 em voo e 2 integrações/frame. Seed bump no VoxelWorld não implica bump `ChunkRemeshTasks::lighting_revisions`, hoje feito pela atualização dinâmica. Investigar freshness de snapshot inicial, propagation, marcação de remesh e resultado stale; separar ausência de skylight real da mesh escura persistente. Não declarar corrigido por CI.

### Hidrologia — túneis, confluências, rios soltos

`generation/density.rs` combina hydrology density + surface carver. Antes de 0.15.14, proteção de água travava túnel indefinidamente acima de bed-ROOF; 0.15.14 limita proteção à água e roof de 3 com fade 4 acima/abaixo e mantém `water_near` lazy. `density_sampling/hydrology.rs::enforce_hydrology_water_volume` limita água real a bed→water_level e headroom específico. Se barreira persistir: investigar cave connector versus surface tunnel, water_near nas bordas e thresholds, sem modificar margens P0.5 confirmadas.

`river/selection.rs::keep_only_complete_downstream_paths` escolhe trajetos para wet ocean/lake; `river.rs::build_river_system` gera arestas por região com margem `RIVER_EDGE_MARGIN_CELLS=4`, reachability e `selected.channels`; `river/path/confluence.rs` inclui segmentos que cruzam região, `spatial.rs::edge_intersects_region` usa margem máxima. **Diagnóstico 0.15.15:** a antiga `confluence_target` colocava afluente em interpolação retilínea downstream→next; a calha downstream→next é sinuosa; afluente podia terminar isolado. Agora ancora todas as arestas no mesmo nó downstream. Continuar investigando flow cache/seleção entre regiões, clipping, outlets, água efetivamente preenchida e confluências verticais. `river_height` usa continentalness threshold para seaLevel mesmo quando outlet requer oceano fisicamente úmido; verificar descontinuidade sem simplesmente prolongar rios ou mexer nas margens. P0.4 permanece aberto até gameplay.

P0.5 margens verticais: **corrigido pelo usuário** em 0.15.12; não reabrir sem nova evidência.

### Distribuição dos biomas

`biome_field/selection.rs` suitability temperatura/humidade/continentalness/erosion, pesos somente elegíveis; candidatos restritos pelo vizinho dominante e `avoidNear` em 8 vizinhos, fallback raw. `surface.rs` usa sites Voronoi e overlay Mountains belt/peak independente. `surface_site_spacing` deriva do maior raio regional, não de raio individual. Overworld `dimension.json`: Wasteland1.30, Witchwood/Enchanted1.25, Plains1.0, Mountains macro .85. Usuário percorreu 3000 blocos quase só Plains/Mountains; medir por seed seleção, caches e proximidade, não alterar pesos sem causa.

### Cores sky/fog

`rendering/environment.rs::update_environment_visuals` calcula HSI sky/fog com pesos `CurrentBiome` e fases de dia/noite quando entradas mudam; `world/biome.rs::track_current_biome` segue player, `world/biome/identity.rs` mistura surface/hydrology/volume. `rendering/sky.rs` muda ClearColor quando visuals muda; `fog/color.rs` e `fog/attachment.rs` usam `visuals.sky_color` em DistanceFog e ignoram `visuals.fog_color`. Isso identifica defeito da fog mas não explica céu imóvel: rastrear CurrentBiome, pesos, definições, registro, fase e câmeras. Trocar diretamente terminal DistanceFog por fog_color pode reintroduzir silhouette/popping; combinar identidade local e horizonte terminal igual ao céu.

### Coast / celestial / nuvens / HUD histórico

Coast: identidade hydrology usa continentalness raw `BiomeField::climate_at`, oceano físico usa macro sample interpolado 5×5 em `HydrologyRegion::macro_sample_at`; material rejeita ocean bed acima seaLevel+3 mas identidade Coast não considera altura. P1 agregado confirmado concluído; preservar diagnóstico sem reabrir à toa. `rendering/celestial.rs` lê textura de `CelestialBodyDefinition`; Overworld tem sun/moon. Nuvens world-space Y relativo a seaLevel, tile recycling, vento desacoplado do player.

P2 agregado concluído conforme usuário: held block observa hotbar e esconde slot vazio, ghost transparência uniforme, dye, HUD: jogador canto inferior esquerdo, Yogg'Sara, vida 50/100 dentro da barra, status acima, target HUD acima da crosshair com bloco à esquerda estilo inventory e texto à direita tooltip shadow. Bug independente: R distorce bloco na mão; rastrear hotkey, orientação placement e Transform/mesh/viewmodel/UV sem mutações acumuladas.

---

# Próxima execução

1. P0.4: conferir CI de 0.15.15, investigar seleção regional e endpoints físicos em testes dirigidos; 0.15.15 ataca apenas uma subcausa, não fechar P0.4 por esse commit.
2. P0.2: sincronizar seed, convergência da iluminação e revisões de remesh, evitar custo na main thread; FPS continua indicador obrigatório.
3. P0.3: 0.15.14 já publicado e CI aprovado, confirmar runtime; se parede persistir seguir composição density/carvers sem regressar margens.
4. P0.6/P0.7: distribuição real dos biomas e pipeline de cores ambiental, em commits próprios.
5. Bug R/viewmodel e novas otimizações de FPS baseadas em diagnóstico; preservar P0.5, P1/P2 conforme feedback.

Todo bloco de código sobe `VERSION` e tem HANDOFF atualizado na mesma sessão. Feedback de usuário reordena prioridades imediatamente.
