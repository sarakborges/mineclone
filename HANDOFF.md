# HANDOFF — Asteria / Mineclone

Repo `sarakborges/mineclone`, trabalho direto em `develop`, Rust + Bevy 0.19.1. **Este documento é a fonte canônica persistente**; anexos são snapshots. Históricos anteriores completos: [HANDOFF pré-.31](https://github.com/sarakborges/mineclone/blob/72b60b5a4e70a9960b3108debdf77d7a8510c4b1/HANDOFF.md) e [HANDOFF detalhado .31 e feedback do usuário](https://github.com/sarakborges/mineclone/blob/6b8051628aa49c3b01ffca782144b8fc84705365/HANDOFF.md). Não reinterpretar compactação como encerramento de defeitos.

## Regras de execução

- `go`/`continua` = executar, não apenas planejar. Ler HEAD, `VERSION` operacional e arquivos reais antes de editar; trabalhar diretamente em `develop`, sem branches não solicitadas. Commits coerentes, VERSION sobe a cada bloco de código/dados de jogo: PATCH fix/refactor compatível, MINOR feature, MAJOR quebra de contrato; atualização só documental não sobe versão. `Cargo.toml` 0.10.16 deliberadamente defasado.
- Atualizar ESTE HANDOFF na mesma sessão de toda alteração material de código, versão, roadmap, arquitetura, processo ou feedback. Distinguir achado, código publicado, CI aprovado, testes unitários executados e gameplay confirmado. Nunca considerar Clippy/check prova de resultado visual.
- Corrigir erros e warnings reais dos arquivos tocados antes de avançar. CI canônico: `cargo clippy --all-targets --all-features -- -D warnings` + `cargo check`; CI não executa unit tests; `fmt` não é gate. Não repetir ausência de cargo local nem esperar o usuário rodar `cargo run`. Teste manual só quando usuário solicitar.
- Não gerar imagens sem solicitação. Assets binários do usuário devem permanecer exatos.

## Invariantes arquiteturais

`ARCHITECTURE.md` é canon: um owner autoritativo por fato; SystemParams por domínio, UI em `src/ui`, targeting único `TargetedBlock`, HUD/sistemas change-driven; compartilhar invariants reais, não formatos superficiais. Usar `DeduplicatedQueue<T>`/`VoxelUpdateQueue` e `FrameWorkBudget`; evitar full scans, alocações, dirty writes e cópias de estado nos hot paths. Pipeline streaming: selection/preload -> geração async -> integração -> seed de luz direta -> snapshot halo 3×3×3 -> mesh task -> render -> visibility -> retention -> archive/unload. Visible supera preload; preempção devolve tarefa não crítica; stale tasks descartadas/reagendadas por revisões sem starvation. Seed direta UMA vez por residência, jamais retry. Mesh inicial deve aparecer rapidamente mas atualizar halo se vizinho/luz mudar durante task. Terrain e Fluid são meshes e filas distintas, e revisionamento de iluminação de remesh (`ChunkRemeshTasks`) difere de mesh revision de `VoxelWorld`; cada output relevante valida conteúdo e iluminação.

HDR: world order0 Skip, viewmodel HDR order1 tonemap final, UI SDR order2. Céu por bioma/relógio `EnvironmentVisualState.sky_color` -> `SkyPlugin` ClearColor EXATO; cálculo visual após track de bioma no mesmo frame. Fog artístico `fogColor` separado ainda não aplicado; com ClearColor plano fog terminal deve convergir para sky, não colorir céu artificialmente. Nuvens world-space, vento independente, seaLevel por dimensão. Celestial rise/set e sequência/fases sempre data-driven por dimensão; não impor órbita ou relógio comum sol/lua. Hidrologia: nós incidentes de rios compartilham X/Z/Y, endpoints autoritativos, regiões determinísticas, margem geral corrigida em .12 é invariante de regressão. Natural water não deve ser enfileirada em massa no solver dinâmico. Held model observa hotbar/visual/tint, R só altera colocação.

## Estado — 16/09/2026

**VERSION operacional = 0.15.32.** Código/dados desta versão: commit `7d00f21a8f56b4b06befc85d846afd2b9601131f` (sky.json + VERSION). HEAD documental posterior: consultar antes de próxima escrita. .31 código `1965466e35683c0775d3a7b79e3bfaf37a49606f`, bump `ea2f9b338c7576e8086e975c7675276434e311ef`; anteriores no histórico linkado.

**0.15.32 — causa documentada e ajuste da lua visível de dia:** `DayNightCycleDefinition::progress_between_phases` inclui TODA a duração da fase de término. No overworld `sequence=[Dawn,Day,Dusk,Night]`, lua `risePhase=Dusk`, `setPhase=Dawn` fazia a lua permanecer visível durante a Dawn inteira (até o começo de Day, meio-dia no relógio configurado). Ajustado só `data/dimensions/overworld/sky.json` moon `setPhase=Night`: fim da janela no começo de Dawn, sem alterar sol, órbita, engine genérica ou fases de outras dimensões. **Código publicado, gameplay NÃO confirmado; defeito da lua parecer seguir câmera permanece ABERTO.** Para este, `src/rendering/celestial.rs::update_celestial_bodies` literalmente põe `camera_position + celestial_offset`, isto explica o referencial de câmera; investigar renderização de esfera celeste/infinito e parallax sem aplicar posição fixa a raio600 que desapareceria após deslocamento grande. Não declarar consertado sem gameplay.

**0.15.31 — correção PARCIAL de iluminação:** `src/world/lighting_updates.rs` passou a notificar 20 diagonais do halo 3×3×3 quando relaxação realmente muda luz, condicionando geometry/fluid às bordas relevantes. Teste unitário adicionado mas não rodado pelo CI. **Usuário confirmou que todos os defeitos abaixo, inclusive chunks pretos e costuras, PERSISTEM após .31.** Não marcar resolvido.

**CI verificado:** .31 run35134931146 job104924817286 COMPLETED/SUCCESS: Clippy+Check SUCCESS, sem unit tests nem gameplay. .24 run35121054358, .25 run35122730177, .26 run35123049209, .27 run35123661593, .28 run35124362037, .29 run35132844975, .30 run35133472565 SUCCESS. .22/.23 falharam antes de fixes. **CI .32 ainda NÃO CONSULTADO/CONFIRMADO nesta atualização; reconsultar.**

**Único feedback visual de encerramento recente:** usuário confirmou **céu por bioma corrigido** em 16/09, pela .24 (ordering biome -> visuals, sky independente do fog). Todo o restante PERSISTE, incluindo `fogColor` artístico pendente. Não inferir confirmação de cor do fog a partir do céu.

### Defeitos de gameplay — nove observações 16/09 ainda abertas

| # | Prioridade | Evidência/critério de aceite |
| --- | --- | --- |
| 1 | P0 | Chunks inteiros pretos e costuras/sombras entre chunks mesmo após .28/.31; inspecionar seed, snapshot, luz halo, revisões e primeiro mesh. Exigir continuidade visual sem piorar FPS. |
| 2 | P0 | Rios morrem no nada. Provar em seeds/coords grafo, leito e água FÍSICA até rio/lago/oceano, atravessando chunks/regiões. .15/.16/.19/.20 não resolveram. |
| 3 | P0 | Percurso de 2.000 blocos revelou só Plains/Mountains; amostrar biome final, seleção regional, suitability, aparência e distribuição; otimização .25 não alterou pesos. |
| 4 | P0 | Em descidas, margem abaixa ANTES do nível da água; medir leito, perfil de margem, nível e fluidos, hipótese de vazamento ainda NÃO comprovada. Preservar margens gerais .12. |
| 5 | P1 | Junção rio-lago abrupta; transição gradual no XY, largura, profundidade, margem e altura; manter endpoints/água física contínua. |
| 6 | P1 | Túneis criam paredes retas na superfície; inspecionar cave connector/density/surface carver, distinto de parede no cruzamento rio-túnel. |
| 7 | P1 | Lua parece seguir câmera; `celestial.rs` usa translação câmera+offset; investigar referencial físico e projeção sem quebrar sky no render distance. |
| 8 | P1 | Lua visível durante o dia: .32 alterou `setPhase` Dawn→Night só no overworld; causa do intervalo identificada, **gameplay após .32 pendente**. |
| 9 | P1 | Highlight do target não inclui todas camadas de textura; examinar bounds, alpha e overlays sem selecionar vizinho. |

Outras pendências persistentes: FPS/carregamento sob andar/voar, sem benchmark confiável de 60 FPS; fogColor artístico; R/held block .18 CI verde mas sem confirmação de gameplay; parede em cruzamento rio-túnel; caves; HUD planejada player inferior esquerdo nome `Yogg'Sara` vida `50 / 100` dentro barra e status acima, target acima crosshair com ícone à esquerda estilo inventory e texto à direita com shadow tooltip. Margens gerais .12, hotbar vazia/ghost alpha/dye/HUD histórica, fog/frontier/nuvens antigas foram informados resolvidos anteriormente; não regredir.

### Iluminação: investigação aberta concreta

1. `src/voxel/mesh_lighting.rs::face_lighting` lê base/lados/cantos do halo 3×3×3. .31 invalidou diagonais SOMENTE ao detectar luz alterada via relaxação.
2. `src/voxel/mesh_snapshot.rs::ChunkMeshDependencies::is_current` ignora vizinho ausente que aparece enquanto mesh inicial está em voo (para preservar time-to-visible). `src/world/streaming.rs::collect_built_chunk_meshes` publica a mesh e chama `notify_loaded_chunk_neighbors`, que hoje é cardinal e NÃO verifica halo/luz novos para a própria mesh publicada. Implementar catch-up após publicação para Geometry+Fluid sem descartar primeiro mesh; detectar vizinho anteriormente ausente e mudança de iluminação na captura, inclusive seed de um chunk que estava carregado mas escuro. Testes de vizinho entre captura/integração e luz em voo. Garantir async revisions e remesh priorities sem starvation/FPS.
3. `notify_loaded_chunk_neighbors` também não notifica renderizados diagonais ao chegar chunk (mesmo vazio), enquanto AO amostra diagonais; adicionar notificação seletiva por borda relevante ou consolidar lógica já existente em .31 para evitar duplicação.
4. Analisar chunk totalmente preto independentemente da costura: `VoxelChunk::empty` nasce DARK, seed só na fila de initial mesh; vizinhos já capturados podem trazer luz escura antes da seed, e `average_shader_light_levels` retorna zero quando nenhum sample vazio/carregado. Não tratar hipótese como prova definitiva; evitar ajustes arbitrários globais de sombra/fog.

### Trabalho anterior a preservar

.15.11 outlet oceano wet; .12 shore grading; .13 lazy cache; .14 protege leito; .15 confluence meander; .16 outlet lago; .17 seed uma vez/residência; .19 endpoints Y; .20 trace limit não cacheia falso; .21 Geometry captura halo luz; .22 Terrain/Fluid filas distintas; .24 sky correto; .25 seleção biome remove alocações sem mudar pesos; .26 SmallVec inline4, .27 uma distância para água+margem, .28 remesh cardinal ao chegar chunk vazio, .29 remove lookup duplicado bloco por face, .30 culling conservador da hidrologia, .31 invalidar diagonais da relaxação, .32 lua setPhase. SHA e CI antigos no handoff histórico.

## Próxima execução (sem aguardar pedido de planejamento)

1. Reconsultar CI .32 e corrigir falhas/warnings antes de avançar.
2. P0 iluminação: catch-up de mesh inicial e notificação diagonal ao chegar chunk, incluindo vazio, seed assíncrona e fluid; preservar first-visible, orçamento por frame e testes. Depois investigar chunk preto residual sem confundir com costura.
3. Provar rios fim-a-fim em seeds/coords; depois biomas reais vs aparência; depois margens em descida/river-lake/tunnels, movimento lunar e highlight. Não declarar visual resolvido sem usuário comprovar.
4. Todo bloco de código/dados: bump VERSION e atualizar HANDOFF mesma sessão; registrar commits e CI estado exato.