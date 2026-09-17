# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`; Rust + Bevy 0.19.1. **Versão raiz `VERSION`: `0.20.3`** (bloco de saves não encerrado); `Cargo.toml`: `0.10.16`, independente. **Último código comprovadamente verde:** `92bcdf5463ba701e769dcf5da75b669292229a2a`, https://github.com/sarakborges/mineclone/actions/runs/35250887559 — auditoria dos três idiomas, Clippy `--all-targets --all-features -- -D warnings` e `cargo check --locked` success. CI não equivale a QA Windows ou roundtrip real.

## Histórico integral — consultar obrigatoriamente antes de alterar decisões

- [Etapas 0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 e requisitos expressos de save](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`.
- [Etapas 28–31 integralmente](docs/handoffs/auditoria-28-31-2026-09-17.md), blob `9b46ae054ed205590d67a353fb5117c471e419a8`.
- [Etapas 32–36 integralmente e requisitos/cache CI](docs/handoffs/auditoria-32-36-2026-09-17.md), blob `33ea041165e699a5a95480f60f9b84a26d33d005`: cópia integral do handoff anterior, sem excluir história ou pendências. Ler também `ARCHITECTURE.md`.

**Regras permanentes:** efetuar commits coerentes na `develop`; incrementar `VERSION` major/minor/patch ao concluir cada bloco funcional, nunca apenas por documentos e não encerrar bloco de saves sem QA. Atualizar este handoff após checkpoints com SHAs, CI, limites e próximos passos, arquivando íntegra antes de condensar. Corrigir erros e warnings sem supressão, Clippy rigoroso e cargo check, não adicionar/executar `cargo test` sem autorização expressa. Não alterar PNG/GLB autorais. Pause NÃO congela mundo/fluidos/horário. Cache efetivo de pacotes Linux, `Cargo.lock` restaurado antes de rust-cache e artefatos compilados entre commits; evitar reinstalar/recompilar Bevy sem necessidade. Não inventar QA ou execução do jogo; informar avanços concretos sem prometer trabalho posterior.

## Base de saves — etapas 28–36

UI de nomes exclusivos, reserva de diretório, manifesto e snapshots imutáveis (`worlds/{id}`, ignorado no git); chunks editados residentes/arquivados com IDs estáveis; inventário, posição, regras, dimensão, relógio e autosave condicional a cada 60s; Leave/Exit salvam antes de sair e erro mantém mundo carregado. Fallback tenta cada geração até validar estado jogável completo; retenção conserva quatro backups recuperáveis, poda valida backups em uma única thread e só trava durante exclusão. Desde etapas 43–44, catálogo trava por ID de mundo e loaders abrem todos os snapshots candidatos sob lock, decodificando após soltá-lo. `save_edit_revision` ignora worldgen/streaming/iluminação e aumenta após mutações efetivas de blocos/fluidos. Referências completas, ressalvas e SHAs no handoff 32–36 arquivado.

**Etapa 35, cache CI validado:** `7a556186943ef777a4d79fd5872a64a999ec697b` criou cache de `Cargo.lock` antes de `Swatinem/rust-cache`; `d5772dcc90b48eacd850a7b5a110c7ba6cae75db`, https://github.com/sarakborges/mineclone/actions/runs/35224192069, confirmou HIT exato no lock e artefatos (~412 MB), geração de lock pulada, Clippy 15,56s vs ~3m35s em run anterior. `Cargo.lock` permanece gitignored por decisão existente; índice crates.io ainda pode atualizar. Sem `cargo test`.

## Etapa 37 — data recuperável na lista de mundos e limite de snapshot [CÓDIGO + CI VERDE; QA PENDENTE]

- `ec6eb38542605c7dcc44d03bb121b564e250cc57`: `src/world/save_catalog.rs` expõe `PruneRegistries` próprio e `list_verified_worlds`, que enumera candidatos e, **somente quando chamado em worker**, tenta gerações em ordem decrescente usando `load_snapshot` + `validate_playable`. Agora usa descritores abertos sob trava curta, etapa 44, em vez de segurar a trava durante reconstrução. Só exibe mundos com ao menos uma geração recuperável; timestamp é o da geração efetivamente validada, não de manifesto corrompido. Também rejeita JSON de snapshot acima de `MAX_SNAPSHOT_BYTES = 512 MiB` antes de publicar.
- `ff3918de1c3bff5d7acb842ebdaee8f3dfe71bd0`, CI https://github.com/sarakborges/mineclone/actions/runs/35227081508 success: `src/screens/world_selection.rs` inicializa uma única tarefa `asteria-world-scan` reutilizável se a tela for reaberta durante varredura; copia registries, verifica snapshots fora do frame e entrega resultado por `Arc<Mutex<Option<Result<...>>>>`, consultado via `try_lock`. Enquanto analisa mostra estado; após retorno cria botões e timestamps verificados. Erros de iniciar/pânico são exibidos; Load não avança antes da análise. Carregamento revalida no clique.

**Limites explícitos:** o scanner ainda reconstitui um mundo completo por candidato e pode usar muita CPU/RAM em mapas grandes; mesmo com etapa 44, faz IO e decodificação no worker. Não há medição ou QA Windows. Se todos os backups falharem, mundo fica fora da lista com warning, sem UI de reparo. Captura e escrita do save ainda síncronas. `VERSION` continua `0.20.3`.

## Etapa 38 — evitar duplicação e instrumentar o save [CÓDIGO + CI VERDE; MÉTRICAS WINDOWS PENDENTES]

`e37dc2b93308f75908e7c43ea54950e6de6e4b1c`, CI https://github.com/sarakborges/mineclone/actions/runs/35230616089 success: `save_session.rs::persist` captura uma vez o estado persistido e deriva baseline do snapshot escrito. Log `World <id> saved: capture=..., publication=...` somente após publicação bem-sucedida, distinguindo captura/serialização dos chunks da publicação de JSON, fsync e dispatch de limpeza. Não inventar durações sem logs reais; detecção periódica por `saved_state()` permanece.

## Etapa 39 — streaming JSON com limite durante a escrita [CÓDIGO + CI VERDE; QA PENDENTE]

`c0c001cec61b09e3c875347bf7aa69c760c1b32a`, CI https://github.com/sarakborges/mineclone/actions/runs/35232620292 success: `publish_json` usa `serde_json::to_writer` num `io::BufWriter` e `SnapshotSizeLimit<W: Write>` que limita bytes incluindo newline a 512 MiB, recusando excesso antes do rename. Só após sucesso faz flush, fsync e rename; falha remove temporário, não publica manifesto. Manifestos usam o mesmo buffered writer sem limite de snapshot. Captura ainda aloca `DiskChunk`, gravação é síncrona; não comprova latência/roundtrip.

## Etapa 40 — leitura JSON buffered [CÓDIGO + CI VERDE; QA PENDENTE]

`eda91b78c0526f17eb6d7a597c39752064e5383d`, https://github.com/sarakborges/mineclone/actions/runs/35233747753 success: `read_json` usa `serde_json::from_reader(io::BufReader::new(fs::File::open(path)?))`, evitando vetor de bytes do arquivo. Na etapa 44, leitura de snapshots usa o arquivo já aberto sob a trava, depois do unlock; manifestos seguem `read_json`. RAM e latência não medidas no Windows.

## Etapa 41 — localizar feedback da seleção [CÓDIGO + CI VERDE; QA PENDENTE]

`410de634137803eecaf847b7a9aa77c2577aacc8` introduziu chaves inglesas; `c2bdbca1aca35d40b5da804dd2466bd3378cfd2b` migrou tela para localização, CI https://github.com/sarakborges/mineclone/actions/runs/35234230147 acusou `too_many_arguments`; `eba1a7a2d7120f143ffec2577396d6091dc81de4`, CI https://github.com/sarakborges/mineclone/actions/runs/35234439682 success: `WorldSelectionScanContent` SystemParam coeso para os cinco registries. As chaves constam atualmente nos idiomas inglês, português brasileiro e espanhol. Scanner preserva semântica anterior.

## Etapa 42 — carregar mundo selecionado fora do frame [CÓDIGO + CI VERDE; QA WINDOWS PENDENTE]

- `b7fce1e30e37fa83f24d78d244c53da98a1dce8d`: tela chama `load_world` em worker `asteria-world-load` que revalida candidatos/reconstitui chunks fora do frame; apenas sistema Bevy aplica inventário, recursos e transição após resultado. Panic/spawn failure retorna erro. Back abandona resultado tardio e impede múltiplos workers de carga. Ainda copia registries no clique. Na etapa 44, não retém lock de mundo durante a decodificação. QA do cancelamento/transição pendente.
- `69394da734ede6fd09904f32280a1144e0619517` adicionou chaves inglesas, CI https://github.com/sarakborges/mineclone/actions/runs/35242713345 falhou na auditoria de tradução; `89378d199cd8c6b7da86189e4917aa117af90f92` e `b1baaa545ccd42543379e94e75cfbd3879d0f2c2` completaram PT-BR/espanhol; https://github.com/sarakborges/mineclone/actions/runs/35242868476 success (idiomas, Clippy rigoroso, check). Sem `cargo test`.

## Etapa 43 — travas por mundo [CÓDIGO + CI VERDE; QA PENDENTE]

`6664fd44fd09d40afafb6cb9e94a43cc0df8d105`, CI https://github.com/sarakborges/mineclone/actions/runs/35250626790 completed/success (idiomas, Clippy rigoroso, cargo check): `src/world/save_catalog.rs` substitui `static SAVE_LOCK: Mutex<()>` por `OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>>` e `world_lock(id)`. Writer, listagem de metadados, scanner, load e exclusão da poda travam apenas o mesmo ID; thread que reconstrói mundo A não bloqueia diretamente save de mundo B. Mapa retém um mutex leve por mundo consultado até encerrar o processo; exige ensaio com dois mundos. Ainda segurava lock do mesmo mundo durante decodificação, resolvido na etapa 44.

## Etapa 44 — abrir fallbacks sob lock, reconstruir após unlock [CÓDIGO + CI VERDE; QA PENDENTE]

`92bcdf5463ba701e769dcf5da75b669292229a2a`, CI https://github.com/sarakborges/mineclone/actions/runs/35250887559 completed/success (idiomas, Clippy rigoroso, cargo check): `pinned_candidates(id)` valida nome/diretório, enumera manifests em ordem decrescente e abre TODOS os arquivos de snapshot válidos sob a trava por mundo, registrando erro de abertura por geração. Libera a trava ao devolver descritores `fs::File`. `load_world` e scanner da data efetivamente recuperável decodificam JSON e reconstruem chunks depois do unlock, inclusive fallback, usando `decode_snapshot` compartilhado com a retenção via `load_snapshot`. A poda só remove caminhos após validar quatro gerações recuperáveis, sob trava por mundo. Num SO que impede excluir arquivo aberto (ex.: Windows sem compartilhamento para delete), a poda pode falhar e deixar arquivos órfãos/retidos; save confirmado não é revertido, e poda futura pode tentar limpar. O conjunto aberto é um retrato temporal: novo save concorrente pode não ser incluído na carga em andamento; próximo clique revalida. Em mundos com muitos backups inválidos pode haver muitos descritores simultâneos; avaliar limites e orçamento. CI não testa corrida nem Windows.

## Próximos passos — prioridade

1. QA Windows da carga assíncrona e lock curto: clique Load em mundo grande, Back imediatamente e simultâneo ao resultado, reentrada da seleção, falha de snapshot, transição apenas após dados preparados. Confirmar que poda com arquivos abertos não perde backups válidos nem impede o próximo save; instrumentar tamanho da fila de descritores e memória.
2. Corrigir possível corrida UI: ordem atual `poll_world_load` antes de `handle_world_selection` faz resultado já pronto ganhar do clique Back no mesmo frame. Priorizar intenção de Back e descartar resultado antes de aplicação, mantendo feedback e execução única. Verificar CI sem supressões.
3. Medir no Windows logs `capture`/`publication` em mundos pequenos/grandes e chunks arquivados, reduzindo gargalo com dados reais; Leave/Exit só saem após gravação confirmada e erro mantém mundo. Captura + escrita continuam na thread principal.
4. Revisar orçamento CPU/RAM do scanner (reidrata mundos para verificar), estratégia de limite de descritores abertos, timeout e cancelamento de trabalhadores; não sacrificar fallback, integridade nem inventar performance.
5. QA integral de save/restart: nomes repetidos/dois mundos, blocos, fluidos, brush, chunks arquivados, inventário, posição, regras, relógio, autosave/Leave/Exit/restart, fallback de snapshot mais recente corrompido, interrupção entre snapshot/manifesto, permissões e recuperação pós poda. Não inventar resultados.
6. Preservar cache CI/`--locked`, Clippy `-D warnings`, nenhum `cargo test` sem pedido. Somente após QA e fechamento funcional subir semanticamente VERSION; não sincronizar Cargo.toml sem decisão expressa.
