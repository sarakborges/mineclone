# HANDOFF — Asteria / Mineclone

Repositório `sarakborges/mineclone`, branch de trabalho `develop`, Rust + Bevy 0.19.1. **Fonte canônica persistente: este arquivo.** Handoffs históricos sem perda: [histórico pré-.31](https://github.com/sarakborges/mineclone/blob/72b60b5a4e70a9960b3108debdf77d7a8510c4b1/HANDOFF.md), [feedback .31](https://github.com/sarakborges/mineclone/blob/6b8051628aa49c3b01ffca782144b8fc84705365/HANDOFF.md), [investigação .32](https://github.com/sarakborges/mineclone/blob/5900057c6e7ec8d53684bf877a47c499ae6d3b5b/HANDOFF.md) e [implementação detalhada .33](https://github.com/sarakborges/mineclone/blob/a90e42a61d18a8aa4e326b17450f80e5b334de53/HANDOFF.md). Documento compactado não fecha bug algum.

## Contrato de execução

`go`/`continua` significa executar. Consultar HEAD, VERSION raiz e arquivos atuais antes de editar, trabalhar em `develop` sem feature branch não solicitada. Todo bloco de alterações em código ou dados jogáveis sobe VERSION (PATCH fix/refactor compatível, MINOR nova funcionalidade, MAJOR quebra de contrato); documento isolado não sobe. `Cargo.toml` 0.10.16 deliberadamente diferente de VERSION operacional. **Atualizar este HANDOFF na mesma sessão** de qualquer alteração material de código, versão, feedback, roadmap, arquitetura ou processo; registrar SHA, CI, pendências e próximo passo. Dar prioridade a erros e warnings reais, sem silenciá-los. CI canônico: `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check`; `fmt` não é gate e CI não roda unit tests. Não repetir observações sobre cargo local/espera `cargo run`; teste manual apenas se usuário solicitar. Distinguir achado, código publicado, CI aprovado e gameplay visual/físico comprovado. Não gerar imagens sem pedido e preservar assets binários fornecidos.

## Arquitetura a preservar

`ARCHITECTURE.md` autoritativo: um owner por fato; `SystemParam` por domínio, UI reutilizável em `src/ui`, targeting único `TargetedBlock`, sistemas change-driven; invariants reais compartilhados, não formas parecidas. Filas `DeduplicatedQueue<T>`/`VoxelUpdateQueue`, orçamentos `FrameWorkBudget`; evitar full scans, alocações e dirty writes desnecessários. Streaming: visible/preload -> geração async -> integração -> seed de luz inicial UMA VEZ por residência -> captura halo 3×3×3 -> mesh async -> render/visibilidade -> retention -> archive/unload. Visible supera preload; preempção restitui não crítico; stale tasks não causam starvation. Geometry e Fluid meshes e filas independentes, ambos dependem de luz; revisões de lighting remesh e mesh world são diferentes. First-visible não deve ser atrasado por vizinhos chegando, mas mesh publicada deve eventualmente reconciliar halo/luz.

HDR: world camera order0 Skip, viewmodel HDR order1 tonemap final, UI SDR order2. `EnvironmentVisualState.sky_color` por bioma/relógio -> `SkyPlugin` ClearColor EXATO, rastrear bioma antes dos visuais; `fogColor` artístico é separado, ainda NÃO implementado no renderer; fog terminal deve convergir ao sky sem mistura fixa de fog no céu. Nuvens world-space, vento independente e altitude por seaLevel da dimensão. Sol/lua rise/set e órbitas data-driven e independentes. Hidrologia: endpoints de rios incidentes compartilham X/Z/Y, regiões determinísticas, margens gerais da .12 preservadas, água natural autoritativa da geração e não enfileirada em massa no solver. Held model observa hotbar/visual/tint; R altera só colocação.

## Estado em 16/09/2026

**VERSION = 0.15.34.** Patch Clippy em `src/world/streaming.rs` commit `0bc584505742cf3696308f265560eb3c6bd80571` move `VoxelChunk` (apenas usado no teste) para o escopo `#[cfg(test)]`; bump `50fc8a1dc3011323f581cdd44bd6fc044cabdd25`. **CI .34 push run 35137428833 e PR run 35137433355 estavam in_progress na última consulta; checar antes de afirmar sucesso.**

**CI .33 FALHOU:** run35136774532, job104931009333, Clippy erro ÚNICO `unused import: chunk::VoxelChunk` em `src/world/streaming.rs:18` no bin normal. Check skipped. Foi corrigido em .34 sem desativar warnings. .33 não pode ser declarado validado. CI .32 push run35136271200 job104929320897 Clippy+Check SUCCESS; .31 run35134931146 job104924817286 SUCCESS; .24–.30 success conforme históricos. .22/.23 falharam antes de fixes posteriores.

**.33 parcial publicada e preservada em .34:** `src/voxel/mesh_snapshot.rs` commit `02eb6d233804287f72b6084b803d024c256523fb` implementa `ChunkMeshDependencies::needs_initial_catchup` quando vizinho de 26 posições estava ausente na captura mas presente na integração. `src/world/streaming.rs` commit `6ff41717948bffa1f88a252910214c7e125fe359` publica mesh primeiro, enfileira Geometry e Fluid separadamente quando halo novo/seed de vizinho em voo, marca seed do vizinho sem cancelar mesh inicial, notifica vizinhos já renderizados diagonais por filtros de borda. Flags removidas após publicação/unload, sem reseed em retry. Testes unitários adicionados, NÃO executados por CI. **A correção de .33 não prova resolução visual; exige gameplay.** Riscos: carga adicional de remesh por vizinhança, filtro baseado em faces independentemente (pode dar falsos positivos), luz mudando por relaxação durante task ainda não registrada na dependência. Não mascarar com luz global/fog.

**ACHADO NOVO CONFIRMADO:** `ChunkMeshDependencies::is_current` compara só `chunk_content_revision`. Enquanto initial mesh task está em voo, `process_dynamic_lighting` pode alterar a luz e notificar quando o chunk ainda NÃO está no render pool. O resultado assíncrono pode publicar luz antiga sem catch-up. `VoxelWorld::chunk_mesh_revisions` sobe ao alterar luz; `chunk_mesh_revision` está limitado hoje a `#[cfg(test)]` no world.rs. Próximo patch: expor getter read-only de revisão mesh, capturar para os 27 chunks no snapshot e comparar no momento da publicação somente para enfileirar catch-up **após first-visible**, sem mudar `is_current` e sem starvation. Cobrir mudanças center/halo, missing->loaded e alterações durante task em testes unitários. Se houver mesh revisões alteradas por conteúdo, `is_current` já rejeita conteúdo stale. Limitar custo a 27 getters por output, sem copiar 27 chunks extras.

**.32 lua visível de dia:** `progress_between_phases` inclui fase de término completa. Lua overworld `setPhase=Dawn` continuava durante Dawn; `data/dimensions/overworld/sky.json` alterado `setPhase=Night`, término no começo de Dawn, sol/orbita/contratos de outras dimensões intactos. Commit `7d00f21a8f56b4b06befc85d846afd2b9601131f`, CI passou; gameplay pós-.32 ainda NÃO confirmado. Lua seguindo câmera P1 separada; `src/rendering/celestial.rs` usa `camera_position + celestial_offset`, investigar sem colocá-la fixa a apenas 600 blocos.

**.31 diagonal parcial:** `lighting_updates.rs` notifica 20 diagonais quando relaxação muda luz, filtro de boundary Geometry/Fluid; CI passou. USUÁRIO confirmou em 16/09 que chunks pretos/costuras e todos demais bugs permaneciam. **ÚNICO DEFEITO COM CORREÇÃO VISUAL CONFIRMADA: CÉU POR BIOMA** da .24; `fogColor` ainda aberto. Não confundir fixes de código com comprovação visual.

## Nove defeitos de gameplay ainda não encerrados

| # | Prioridade | Defeito e aceite |
| --- | --- | --- |
| 1 | P0 | Chunks inteiros pretos e costuras/sombras entre chunks persistentes pós-.31; .33/.34 publicaram algumas correções de race ainda sem validação visual. Investigar luz in-flight, seed/snapshot/remesh e FPS real. |
| 2 | P0 | Rios morrem no nada; provar por seed/coords grafo, leito E água física até lago/rio/oceano cruzando chunks e regiões; .15/.16/.19/.20 insuficientes. |
| 3 | P0 | Percurso 2.000 blocos só Plains/Mountains; medir bioma FINAL versus aparência, suitability e distribuição, .25 só removeu alocações. |
| 4 | P0 | Descida de rio: margem baixa ANTES da água; medir leito, nível, perfil, fluido; vazamento só hipótese. Preservar margem geral .12. |
| 5 | P1 | Rio↔lago junção abrupta; transição suave XY e largura/profundidade/altura sem destruir endpoints/água contínua. |
| 6 | P1 | Túneis abrem paredes retas na superfície, distinto da parede no cruzamento rio-túnel; auditar cave connector/density/carver. |
| 7 | P1 | Lua parece seguir câmera; referencial câmera+offset confirmado, solução ainda aberta. |
| 8 | P1 | Lua de dia: ajuste .32 publicado e CI verde, gameplay não confirmado. |
| 9 | P1 | Highlight target não abraça todas as camadas de textura; verificar bounds/alpha/overlays. |

Outras pendências: fog artístico, FPS/streaming ao andar/voar sem benchmark 60 FPS, R/held .18 CI verde sem gameplay, paredes cave-river, player HUD planejado inferior esquerdo `Yogg'Sara`, vida `50 / 100` dentro barra, status acima; target HUD acima crosshair ícone esquerdo estilo slot inventory e texto direito com shadow tooltip. Hotbar vazia, ghost alpha uniforme, dye, HUD histórica e fog frontier/nuvens antigas anteriormente informadas resolvidas; preservar. Margens gerais .12 confirmadas resolvidas; descida é bug novo distinto. Histórico completo .15.11–.30 nos links.

## Próxima execução

1. Confirmar CI .34, corrigir novos erros/warnings se houver, sempre bump patch de código e atualizar HANDOFF.
2. Implementar revision snapshot de iluminação como acima com testes cobrindo center, vizinhos e novo neighbor; publicar initial antes de agendar catch-up. Medir impacto de remesh; não alterar cor de iluminação, fog ou shadow global arbitrariamente.
3. Depois investigar chunks pretos residuais e rios end-to-end; biomas; margem/river-lake/tunnels; lua seguindo câmera; highlight. Não declarar um dos nove resolvido sem confirmação de gameplay.