# HANDOFF — Asteria / Mineclone

Repositório `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy **0.19.1**. Este arquivo é a fonte canônica persistente: atualizar NA MESMA SESSÃO de toda mudança material. Preservar histórico e bugs via snapshots: [HANDOFF 0.15.48](https://github.com/sarakborges/mineclone/blob/8cdebad5fabbe55b93a2bfa45f4fd9e93dbf4739/HANDOFF.md), [0.15.47](https://github.com/sarakborges/mineclone/blob/457de45c0dc04e40a52ef6f5d567444e2b4f30da/HANDOFF.md), [0.15.46](https://github.com/sarakborges/mineclone/blob/c778fee85c9b4cbc10a38be9e413bfda9174eaab/HANDOFF.md), [0.15.45 e links históricos .31–.44](https://github.com/sarakborges/mineclone/blob/46df141fe88ba7f96977482906362a189abab7e0/HANDOFF.md). Compactação NÃO encerra defeitos.

## Regras obrigatórias

- `continua` / `go` / `continua até não conseguir mais`: investigar arquivos reais, produzir código/testes fundamentados quando viável, não somente consultar CI ou responder com planos; trabalhar diretamente na `develop`, sem feature branch, verificando HEAD, VERSION e CI antes de editar. Commits pequenos/coerentes.
- Mudança de código ou dados jogáveis = bump `VERSION` SemVer (PATCH fix/refactor/testes, MINOR feature compatível, MAJOR quebra). `Cargo.toml` 0.10.16 é versão independente, NÃO incrementar. Docs isoladas não requerem bump. Atualizar este HANDOFF NA MESMA SESSÃO com versão, SHAs, observações de CI, nove defeitos e próximos passos.
- CI somente `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`; não executa testes unitários, fmt, gameplay ou FPS. Corrigir erros/warnings sem silenciar. Não repetir disclaimers de cargo local, prometer background, ou afirmar correção visual sem confirmação.
- Preservar assets, não gerar imagens sem pedido, não criar arquitetura lateral desnecessária; informar concretamente o que foi publicado, não anunciar commit antes do SHA.

## Arquitetura / invariantes

`ARCHITECTURE.md` autoritativo: ownership único por fato, `SystemParam`s por domínio, UI em `src/ui`, `TargetedBlock` autoritativo, trabalho change-driven, filas deduplicadas `DeduplicatedQueue<T>`/`VoxelUpdateQueue` e `FrameWorkBudget`; evitar scans globais/dirty writes repetidos. Streaming visible/preload → geração async → integração e seed luz uma vez/residência `.38` → halo 3×3×3 → mesh async → render → archive/unload; snapshots revisionados, stale reencaminhado. Geometria/fluido/luz têm filas e meshes separadas; `ChunkRemeshTasks::lighting_revisions` ≠ `VoxelWorld::chunk_mesh_revisions`; halo reconciliado `.33`, unload atualiza 26 vizinhos `.40`, iluminação por lote `.41`. HDR world order0 Skip, viewmodel order1 tonemap, UI SDR order2. Céu por bioma/relógio `.24`, fogColor artístico pendente, sol/lua em órbita e nuvens world-space. Hidrologia usa endpoints XYZ compartilhados e margem global `.12`, fontes naturais fora solver; NÃO criar água suspensa. Held block segue hotbar e R só placement.

## Estado em 16/09/2026 — versão 0.15.49

**VERSION = 0.15.49.** HEAD depois do teste+bump e antes deste commit documental: `497e1ff1e77afd259b5c7b0c13c5a55b315aba0b`. Teste `.49` `src/world/hydrology/river/path.rs` commit `4e733ea0b6d06107019afd50bb1175e5e0d9ce4a`; bump `497e1ff1e77afd259b5c7b0c13c5a55b315aba0b`. O presente HANDOFF altera só documentação: não incrementar VERSION. Consultar novo HEAD para qualquer próxima escrita.

**CI .47 CONFIRMADA SUCCESS:** push run `35153109195`, commit `8b19dbf...`, 16/09 21:38 UTC. CI .46 run `35152291521` success, .45 `35149581610` success, .44 `35148263437` success; .43 `35147277012`, .42 `35146865609`, .41 `35144120702`, .40 `35143134455`, .39 `35142554264`, .38 `35141651000`, .37 `35140172992` success. **CI .48 run push `35153673196`, commit `5c63703...`: na última consulta status `in_progress`, conclusão nula; NÃO afirmar success/failure sem consultar novamente. CI .49 push do commit de bump deve ser localizada e verificada.** CI NÃO roda testes unitários; nenhum novo teste foi executado localmente, sem confirmação de gameplay/FPS.

### .49 — regressão adicional de costura entre duas regiões

`src/world/hydrology/river/path.rs` amplia teste existente `neighboring_regions_reproduce_identical_water_height_at_the_same_river_crossing`: seed 42, origem `(64,64)`, destino `(192,64)`, elevação 100→90, mar 64, terreno sintético 120. Dois grafos gerados com `add_curved_river_edge`, um para `IVec2::ZERO` e outro `IVec2::X`, cruzamento x=128 derivado do `river_path`. Além da igualdade do sample do grafo, constrói duas `HydrologyRegion` e verifica `supported_water_at` (water_level e bed_level) e `density_deltas_for_column` (igualdade e escavação negativa) em x=127.5, 128.0 e 128.5 na coordenada z do cruzamento. Teste **escrito e commitado, não executado**. Ele cobre grafos/sampling/carving de uma aresta sintética em ambas regiões; NÃO exercita `build_river_system` completo, nem `rasterize_fluid_pass`/registro efetivo de fluidos, nem dezenas de bacias; não chamar de end-to-end concluído. A lógica produtiva de geração não foi alterada nesta versão.

### .48 — sobreposição de segmentos: filtro físico antes da seleção

`feature_graph.rs` `3e96e19de6436225728a0e1dced5f41da12980e2` introduz `sample_horizontal_filtered`, aplica predicado por aresta antes do ranking, mantém função genérica usando sempre `true`; teste do sample elegível mais fraco. `region/water.rs` `eefb498b6283032811f3bc720122b9d6f2e5d519` passa superfície original para filtragem física por segmento, compartilha seleção entre `supported_water_at` e `supported_river_surface_at`, preserva `water_at`/`water_near`/`river_surface_at` genéricas. Novo teste de aresta suspensa 100 e outra suportada 86, superfície original 85, duas ordens, compara fonte/headroom/carving com a superfície baixa 60. `region/density.rs` `3ea0f38e150d9553e24b9b87a3c56f921f7b09bd` filtra cada candidata quando altura original conhecida; consultas genéricas usam amostragem anterior; deixa shore seco `profile <= 0` elegível. Bump `5c63703b51a72ab003fcdc98625eda53b5077706`. Diff da `.47` até bump `.48` conferido: três `.rs` e VERSION, quatro commits. Testes escritos, não executados. Corrige mecanismo demonstrável de seleção física, não prova bug visual de rios globalmente solucionado.

### Histórico recente

**.47:** código `selection.rs` `0d1b34e17533173a36b4b7dc83b8aac0997fd0a7`, bump `8b19dbf811417dc201664fa17e9d398a7bf61af6`, docs `457de45c0dc04e40a52ef6f5d567444e2b4f30da`: esgotar rastreio de 64 passos não registra falso negativo inconclusivo no cache de conectividade; destino verdadeiro e dead end real continuam cacheados, testes escritos. CI success. **.46:** `water.rs` `0aaceed068866e64830e7ac9cd20265b33543934`, bump `b3b7798826b9088540f22c4c9e2bd3a962fc4279`: água oceânica física usa superfície original na interpolação do fundo, consistente com densidade, fallback macro para consulta genérica; CI success. **.45:** `water.rs` `8c9df7f0a21a57b9bdadd57dfb7c12757ae5886f`, `density_sampling/hydrology.rs` `71d6969032bf951d8cc864d2487facad1017488e`, `region/density.rs` `398a5ca520759cd85016e9bd1073ecb083b86f9a`, bump `96acecc45098948686a8eecad80ddc8e843a0788`: `bed_has_support` harmoniza carving/headroom com leitos; CI success. `.44` densidade do volume molhado escolhe `supported_water_at`; `.43` filtra cada fonte antes do nível; `.42` prioriza foz oceânica molhada; `.41` revisões luz por lote; `.40` unload halo 26; `.39` sky provisório Top; `.38` seed luz ao integrar; `.37` margem raio*2.5; `.36` perfil compartilhado; `.35` mountain belt estreito. Links e detalhes nos snapshots `.48/.47/.46/.45`. ÚNICO bug de gameplay confirmado como encerrado pelo usuário no ciclo: céu por bioma `.24`.

## Nove defeitos relatados em 16/09 — TODOS ABERTOS até confirmação

| # | Prio | Sintoma / evidência ainda necessária |
|---|---|---|
| 1 | P0 | Chunks inteiros pretos/costuras de iluminação/AO: `.33/.38/.39/.40/.41` mecanismos parciais. Distinguir transiente/persistência e medir FPS. |
| 2 | P0 | Rios terminam no nada: `.42–.46` seleção/carving/fluidos, `.47` cache inconclusivo, `.48` sobreposição, `.49` teste sintético de costura. Falta teste REAL grafo→seleção→densidade→`rasterize_fluid_pass` em duas regiões, bacias remotas e runtime. |
| 3 | P0 | Percurso 2000 blocos só Plains/Mountains: `.35` belt estreito não confirmado. Medir `BiomeField::sample_surface().primary_id` vs aparência e várias seeds. |
| 4 | P0 | Margem em descida afunda antes de existir água: `.36/.37/.43–.48` parciais. Preservar margem global `.12`, nunca água flutuante. |
| 5 | P1 | Confluência rio–lago abrupta: suavização XY/largura/profundidade/nível, endpoints autoritativos. |
| 6 | P1 | Túneis com paredes retas na abertura da superfície e interseção rio–túnel. |
| 7 | P1 | Lua parece seguir câmera: `celestial.rs` usa camera_position + offset, estudar referencial sem mudança cega de órbita. |
| 8 | P1 | Lua visível de dia: `.32` setPhase Night ainda não confirmado visualmente. |
| 9 | P1 | Highlight alvo não cobre todas camadas: `targeting/highlight.rs` `Cuboid` branco, scale 1.01, AlphaMode Blend; avaliar bounds/overlay. |

Outros pendentes: fogColor artístico, FPS baixo em caminhada/streaming sem benchmark, R/held `.18` sem validação. HUD planejado: avatar placeholder `Yogg'Sara` inferior esquerdo, `50 / 100` na vida, status acima; target acima da mira, ícone slot esquerda e texto sombra direita. Feedback histórico: céu `.24`, margem global `.12`, hotbar vazia, ghost alpha uniforme, dye e HUD/fog anteriores.

## Próximos passos executáveis

1. Consultar HEAD e CI `.48` `35153673196` e localizar `.49` push; se falhar, ler logs do job e corrigir erros/warnings com novo bump/handoff. Testes unitários não são executados por CI. Atenção: teste `.49` adicionado mas nunca executado.
2. P0 rios: construir teste genuíno de `build_river_system` por seed/coords e `rasterize_fluid_pass` em duas regiões; validar fluido efetivamente colocado em blocos, não apenas elegibilidade `supported_water_at`. Revisar `river/selection.rs`: `build_flow_cache`, `keep_only_complete_downstream_paths` mantêm limite 64; `drainage.rs` oceano raio4 (~512 blocos), medir custo antes de ampliar.
3. P0 iluminação: auditar `chunk_remesh.rs`, `chunk_remesh_tasks.rs`, snapshots halo e `lighting_updates.rs`; não otimizar remesh cardinal sem prova de fronteira e teste. Não aumentar brilho global para esconder chunks pretos.
4. P0 biomas: IDs vs aparência por 2000 blocos, múltiplas seeds; `biome_field/selection.rs` usa `dominant_neighbor_biome` e `proximity_allows`, `surface.rs` mescla macro montanhas, dimensão declara Plains/Wasteland/Witchwood/Enchanted Forest/Mountains. Não mudar pesos sem medições. Depois P1 túneis, lua, highlight.
5. `.48` mantém shore seco `profile<=0` elegível quando superfície presente. No algoritmo de raio uniforme 2.5 e margem zero a strength diminui com normalized_distance, então uma amostra de core (<0.75) é mais forte que bank (>=0.75); não afirmar falha de core por bank sem exemplo reproduzível. Auditoria FPS/benefício da seleção por aresta pendente.

Cada bloco futuro: código real → regressão quando cabível → bump VERSION → HANDOFF e SHA.