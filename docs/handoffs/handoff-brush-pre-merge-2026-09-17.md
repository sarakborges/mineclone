# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Esta cópia está na branch isolada `fix/brush-hand-render-layer-order`**, criada de `develop` em `c087f09b5d06a00ddbfebf5e0fbb18359d9995e3`, PR draft [#13](https://github.com/sarakborges/mineclone/pull/13). `VERSION` nesta branch: **`0.20.4`**, patch independente do bloco de save ainda aberto e sem QA real; não interpretar o bump como fechamento do save. `Cargo.toml` `0.10.16` tem versionamento independente. O código inicial do Brush passou na [CI 35257944441](https://github.com/sarakborges/mineclone/actions/runs/35257944441) (idiomas, Clippy `-D warnings`, check); **o ajuste posterior de escala/pivô necessita CI própria e QA visual no Windows**. Último código verde de `develop` documentado: `c05e5a760d274efd531d065cdb83967e72dce179`, [CI 35256843392](https://github.com/sarakborges/mineclone/actions/runs/35256843392). Não alegar execução ou QA de jogo apenas pela CI.

## Arquivos integrais e história obrigatória

**O handoff anterior completo, sem cortes, foi arquivado byte a byte em [handoff-pre-brush-2026-09-17.md](docs/handoffs/handoff-pre-brush-2026-09-17.md), mesmo blob original `49d20d15d857118bd660a0d619ef58a9cf37d3ab`.** Não descartar esse arquivo: contém narrativa integral das etapas 45–50, SHAs, resultados de CI e ressalvas. Ler também [ARCHITECTURE.md](ARCHITECTURE.md).

- [Etapas 0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 e requisitos de save](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`.
- [Etapas 28–31](docs/handoffs/auditoria-28-31-2026-09-17.md), blob `9b46ae054ed205590d67a353fb5117c471e419a8`.
- [Etapas 32–36](docs/handoffs/auditoria-32-36-2026-09-17.md), blob `33ea041165e699a5a95480f60f9b84a26d33d005`.
- [Etapas 37–44 e handoff anterior integral](docs/handoffs/auditoria-37-44-2026-09-17.md), blob `7a097aac02ebbcad2b5cadf0a2a79dc3cf191b67`, histórico prévio arquivado em `eb4d0346aed06db3110b704a57162726a45d3940`.

## Regras permanentes

Commits coerentes em branch isolada quando o usuário a pedir, PR direcionada à `develop`; nunca fundir sozinho. A cada checkpoint, atualizar este handoff (commit, CI, limites, próximos passos), preservando versão integral anterior em arquivo antes de condensar. Incrementar `VERSION` semanticamente por bloco funcional, não por documentação; não fechar save sem QA real. Corrigir warnings sem supressão. CI: auditoria de PT-BR, espanhol e inglês; `cargo clippy --locked --all-targets --all-features -- -D warnings`; `cargo check --locked`. Não executar/adicionar `cargo test` sem autorização expressa. Não alterar PNG/GLB autorais. Pause NÃO congela mundo, fluidos ou horário. `Cargo.lock` é gitignored e CI cacheia lock/compilados: não deteriorar cache. Não inventar resultado de compilação, execução, QA ou desempenho. Informar progresso concreto, sem prometer trabalho em segundo plano.

## Estado do save até etapa 50 — continua em aberto

UI de nome único e mundos isolados; manifesto/snapshot imutáveis com commit marcador, chunks residentes/arquivados, IDs portáveis, inventário, posição, regras, dimensão, relógio, autosave por mudança após 60 s, Leave/Exit somente após save confirmado. Fallback valida geração inteira e retenção protege quatro gerações recuperáveis. Prune e scan/load em workers, leases por mundo, poda retomada após último leitor, descarte de load abandonado fora do frame; validações de nomes reservados Windows e redução de alocações de propriedades; telemetria de tempos para diferenciar main thread de worker e feedback de erro inicial restaurado. Histórico detalhado e CI em [handoff integral anterior](docs/handoffs/handoff-pre-brush-2026-09-17.md). [Protocolo Windows/roundtrip](docs/save-roundtrip-qa.md) ainda **NOT RUN**. Não alegar que código novo do Brush valida saves.

## Checkpoint independente — Brush na mão e ordem das camadas [CÓDIGO NA BRANCH; QA VISUAL PENDENTE]

- Solicitação de 2026-09-17: imagem mostra cabo fora da mão; camada de tinta parece atrás da arte-base, divergente da hotbar. Branch `fix/brush-hand-render-layer-order` nasceu de `develop` em `c087f09b5d06a00ddbfebf5e0fbb18359d9995e3`.
- **Primeira tentativa equivocada** em `c78adff2d62682cd58cdce48fe03cd083f263885` diminuiu Brush de `0.43` para `0.30`, mantendo sprite centrado no ponto inadequado; usuário corrigiu expressamente: o item deveria ser AUMENTADO e o CABO ficar DENTRO DA MÃO. Não voltar à estratégia de reduzir a textura. Dessa primeira tentativa preservou-se a separação de passes: base `AlphaMode::Mask(0.5)` antes da tinta `AlphaMode::Blend`, com Z `+0.004` para evitar conflito de profundidade; PNGs e hotbar intactos.
- **Correção solicitada aplicada em `89965d758e7c454037a162fb0bfccac49dd8a8ba`** (`src/player/viewmodel/held_brush.rs`): tamanho agora `0.52` (maior que `0.43` original); o root foi deslocado para `(-0.08, 0.46, 0.21)` no referencial da mão, e passou a ser o ponto de pegada do cabo, que fica dentro dos limites Y=0..0.60 do braço. O transform da textura compensa o offset normalizado aproximado da extremidade do cabo `(-0.36, -0.36)` e aplica rotação local Z `+0.30` rad ao redor dessa pegada. Base e tinta usam exatamente o mesmo transform calculado, exceto offset Z do overlay, preservando alinhamento das duas camadas durante animações. Dimensões e offset são estimativas de composição a serem confirmadas em execução, não QA aprovado.
- `457eec2c5c5ecc7dd42b58affc7cc459e315aa2f` havia incrementado somente `VERSION` `0.20.3` → `0.20.4`. **Manter `0.20.4`:** a correção do tamanho/pivô é continuação do mesmo patch funcional, não uma nova versão nem conclusão de save. `Cargo.toml` e formato de save permanecem intactos.
- [PR draft #13](https://github.com/sarakborges/mineclone/pull/13) para `develop`: primeiro head com [CI 35257944441 concluída com sucesso](https://github.com/sarakborges/mineclone/actions/runs/35257944441) (idiomas, Clippy, check). Novo commit da geometria e este checkpoint ainda exigem CI correspondente. Nenhum `cargo test`, `cargo run`, screenshot pós-correção ou QA visual Windows executado. Não fazer merge antes de validar visualmente.
- QA visual pendente: selecionar Brush sem tinta e com pelo menos duas tintas; conferir ponta/cor/ordem com hotbar; alternar Brush/bloco e inventário; observar que o cabo está efetivamente coberto/segurado pela mão em repouso e durante animações de quebrar/colocar/troca, sem cortar a cabeça na borda da tela; validar silhueta de alpha mask, transparência e ausência de z-fighting. Se necessário ajustar somente escala, pivô e pose com base em screenshot real, sem reduzir novamente o sprite.

## Próximos passos

1. Confirmar CI do head atualizado da PR #13 e corrigir qualquer warning/erro sem supressão. Não declarar verde sem checar run associado ao SHA final.
2. Executar QA visual real no Windows do Brush conforme roteiro acima e registrar PASS/FAIL/NOT RUN, com screenshot comparando hotbar vs mão. Manter PR draft até revisão.
3. Retomar QA real de saves no Windows com [protocolo](docs/save-roundtrip-qa.md), logs de cópia na main thread, scan/load workers, captura/publicação, memória, CPU, Back cancelado e dois mundos; não extrapolar CI Linux.
4. Fazer roundtrip/restart cobrindo nomes repetidos/Unicode reservado, blocos/fluidos/brush arquivados, inventário, posição, regras/relógio, autosave, Leave/Exit, corrupção/fallback com timestamp, publicação interrompida, permissões e poda; registrar PASS/FAIL/NOT RUN.
5. Com medições, analisar orçamento global de scan/load/prune, eventual cancelamento cooperativo e timeout de lease; otimizar somente gargalos reais preservando backups e sem travar frame. Leave/Exit só deve sair depois de publicação confirmada.
6. Preservar cache, Clippy rigoroso e check `--locked`; `Cargo.toml` independente, sem `cargo test` sem autorização. O bump de Brush NÃO fecha o bloco de save.
