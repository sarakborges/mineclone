# HANDOFF — Asteria / Mineclone

Repositório `sarakborges/mineclone`, branch de trabalho **`develop`**, Rust + Bevy **0.19.1**. Fonte canônica persistente: atualizar na MESMA sessão de toda mudança material. Histórico completo e todos os detalhes anteriores em [HANDOFF 0.15.49](https://github.com/sarakborges/mineclone/blob/98d5e1b181e3a8b317d24d90195a3495f61d53d4/HANDOFF.md), [0.15.48](https://github.com/sarakborges/mineclone/blob/8cdebad5fabbe55b93a2bfa45f4fd9e93dbf4739/HANDOFF.md), [0.15.47](https://github.com/sarakborges/mineclone/blob/457de45c0dc04e40a52ef6f5d567444e2b4f30da/HANDOFF.md), [0.15.46](https://github.com/sarakborges/mineclone/blob/c778fee85c9b4cbc10a38be9e413bfda9174eaab/HANDOFF.md) e [0.15.45 + links .31–.44](https://github.com/sarakborges/mineclone/blob/46df141fe88ba7f96977482906362a189abab7e0/HANDOFF.md). Compactação não fecha defeitos.

## Regras obrigatórias

- `continua`/`go`/`continua até não conseguir mais`: conferir HEAD, VERSION, arquivos reais e CI; investigar e fazer código/testes fundamentados na `develop` diretamente, commits pequenos; não limitar resposta a plano/CI.
- Código/dados jogáveis/testes relevantes exigem bump `VERSION` SemVer: patch fix/refactor/testes, minor feature compatível, major quebra. `Cargo.toml` `0.10.16` é independente e NÃO deve ser alterado para espelhar VERSION. Docs isoladas não exigem bump.
- Atualizar este HANDOFF na mesma sessão: versão, HEAD, commits e status real da CI, mudanças, limites de testes, 9 bugs e próximo passo. Verificar HEAD antes de qualquer nova escrita.
- CI `.github/workflows/ci.yml` executa **Clippy `--all-targets --all-features -D warnings` e cargo check**. NÃO executa `cargo test`, fmt, gameplay ou benchmark. Não dizer que teste escrito passou só porque a CI passou; corrigir erros/warnings se aparecerem.
- Ownership em `ARCHITECTURE.md`; não criar arquitetura paralela ou mudar gameplay sem causa reproduzível. Assets do usuário devem ser preservados, não gerar imagens sem pedido. Sem execução em background nem promessa de entregar depois.

## Invariantes arquiteturais

`ARCHITECTURE.md` autoritativo: ownership único por fato, `SystemParam`s por domínio, UI em `src/ui`, `TargetedBlock` autoritativo, sistemas change-driven, `DeduplicatedQueue<T>`/`VoxelUpdateQueue`, `FrameWorkBudget` e evitar scans globais. Pipeline: streaming visible/preload → geração async → integração+seed luz por residência `.38` → snapshot halo 3×3×3 → mesh async → render → archive/unload. Resultados revisionados; stale reencaminhado; geometry/fluid/lighting separados. Iluminação em lote `.41`, unload halo 26 vizinhos `.40`, HDR world order0 Skip/viewmodel order1 tonemap/UI SDR order2. Céu por bioma/relógio `.24`, lua/sol em órbita e nuvens world-space. Hidrologia usa endpoints XYZ compartilhados e margem global `.12`; proibir água flutuante. Held block segue hotbar, R só placement.

## Estado confirmado em 16/09/2026 — v0.15.50

**VERSION `0.15.50`**: bump commit `0a10707189d6b1c51289727e187f98ca288a6966`. Último commit de código registrado ANTES deste handoff: `599d76791d6048f887d7c69d842645c74b74de1f`. Este documento gera HEAD posterior; consultar branch antes de nova escrita.

**CI .49 sucesso**: push `35154000629`, SHA de bump `497e1ff1e77afd259b5c7b0c13c5a55b315aba0b`, Clippy e Check concluídos com sucesso. **CI .48 sucesso**: push `35153673196`. **CI .50 no último acesso**: push run `35154962840`, SHA de bump `0a10707`, status `in_progress` (conclusão nula); a execução referente ao commit corretivo `599d767` deve ser localizada/verificada separadamente. Não reportar CI .50 sucesso sem confirmar. CI não roda asserts dos testes. Versões `.47` a `.37` com CI success constam no snapshot `.49`.

### .50 — teste de `build_river_system` até `rasterize_fluid_pass` na costura

`src/world/generation/fluids.rs`: commits `77f0b67ff0758ff493dd5e0d16d4a19e9685428a` (teste), `e0dbf7f0219e8b888994c41ff03d79f987ca00b0` (tipos assinados da API `fluid_at`) e `599d76791d6048f887d7c69d842645c74b74de1f` (preserva Z negativo no scan). Versão patch `.50`, bump `0a10707`.

Novo teste `generated_river_crossing_places_sources_on_both_sides_of_region_seam`: constrói duas regiões com `HydrologyField::region_from_macro_terrain` e seed 42, declive continental sintético até x<256 e oceano molhado depois; procura amostras River fisicamente elegíveis em x=127.5 e x=128.5, no mesmo Z; determina um voxel interior ao leito nas duas regiões. Usa `FluidRegistry` com o JSON de água REAL, `GenerationRegion`, `GenerationColumnSample`, campo de densidade **sintético totalmente vazio** e chama `rasterize_fluid_pass` para os chunks x=7 e x=8, verificando células de água source cheias nas duas faces. O teste exercita geração REAL de grafo/drenagem/seleção e rasterização REAL de fluido, mas **não é end-to-end de terreno real**: bypassa o cálculo de densidade/material/structures e não prova conectividade de rios por 2000 blocos. TESTE ESCRITO, ainda sem execução de asserts. Primeiro verificar que Clippy aceita o código. O conjunto original de testes `.49` continua não executado.

### .49 e .48 preservados

`.49` `4e733ea0b6d06107019afd50bb1175e5e0d9ce4a`: regressão de grafo/água/densidade em regiões adjacentes (seed 42, x=127.5/128/128.5), bump `497e1ff`, HANDOFF `98d5e1b`. CI `.49` sucesso. Teste sintético e sem rasterização. `.48` `feature_graph.rs` `3e96e19`, `region/water.rs` `eefb498`, `region/density.rs` `3ea0f38`, bump `5c63703`: filtro por aresta antes do ranking físico; preserva consultas genéricas; testes sobre sobreposição, também sem executar asserts. CI `.48` sucesso. `.47` cache não guarda falso negativo inconclusivo de trace esgotado; `.46` oceano usa altura original para floor; `.45` suporte unificado entre carving/água; detalhes nos snapshots. Não chamar P0 gameplay resolvido automaticamente.

## Nove bugs relatados em 16/09 — TODOS ABERTOS sem confirmação runtime

| # | Prio | Sintoma / lacuna |
|---|---|---|
| 1 | P0 | Chunks pretos/costuras de iluminação/AO; mecanismos `.33/.38/.39/.40/.41` parciais, medir transiente/persistência e FPS. |
| 2 | P0 | Rios terminam no nada: `.42–.48` seleção/carving, `.49` costura, `.50` teste de fonte física. Falta execução dos testes, densidade REAL + fluido, múltiplas seeds/bacias e gameplay. |
| 3 | P0 | 2000 blocos somente Plains/Mountains: `.35` belt estreito não confirmado. Medir `BiomeField::sample_surface().primary_id` vs aparência, seeds distintas. |
| 4 | P0 | Margem de descida afunda antes da água: `.36/.37/.43–.48` parciais. Respeitar margem global `.12`, proibir água suspensa. |
| 5 | P1 | Confluência rio–lago abrupta: XY/largura/profundidade/nível, endpoints autoritativos. |
| 6 | P1 | Túneis com abertura de paredes retas e interseção com rios. |
| 7 | P1 | Lua segue câmera: `celestial.rs` usa `camera_position + offset`; estudar referencial, sem alterar órbita às cegas. |
| 8 | P1 | Lua visível de dia: `.32` setPhase Night ainda sem confirmação visual. |
| 9 | P1 | Highlight não cobre todas as camadas: `targeting/highlight.rs` Cuboid branco, scale 1.01, AlphaMode Blend; verificar bounds/overlay. |

Outros: fogColor artístico; FPS baixo no streaming/caminhada sem benchmark; R/held `.18` não validado. HUD planejado: avatar placeholder `Yogg'Sara` inferior esquerdo, `50 / 100` na vida, status acima; target acima da mira, ícone slot à esquerda e texto com sombra à direita. Único bug visual explicitamente confirmado resolvido pelo usuário até aqui: céu por bioma `.24`.

## Próximas ações verificáveis

1. Consultar HEAD e CI do SHA final `599d767` em `.50`, ler logs se falhar e corrigir. CI Clippy compila testes mas não executa asserts. Conferir detalhe do teste `.50`: o `find_map` só coleta uma linha quando **ambos** os lados reportam River suportado; se nenhum cruzamento, assert falhará e exigirá inspecionar seleção/fixture antes de afirmar quebra de runtime. O scan usa `world_z` i32 preservado, inclusive valores negativos.
2. Estender o teste `.50` para a densidade REAL da geração (`sample_density_field` e influências necessárias) quando houver fixture coerente. Não afirmar que um campo sempre vazio representa escavação real. Verificar múltiplas seeds e destino oceânico. Mapear `river/selection.rs`: `build_flow_cache` e `keep_only_complete_downstream_paths` usam cap 64; `drainage_reaches_water_destination` também cap64. Um destino além do cap pode ficar inconclusivo e o rio ser excluído; NÃO aumentar cegamente, provar cenário e medir custo antes.
3. P0 iluminação: auditar `chunk_remesh.rs`, `chunk_remesh_tasks.rs`, `voxel/mesh_snapshot.rs`, `lighting_updates.rs`; não aumentar brilho ou reduzir invalidações de halo sem prova.
4. P0 biomas: medir IDs e presença efetiva em percurso 2000 blocos, seeds diversas; `biome_field/selection.rs` usa `dominant_neighbor_biome`/`proximity_allows`, `surface.rs` mescla montanhas; dimensão declara Plains/Wasteland/Witchwood/Enchanted Forest/Mountains. Não alterar pesos sem amostragem.
5. Depois P1: confluência, túneis, lua, highlight. Cada bloco: arquivos reais → código/teste quando justificável → bump VERSION → HANDOFF com SHAs e status verificado.
