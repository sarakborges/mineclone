# HANDOFF — Asteria / Mineclone

Repositório `sarakborges/mineclone`; branch de trabalho **`develop`**; Rust + Bevy **0.19.1**. Este arquivo é a fonte canônica persistente, atualizar NA MESMA SESSÃO de toda mudança material. Snapshot integral da `.46` anterior ao bloco atual: [HANDOFF 0.15.46](https://github.com/sarakborges/mineclone/blob/c778fee85c9b4cbc10a38be9e413bfda9174eaab/HANDOFF.md). Histórico expandido integral e links de `.31–.45`: [HANDOFF 0.15.45](https://github.com/sarakborges/mineclone/blob/46df141fe88ba7f96977482906362a189abab7e0/HANDOFF.md). Compactação NÃO encerra defeitos.

## Regras obrigatórias

- `continua` / `go` = investigar código real e executar mudanças fundamentadas, não apenas consultar CI ou responder com plano. Trabalhar diretamente em `develop`, sem feature branch; buscar HEAD, VERSION, CI e arquivos reais antes de editar. Commits pequenos/coerentes.
- Mudança material de código ou dados jogáveis exige bump SemVer em `VERSION`: PATCH correção/refactor, MINOR feature compatível, MAJOR quebra de contrato. Documentação isolada não requer bump. `Cargo.toml` 0.10.16 é independente: NÃO atualizar junto com `VERSION`.
- Atualizar HANDOFF na mesma sessão, registrar SHAs, versão, CI observada, status dos nove bugs e próximo trabalho. Nunca afirmar publicação antes do SHA confirmado; não declarar bug visual resolvido sem runtime do usuário.
- CI contém apenas `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`. Não roda testes unitários, fmt, gameplay ou medição de FPS. Corrigir erros/warnings reais sem silenciá-los; não repetir disclaimers de cargo local, não prometer trabalho em background.
- Preservar assets originais; não gerar imagens sem pedido; não introduzir mudanças arquiteturais alheias ao bloco. Comunicar resultados concretos e bloqueios precisamente.

## Arquitetura

`ARCHITECTURE.md` é autoritativo: um owner por fato, `SystemParam`s por domínio, UI em `src/ui`, `TargetedBlock` autoritativo, change-driven, filas deduplicadas `DeduplicatedQueue<T>`/`VoxelUpdateQueue` e `FrameWorkBudget`, sem scans globais/dirty writes redundantes. Streaming visible/preload → geração async → integração e seed de luz imediatamente uma vez por residência (`.38`) → halo 3×3×3 → mesh async → render → archive/unload; snapshots revisionados, stale reencaminhado, main thread budgetada. Geometria/fluido/luz têm filas e meshes separadas; `ChunkRemeshTasks::lighting_revisions` ≠ `VoxelWorld::chunk_mesh_revisions`; reconciliação halo `.33`, unload 26 vizinhos `.40`, iluminação por lote `.41`. Render HDR world order0 Skip, viewmodel order1 tonemap, UI SDR order2. Céu por bioma/relógio `.24`, fogColor artístico pendente, sol/lua em órbita e nuvens world-space. Hidrologia usa endpoints XYZ compartilhados, margem global `.12`, fontes naturais autoritativas fora solver; NÃO gerar água suspensa. Held block segue hotbar; R só placement.

## Estado confirmado em 16/09/2026 — 0.15.47

**VERSION = 0.15.47.** HEAD confirmado após código+bump e antes desta atualização documental: `8b19dbf811417dc201664fa17e9d398a7bf61af6`. Código `.47`: `0d1b34e17533173a36b4b7dc83b8aac0997fd0a7`; bump: `8b19dbf811417dc201664fa17e9d398a7bf61af6`. O commit do presente HANDOFF é exclusivamente documental e NÃO exige novo bump. Consultar HEAD novamente antes do próximo write.

**CI `.46` CONFIRMADA SUCCESS** em 16/09, run push `35152291521`, commit `b3b7798826b9088540f22c4c9e2bd3a962fc4279`, conclusão `success`; corrigir a informação `in_progress` do handoff `.46`, que ficou desatualizada. `.45` run `35149581610` success, `.44` `35148263437` success, `.43` `35147277012`, `.42` `35146865609`, `.41` `35144120702`, `.40` `35143134455`, `.39` `35142554264`, `.38` `35141651000`, `.37` `35140172992` success. **CI `.47` ainda não verificada na redação deste handoff**; verificar run do commit `8b19dbf...` e, se falhar, inspecionar job e corrigir. Os testes unitários descritos não são executados pelo workflow. Não há confirmação de gameplay ou FPS.

### .47 — cache de conectividade de rios: negativo inconclusivo não é fato

`src/world/hydrology/river/selection.rs` `cache_reachability` inseria `start -> false` quando `drainage_reaches_water_destination` atingia `RIVER_FLOW_TRACE_STEPS` (64), mesmo sem provar beco sem saída. O limite prova apenas que a saída não foi encontrada NESTE percurso; o valor falso persistia no cache compartilhado de `build_river_system` e podia sobrepor uma futura evidência positiva obtida a partir de uma célula downstream. Agora trace esgotado NÃO grava nada; false ainda é cacheado quando o traço realmente termina sem downstream, true quando alcança destino. Removido parâmetro `start` redundante de `cache_reachability`. Atualizado teste `exhausted_trace_does_not_cache_an_unproven_dead_end`: cache fica vazio após limite e resultado positivo posterior pode ser registrado sem negativo anterior; preservado `proven_destination_or_dead_end_caches_complete_path`. Testes escritos, não executados; NÃO afirmar que rios end-to-end foram corrigidos. `build_flow_cache`, raio, threshold, loop de path completo, terreno e água física não foram modificados.

### Histórico recente preservado

**.46** `water.rs` fix `0aaceed068866e64830e7ac9cd20265b33543934`, bump `b3b7798826b9088540f22c4c9e2bd3a962fc4279`: fonte oceânica física passa a usar superfície original na interpolação de fundo quando disponível, igual a escavação `density_column_profile`; fallback macro mantido para consultas genéricas. Teste determinístico de macro 120/superfície 80 verifica fonte física oceânica e que a consulta genérica permanece seca. CI success, gameplay não confirmado. **.45** `water.rs` `8c9df7f0a21a57b9bdadd57dfb7c12757ae5886f`, `density_sampling/hydrology.rs` `71d6969032bf951d8cc864d2487facad1017488e`, `region/density.rs` `398a5ca520759cd85016e9bd1073ecb083b86f9a`, bump `96acecc45098948686a8eecad80ddc8e843a0788`: `bed_has_support` harmoniza carving e headroom com leitos suportados; CI success. `.44` densidade do volume molhado escolhe `supported_water_at`; `.43` filtra cada fonte física antes de escolher nível; `.42` prioriza foz oceânica molhada; `.41` revisões luz por lote; `.40` unload halo 26; `.39` sky provisório Top; `.38` seed luz ao integrar; `.37` margem raio*2.5; `.36` perfil compartilhado; `.35` mountain belt mais estreito. Detalhes integrais e links no snapshot `.46` e histórico `.45`. ÚNICO bug de gameplay confirmado como encerrado pelo usuário neste ciclo: céu por bioma `.24`.

## Nove defeitos relatados em 16/09 — TODOS ABERTOS até confirmação

| # | Prio | Sintoma e evidência ainda necessária |
|---|---|---|
| 1 | P0 | Chunks pretos/costuras iluminação/AO: `.33/.38/.39/.40/.41` trataram mecanismos sem confirmação. Distinguir transiente/persistência e medir FPS. |
| 2 | P0 | Rios terminam no nada: `.42–.46` aproximaram seleção/carving/fluidos; `.47` corrige cache negativo inconclusivo, NÃO prova conectividade. Testar grafo→water_at→density→fluidos entre DUAS regiões com seed/coords reproduzíveis. |
| 3 | P0 | Percurso de 2000 blocos só Plains/Mountains: `.35` belt estreito não confirmado. Medir `BiomeField::sample_surface().primary_id` vs aparência e várias seeds. |
| 4 | P0 | Margem em descida afunda antes de ter água: `.36/.37/.43–.46` parciais. Preservar margem global `.12`, nunca água flutuante. |
| 5 | P1 | Confluência rio–lago abrupta: suavizar geometria XY/largura/profundidade/nível mantendo endpoints. |
| 6 | P1 | Túneis com paredes retas na abertura de superfície e interseção rio–túnel. |
| 7 | P1 | Lua parece seguir câmera: `celestial.rs` usa camera_position + offset; estudar referencial sem mudar órbita sem teste. |
| 8 | P1 | Lua visível de dia: `.32` setPhase Night sem confirmação visual. |
| 9 | P1 | Highlight não cobre todas as camadas de textura: `targeting/highlight.rs` `Cuboid` branco, `HIGHLIGHT_SCALE=1.01`, `AlphaMode::Blend`; conferir overlay/bounds/alpha. |

Outros pendentes: fogColor artístico, FPS baixo em caminhada/streaming sem benchmark, R/held `.18` sem validação. HUD planejado: avatar placeholder `Yogg'Sara` no canto inferior esquerdo, vida `50 / 100`, status acima; target acima da mira, ícone slot à esquerda e texto com sombra à direita. Feedback histórico preservado: céu `.24`, margem global `.12`, hotbar vazia, ghost alpha uniforme, dye e HUD/fog anteriores.

## Investigações e próximos passos executáveis

1. Conferir HEAD, CI `.47` e warnings; se CI falhar, investigar e corrigir antes de expandir. CI não substitui teste unitário ou runtime.
2. P0 rios: criar teste determinístico com arestas sobrepostas na mesma coluna. `FeatureGraph::sample_horizontal_expanded` escolhe maior `strength` ANTES do `bed_has_support` em `region/density.rs`; `water.rs::river_water_with_margin` também escolhe um único segmento antes do filtro de suporte final. Uma aresta alta sem leito pode ocultar outra suportada. **Hipótese auditada, sem teste/reprodução ainda; não publicar alteração geométrica às cegas.** Preservar consultas genéricas `water_at`/`water_near`, selecionar candidato físico suportado por aresta apenas se teste comprovar. Depois testar a cadeia completa cruzando regiões e bacias remotas.
3. Ainda em rios: `build_flow_cache` e `keep_only_complete_downstream_paths` usam limite de 64; `.47` só muda o cache de `drainage_reaches_water_destination`. `drainage.rs` prioriza oceano molhado raio quatro (~512 blocos de domínio). Não aumentar raio/cap arbitrariamente sem caso determinístico e medição de custo.
4. P0 chunks pretos: auditar revisões/prioridades `chunk_remesh.rs`, snapshots e `lighting_updates`. Vizinhos cardinais podem agendar remesh sem conteúdo perto da borda; otimizar via contadores O(1) só com prova de segurança/regressão. P0 biomas: aferir IDs vs visual ao longo de 2000 blocos e várias seeds antes de alterar pesos. P1 túneis, lua, highlight depois.
5. Auditoria residual: custo de oceano raio4/FPS sem afirmar benchmark; `src/world/biome_field/selection.rs` tem `dominant_neighbor_biome`/`proximity_allows`; `surface.rs` mescla macro montanhas; dimensão declara Plains/Wasteland/Witchwood/Enchanted Forest/Mountains. `highlight.rs` é cubo translúcido independente de camadas. Não reabrir bugs antigos automaticamente e não declarar visual resolvido sem usuário.

Cada bloco novo: código real → testes de regressão quando cabíveis → bump `VERSION` → HANDOFF atualizado e SHA publicado.