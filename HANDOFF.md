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

Último commit **de código** publicado: `02f1b18f59284552973b9ffb0599c46406635a33` (validação dos destinos dos lagos). Bump: `e87315906328441e0119a01654531d0a7de068ec`; `VERSION = 0.15.16`. Fix complementar anterior: `f03b5bcc3b0060009e17da13cbe62dbe97be4e5b` no bloco 0.15.15. Este commit do handoff só documenta.

CI: 0.15.3 `5297a69` run `35054771262` success; 0.15.4 `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; 0.15.13 `68e5062` #2086 / run `35109440316` success; 0.15.14 `a6f770f` run `35111512775` Clippy+Check success. 0.15.15 `f03b5bc` run `35112785390` em execução na última verificação. 0.15.16 `e873159` run `35112964210` em execução. **Não declarar CI das versões novas aprovado sem resultado dos respectivos runs.**

## Estado de prioridades — feedback do usuário

Feedback de runtime de 2026-09-16 é autoritativo para status.

| Item | Status | Próximo passo |
| --- | --- | --- |
| P0.1 FPS / carregamento | Melhorou, ainda pode melhorar | Profiling e otimizações mensuráveis; não marcar resolvido |
| P0.2 chunks escuros + sombras tardias | Aberto | Seed→propagação→remesh→integração; revisar trackers distintos |
| P0.3 rios cortam túneis com paredes retas | Aberto em gameplay; patch matemático 0.15.14 não confirmado visualmente | Se persistir, investigar composição density e carvers |
| P0.4 rios sem conexão lago/oceano | Aberto em gameplay; subcausas corrigidas em 0.15.15 e 0.15.16 | Investigar conexões completas entre regiões, lagos e fluido físico |
| P0.5 margens verticais dos rios | CORRIGIDO segundo usuário após 0.15.12 | Preservar |
| P0.6 biomas regionais ausentes | Aberto: ~3 mil blocos, quase Plains/Mountains | Diagnóstico por seed, seleção, clima, proximidade e macro overlay, não só pesos |
| P0.7 sky/fog sem cores do bioma | Aberto, reconfirmado | Corrigir toda cadeia sem voltar a silhuetas no horizonte |
| P1 anterior: streaming/Coast/nuvens | Usuário informou `1 feito` | Preservar, reabrir só com nova evidência |
| P2 anterior: hotbar/ghost/dye/HUD histórico | Usuário informou `2 feito` | Preservar, bug R é independente |
| Novo: R distorce held block | Aberto | Separar orientação placement e geometria/viewmodel |

**Ordem:** preservar FPS e P0.5; validar patches 0.15.14–16, continuar P0.4/P0.2 em blocos independentes; P0.6/P0.7 críticos; bug R após bloqueadores. P1/P2 históricos encerrados por feedback, sem transferir esse status aos novos relatos.

## Histórico de código e confirmação

- 0.14.70 `9302118`: terminal fog alinhada ao sky, silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial deixou mundo preto/HUD corrompido; 0.14.72 `57305f4`: sol/lua texturizados; 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: stack HDR explícita (world Skip, viewmodel final tone mapping, UI SDR), CI success. 0.15.1 `4aeb119`: fog frontier limitada à coluna ausente mais próxima, AABB real menos um chunk; usuário confirmou que chunks pararam de aparecer dentro da fog. 0.15.2 `7b16f8f`: retira recovery temporal. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache active_columns invalidado por membership_revision, inclui câmera recriada; CI success.
- 0.15.4 `766d970`/`fbe301f`: lighting lane interativa deduplicada preempta streaming, propagação mantém prioridade, remesh publicado quando onda converge. CI success, sombras ainda atrasam. 0.15.5 `fa84d81`: lighting remesh async valida halo 3×3×3 e revisão própria distinta de mesh content. Initial seed altera VoxelWorld, não necessariamente tracker de remesh; verificar.
- 0.15.6 `f8d244c`: surface tunnels sob água passam a fade vertical pelo leito; ainda havia paredes retas. 0.15.7 `5c99616`: mountain belt threshold .86→.90, width .22→.15; peaks spacing760→850, chance .48→.36, radius120–230→110–205; Wasteland weight1→1.30, Witchwood/Enchanted1→1.25, oak Plains chance .48→.54; size/avoidNear não mudaram. 3000 blocos quase só Plains/Mountains: pesos insuficientes.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvens seaLevel+34..48 ainda grudavam câmera. 0.15.9 `b68ecb3`: world-space, wind e pool180. 0.15.10 `eadb6ea`: 3 cubos transparentes→1 mesh/nuvem, ~2/3 menos entidades/draw calls. FPS melhorou segundo usuário, mas aberto.
- 0.15.11 `780df02`: drenagem requer ocean bed pelo menos 4 abaixo seaLevel, continua fringe seco, evita amostra ocean seca substituir river mouth; CI success, rios soltos persistem. 0.15.12 `68a26a3`: shore grading assinado rios/lagos, altura exata, faixa lateral maior, rebaixa high bank, eleva apenas outer bank baixo, sem preencher canal; ocean density usa altura exata. CI success; **usuário confirma margens verticais corrigidas.**
- 0.15.13 `68e5062`: generation/density.rs resolve carver primeiro e chama hydrology.water_near somente se voxel realmente carvado, cache Option<Option<...>> inclusive None por coluna; testes de scans 0 sem carve/1 com carve, CI success. FPS melhor segundo usuário, sem atribuir exclusivamente a esse patch.
- 0.15.14 `cc77c4f` + bump `296960a`: surface_carver_water_factor com proteção de bed_level-3 até water_level+3 e fades de 4 blocos superior/inferior. Testes nos limites, fades e lazy water_near; sem mudar margem nem adicionar scans. CI run `35111512775` Clippy/Check success; gameplay não confirmado.
- 0.15.15 `8528960` + fix `f03b5bc` + bump `2ab8466`: `hydrology/river.rs` removia artificialmente a chegada de afluentes ao deslocá-los ao longo da linha reta downstream→next, enquanto calha principal meandra; agora todos terminam no mesmo nó downstream que inicia aresta principal. Remove varreduras `channel_upstreams` e classificação de rank, reduz overhead; fix complementar anota DrainageNode explicitamente para evitar import sem uso. Corrige **subcausa horizontal**; diferença vertical, grafo e fluidos seguem a investigar. CI run `35112785390` pendente; runtime não confirmado.
- 0.15.16 `02f1b18` + bump `e873159`: `river.rs::valid_lake_outlet` evita gerar aresta de saída de lago que é marcado connected por RECEBER rio, quando o caminho seguinte não chega a oceano molhado ou outro lago conectado. Preserva a conexão de entrada e o lago terminal, apenas elimina ramal de saída para terreno seco. Usa destination_cache já existente e rastreia downstream adicional somente para lago; teste isolado usa drenagem plana e verifica saída falsa em dead end, verdadeira ao conectar próximo lago. **Ainda não garante continuidade completa de rios entre regiões ou water fill; não declarar P0.4 resolvido antes de gameplay.** CI run `35112964210` pendente.

## Subsistemas e diagnósticos a preservar

### Streaming e FPS

Pipeline: `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction +2 chunks e corredor frontal +8 andando. Fog nominal começa a ~78% e termina ~98% do raio, recua conforme readiness. Histerese RD4=5/6 RD12=14/16 RD24=26/30; retention `R + max(ceil(R/2),10)` (RD4=14 RD12=22 RD24=36). Tasks async, integração com budget. Não subir budgets de luz cegamente. Profiling parado/andando para CPU worldgen/lighting/remesh versus GPU cloud transparency. `hydrology/region/density.rs` ainda constrói Vec<VerticalDensityDelta> por coluna, verifica lago carve+shore; só otimizar com evidência preservando geometria. 0.15.10 cloud overdraw e 0.15.13 lazy water scan são melhorias concretas; FPS permanece aberto.

### Iluminação

`streaming.rs::dispatch_initial_mesh_tasks` chama `seed_chunk_direct_lighting` e enfileira relaxation, mas snapshot/mesh inicial pode anteceder convergência entre vizinhos. `lighting_updates.rs` budget 2ms/4096 voxels; remesh até 2 tasks/frame, 4 in-flight, 2 integrações/frame. Initial seed faz bump no VoxelWorld e não necessariamente no `ChunkRemeshTasks::lighting_revisions`, que atualizações dinâmicas alimentam. **Nova hipótese a testar:** `dispatch_initial_mesh_tasks` reseeda em toda tentativa se snapshot inicial fica stale e coord volta à fila ready; isso pode sobrescrever luz já convergida e repetir trabalho. Se confirmado, controlar seed uma vez por residência e limpar estado no unload/restore sem vazamento, mantendo remesh via revisões. Separar chunk sem skylight de mesh persistente escura e não marcar resolvido por compilação.

### Hidrologia — túnel, rio, lagos

`generation/density.rs` compõe hydrology density+surface carver; fator antigo protegia toda coluna acima de bed-ROOF, 0.15.14 limitou à água e roof+3 com fades superior/inferior4 e water_near lazy. `density_sampling/hydrology.rs::enforce_hydrology_water_volume` limita água real a bed→water_level e headroom delimitado. Se persistir parede, investigar cave connector, water_near nas bordas e thresholds, nunca modificar margens P0.5 sem evidência.

`river/selection.rs::keep_only_complete_downstream_paths` seleciona trajetos para wet ocean ou lake; `river.rs::build_river_system` cria arestas por região em margem 4, reachability, selected.channels; `river/path/confluence.rs` insere segmentos que intersectam região; `spatial.rs` usa raio máximo. 0.15.15 remove alvo de confluência deslocado na reta, ancora afluentes em nó real. 0.15.16 distingue lago conectado por entrada de lago com saída válida, não inventa ramal para seco. Resta validar regional flow cache, seleção fora de raio, path/edge clipping, confluências verticais, water fill e ocean outlet físico. `river_height` trata continentalness abaixo threshold como seaLevel, mesmo quando outlet exige oceano fisicamente submerso; verificar descontinuidade sem prolongar rios arbitrariamente. P0.4 permanece aberto em gameplay; P0.5 confirmado corrigido após 0.15.12.

### Biomas e cores de ambiente

`biome_field/selection.rs`: suitability de temperatura/humidade/continentalness/erosion, pesos só elegíveis, evita vizinho dominante e `avoidNear` em 8 vizinhos, fallback raw. `surface.rs`: sites Voronoi e overlay independente Mountains belt/peak. `surface_site_spacing` deriva do maior mínimo regional, não de cada tamanho individual; Overworld data: Plains size120–420, Wasteland120–420, Witchwood/Enchanted140–460, Mountains macro .85; pesos Wasteland1.30 Witchwood/Enchanted1.25 Plains1.0. Usuário andou 3000 blocos encontrando quase só Plains/Mountains. Inspecionar seleção real por seed, cache/proximidade, escala de sites e domínio macro, não mais peso aleatório.

`rendering/environment.rs` calcula HSI sky_color/fog_color com `CurrentBiome` e fase dia/noite se inputs changed. `world/biome.rs::track_current_biome` segue player; `world/biome/identity.rs` mistura surface/hydrology/volume. `rendering/sky.rs` escreve ClearColor sky quando resource muda. `fog/color.rs` e `fog/attachment.rs` usam sky_color para DistanceFog, ignoram fog_color artístico. Isso explica fog ignorar paleta mas não sky sem variar; rastrear updates, inputs, definições, fases e câmeras. Mudar fog terminal para fog_color diretamente reintroduz diferença em relação ao céu e pode causar silhouette/popping; preservar terminal combinado e variar cor local com horizonte compatível.

### Coast, céu, HUD e históricos fechados

Coast identidade usa continentalness raw `BiomeField::climate_at`, enquanto oceano físico usa macro sample interpolado 5×5 `HydrologyRegion::macro_sample_at`; material rejeita ocean bed acima seaLevel+3, identidade Coast não considera altura. P1 agregado usuário marcou concluído; preservar histórico sem reabrir sem relato novo. `rendering/celestial.rs` lê texturas por `CelestialBodyDefinition`; Overworld tem sun/moon. Nuvens world-space Y seaLevel relativo, vento não segue player, tile recycling.

P2 agregado usuário marcou concluído: held block observa hotbar, slot vazio oculta; ghost uniformidade de alpha por bloco; dye intensificado; HUD planejada jogador inferior esquerdo, Yogg'Sara e 50/100 dentro da barra, status acima; target acima crosshair com bloco esquerda slot inventory e texto à direita com tooltip shadow. Novo bug independente: R distorce modelo segurado; rastrear hotkey, orientação placement, Transform, UV e escala sem mutações acumulativas.

---

# Próxima execução

1. Checar CI 0.15.15/0.15.16; corrigir warnings ou erros encontrados sem declarar gameplay resolvido.
2. P0.4: instrumentar/testar continuidade de seleção, edges e água entre regiões e saídas reais; preservar margens 0.15.12, confluência 0.15.15 e filtro de lago 0.15.16.
3. P0.2: confirmar hipótese reseed em stale task, sincronizar seed/convergência/tracker/remesh sem custo alto na main thread.
4. P0.3: CI 0.15.14 aprovado; conferir gameplay para paredes, aprofundar carvers somente com ocorrência persistente.
5. P0.6/P0.7: diagnosticar biome distribution e cores do sky/fog em commits separados.
6. Bug R/viewmodel e FPS mensurado; preservar P0.5 e P1/P2 históricos.

Todo bloco de código incrementa `VERSION` e é seguido por atualização deste HANDOFF. Feedback do usuário reordena prioridades imediatamente.
