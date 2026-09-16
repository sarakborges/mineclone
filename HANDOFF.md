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
12. Held/viewmodel observa **somente hotbar, definições visuais e posição para tint**; não depende de `PlacementOrientation` e não muda Transform geométrico quando R altera placement. Rotação de placement deve atingir o bloco colocado, nunca distorcer mão.
13. Produto: caminhada, círculos e voo máximo sem void, flicker, chunks inteiramente escuros, popping ou fog breathing; não mascarar throughput só com fog.

---

# Estado atual — 2026-09-16

Último commit **de código**: `3b337bfa901706413f19c819d3058577a74afc33` (isola rotação de viewmodel); bump `363d2056dc9244eb7b0cbb19db1c87c231c587ee`; `VERSION = 0.15.18`. Commit do handoff é documental.

CI: 0.15.3 `5297a69` run `35054771262` success; 0.15.4 `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; 0.15.13 `68e5062` #2086 / run `35109440316` success; 0.15.14 `a6f770f` run `35111512775` Clippy+Check success; **0.15.15** final `f03b5bc` run `35112785390` Clippy+Check success; **0.15.16** final `e873159` run `35112964210` Clippy+Check success. **0.15.17** handoff `1fa516d` run `35113608338` ainda in_progress (Clippy) na última verificação. **0.15.18** CI/gameplay ainda sem confirmação nesta edição. Nunca inferir status de run anterior como status do HEAD.

## Prioridades — feedback de runtime do usuário em 2026-09-16

| Item | Status | Próximo passo |
| --- | --- | --- |
| P0.1 FPS / carregamento | Melhorou, ainda pode melhorar | Profiling e otimizações mensuráveis; não marcar resolvido |
| P0.2 chunks escuros + sombras tardias | Aberto em gameplay; correção parcial 0.15.17 sem confirmação | Conferir seed única, convergência, revisões halo, remesh, integração |
| P0.3 rios cortam túneis com paredes retas | Aberto em gameplay; patch matemático 0.15.14, CI aprovado, sem confirmação visual | Se persistir, composição density/carvers |
| P0.4 rios sem conexão lago/oceano | Aberto em gameplay; subcausas corrigidas 0.15.15 e 0.15.16 | Testar continuidade entre regiões, lagos e água física |
| P0.5 margens verticais dos rios | CORRIGIDO segundo usuário após 0.15.12 | Preservar |
| P0.6 biomas regionais ausentes | Aberto: ~3 mil blocos quase Plains/Mountains | Diagnóstico seed/clima/proximidade/overlay e apresentação, não só pesos |
| P0.7 sky/fog sem cores do bioma | Aberto, reconfirmado | Corrigir toda cadeia sem silhuetas no horizonte |
| P1 anterior streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar, reabrir só por nova evidência |
| P2 anterior hotbar/ghost/dye/HUD histórico | Usuário informou `2 feito` | Preservar, novo bug R independente |
| Novo R distorce held block | Patch estrutural 0.15.18 publicado, **aguarda confirmação visual e CI** | Testar R em todos os blocos orientáveis e escala do modelo |

**Ordem:** preservar FPS e margens P0.5; checar CI 0.15.17–18 e corrigir warnings; terminar P0.4/P0.2 em blocos independentes; P0.6/P0.7 críticos; R agora corrigido em código mas não encerrado por gameplay. P1/P2 históricos confirmados pelo usuário não implicam resolução dos bugs novos.

## Histórico do código e confirmação

- 0.14.70 `9302118`: terminal fog alinhada ao sky mas silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial deixou mundo preto/HUD corrompido; 0.14.72 `57305f4`: texturas sol/lua; 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: stack HDR world Skip, viewmodel final tone map, UI SDR, CI success. 0.15.1 `4aeb119`: frontier fog limitada à coluna ausente mais próxima, AABB real menos 1 chunk; usuário confirmou chunks pararam de brotar na fog. 0.15.2 `7b16f8f`: retira recovery temporal. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, active_columns por membership_revision, câmera recriada, CI success.
- 0.15.4 `766d970`/`fbe301f`: iluminação interativa preempta streaming, propagação prioritária, remesh após convergência, CI success; sombras ainda atrasam. 0.15.5 `fa84d81`: remesh lighting async valida halo 3×3×3 e tracker independente de revisões de conteúdo, CI success.
- 0.15.6 `f8d244c`: surface tunnels sob água fade vertical pelo leito; usuário ainda viu paredes retas. 0.15.7 `5c99616`: mountain belt threshold .86→.90 width .22→.15, peaks spacing760→850 chance .48→.36 radius120–230→110–205, Wasteland weight1→1.30 Witchwood/Enchanted1→1.25, oak Plains chance .48→.54. Size/avoidNear inalterados; 3k blocos quase Plains/Mountains.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens Y seaLevel+34..48 ainda seguiam câmera. 0.15.9 `b68ecb3`: nuvens world-space, wind e tile pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes→1 mesh/nuvem (~2/3 entidades/draw calls a menos), CI success; FPS melhor segundo usuário, ainda aberto.
- 0.15.11 `780df02`: drenagem exige ocean bed >=4 abaixo seaLevel, continua fringe seco, ocean seco não substitui river mouth; CI success, rios soltos persistem. 0.15.12 `68a26a3`: shore grading assinado, altura exata, faixa lateral maior, rebaixa banco alto/eleva outer bank baixo sem preencher canal, ocean density usa altura exata; CI success e **margens verticais confirmadas corrigidas**.
- 0.15.13 `68e5062`: generation/density.rs resolve carver antes e chama water_near só com carve, cache Option<Option<...>> incluindo None, testes 0 scans sem carve/1 com; CI success, FPS melhor segundo usuário sem atribuição exclusiva.
- 0.15.14 `cc77c4f` + bump `296960a`: surface_carver_water_factor protege bed_level-3 até water_level+3 com fade4 superior/inferior. Testes limites/fades/lazy water; sem alterar margem/scans. CI run35111512775 success, gameplay não confirmado.
- 0.15.15 `8528960` + fix `f03b5bc` + bump `2ab8466`: `river.rs` remove confluence_target que deslocava tributários à reta downstream→next enquanto canal principal faz curvas; agora termina todos no mesmo nó downstream. Remove varreduras upstream/rank. Fix de import DrainageNode, CI run35112785390 success; rios ainda abertos visualmente.
- 0.15.16 `02f1b18` + bump `e873159`: `valid_lake_outlet` impede saída seca em lago conectado apenas por receber rio; exige downstream wet ocean, outro lago conectado ou caminho com destino. Preserva entrada e lago terminal; usa destination_cache, teste dead end vs lago seguinte. Não garante conexão inter-região/água física. CI run35112964210 success, gameplay não confirmado.
- 0.15.17 `68d1d6b` + `ca3bb60` + bump `28f4f48`: `ChunkStreamingState::initial_lighting_seeded` acompanha residência. Primeira passagem faz `enqueue_loaded_fluid_frontier`, seed e relax; retries mesh stale/preempted usam luz já propagada sem ressetar, evitando bump/scan repetido. `chunk_unloading.rs` esquece marcador apenas no archive, restauração recebe novo seed; teste once/retry/forget/reseed. Mantém budgets e algoritmo HSI. CI run35113608338 pendente, gameplay não confirmado.
- **0.15.18 `3b337bf` + bump `363d205`:** `player/viewmodel/model.rs` remove `PlacementOrientation` de `ViewModelSelection`, remove `orientation_changed` como gatilho de sync, elimina escrita de rotação no Transform durante sync e fixa a rotação de apresentação em `base_viewmodel_transform().rotation.inverse()`, com escala constante `HELD_BLOCK_SCALE`. Model da mão continua observando hotbar, definições e tint; tecla R afeta apenas placement. Teste unitário `held_block_geometry_has_fixed_display_scale_and_rotation`. **Bug confirmado em código, mas CI e validação visual pendentes; ainda não encerrar feedback do usuário.**

## Subsistemas e diagnósticos a preservar

### Streaming / FPS

Pipeline: `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction +2 chunks, corredor frontal +8 ao andar; fog nominal 78%→98% do raio mas recua com readiness. Histerese RD4=5/6 RD12=14/16 RD24=26/30, retention `R+max(ceil(R/2),10)` (RD4=14 RD12=22 RD24=36). Async geração/remesh, integração budgetada. Não elevar budgets luz cegamente. Perfil parado/andando CPU worldgen/lighting/remesh versus GPU cloud transparency. `hydrology/region/density.rs` ainda Vec<VerticalDensityDelta> por coluna, lago em carve+shore; otimizar só por evidência preservando geometria. 0.15.10 cloud, 0.15.13 lazy water, 0.15.17 seed única são melhorias localizadas, FPS aberto.

### Iluminação

Antes `streaming.rs::dispatch_initial_mesh_tasks` reseed direto a cada mesh retry stale, reconstruindo toda a luz (`VoxelWorld::rebuild_chunk_light`) e bump de revision, apagando luz convergida. 0.15.17 evita reseed enquanto resident; unload limpa marcador. `lighting_updates.rs` budget2ms/4096 voxels, remesh até 2 tasks/frame, 4 in-flight e 2 integrações/frame. Initial seed faz bump no VoxelWorld mas não necessariamente no `ChunkRemeshTasks::lighting_revisions`, alimentado por atualizações dinâmicas. Ainda falta validar convergência, freshness do halo, ausência real de skylight versus mesh escura persistente; não fechar por CI e medir retries.

### Hidrologia — túnel, rios, lagos

`generation/density.rs` compõe hydrology density+surface carver; antes 0.15.14 proteção acima de bed-ROOF ilimitada. 0.15.14 limitou à água/roof com fades e water_near lazy; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` limita água bed→water_level/headroom. Persistindo barreira, investigar cave connector vs surface tunnel, water_near borda e thresholds; não mudar margens P0.5 sem evidência.

`river/selection.rs::keep_only_complete_downstream_paths` seleciona até wet ocean/lake, `river.rs::build_river_system` gera edges por região margem4, reachability e channels, `river/path/confluence.rs` inclui segmentos que cruzam região, `spatial.rs` usa raio máximo. 0.15.15 ancora afluentes no nó real; 0.15.16 distingue lago conectado por ENTRADA de saída válida. Restam regional flow cache/seleção, clipping, confluências verticais, water fill. `river_height` usa continentalness threshold→seaLevel mesmo quando wet outlet requer oceano submerso; verificar sem prolongar rios arbitrariamente. P0.4 aberto gameplay, P0.5 corrigido segundo usuário.

### Biomas e cores de ambiente

`biome_field/selection.rs` suitability temperatura/humidade/continentalness/erosion, pesos só elegíveis, evita vizinho dominante e avoidNear em 8 vizinhos, fallback raw. `surface.rs` usa sites Voronoi + overlay Mountains belt/peak independente. `surface_site_spacing` deriva do maior mínimo regional, não cada tamanho. Overworld: Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; Wasteland weight1.30 Witchwood/Enchanted1.25, Plains1.0. Após 3k blocos quase Plains/Mountains, medir seleção por seed/cache/proximidade/clima e overlay; Wasteland/Witchwood compartilham base rolling, grass/dirt/stone com Plains, apresentação pode diminuir distinção; não trocar pesos sem causa.

`rendering/environment.rs` calcula HSI sky_color/fog_color com CurrentBiome+fase dia/noite quando inputs mudam. `world/biome.rs::track_current_biome` segue player, `world/biome/identity.rs` mistura surface/hydrology/volume. `rendering/sky.rs` atualiza ClearColor quando visuals muda. `fog/color.rs` e `fog/attachment.rs` usam sky_color em DistanceFog, ignoram fog_color artístico; causa fog não refletir paleta mas não explica sky imóvel. Mudar terminal fog_color diretamente reintroduz silhuetas/popping; preservar terminal igual céu, encontrar transição de cor local separada, rastrear inputs/definições/câmeras.

### Coast, celestial, HUD e históricos fechados

Coast identidade usa continentalness raw `BiomeField::climate_at`, oceano físico usa macro sample interpolado 5×5 `HydrologyRegion::macro_sample_at`; material rejeita ocean bed acima seaLevel+3 e identidade Coast não considera altura. P1 histórico concluído pelo usuário, só reabrir com evidência. `rendering/celestial.rs` lê textura por CelestialBodyDefinition; Overworld sun/moon. Nuvens world-space Y relativo seaLevel, vento desacoplado do player e tile recycling.

P2 histórico concluído pelo usuário: held bloco observava hotbar/esconde vazio, ghost alpha uniforme, dye, HUD planejada player canto inferior esquerdo Yogg'Sara, vida50/100 dentro da barra, status acima; target HUD acima crosshair bloco esquerda slot inventory e texto direita tooltip shadow. Bug novo R teve code fix 0.15.18, validação runtime pendente. Não reaplicar orientação de placement à mesh/mão.

---

# Próxima execução

1. Conferir CI final 0.15.17/0.15.18, corrigir warnings/erros sem declarar gameplay corrigido. Testes unitários adicionados não implicam que `cargo test` executou.
2. P0.4 testes determinísticos de continuidade seleção/edges/água entre regiões, outlet físico; preservar margem0.15.12, confluência0.15.15 e filtro lago0.15.16.
3. P0.2 acompanhar seed única, freshness dos trackers, convergência skylight e remesh; FPS e chunks dark antes/depois.
4. P0.3 conferir gameplay 0.15.14, seguir density/carver apenas se persistir.
5. P0.6/P0.7 diagnosticar biome distribution e cores sky/fog separadamente.
6. Verificar visual de R/viewmodel em orientação de cada bloco; FPS mensurado; preservar P0.5 e P1/P2 históricos.

Cada bloco de código incrementa `VERSION` e atualiza este HANDOFF na mesma sessão. Feedback do usuário reordena prioridades imediatamente.
