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
12. Block held/viewmodel deve observar hotbar e manter transform geométrico próprio. Rotação de placement acionada por R não deve distorcer nem reescalar o modelo da mão.
13. Produto: caminhada, círculos e voo máximo sem void, flicker, chunks inteiramente escuros, popping ou fog breathing; não mascarar throughput só com fog.

---

# Estado atual — 2026-09-16

Último commit **de código**: `ca3bb60d341c792d2ccb575e791213bb226ac37e` (`streaming.rs` + unload, primeira seed somente por residência); início do bloco `68d1d6bc544a67cd8dc0dbe0ff30b959947d0829`; bump `28f4f48d36eccb743e7f559757c75435b32f3760`; `VERSION = 0.15.17`. O presente handoff só documenta, sem bump.

CI: 0.15.3 `5297a69` run `35054771262` success; 0.15.4 `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; 0.15.13 `68e5062` #2086 / run `35109440316` success; 0.15.14 `a6f770f` run `35111512775` Clippy+Check success. 0.15.15 `f03b5bc` run `35112785390` em execução na última consulta, 0.15.16 `e873159` run `35112964210` em execução. **0.15.17 CI e gameplay ainda não confirmados nesta edição.** Nunca inferir status de run anterior como status da HEAD atual.

## Prioridades — feedback de runtime do usuário em 2026-09-16

| Item | Status | Próximo passo |
| --- | --- | --- |
| P0.1 FPS / carregamento | Melhorou, ainda pode melhorar | Profiling e otimizações mensuráveis; não marcar resolvido |
| P0.2 chunks escuros + sombras tardias | Aberto em gameplay; correção parcial 0.15.17 sem confirmação visual | Conferir seed única, convergência, revisões de halo, remesh e integração |
| P0.3 rios cortam túneis com paredes retas | Aberto em gameplay; patch matemático 0.15.14, CI aprovado, sem confirmação visual | Se persistir, composição density/carvers |
| P0.4 rios sem conexão lago/oceano | Aberto em gameplay; subcausas corrigidas 0.15.15 e 0.15.16 | Investigar continuidade completa entre regiões, lagos e fluido físico |
| P0.5 margens verticais dos rios | CORRIGIDO segundo usuário após 0.15.12 | Preservar |
| P0.6 biomas regionais ausentes | Aberto: ~3 mil blocos quase Plains/Mountains | Diagnóstico por seed/clima/proximidade/overlay, não só pesos |
| P0.7 sky/fog sem cores do bioma | Aberto, reconfirmado | Corrigir toda cadeia sem voltar a silhuetas no horizonte |
| P1 anterior streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar, reabrir só por nova evidência |
| P2 anterior hotbar/ghost/dye/HUD histórico | Usuário informou `2 feito` | Preservar, bug R independente |
| Novo R distorce held block | Aberto | Separar orientação de placement e geometria/viewmodel |

**Ordem:** preservar FPS e margens P0.5; checar CI 0.15.15–17 e corrigir warnings; terminar investigação P0.4/P0.2 em blocos independentes; P0.6/P0.7 críticos; R depois. Nunca transformar patch matemático ou CI em confirmação de gameplay. P1/P2 históricos encerrados por feedback sem transferir status a bugs novos.

## Histórico do código e confirmação

- 0.14.70 `9302118`: terminal fog alinhada ao sky mas silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial deixou mundo preto/HUD corrompido; 0.14.72 `57305f4`: texturas sol/lua; 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: stack HDR world Skip, viewmodel final tone map, UI SDR, CI success. 0.15.1 `4aeb119`: frontier fog limitada à coluna ausente mais próxima, AABB real menos 1 chunk; usuário confirmou chunks pararam de brotar dentro da fog. 0.15.2 `7b16f8f`: retira recovery temporal. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, active_columns por membership_revision, câmera recriada, CI success.
- 0.15.4 `766d970`/`fbe301f`: lane interativa deduplicada preempta streaming, propagação prioritária, remesh publicado após convergência, CI success; sombras ainda atrasam. 0.15.5 `fa84d81`: remesh lighting async valida halo 3×3×3 e tracker independente das revisões de conteúdo, CI success.
- 0.15.6 `f8d244c`: surface tunnels sob água passam a fade vertical pelo leito; usuário ainda viu paredes retas. 0.15.7 `5c99616`: mountain belt threshold .86→.90 width .22→.15, peaks spacing760→850 chance .48→.36 radius120–230→110–205, Wasteland weight1→1.30 Witchwood/Enchanted1→1.25, oak Plains chance .48→.54. Size/avoidNear inalterados; 3k blocos quase Plains/Mountains.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens Y seaLevel+34..48 ainda seguiam câmera. 0.15.9 `b68ecb3`: nuvens world-space, wind e tile pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes→1 mesh/nuvem (~2/3 entidades e draw calls a menos), CI success; FPS melhor segundo usuário, ainda aberto.
- 0.15.11 `780df02`: drenagem exige ocean bed >=4 abaixo seaLevel, continua fringe seco, não deixa ocean sample seco substituir river mouth; CI success, mas rios soltos persistem. 0.15.12 `68a26a3`: shore grading assinado, superfície exata, faixa lateral maior, rebaixa banco alto e eleva outer bank baixo sem preencher canal, ocean density usa altura exata, CI success; **margens verticais confirmadas corrigidas pelo usuário**.
- 0.15.13 `68e5062`: generation/density.rs resolve carver antes e chama hydrology.water_near apenas no primeiro voxel com carve, cache Option<Option<...>> incluindo None por coluna. Testes 0 scans sem carve / 1 com; CI success, FPS melhor segundo usuário mas não atribuir ganho exclusivamente.
- 0.15.14 `cc77c4f` + bump `296960a`: surface_carver_water_factor protege bed_level-3 até water_level+3, fade 4 blocos acima/abaixo; testes dos limites/fade/lazy water_near, sem mudar margem nem scans. CI run 35111512775 success; gameplay sem confirmação.
- 0.15.15 `8528960` + fix `f03b5bc` + bump `2ab8466`: `hydrology/river.rs` remove confluence_target que deslocava afluentes a pontos na reta downstream→next enquanto canal principal faz curvas; termina todos no downstream autoritativo, reduz varredura de upstreams/rank. Fix complementar anotou DrainageNode explicitamente para evitar import sem uso. **Subcausa horizontal corrigida, rios ainda abertos em gameplay**. CI 35112785390 a verificar.
- 0.15.16 `02f1b18` + bump `e873159`: `valid_lake_outlet` impede criar saída seca em lago considerado conectado apenas porque recebe rio; exige downstream wet ocean, outro lago conectado ou percurso que atinja destino. Preserva a entrada e o lago terminal; usa destination_cache, teste de dead end vs lago seguinte. Não garante conectividade entre regiões nem água física; CI run 35112964210 a verificar.
- **0.15.17 `68d1d6b` + `ca3bb60` + bump `28f4f48`:** `ChunkStreamingState::initial_lighting_seeded` acompanha residência de cada chunk. `dispatch_initial_mesh_tasks` agora executa `enqueue_loaded_fluid_frontier`, `seed_chunk_direct_lighting` e primeira `enqueue_chunk_relaxation` apenas na primeira passagem; retries por mesh stale/preempção capturam estado atual sem apagar iluminação propagada nem invalidar snapshot por novo seed. `chunk_unloading.rs` remove o marcador apenas após `archive_chunk`, garantindo novo seed ao restaurar/regenerar e não acumulando entradas após descarte. Teste unitário de once/retry/forget/reseed. Não altera budgets de luz/streaming ou algoritmo HSI. **Corrige ciclo de reseed identificado no código, mas luz escura, sombreamento e FPS ainda requerem gameplay.** CI ainda não confirmado nesta edição.

## Subsistemas e diagnósticos a preservar

### Streaming / FPS

Pipeline: `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction +2 chunks, corredor frontal +8 andando; fog nominal 78%→98% do raio mas recua conforme readiness. Histerese RD4=5/6 RD12=14/16 RD24=26/30, retention `R+max(ceil(R/2),10)` (RD4=14 RD12=22 RD24=36). Tasks geração/remesh async e integração budgetada. Não elevar budgets de iluminação indiscriminadamente. Perfil parado/andando CPU worldgen/lighting/remesh versus GPU cloud transparency. `hydrology/region/density.rs` ainda gera Vec<VerticalDensityDelta> por coluna e verifica lago em carve+shore; otimizar só por evidência, preservando geometria. 0.15.10 cloud overdraw, 0.15.13 lazy water, 0.15.17 seed única são melhorias localizadas, FPS continua aberto.

### Iluminação

`streaming.rs::dispatch_initial_mesh_tasks` antes chamava seed direta **a cada retry de mesh stale**, o que reconstruía toda a luz de chunk via `VoxelWorld::rebuild_chunk_light` e fazia bump revision. 0.15.17 evita reseed enquanto resident; unload limpa marcador. `lighting_updates.rs` orçamento 2ms/4096 voxels; remesh 2 tasks/frame, 4 in-flight, 2 integrações/frame. Seed inicial bump no VoxelWorld não necessariamente no `ChunkRemeshTasks::lighting_revisions`, alimentado por atualizações dinâmicas; falta validar seed/convergência/remesh halo e ausência real de skylight versus mesh escura persistente. Não declarar tudo corrigido por CI, e medir impacto de retries.

### Hidrologia — túnel, rio, lagos

`generation/density.rs` combina hydrology density + surface carver; antes de 0.15.14, proteção prendia túnel indefinidamente acima de bed-ROOF. 0.15.14 limita à água/roof com fades e mantém water_near lazy; `density_sampling/hydrology.rs::enforce_hydrology_water_volume` limita água bed→water_level e headroom específico. Se barreira persistir: cave connector vs surface tunnel, water_near borda, thresholds, nunca mudar margens P0.5 sem evidência.

`river/selection.rs::keep_only_complete_downstream_paths` seleciona trajetos a wet ocean/lake; `river.rs::build_river_system` gera arestas por região em margem 4, reachability, selected.channels; `river/path/confluence.rs` inclui segmentos que cruzam região e `spatial.rs` usa raio máximo. 0.15.15 ancora afluentes no nó real; 0.15.16 distingue lago conectado por ENTRADA de saída válida. Investigar regional flow cache/seleção entre regiões, clipping/edge, confluências verticais e water fill. `river_height` usa continentalness threshold→seaLevel enquanto wet outlet exige oceano fisicamente submerso; verificar descontinuidade sem prolongar rios arbitrariamente. P0.4 aberto em gameplay; P0.5 confirmado corrigido após 0.15.12.

### Biomas e cores de ambiente

`biome_field/selection.rs` suitability de temperatura/humidade/continentalness/erosion, pesos só elegíveis, evita vizinho dominante e `avoidNear` em 8 vizinhos, fallback raw. `surface.rs` usa sites Voronoi + overlay independente Mountains belt/peak. `surface_site_spacing` deriva do maior mínimo regional, não de cada tamanho individual. Overworld: Plains120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; Wasteland weight1.30, Witchwood/Enchanted1.25, Plains1.0. Após 3000 blocos quase só Plains/Mountains, medir por seed seleção/caches/proximidade/clima e overlay, não ajustar mais pesos sem evidência.

`rendering/environment.rs` calcula HSI sky_color/fog_color via `CurrentBiome`+fase dia/noite se inputs mudam. `world/biome.rs::track_current_biome` segue player; `world/biome/identity.rs` mistura surface/hydrology/volume. `rendering/sky.rs` altera ClearColor se visuals muda. `fog/color.rs` e `fog/attachment.rs` usam sky_color em DistanceFog, ignoram fog_color artístico. Isso explica fog não refletir paleta mas não por si só sky sem variar. Trocar terminal fog para fog_color diretamente reintroduz silhueta/popping; preservar terminal compatível com céu e criar composição que respeite fog do bioma no primeiro plano, investigar mudanças de inputs, definições e câmeras.

### Coast, celestial, HUD e históricos fechados

Coast identidade usa continentalness raw `BiomeField::climate_at`, oceano físico usa macro sample interpolado 5×5 `HydrologyRegion::macro_sample_at`; material rejeita ocean bed acima seaLevel+3 e identidade Coast não considera altura. P1 histórico concluído segundo usuário, reabrir só por nova evidência. `rendering/celestial.rs` lê texturas por CelestialBodyDefinition; Overworld tem sun/moon. Nuvens world-space Y relativo a seaLevel, vento não segue player, tile recycling.

P2 histórico concluído segundo usuário: held block observa hotbar e slot vazio oculta, ghost alpha por bloco, dye intensificado; HUD planejada jogador canto inferior esquerdo Yogg'Sara 50/100 dentro da barra, status acima, target HUD acima crosshair com bloco esquerda estilo inventory e texto à direita tooltip shadow. Bug NOVO: R distorce modelo na mão; rastrear hotkey, orientação de placement, Transform/mesh/UV sem mutações acumuladas.

---

# Próxima execução

1. Checar CI dos commits finais 0.15.15–17 e corrigir warnings/erros se houver; testes unitários adicionados não significam que `cargo test` foi executado.
2. P0.4: testes determinísticos de continuidade de seleção, edges e água entre regiões, validação da saída física, preservando margem 0.15.12, confluência 0.15.15 e filtro lago 0.15.16.
3. P0.2: verificar seed uma vez em runtime, acompanhar freshness dos dois trackers, convergência de skylight e remesh; comparar FPS e chunks dark antes/depois. Não subir budgets cegamente.
4. P0.3: CI 0.15.14 aprovado, aguarda gameplay; seguir carvers/density só se barreira persistir.
5. P0.6/P0.7: biome distribution e sky/fog com investigações estruturais e commits próprios.
6. R/viewmodel e FPS mensurado; preservar P0.5 e P1/P2 históricos.

Todo bloco de código incrementa `VERSION` e atualiza este HANDOFF na mesma sessão. Feedback do usuário reordena prioridades imediatamente.
