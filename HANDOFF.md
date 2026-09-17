# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Esta cópia está na branch isolada `fix/brush-hand-render-layer-order`**, criada de `develop` em `c087f09b5d06a00ddbfebf5e0fbb18359d9995e3`, PR draft [#13](https://github.com/sarakborges/mineclone/pull/13). `VERSION` nesta branch: **`0.20.4`**, patch independente do bloco de save ainda aberto e sem QA real; não interpretar o bump como fechamento do save. `Cargo.toml` `0.10.16` tem versionamento independente. Último código com CI verde comprovado no momento deste registro: `c05e5a760d274efd531d065cdb83967e72dce179` ([run 35256843392](https://github.com/sarakborges/mineclone/actions/runs/35256843392)). CI da PR do Brush ainda não confirmado; não alegar compile ou QA do Windows.

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

## Checkpoint independente — Brush na mão e ordem das camadas [CÓDIGO NA BRANCH; CI/QA VISUAL PENDENTES]

- Solicitação de 2026-09-17: screenshot mostra Brush grande e encaixe pouco natural na mão; camada de tinta parece atrás da arte-base, divergente da hotbar. Novo branch `fix/brush-hand-render-layer-order` nasceu de `develop` em `c087f09b5d06a00ddbfebf5e0fbb18359d9995e3`.
- Commit `c78adff2d62682cd58cdce48fe03cd083f263885`: somente `src/player/viewmodel/held_brush.rs`. Sprite 64x64 reduzido de `0.43` para `0.30`, mantendo o pivô no mesmo root para trazer a ponta do cabo para perto da extremidade da mão. Base agora `AlphaMode::Mask(0.5)` no passe opaco, garantindo desenho antes da camada de tinta `AlphaMode::Blend` e preservando recortes transparentes; tinta fica em Z `+0.004` rumo à câmera para afastar superfícies coincidentes, sem mexer nos PNGs ou na lógica da hotbar. O overlay permanece oculto quando não há dye e sua cor continua sincronizada com a paleta. A mudança de modo da base é adequada a pixel art, mas suas bordas e alinhamento ainda precisam ser avaliados visualmente no jogo.
- Commit `457eec2c5c5ecc7dd42b58affc7cc459e315aa2f` incrementa apenas `VERSION` `0.20.3` → `0.20.4` como patch independente. `Cargo.toml` e formato de save intactos.
- [PR draft #13](https://github.com/sarakborges/mineclone/pull/13) para `develop`, aberta para disparar validação; ao escrever esta seção ainda sem resultado de CI confirmado. Nenhum `cargo test`, `cargo run`, QA visual Windows ou prova de screenshot após o patch.
- QA visual pendente: selecionar Brush sem tinta, comparar arte e silhueta com ícone da hotbar; selecionar pelo menos duas tintas e comparar cobertura da ponta/cor/ordem; alternar Brush/bloco e inventário; observar encaixe do cabo com mão em repouso e durante animações de quebrar/colocar/troca; verificar recorte transparente e se não há z-fighting nem braço sobrepondo incorretamente. Ajustar posicionamento/escala somente após screenshot real se persistir problema.

## Próximos passos

1. Acompanhar o CI da PR #13 e corrigir qualquer warning/erro sem supressão. Não declarar verde sem check verificável.
2. Executar QA visual real no Windows do Brush conforme roteiro acima e registrar PASS/FAIL/NOT RUN, com screenshot comparando hotbar vs mão. Manter PR draft até revisão.
3. Retomar QA real de saves no Windows com [protocolo](docs/save-roundtrip-qa.md), logs de cópia na main thread, scan/load workers, captura/publicação, memória, CPU, Back cancelado e dois mundos; não extrapolar CI Linux.
4. Fazer roundtrip/restart cobrindo nomes repetidos/Unicode reservado, blocos/fluidos/brush arquivados, inventário, posição, regras/relógio, autosave, Leave/Exit, corrupção/fallback com timestamp, publicação interrompida, permissões e poda; registrar PASS/FAIL/NOT RUN.
5. Com medições, analisar orçamento global de scan/load/prune, eventual cancelamento cooperativo e timeout de lease; otimizar somente gargalos reais preservando backups e sem travar frame. Leave/Exit só deve sair depois de publicação confirmada.
6. Preservar cache, Clippy rigoroso e check `--locked`; `Cargo.toml` independente, sem `cargo test` sem autorização. O bump de Brush NÃO fecha o bloco de save.
