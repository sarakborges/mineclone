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

Último commit **de código** publicado: `68e506217ee04f6dbbe0ba12f81c74d368050804` (0.15.13); `VERSION = 0.15.13`. Este commit do handoff apenas documenta e não sobe VERSION.

CI: 0.15.3 `5297a69` run `35054771262` success; 0.15.4 fix `fbe301f` #2064 success; 0.15.5 `fa84d81` #2066 success; 0.15.6 `f8d244c` #2068 success; 0.15.7 `5c99616` #2070 success; 0.15.8 final `c22a199` #2075 success; 0.15.9 `b68ecb3` #2078 success; 0.15.10 `eadb6ea` #2080 success; 0.15.11 `780df02` #2082 success; 0.15.12 `68a26a3` #2084 success; **0.15.13 `68e5062` #2086 / run `35109440316` success, Clippy + Check.** Nenhuma 0.15.14 foi publicada neste registro.

## Estado de prioridades — confirmação mais recente do usuário

Numeração abaixo referencia a listagem consolidada anterior. **Feedback de runtime 2026-09-16 é autoritativo para status.**

| Item | Status | Próximo passo |
| --- | --- | --- |
| P0.1 FPS / carregamento | **Melhorou, ainda pode melhorar** | Continuar profiling e otimizações mensuráveis; não marcar resolvido |
| P0.2 chunks totalmente escuros + atualização tardia de sombras | **Aberto** | Revisão seed→propagação→remesh→integração, com atenção a revisões distintas |
| P0.3 rios cortam túneis com paredes/barreiras retas | **Aberto, reproduzido pelo usuário mesmo após 0.15.6 e 0.15.13** | Corrigir proteção de leito no volume exato e junção de density/carver; testar gameplay |
| P0.4 rios param no meio do nada, sem conexão lago/oceano | **Aberto, reproduzido após 0.15.11** | Unificar grafo de drenagem, seleção, fluxo, edges e água física entre regiões |
| P0.5 margens dos rios fazem corte vertical | **CORRIGIDO conforme confirmação explícita do usuário** após 0.15.12 | Não voltar a mexer sem nova evidência; preservar a correção |
| P0.6 biomas regionais quase não aparecem | **Aberto: 3 mil blocos, quase só Plains e Mountains** | Diagnosticar seleção espacial/clima/proximidade + overlay macro; não só pesos |
| P0.7 céu e fog não mudam conforme o bioma | **Aberto, usuário reconfirmou** | Restaurar identidade sky/fog em toda cadeia sem reintroduzir silhuetas no frontier |
| P1 anterior (streaming/Coast/nuvens) | **Usuário informou "1 feito"** | Registrar como concluído segundo relato, preservar invariantes; reabrir só por nova evidência |
| P2 anterior (hotbar/ghost/dye/HUD histórico) | **Usuário informou "2 feito"** | Registrar como concluído segundo relato; novo bug R é independente |
| NOVO: R / held block | **Aberto: R ao rotacionar bloco distorce o modelo na mão** | Rastrear estado de orientação de placement versus Transform/mesh/viewmodel; evitar mutar escala ou aplicar rotação cumulativa na mão |

**Ordem de execução atual:** evitar regressão de FPS; P0.2 e P0.3/P0.4 em blocos separados de correção; P0.6 e P0.7 igualmente críticos e precisam de investigação estrutural; bug R como próximo fix de viewmodel após bloqueadores. P0.5 encerrado por confirmação runtime. P1/P2 anteriores concluídos por feedback, mas não usar isso para declarar os novos bugs corrigidos. Não somar margens resolvidas ao problema ainda aberto de conectividade.

## Histórico dos blocos de código

- 0.14.70 `9302118`: terminal fog alinhada ao sky mas silhuetas persistiam. 0.14.71 `b028fbe`: HDR parcial deixou mundo preto/HUD corrompido; 0.14.72 `57305f4`: texturas sol/lua, 0.14.73 `f864669`: revert HDR parcial.
- 0.15.0 `1b16bee`: stack HDR explícita (world Skip, viewmodel final tone mapping, UI SDR). CI success. 0.15.1 `4aeb119`: frontier fog limitada à coluna ausente mais próxima do `ChunkRenderPool`, AABB real menos 1 chunk; usuário confirmou que chunks pararam de brotar dentro da fog. 0.15.2 `7b16f8f`: retira recovery temporal da fog. 0.15.3 `122d0ff`/`86da5c2`/`5297a69`: readiness change-driven, cache `active_columns` invalidado por `membership_revision`, inclui recriação de câmera; CI success.
- 0.15.4 `766d970`/`fbe301f`: lighting lane interativa deduplicada preempta streaming, propagação mantém prioridade, remesh publicado quando onda converge. CI success, feedback ainda relata sombras não imediatas.
- 0.15.5 `fa84d81`: async lighting-remesh valida halo 3×3×3 de lighting além de conteúdo via tracker próprio `ChunkRemeshTasks::lighting_revisions`, não é `VoxelWorld::chunk_mesh_revisions`. Seed inicial altera revisão world, não necessariamente tracker da remesh: possível freshness inconsistente; verificar antes de mexer.
- 0.15.6 `f8d244c`: surface tunnels sob água trocam gate binário por fade vertical baseado em bed. Bug runtime: rios ainda cortam túneis com paredes retas; proteção no código original acima do leito não tem limite superior. Hipótese específica: `surface_carver_water_factor` mantém fator zero indefinidamente acima de `water.bed_level - ROOF`; limitar ao volume hídrico/roof com fades superior e inferior e inspecionar outras camadas de densidade antes de concluir. **Nenhum patch 0.15.14 foi enviado até este handoff.**
- 0.15.7 `5c99616`: mountain_belt threshold .86→.90, width .22→.15; mountain peaks spacing760→850, chance .48→.36, radius120–230→110–205; pesos regionais Wasteland1→1.30 e Witchwood/Enchanted1→1.25; oak Plains chance .48→.54; não mudou size/avoidNear. Feedback novo 3k blocos quase só Plains/Mountains: ajustes de peso insuficientes, não marcar P1.1 de distribuição resolvido só porque P1 agregado foi marcado.
- 0.15.8 `ebe0b7c`/`c22a199`: nuvem Y=seaLevel+34..48; X/Z ainda grudados na câmera. 0.15.9 `b68ecb3`: nuvens world-space com wind e tile pool 180, CI success; 0.15.10 `eadb6ea`: reduz 3 cubos transparentes a 1 mesh por nuvem (~2/3 entidades/draw calls do pool), CI success. FPS melhorou segundo usuário, ainda requer otimização.
- 0.15.11 `780df02`: drainage exige oceano fisicamente submerso (bed pelo menos 4 abaixo seaLevel), continua pelo fringe seco e impede ocean water_at seco de substituir river mouth. CI success, mas **usuário confirma rios soltos ainda**: distinguir destino lógico de edges efetivamente geradas no `river_system` e água preenchida.
- 0.15.12 `68a26a3`: river/lake shore grading assinado, amostra altura exata do terreno e faixa lateral maior, rebaixa high bank, eleva apenas outer bank baixo, sem preencher interior do canal. Ocean density usa altura exata em blend. CI #2084 success. **Usuário confirma P0.5 corrigido.**
- 0.15.13 `68e5062`: `generation/density.rs` resolve surface carver primeiro e executa `hydrology.water_near` lazy apenas no primeiro voxel com delta de carve diferente de 0, cache `Option<Option<HydrologyWaterSample>>` por coluna (inclusive None); elimina scans redundantes em colunas sem carve. Teste verifica zero scans sem carve e exatamente um entre voxels carvados. CI #2086 success. FPS melhorou segundo usuário, porém não atribuir todo ganho a uma causa isolada sem métricas.

## Subsistemas e diagnósticos a preservar

### Streaming / FPS

Pipeline: `selection/prefetch -> generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> integrate -> render allocation -> visibility hysteresis -> retention -> archive/unload`. Preload all-direction +2 chunks, corredor frontal +8 ao andar, fog nominal start ~78% / end ~98% do raio mas recua conforme readiness. Histerese visibility RD4=5/6, RD12=14/16, RD24=26/30; retention `R + max(ceil(R/2),10)` (RD4=14 RD12=22 RD24=36); horizontais. Geração e remesh async, integração budgetada. Não sacrificar FPS elevando budgets de luz indiscriminadamente. Perfil parado versus andando, CPU worldgen/lighting/remesh versus GPU cloud transparency. `hydrology/region/density.rs` ainda constrói `Vec<VerticalDensityDelta>` por coluna e verifica lago duas vezes (carve+shore); só otimizar com evidência e preservar geometria. 0.15.10 cloud overdraw e 0.15.13 lazy water scans foram mudanças concretas, FPS ainda pode melhorar.

### Iluminação

`world/streaming.rs::dispatch_initial_mesh_tasks` executa `seed_chunk_direct_lighting` e enfileira chunk relaxation, mas snapshot/mesh inicial pode ocorrer antes de luz entre vizinhos convergir. `lighting_updates.rs` budget 2 ms / até 4096 voxels, remesh despacha até 2 tasks/frame, 4 in-flight e integra até 2/frame. Initial light bump no VoxelWorld não implica bump do tracker lighting_revisions do ChunkRemeshTasks, que recebe bump no dynamic lighting quando changed_chunks publicados. Investigar stale mesh de luz, ausência de propagação, remesh scheduling e integração; separar chunk realmente sem skylight de mesh escura persistente. Não declarar concluído por CI.

### Hidrologia — túnel versus rio e rios soltos

`world/generation/density.rs` combina `sample_density_with_hydrology` + `surface_carver_density_delta` e water factor. Código 0.15.13 evita custo redundante, mas o fator original tem apenas fade em profundidade: se o voxel está acima do leito, proteção segue ativa até alturas arbitrárias. Este é um defeito concreto na fórmula que pode explicar parede reta no túnel; corrigir com janela vertical limitada à água/roof e fade no topo, além de inspecionar `density_sampling/hydrology.rs::enforce_hydrology_water_volume` e se diferentes carvers (surface tunnel e cave connector) divergem. Registrar patch só quando commit existir, gameplay ainda necessário.

`world/hydrology/river/selection.rs::keep_only_complete_downstream_paths` avalia destinos por wet ocean/lake e estende cells selecionadas; `river.rs::build_river_system` constrói edges por região em loop com raio `RIVER_EDGE_MARGIN_CELLS=4`, aplica filtro de reachability e `selected.channels`. `river/path/confluence.rs::add_path_to_graph` só inclui segmentos que intersectam a região e `hydrology/spatial.rs::edge_intersects_region` usa margem máxima do rio. Investigar discrepância de flow cache entre regiões, canais fora de seleção local, path/edge clipping, confluências e water fill; wet outlet lógico não garante ligação visual. `river_height` também trata continentalness abaixo do threshold como `seaLevel`, embora o wet outlet use critério físico; checar descontinuidade. Não estender rios arbitrariamente nem aumentar margens apenas.

P0.5 margens verticais: **corrigido confirmado pelo usuário**, preservar 0.15.12; não reabrir como problema ativo sem nova ocorrência.

### Distribuição dos biomas

`world/biome_field/selection.rs` amostra suitability em temperatura/humidade/continentalness/erosion, pesos só dos elegíveis; seleção restringe candidatos pelo vizinho dominante e `avoidNear` em 8 vizinhos, fallback ao raw. `surface.rs` associa amostras de site Voronoi e aplica overlay Mountains via mountain belt/peak independente da seleção regional. BiomeField `surface_site_spacing` deriva do maior raio regional, não do individual. Overworld `dimension.json` já tem pesos Wasteland 1.30, Witchwood/Enchanted 1.25, Plains 1.0, Mountains macro .85. Usuário andou 3000 blocos encontrando praticamente Plains/Mountains: inspecionar distribuição real por seed, seleção/caches e proximidade; não insistir em mudar pesos sem causa identificada. Este item P0.6 continua **aberto**, independentemente da confirmação agregada P1.

### Cores do céu e fog

`rendering/environment.rs::update_environment_visuals` calcula HSI sky_color/fog_color com pesos de `CurrentBiome` e fases day/night se inputs mudaram. `world/biome.rs::track_current_biome` usa player position e muda CurrentBiome só se amostra difere; `world/biome/identity.rs` mistura surface/hydrology/volume. `rendering/sky.rs` escreve `ClearColor` com `visuals.sky_color` se recurso changed. `rendering/fog/color.rs` e `fog/attachment.rs` escolhem `visuals.sky_color` para DistanceFog, ignorando `visuals.fog_color`; portanto fog artístico do bioma não está aplicado. Isto sozinho não explica por que sky não varia: rastrear atualizações de CurrentBiome, combinação/influências, origem das definições, registro de visuais, fase e stack de câmera. Substituir DistanceFog por fog_color sem casar seu terminal com sky pode reintroduzir chunk popping/silhuetas; projetar composição horizonte/gradiente mantendo frontier mascarado. Usuário reconfirmou ambos céu/fog sem cor por bioma.

### Coast / celestial / nuvens / histórico concluído

Coast diagnóstico anterior: identidade hydrology overlay usa continentalness raw `BiomeField::climate_at`, enquanto oceano físico usa macro sample interpolado 5×5 `HydrologyRegion::macro_sample_at`; material rejeita ocean bed acima seaLevel+3 sem que a identidade Coast considere altura. P1 agregado foi reportado concluído pelo usuário; preservar diagnóstico como histórico, reabrir apenas mediante novo relato.

Celestial `rendering/celestial.rs` utiliza `CelestialBodyDefinition.texture` como base_color_texture; Overworld tem sun.png/moon.png. Clouds world-space Y seaLevel relativo e tile recycling; P1 agregado concluído por feedback.

P2 agregado reportado concluído pelo usuário: held block observa hotbar e esconde slot vazio; ghost block transparência uniforme por bloco; dye intensidade; HUD planejamento jogador canto inferior esquerdo, Yogg'Sara, vida 50/100 dentro da barra, status acima, target HUD acima da crosshair, bloco à esquerda estilo inventory e texto à direita tooltip shadow. **Novo bug independente:** tecla R de rotação do bloco distorce o modelo na mão; inspecionar hotkey, estado da orientação da colocação, transform da entidade held block/viewmodel e rotação de UV/mesh, evitar alterações acumuladas na geometria ou escala.

---

# Próxima execução

1. Tratar P0.3 em patch pequeno: correção limitada da proteção hídrica vertical, teste de fator acima/no/leito/abaixo da água e CI; **não dizer que o túnel está correto sem gameplay**. Se feedback indicar barreira persistente, seguir composição de density e carvers.
2. P0.4: alinhar caminho completo do rio entre seleção/grafo/região/água física, manter margem corrigida, testar destinos e edges; revisão específica do renderer se fluido falta.
3. P0.2: sincronizar inicialização e freshness da iluminação, evitar remesh stale e custo de main thread; FPS continua indicador obrigatório.
4. P0.6 e P0.7: seleção real de biomas e pipeline de cores do ambiente, cada um em commit próprio.
5. Novo bug R/viewmodel e otimizações adicionais de FPS baseadas em diagnóstico; preservar P0.5, P1 e P2 conforme relatados concluídos.

Todo novo bloco de código incrementa `VERSION` e é seguido por atualização do HANDOFF. Feedback do usuário reordena imediatamente estas prioridades.