# HANDOFF — Asteria / Mineclone

Repositório `sarakborges/mineclone`, branch autoritativa de trabalho **`develop`**, Rust + Bevy **0.19.1**. Atualizar este HANDOFF na MESMA sessão de cada mudança material. Histórico anterior preservado nos snapshots: [0.15.50](https://github.com/sarakborges/mineclone/blob/b12e4ddd4f29c31f3cf21ce2f8ca70d237af8a74/HANDOFF.md), [0.15.49](https://github.com/sarakborges/mineclone/blob/98d5e1b181e3a8b317d24d90195a3495f61d53d4/HANDOFF.md), [0.15.48](https://github.com/sarakborges/mineclone/blob/8cdebad5fabbe55b93a2bfa45f4fd9e93dbf4739/HANDOFF.md), [0.15.47](https://github.com/sarakborges/mineclone/blob/457de45c0dc04e40a52ef6f5d567444e2b4f30da/HANDOFF.md), [0.15.46](https://github.com/sarakborges/mineclone/blob/c778fee85c9b4cbc10a38be9e413bfda9174eaab/HANDOFF.md), [0.15.45 e links .31–.44](https://github.com/sarakborges/mineclone/blob/46df141fe88ba7f96977482906362a189abab7e0/HANDOFF.md). Compactar o relato não fecha defeitos.

## Regras obrigatórias

- `continua`/`go`/`continua com roadmap`: consultar HEAD, `VERSION`, arquivos reais, CI e este documento; continuar com mudanças reais justificadas em `develop`, commits pequenos. Não responder somente com planos nem só status da CI.
- Alteração de código, dados jogáveis ou testes relevantes exige bump do `VERSION` raiz segundo SemVer: patch fixes/refactor/testes, minor feature compatível, major quebra. `Cargo.toml` versão independente `0.10.16`, **não** sincronizar com `VERSION`. Alterações isoladas de documentação não exigem bump.
- Atualizar este HANDOFF na mesma sessão: versão, SHA do último código, commits, validação efetiva, limitações, nove bugs e próximo passo. Conferir HEAD antes de escrever; não sobrescrever mudanças alheias.
- CI `.github/workflows/ci.yml` em .51 passou a executar **Clippy `cargo clippy --all-targets --all-features -- -D warnings`**, **`cargo check`**, e **`cargo test --bin asteria -- --test-threads=2`**. Antes de .51 não havia etapa de testes. Não alegar testes aprovados enquanto etapa não concluir. A CI não cobre gameplay visual, FPS, Windows ou cenários grandes por si só; investigar e corrigir erros/warnings concretos.
- `ARCHITECTURE.md` é o cânone. Ownership único, `SystemParam` por domínio, UI em `src/ui`, `TargetedBlock` autoritativo, sistemas change-driven, `DeduplicatedQueue<T>`/`VoxelUpdateQueue`, `FrameWorkBudget`, evitar scans globais sem orçamento. Não criar nova arquitetura sem necessidade reproduzida. Preservar assets do usuário; não gerar imagens sem pedido. Não prometer execução assíncrona.

## Invariantes de runtime e pipeline

Pipeline: streaming visible/preload → geração async → integração/seed luz por residência (.38) → snapshot halo 3×3×3 → mesh async → render → archive/unload. Resultados revisionados; desatualizados são reenfileirados, geometria/fluidos/luz separados. Luz em lote (.41), unload halo 26 vizinhos (.40), HDR world order0 Skip/viewmodel order1 tonemap/UI SDR order2. Hidrologia: endpoints XYZ compartilhados e margem global (.12), suporte físico por aresta antes de ranking (.48), nunca permitir água flutuante; água natural pertence à geração, não deve entrar em massa no solver de fluido dinâmico. `BiomeField` determina bioma autoritativo, `GenerationColumnSample` altura original, densidade antes de material e fluidos. Sol/lua seguem céu/relógio, nuvens world-space; `celestial.rs` usa camera_position+offset e usuário relata lua seguir câmera. Held block acompanha hotbar; R apenas placement.

## Estado verificado 16/09/2026 — v0.15.51

**`VERSION`: `0.15.51`**, commit `966989937b5b80fe3414481a6bc58c2c191b9c1c`. Último commit de código antes deste handoff: **`e80243f39ce7b81e4a031a71d02109c855442072`**. A atualização deste documento produz HEAD posterior; consultar branch antes do próximo bloco.

**CI .50 final-code sucesso**: push `35155018897`, SHA `599d76791d6048f887d7c69d842645c74b74de1f`, Clippy e Check success; workflow antigo NÃO executava asserts. Também sucesso bump .50 `35154962840`, .49 `35154000629` e .48 `35153673196`. **CI .51 código final**: push `35156113558`, SHA `e80243f`, status `in_progress` na última consulta, conclusão nula. Executará Clippy + Check + Tests; verificar as três etapas e logs, especialmente regressão de rios. A CI intermediária bump `35156058752` não inclui ajuste de visibilidade `e80243f`; a do código final tem precedência. Não alegar sucesso de .51 até conferir resultado.

### .51 — densidade real e água na costura das regiões

- `src/world/generation/fluids.rs`, commit `208bbbbb2717e4aca50bc9b52264d470d167b34c`: substituiu vetor de densidade inteiramente vazio da .50 pela chamada **real** `sample_density_field`, com `DensityPassContext`, `VolumeBiomeRegion::default`, `BiomeField` construído da definição REAL de Plains e da dimensão Overworld filtrada apenas para Plains, com seaLevel 64 compatível. `GenerationColumnSample` mantém altura original plana 104 e influência Plains 100%; o gerador de rios usa declive macro sintético e seed 42. Seleção da travessia exige River fisicamente suportado nos dois lados, voxel molhado e **abaixo da superfície original**, garantindo `terrain_density` inicial positivo. Para cada chunk x=7/8, verifica densidade final não sólida e usa `rasterize_fluid_pass` para exigir fonte cheia em ambas as faces.
- A busca .50 examinava Z de -384 a +384 nos grafos das regiões `(0,0)` e `(1,0)`, embora muitos Z pertençam a outras regiões. Agora constrói pares de regiões corretos `(0, region_z)` e `(1, region_z)` para `region_z` em `-2..=2`, amostra apenas cada faixa de 128 blocos e preserva negativos com `div_euclid`/`rem_euclid`. Se a fixture não encontrar travessia, o teste falha explicitamente: investigar fixture/seleção antes de afirmar que há bug no jogo.
- `src/world/hydrology/region.rs`, commit `e80243f39ce7b81e4a031a71d02109c855442072`: coord de `HydrologyRegion` tornou-se `pub(crate)` para verificar `GenerationRegion.coord.xz() == hydrology.coord` no teste; não duplica estado.
- `.github/workflows/ci.yml`, commit `6ca679d47181d01f26a7be7b86c4f211d775b802`: adicionou `cargo test --bin asteria -- --test-threads=2`, antes só Clippy e Check. Bump `.51` commit `966989937b5b80fe3414481a6bc58c2c191b9c1c`.
- **Limitações**: fixture tem coluna de altura e terreno macro sintéticos, Plains isolado, sem cavernas, sem rasterização de materiais/structures, sem mundo de jogo completo. Usa as funções reais de densidade e colocação de fluidos. Só a execução da CI comprova asserts. Teste único de costura não prova rios em 2000 blocos, destino final oceânico, múltiplas seeds, FPS ou ausência de chunks pretos.

### Trabalho anterior preservado

`.50`: `77f0b67`/`e0dbf7f`/`599d767` teste gerador→rasterização com vetor todo vazio, bump `0a10707`, handoff final `b12e4dd`; CI final-code success `35155018897`. `.49`: `4e733ea` regressão grafo/água/densidade em regiões adjacentes, bump `497e1ff`, CI success. `.48`: `feature_graph.rs` `3e96e19`, `region/water.rs` `eefb498`, `region/density.rs` `3ea0f38`, filtro por aresta suportada antes de escolher mais forte, CI success. `.47`: cache de drenagem não memoriza falso negativo inconclusivo de trace limitado a 64. `.46`: oceano usa altura original na amostragem física. `.45`: suporte consistente carving/água. Histórico completo nos links acima. Não confundir CI check com teste executado.

## Nove bugs do usuário — TODOS ABERTOS até confirmação em runtime

| # | Prio | Sintoma e lacuna |
|---|---|---|
| 1 | P0 | Chunks pretos/costuras iluminação e AO. Mecanismos `.33/.38/.39/.40/.41` parciais; medir transiente/persistência e FPS. |
| 2 | P0 | Rios terminam no nada. `.42–.48` seleção/carving, `.49` costura, `.50–.51` regressões de água+densidade; falta asserts .51, múltiplas seeds, destino até oceano e jogo. |
| 3 | P0 | Percurso de 2000 blocos só Plains/Mountains. `.35` belt estreito não confirmado; medir `sample_surface().primary_id` por seed/percurso e comparar aparência, sem alterar pesos às cegas. |
| 4 | P0 | Margem afunda antes da água. `.36/.37/.43–.48` parciais, preservar margem global `.12` e suporte físico. |
| 5 | P1 | Confluência rio–lago abrupta: conferir XY/largura/profundidade/nível e endpoints. |
| 6 | P1 | Aberturas de túneis com paredes retas e interseção com rios. |
| 7 | P1 | Lua segue câmera: `celestial.rs` usa `camera_position + offset`; não mudar órbita às cegas. |
| 8 | P1 | Lua visível de dia: `.32` setPhase Night não confirmado visualmente. |
| 9 | P1 | Highlight ignora camadas: `targeting/highlight.rs` cubo branco scale1.01/AlphaMode Blend; checar bounds/overlay. |

Outros pontos não fechados: fogColor artístico; FPS baixo no streaming/caminhada sem benchmark; R/held `.18` não validados. HUD planejado: avatar placeholder `Yogg'Sara` inferior esquerdo, `50 / 100` vida, status acima; target acima mira, ícone slot à esquerda e texto com sombra à direita. Único bug visual explicitamente confirmado resolvido pelo usuário: céu por bioma `.24`.

## Roadmap de execução — próximos blocos

1. **CI .51**: ler steps/logs do run push `35156113558`; se `cargo test` falhar, corrigir causa sem apagar regressão ou inventar travessia. Reavaliar custos da fixture e CI. `cargo check` e Clippy não substituem Tests; registrar quantos passaram/falharam.
2. **Rios P0**: adicionar cenário determinístico que prova conexão fonte→destino oceânico sobre mais de uma região e leito físico não suspenso; analisar cap 64 em `river/selection.rs` em cenário reproduzível e medir impacto antes de aumentar. Expandir fixture para mais seeds e terreno realmente gerado pelo `BiomeField`, sem falsear altura ou buracos.
3. **Biomas P0**: testar/amostrar `BiomeField::sample_surface().primary_id` em trajeto de 2000 blocos e múltiplas seeds, usando dimensão/biomas reais; distinguir ausência na seleção de material/visual. `selection.rs` exclui bioma dominante quando 5/8 vizinhos e aplica `avoidNear`; `surface.rs` mistura montanhas; não alterar pesos sem evidência.
4. **Luz P0**: examinar `chunk_remesh.rs`, `chunk_remesh_tasks.rs`, `voxel/mesh_snapshot.rs`, `lighting_updates.rs`, reproduzir estado preto por halo/relaxamento; mecanismos de seed/catchup e 26 vizinhos de `streaming.rs` foram auditados na .50, não provam resolução. Conferir também margem de água P0 e depois P1 confluências, túneis, lua e highlight.
5. Cada bloco: consultar arquivos/HEAD → reproduzir/testar → correção fundamentada → bump `VERSION` → registrar commits e status exato no HANDOFF. Nenhum dos nove bugs é encerrado só com CI verde.
