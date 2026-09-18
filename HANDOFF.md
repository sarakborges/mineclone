# HANDOFF — Asteria / Mineclone

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.29.0`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. **HEAD funcional/versionado imediatamente anterior a esta atualização documental:** `e848e9e918a87527764ed79616ac3d8134c0830a`. A feature de lava + fluidos de superfície do Volcano passou na CI de push `35397447773` com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. Não houve `cargo test`, `cargo run` ou QA Windows neste bloco.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.29.0`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. **HEAD funcional/versionado imediatamente anterior a esta atualização documental:** `e848e9e918a87527764ed79616ac3d8134c0830a`. O bloco funcional de lava + crater surface fluid passou na CI de push `35397375586` em `bfc41995f85aeddb54a45d762a40750b7d3291ae` com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. A run específica do bump `0.29.0` é `35397447773` — **success** — com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. Não houve `cargo test`, `cargo run` ou QA Windows neste bloco.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.28.0`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. **HEAD funcional/versionado imediatamente anterior a esta atualização documental:** `5acad2c8a2003b5c4f0e2143722908d3c3a36e11`. O bloco de adjacência obrigatória de surface biomes passou na CI de push `35396733423` com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. Não houve `cargo test`, `cargo run` ou QA Windows neste bloco.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.27.1`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. O PR #14 já foi mergeado em `develop`. **HEAD funcional/versionado imediatamente anterior a esta atualização documental:** `5e7dbdad861e4b8e877fcab47c039651525f4536`. O fix de validação Ocean/Coast está em `15b569b30a81c6c753cb76bcbe3d53bcb3fabaa7` e passou na CI de push `35395972922` com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. Não houve `cargo test`, `cargo run` ou QA Windows neste bloco.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.27.0`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. O PR #14 já foi mergeado em `develop`. **HEAD funcional/versionado imediatamente anterior a esta atualização documental:** `4778ac925d95074a0b2b8606f6c64b2da3e4a156`. CI canônica de push `35395484913` — **success** — cobrindo auditoria de localizações, Clippy com `-D warnings` e `cargo check --locked`, já com `VERSION 0.27.0`. Não houve `cargo test`, `cargo run` ou QA Windows neste bloco.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.26.1`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. O PR #14 já foi mergeado em `develop`. **HEAD funcional/versionado imediatamente anterior a esta atualização documental:** `9367da25dc6b15b4ddcd1175fd5ae3dbb0a1909c`. O fix funcional de terrain forçado está em `db05e21ed9a28f25b23202c5b5dbfe1741194319` e passou na CI de push `35393716818`; o bump `0.26.1` está em `9367da25dc6b15b4ddcd1175fd5ae3dbb0a1909c`, com run de push `35393788708` ainda enfileirada no momento desta atualização. Não houve `cargo test`, `cargo run` ou QA Windows neste bloco.

## Histórico integral obrigatório

- Etapas [0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), [13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), [17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), [21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), [25–27](docs/handoffs/auditoria-25-27-2026-09-17.md), [28–31](docs/handoffs/auditoria-28-31-2026-09-17.md), [32–36](docs/handoffs/auditoria-32-36-2026-09-17.md), [37–44](docs/handoffs/auditoria-37-44-2026-09-17.md) e [45–53](docs/handoffs/auditoria-45-53-2026-09-17.md). Ler `ARCHITECTURE.md`.
- [Handoff develop pré-Brush, etapa 54 integral](docs/handoffs/handoff-develop-pre-brush-merge-2026-09-17.md), blob `27b903e2b8d68d5603e379dc20b909a9ef8eefa0`; [branch Brush pré-merge integral](docs/handoffs/handoff-brush-pre-merge-2026-09-17.md), blob `533d6565f2044697fe70ab8135a0852dbd97ca1a`; [handoff anterior da branch Brush](docs/handoffs/handoff-pre-brush-2026-09-17.md), blob `49d20d15d857118bd660a0d619ef58a9cf37d3ab`.
- [Branch Chisel pré-merge integral](docs/handoffs/handoff-chisel-pre-merge-2026-09-17.md), blob `b2a6f4416fe6ae07998a0dac7416778502833e22`. [Handoff integrado ANTES do save de microblocks, preservado integralmente](docs/handoffs/handoff-before-chisel-save-2026-09-17.md), blob `b973a10daa44354b7531c063f2353d35cfdeb840`, commit `9c00f8f9ab5e7782c60423bc97ec1d535d1bfd53`. Nele, afirmações sobre Chisel apenas em sessão são históricas e foram substituídas pela etapa atual. Especificação vigente: [docs/chisel-microblocking.md](docs/chisel-microblocking.md).

**Regras permanentes:** commits coerentes na `develop`; atualizar este handoff após cada checkpoint com SHA, CI, limitações, próximos passos e arquivar integral antes de condensar. Semver por bloco funcional, nunca por docs isoladas. Save só pode ser considerado concluído após QA Windows real. Corrigir warnings sem supressão. CI exige auditoria EN/PT-BR/ES, Clippy rigoroso e `cargo check --locked`; NÃO adicionar nem executar `cargo test` sem autorização expressa. Não alterar PNGs/GLBs autorais. Pause NÃO congela mundo/fluidos/tempo. Preservar cache da CI, `Cargo.lock` gitignored. Nunca alegar execução Windows/medição FPS/memória ou sucesso de roundtrip sem evidência. **Feedback obrigatório de andamento:** toda vez que um novo passo de trabalho for iniciado, informar o usuário imediatamente sobre o que está sendo feito/verificado e o motivo; não executar uma sequência longa de passos em silêncio.

## Estado consolidado do projeto

Saves com mundos isolados, snapshots imutáveis e manifestos de commit, chunks modificados residentes/arquivados, inventário, posição, modo, regras, dimensão, relógio, autosave após alterações, Leave/Exit somente após publicação bem-sucedida. Fallback valida geração completa e retenção preserva quatro gerações recuperáveis. Scan, load e prune em workers; mutex/leases por mundo; logs da main thread e workers. Etapa 54: `tools/make_save_fixture.py`, fixtures em cópias descartáveis, script exercitado separadamente com dados sintéticos; [protocolo Windows](docs/save-roundtrip-qa.md) NÃO EXECUTADO. Corrida de `create_new_world` no ramo `AlreadyExists` com nome de 200 UTF-16 continua pendente. A PR #13 Brush alterou escala/pivô sem mexer em PNG e introduziu `VERSION 0.20.4`; QA visual Windows segue NOT RUN. PR #12 Chisel trouxe três precisões, opt-in `fragmentable`, preview, tooltip compartilhado, raycast/colisão subvoxel, micro-mesh e máscaras; a etapa 55 passou a salvar a forma. **Etapa 56 abaixo adiciona identidade ECS explícita ao Player e step-up compartilhado de até 0.5 bloco.**

## Etapa 55 — persistência real do Chisel, restauração sem blocos novos e mão [CÓDIGO + CI VERDE; QA WINDOWS PENDENTE]

Solicitações do usuário: microblocks devem ser salvos junto do mundo; Chisel só coloca fragmento se o bloco-alvo já teve material retirado, sem construir macrobloco novo; ícone do Chisel deve aparecer na mão pelo mesmo caminho visual do Brush.

- `cd9282952fb2da8051c468fb1feefd784a317636` (`src/tools/chisel.rs`): clique direito exige `Some(source)` no voxel destino, **`voxel == hit.voxel`** e `MicroblockMask::can_restore(source)`; remove o caminho que criava pai no ar e herdava material de bloco vizinho. Clique esquerdo permanece para remover. Pais legados `t` podem ser removidos, mas não recebem novas colocações. Verificação `fragmentable`, checagem de interseção com jogador e revision/remesh/lighting preservadas.
- `e8935efeaecde16d251b027b6daa9dc7ce985dd4` (`microblock.rs`): `can_restore` requer máscara parcial num bloco não-transiente; `valid_saved` exige 128 dígitos hexadecimais, aceita prefixo legado `t` totalizando 129 e rejeita máscara inválida. Documentação da estrutura alterada para persistente. `6650dc6137146ab4832f46564129817feb189779` (`targeting/highlight.rs`): preview verde exige mesmo macrobloco alvo, `can_restore` e `mask.edit(..., true)` efetivamente alterando microcélula. Preview no ar, vizinho intacto ou ação sem efeito é escondido.
- `28ac0176f75751f1a7e9f1cb11e2722acb4b8986` (`chunk_disk.rs`): retira filtro que omitia pais transitórios, preserva blocos esculpidos, valida tag `fragmentable`/máscara na reconstrução ANTES de internar propriedades, assegurando fallback de geração inválida sem cubo inteiro silencioso. **Armadilha encontrada na revisão:** `SecondaryProperties::iter()` ocultava máscara mesmo em blocos originais; portanto o primeiro commit NÃO persistia a forma sozinho. `9f23b1fe8c9ce2acb11c28007c87632a8d65bd4a` introduziu `iter_for_save()` sem expor a máscara a HUD/render; `87758a18e7bf26d382e7c5536549290c50b02d57` efetivamente usa `iter_for_save()` no serializador. `010aaa0aa9f68142d2a3c6d8016319347480b733` restaura assertivas preexistentes na função de regressão de propriedades; não adicionou ou executou cargo test. Tanto `save_modified_chunks()` para residente quanto para arquivado usam o mesmo DiskChunk; save snapshot/inventory/manifest, limites e fallback inalterados.
- `56ff3bd42b89e1c21a3a62a75704ee8d98730ae5` adiciona `src/player/viewmodel/held_chisel.rs` com mesh de sprite, textura do ícone cadastrado, material alpha mask/unlit, RenderLayers 1, root/grip/tamanho/rotação idênticos ao Brush e visibilidade apenas com Chisel na hotbar. `17251db75db79bbecc2029293f5c805632c2efb55` registra setup/spawn/sync no plugin de viewmodel, dentro do braço animado. Arte original intacta; não houve inspeção visual da pegada. Não há entidade por microcélula.
- `41a7bb0c007a07731931eee424d88b266e3746b4` sobe semver raiz `0.20.4 → 0.21.0` pela nova capacidade de persistência; não altera versão de Cargo ou formato de saves. `14a5f7c347a985df9638f176f547438753cc0249` atualiza [especificação e QA do Chisel](docs/chisel-microblocking.md) com comportamento atual e reconhece o modelo histórico. `9c00f8f9ab5e7782c60423bc97ec1d535d1bfd53` arquiva fonte integral pré-condensação.
- **CI verificada:** source final `010aaa0aa9f68142d2a3c6d8016319347480b733`, run https://github.com/sarakborges/mineclone/actions/runs/35262573811 completed/success: idiomas, Clippy `-D warnings`, check. Run de `87758a` https://github.com/sarakborges/mineclone/actions/runs/35262435331 também green. Não houve cargo test, cargo run, QA Windows ou teste real de reinício. Um snapshot ANTIGO sem máscara perdeu a informação; nenhuma atualização consegue reconstruir forma já descartada.

## Etapa 56 — Player como entidade ECS e step-up de até meio bloco [CÓDIGO + CI VERDE; QA WINDOWS PENDENTE]

Solicitação do usuário: **“jogador deve ser uma entidade. ajustar comportamento de entidades para poder subir meio blocos ou menor sem pulo”**.

- `cad91c8e0d6055837221b8acaf09217b7248b67e` (`src/player/mod.rs`) adiciona `PlayerEntity` como componente explícito e o coloca no root de gameplay que também carrega a câmera. O root continua sendo a entidade do jogador; a câmera permanece componente desse mesmo entity, portanto sistemas de visão continuam compatíveis. `8784bc516b4848820dc21e9b22fb793fc6bd14a2` passa a usar `PlayerEntity` no contato jogador/creatura, separando a identidade do corpo da identidade da câmera. O follow-up `7275f4b7cbb60c0271bb68fe4b72b5da5a08ef90` preserva o `Name` existente via `.insert(...)`, evitando o limite de 15 componentes da tupla.
- `bfe64298fc03a53ccf4e3f31847af6a569469d59` (`src/voxel/collision.rs`) consolida o primitivo compartilhado `try_step_up_aabb`: testa degraus de **1/8 até 1/2 bloco**, procura o menor desnível que libera o AABB, atravessa horizontalmente em subpassos e assenta sobre a superfície. Geometria microblock é consultada pela mesma `collides_aabb`; blocos de altura integral ou obstáculos acima de 0.5 continuam bloqueando. O helper rejeita volumes que atravessam células ainda não carregadas. O commit final também removeu os `collapsible_if` apontados pelo Clippy.
- `f1a96c4b9f94762262d0d83e78edb9459952a1bf` (`src/player/movement/collision.rs`) integra o step-up no movimento horizontal do Player e conserva a colisão vertical separada. `2bc51bdcce5c6fae7dd4bdb89012d690013abec8` só habilita step-up quando `GravityState.grounded` é verdadeiro, evitando que voo/pulo usem o mecanismo como teleporte vertical. A velocidade horizontal é zerada somente quando tanto o movimento normal quanto a tentativa de degrau falham.
- `cea8107fdf06012065adf421318bd5050f8b0dee` e `b3a2d92a60d7a595bf202b13a030cbf1b0607506` ajustam gravidade/voo para o novo `move_axis(..., step_up_height)` e fazem o corpo do Player ser consultado por `PlayerEntity`; natação recebeu o quinto argumento `None` em `e0cd5bbe81d6952eab9dd093cca16c8b2dc7d19b`. Voo e movimento vertical nunca ganham step-up.
- `14a9c93186e5ef7909abaede315441fc0287a47a` iniciou `VERSION 0.21.0 → 0.22.0`. A CI correspondente, run 35264644200, falhou em `swimming.rs` (chamada antiga com 4 argumentos) e no `commands.spawn` por exceder o limite de 15 componentes ao adicionar `Name` + `PlayerEntity`. `e0cd5bbe81d6952eab9dd093cca16c8b2dc7d19b` corrigiu a assinatura de natação; `d67b601a74f352c8dff59a47c358745ae145cbb7` removeu o `Name` da tupla, mantendo 15 componentes; `bfe64298fc03a53ccf4e3f31847af6a569469d59` corrigiu os warnings do Clippy no helper; `7275f4b7cbb60c0271bb68fe4b72b5da5a08ef90` recolocou o `Name` por `.insert(...)`.
- **CI final verificada:** run `35265288988` no commit `7275f4b7cbb60c0271bb68fe4b72b5da5a08ef90` ficou **success** em auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`. Não houve `cargo test`, `cargo run` ou QA Windows.
- `PlayerEntity` não cria uma entidade separada para câmera, braços ou microcélulas; é o root lógico do corpo do jogador. O step-up é compartilhável por outros AABBs, mas nesta etapa só o Player usa-o efetivamente, porque criaturas atuais têm locomoção baseada em salto e não possuem um estado de caminhada no solo que justifique aplicar a mesma subida sem alterar sua mecânica.

## Checkpoint 57 — corrida de nomes longos em AlreadyExists [CÓDIGO; CI VERDE]

- `4e3690a1acef41f678627763069927992251ae53` corrige `src/world/save_catalog.rs`: após uma corrida em `fs::create_dir` que retorna `AlreadyExists`, `create_new_world` volta a resolver a partir de `requested_name`, em vez de prefixar `Copy of ` ao candidato já resolvido. Isso evita exceder o limite de 200 unidades UTF-16 quando um nome longo colide entre o scan e a criação do diretório, e deixa `available_world_name` escolher a forma numerada/truncada segura.
- `bab9589afca22b45558716cc6dc90199c85c0401` sobe `VERSION` de `0.22.0` para `0.22.1` (fix sem mudança de formato/protocolo).
- **Validação:** não executei `cargo test`, `cargo run` nem QA Windows. A CI obrigatória concluiu com sucesso no run `35280063915` para o HEAD `0c54f7d601d78473f01d3a6553b5059dbe2f5c69`.

**Próximo passo:** QA Windows real do step-up/Player/Chisel/save roundtrip. Em paralelo, investigar macro-iluminação/fluidos/solid e crescimento do interner de máscaras com medições reais.

## Próximas ações

1. QA real Windows do step-up: caminhar contra degraus de 1/8, 1/4 e 1/2 bloco sem pressionar Space; confirmar que bloco >1/2 continua bloqueando, que não há subida enquanto airborne/voando, e que a altura final fica sobre a superfície sem jitter. Testar formas de Chisel e blocos macro normais.
2. QA do Player como entidade: salvar/recarregar posição e inventário, verificar contatos jogador-creatura, câmera/mira, hotbar/viewmodel e transição Gameplay/Pause sem depender de `GameplayCamera` como identidade do corpo. Confirmar que não surgem múltiplos Players.
3. QA real Windows do Chisel conforme [docs/chisel-microblocking.md](docs/chisel-microblocking.md) e [docs/save-roundtrip-qa.md](docs/save-roundtrip-qa.md): esculpir pedra/tronco em Thick/Thin/Extra Thin, forma assimétrica/oco/placa 1/8, arquivar chunk por streaming, salvar, fechar PROCESSO, reabrir e comparar máscara/material/UV/rotação; validar autosave e Leave/Exit. Registrar PASS/FAIL/NOT RUN, logs e medições.
4. Em cópia descartável com gerações anteriores válidas, corromper `asteria:chisel_mask` no snapshot mais recente, verificar fallback sem cubo inteiro inesperado; validar snapshot legado SEM máscara, prune e arquivos acima do render distance. Manter limite de 512 MiB e retenção de quatro backups.
5. Investigar macro-iluminação/fluidos/solid e crescimento do interner de máscaras com medições reais. Não executar `cargo test` sem autorização expressa.


## Checkpoint 58 — reduzir duplicação dos interners de IDs e propriedades [CÓDIGO; CI PENDENTE]

- 64e905d7b350ddd002721d03cebb7ed30f864e38 e f1b30bf84c42c13948054d6935b6b8ce2b00ccb9 substituem o HashMap<String, &'static str> do interner de block IDs por HashSet<&'static str>. O token canônico vazado passa a ser também a chave, eliminando a segunda String alocada para cada ID único.
- f4719fa42cb730ad49234fd46f5c102a1c407183 e efd6d412d7623fc0d4954c1c4939d6cb02dd9f8d aplicam a mesma redução ao interner global de secondary-property tokens. Isso é especialmente relevante para asteria:chisel_mask, cujos valores são strings hexadecimais grandes e podem variar conforme o jogador esculpe formas.
- 64fa681c6a2c6becd62a246b531a6b49fa786015 sobe VERSION de 0.22.1 para 0.22.2.
- A mudança não elimina o crescimento monotônico do conjunto de strings: os &'static str continuam deliberadamente vivos durante o processo para preservar VoxelCell: Copy. O ganho é eliminar a duplicação de armazenamento e reduzir overhead por entrada. Medição de heap/runtime e QA Windows ainda estão pendentes.
- Validação: não executei cargo test, cargo run ou QA Windows. No momento do registro, ainda não havia workflow reportado para o último commit.

Próximo passo: obter CI do checkpoint 58; se verde, medir/inspecionar o crescimento real do interner em uma sessão com bastante Chisel e então atacar macro-iluminação/fluidos/solid conforme o resultado.


## Checkpoint 59 — iluminação macro respeita ocupação do Chisel [CÓDIGO; CI PENDENTE]

- e5bb8568b8936c31cc2b8754373f50f409827347 adiciona MicroblockMask::light_dampening(), uma aproximação de atenuação na resolução macro baseada na fração de microcélulas ocupadas.
- a8faea87b02b10f37cc8c648f2b0eff217505b1b usa essa atenuação em lighting/medium.rs. Bloco sem máscara continua equivalente a FULL; máscara parcial deixa de bloquear luz como se fosse um cubo integral; máscara vazia transmite sem atenuação de bloco.
- ac2224c2b130395c9d3931915e7a61df0fffb8b7 adiciona regressões para EMPTY/FULL e ocupação de meio volume.
- 117f7ddf45e3d783bdc2353437fc21a1a159a836 sobe VERSION para 0.22.3.
- A aproximação é deliberadamente isotrópica porque VoxelLight existe na resolução de macrovoxel; não tenta reconstruir transmissão direcional de cada furo do Chisel.
- Validação: não executei cargo test, cargo run ou QA Windows. CI ainda não estava disponível para os commits novos no momento do registro.

Próximo passo: verificar CI; depois revisar se AO/face lighting também precisa distinguir máscaras parciais e investigar o caminho de fluidos/solid com a mesma preocupação de macro resolução.


## Checkpoint 60 — atenuação de luz de fluidos por nível [CÓDIGO; CI PENDENTE]

- 15ab12f6cb177909889bd753d0893233c1ce41bd escala a atenuação configurada do fluido pela altura real do FluidCell. Um fluido de nível 8 mantém a atenuação integral; níveis menores deixam passar proporcionalmente mais luz em vez de tratar qualquer presença como uma coluna cheia.
- 25bde922682378bac6d31858a668240d81b36dfc adiciona regressão para níveis 1/4/8 com dampening integral 15.
- a23cadf1f903f65dc7760b4bbd99bfaecdbaf2b1 sobe VERSION para 0.22.4.
- Validação: não executei cargo test, cargo run ou QA Windows. Há CI da PR em andamento, mas ainda não foi observado resultado verde para os commits mais novos.

Próximo passo: aguardar/consultar a CI atual; revisar face_lighting/AO para máscaras parciais e, se não houver regressão, continuar nos casos de solid/fluid boundary.


## Checkpoint 61 — corrigir falha de CI da atenuação de fluidos [CÓDIGO; CI PENDENTE]

- A CI do checkpoint 60 falhou no Clippy antes do `check`; a causa objetiva foi o teste referenciar `scale_dampening` antes de a função auxiliar existir.
- 3db2d1668d81c17fae10bdbc135b8130726da94b adiciona a função e a usa no caminho de produção.
- Não executei `cargo test` nem `cargo run`; a correção depende da nova CI.

Próximo passo: verificar a CI do checkpoint 61; se verde, prosseguir com AO/face lighting ou solid/fluid boundaries.


## Checkpoint 62 — corrigir os construtores dos interners após a otimização [CÓDIGO; CI AGUARDANDO]

- A CI do checkpoint 61 confirmou a falha objetiva já suspeitada: os tipos foram trocados para HashSet, mas os closures de inicialização ainda chamavam HashMap::new(). O Clippy/compilação parou em src/content/block_id.rs e src/voxel/secondary_properties.rs.
- 6082869c8877081173b05557249c1538630f9938 corrige o construtor do interner de block IDs para HashSet::new().
- 91b2d001672fbf6f74121afe5fdee353ed954f67 corrige o construtor do interner de tokens de secondary properties para HashSet::new().
- O código de produção agora é coerente com a estrutura HashSet<&'static str>; não executei cargo test nem cargo run.
- A run 35280905635 falhou no Clippy antes do check; depois das correções não há run reportada para os novos commits nesta integração, portanto CI ainda está pendente.

Próximo passo: validar a nova sequência por CI quando disponível e, sem esperar passivamente, continuar a revisão de iluminação/AO e fronteiras sólido-fluido, sempre sem cargo test/cargo run.


## Checkpoint 63 — AO de geometria parcial respeita ocupação do Chisel [CÓDIGO; CI AGUARDANDO]

- d40bd62b4d6f224ac8043af6fe7626c39b0b8ecb adiciona MicroblockMask::occupied_fraction(), reutilizando a mesma representação 8³ do Chisel sem criar entidades por microcélula.
- ba4f0cf98e279867fea3792055fa8680c8978d02 e d9f3505fb511c89f2912c0d4de7132d5afe4b7f fazem o AO de vértices interpolar a ocupação do vizinho parcial em vez de tratar qualquer máscara Chisel como cubo sólido. Blocos integrais mantêm o resultado anterior; uma máscara vazia deixa de contribuir como occluder. O caso de dois lados totalmente sólidos continua saturando o AO máximo.
- A mudança é deliberadamente isotrópica no AO, assim como a aproximação de dampening: ela usa fração ocupada do macrobloco, não inventa direção subvoxel que a resolução atual de VoxelLight não armazena.
- 0.22.5 registra a nova alteração funcional. Não executei cargo test, cargo run ou QA Windows.

Próximo passo: obter CI; se verde, continuar a revisão de fronteira sólido/fluido e de máscaras parciais na geometria de faces. Se a CI apontar regressão, corrigir antes de outro checkpoint.


### Checkpoint 63 follow-up — ajuste da regressão de teste

- 8bb1c86fea50c9a995658c57fb3180c71fe80d17 corrige o caso de regressão do AO: uma máscara EMPTY sem prefixo transitório é normalizada para FULL ao persistir, então o teste precisa manter a máscara com o caminho transitório para verificar ocupação zero. Nenhuma lógica de produção foi alterada neste follow-up.
- A run 35281227881 foi observada para o commit anterior d9f3505fb511c89f2912c0d4de7132d5afe4b7f4 e estava em andamento; este follow-up ainda não tem run reportada no conector.


## Checkpoint 64 — correções encontradas pela CI do AO/fluid [CÓDIGO; CI PENDENTE]

- A CI 35281260132 encontrou cinco erros no checkpoint 63: os testes novos não importavam os helpers privados, e Clippy rejeitou a implementação manual de divisão por teto em scale_dampening.
- 6482ba4902091ff81f929f2d8e9ec2eefcb9b24b importa ao_brightness/sample_occlusion nos testes.
- 5bc03e5e768702ed9325172aa870a82d48daf692 usa u16::div_ceil(), preservando a fórmula anterior e removendo o lint clippy::manual_div_ceil.
- CI 35281390697 concluiu GREEN após esses ajustes.
- Não executei cargo test/cargo run; a CI é a validação automática usada para estes ajustes.


## Checkpoint 65 — iluminação de Chisel parcial pondera abertura [CÓDIGO; CI CORREÇÃO PENDENTE]

- 74c41cb36314f08663e81ed1e06d705bea61ba61 alterou average_shader_light_levels para ponderar sky/block light pela fração aberta do macrocell; blocos completos continuam contribuindo 0 e células vazias 1.
- 1ad52d13eedc2b4e4f228afb3e69267fbdd67c6e simplificou o teste de fração aberta de um Chisel 50% ocupado.
- Isso complementa o AO/dampening isotrópico: aberturas de Chisel agora também deixam a luz amostrada atravessar proporcionalmente, sem fingir direção que a resolução macro não possui.
- Não executei cargo test/cargo run.

- CI 35281655138 falhou apenas nos testes recém-adicionados: havia `#[test]` duplicado e o teste acessava o campo privado `layers`. 1ad52d13 foi corrigido em a1f6141bc1f77107926eb9b605576e300ab2a82d usando a API pública `edit` e removendo o atributo duplicado. Nova execução de CI ainda não apareceu no endpoint de runs.

## Checkpoint 66 — fluido atravessa aberturas de Chisel na fronteira [CÓDIGO; CI PENDENTE]

- 0edbf363e6768760ec0f592a50068a09e61f2baa adiciona detecção de abertura na face do vizinho Chisel parcial; 44ce27a899b36b00bf75cec4d243dcc9ade34035 corrige os imports dos testes.
- O mesh de fluido agora mantém a face oculta diante de bloco integral, mas permite a face macro do fluido quando a face do bloco vizinho tem ao menos um microvazio. A geometria sólida continua na frente e o depth buffer oculta a parte coberta, deixando a superfície do fluido aparecer apenas pelos vãos.
- Foram adicionados testes para face parcial aberta e bloco integral fechado.
- VERSION foi atualizado para 0.22.6 em 19f593d958d6b3ac5e2ff29ec92f8f8f8142778b.
- Não executei cargo test/cargo run. CI pendente.


## Checkpoint 67 — cobertura direcional da fronteira fluido/Chisel [CI VERDE]

- 4c15ac4341b9f2249f4e1c156a816aabf0f58b88 amplia o teste da abertura parcial para Right/Left/Top/Bottom/Front/Back, validando o mapeamento de cada face.
- CI 35281985377 (run 3037) concluiu GREEN; Clippy e Check passaram.
- O checkpoint 66/67 não foi validado por cargo test/cargo run localmente, conforme restrição do projeto.

Próximo passo: continuar a revisão de geometria parcial/iluminação e corrigir regressões encontradas pela CI antes de avançar.


## Checkpoint 68 — correção de orientação da face fluido/Chisel [CI PENDENTE]

- 2812484587f5bd9e32fa0bbe0cdfecf71d9b2a37 corrige a leitura da fronteira do vizinho: Front consulta a camada Z=7 (Back do vizinho) e Back consulta Z=0 (Front do vizinho). Right/Left e Top/Bottom já estavam orientados corretamente.
- CI nova 35282185332 (run 3046) está em andamento.


## Checkpoint 69 — teste de fronteira específica do Chisel [CÓDIGO; CI PENDENTE]

- 474761b1516b2524001ef1c447f39c330a539ab4 adiciona teste que exige a abertura na própria fronteira consultada e rejeita abertura somente na fronteira oposta para Right/Left/Top/Bottom/Front/Back. Isso fixa o detalhe de orientação corrigido no checkpoint 68 e reduz o risco de regressão silenciosa.
- e6c2b959ae61f3a0e83ee51d00ef1cbe0ce6b4f7 extrai occupied_count() em MicroblockMask para reutilizar o mesmo cálculo em occupied_fraction() e light_dampening(), sem mudança funcional.
- 79961ab3baf8ad4b2924f27994a390de0c9f71be adiciona cobertura para ponderação de amostras de luz pela fração aberta do Chisel.
- Não executei cargo test/cargo run; validação segue pela CI.


## Checkpoint 70 — correção do mapeamento Front/Back após revisão de semântica [CÓDIGO; CI VERDE]

- 1a06f434f4574a3ef175b9d35ac6a8b81e917768 corrige a correção anterior: como face_is_exposed recebe o vizinho em world_voxel + face.offset(), a face Front (+Z) consulta a fronteira Z=0 do vizinho e Back (-Z) consulta Z=7. Os testes direcionais foram ajustados para validar a fronteira próxima correta.
- A revisão de BlockFace::offset, unit_vertices e emit_neighbor_openings confirmou essa orientação: para uma face positiva do voxel de origem, a fronteira correspondente do vizinho é a camada 0; para uma face negativa, é a camada 7.
- CI 35282728940 (run 3057) concluiu GREEN após a correção e a cobertura direcional. Não executei cargo test/cargo run.

## Checkpoint 71 — clareza da fronteira Chisel/fluido [CÓDIGO; CI PENDENTE]

- d5274635815ed1ffaca69dc10b740886ae79a615 elimina índices mágicos `7` em `partial_block_face_has_opening`, reutiliza a dimensão `MICROBLOCK_EDGE` e remove qualificação redundante de `VoxelCell`; comportamento e mapeamento direcional permanecem inalterados.
- A mudança é deliberadamente pequena: mantém o loop explícito de 64 microcélulas, evitando uma otimização bit-level sem API pública adequada.
- CI 35282920752 (run 3060) detectou que a limpeza removeu uma importação necessária de `VoxelCell`; correção aplicada em ca98dc1b9564d10b10cf0353932ea8e43de35ca8.
- Não executei cargo test/cargo run.

## Checkpoint 72 — correção real do import do VoxelCell [CÓDIGO; CI PENDENTE]

- 3060/35282920752 falhou porque a tentativa anterior de cleanup referenciava VoxelCell sem import no escopo de produção. A primeira correção não foi aplicada no local correto; revisão do arquivo confirmou isso.
- c3810b618aa54ff808a0270482226d5546a5ac67 adiciona explicitamente voxel::cell::VoxelCell ao import do módulo. Esta é a correção efetiva.
- CI 35283168150 (run 3065) concluiu GREEN, validando a correção do import. Não executei cargo test/cargo run.


## Checkpoint — smooth stair stepping + target highlight depth
- Ajustado o movimento de subida de degrau para separar a posição física elevada da elevação visual: a colisão continua usando a altura final segura, enquanto a câmera/jogador sobe progressivamente com `STEP_SMOOTH_SPEED`.
- `move_axis` agora diferencia movimento livre, bloqueio e `Stepped(position)`; gravidade foi adaptada ao novo resultado.
- O highlight de alvo recebeu `StandardMaterial.depth_bias = 0.01` para evitar que a malha translúcida fique atrás da textura por disputa de profundidade. Bevy 0.19 documenta `depth_bias` como ajuste de profundidade para malhas com profundidade semelhante.
- Não executei `cargo test`, `cargo run` ou QA Windows; validação deve ser feita pela CI.


## Checkpoint 73 — fluido recortado pelos vãos do Chisel [CÓDIGO; CI PENDENTE]

- daebec8d1118e2543323ab9f1977f6b422f86a49 passou a recortar as faces do fluido quando o mesmo macrovoxel também possui um bloco Chisel parcial. Antes, o fluido gerava uma quadra macro inteira e podia aparecer atravessando a parte sólida; agora só os microvãos da fronteira são emitidos.
- 1f873230809a92deecd395e93042355552f1b396 corrige a interpolação da altura nas faces laterais, mantendo a superfície do fluido alinhada às alturas dos cantos do macrovoxel.
- a071432b16cc0352d8f52c58a9840e907dd7e019 adiciona regressão para impedir que a malha lateral do fluido continue acima da altura da água.
- c5be8c12648a348cc951894f99eab5c2ed8f4e47 atualiza VERSION para 0.22.7.
- Não executei cargo test/cargo run/QA Windows. A CI de push para develop é configurada no repositório, mas a integração disponível para consulta de runs neste contexto expõe apenas runs associados a pull requests; portanto o status deste HEAD precisa ser confirmado pela CI do GitHub.


## Checkpoint 74 — correção da explosão de erros do recorte Chisel/fluido [CÓDIGO; CI NÃO VISÍVEL]

- c8c820f8e203073e157cc49e401b186908afd18a torna `FaceLighting` `Clone + Copy`, permitindo reutilizar a mesma iluminação ao emitir várias microquads sem erro de move.
- 94af19e2100edfef2a93eb95723414a2b41a5416 corrige a máscara usada pelo recorte: a face do fluido agora consulta o bloco Chisel vizinho em `world_voxel + face.offset()`, em vez do bloco do próprio voxel do fluido.
- 6387a4e8ba6b5f60ec5159ae0863bd8199d1a4c4 marca explicitamente os dois helpers de recorte com `#[expect(clippy::too_many_arguments)]`, evitando que o `-D warnings` do CI derrube o build por esses helpers deliberadamente parametrizados.
- Não executei `cargo test`, `cargo run` ou QA Windows. A integração disponível aqui continua sem expor os runs de push do branch `develop` via consulta por commit, então não marquei CI como verde.


## Checkpoint 75 — estabilização do degrau suave [CÓDIGO; CI NÃO VISÍVEL]

- 9f2c393a728b84fe69374630fecadf68a72d1fe6 torna o `GravityState` mutável no sistema de caminhada apenas durante o step-up suave e mantém `grounded = true` com velocidade vertical zerada enquanto a interpolação de altura está ativa. Isso evita a gravidade competir com a subida visual e puxar o jogador para baixo no meio do degrau.
- 4c7e6b1c5bab10efc01efb42dea2c176a820baf7 adiciona cobertura unitária para a interpolação do step, incluindo aproximação sem overshoot e movimento descendente.
- A ordem dos sistemas já é encadeada em `PlayerMovementPlugin`, então `walk` termina de atualizar o estado antes de `apply_gravity`.
- Não executei `cargo test`, `cargo run` ou QA Windows.


## Checkpoint 76 — robustez do recorte lateral de fluido [CÓDIGO; CI NÃO VISÍVEL]

- d370e3e46cbb75c2bc814f599b8c8a0ac98974ff corrige a decisão de emissão das microfaces laterais inclinadas: o recorte agora considera a maior altura entre os dois cantos da célula, em vez de depender apenas da altura no ponto médio. Isso evita sumir com uma microface válida em bordas inclinadas da superfície.
- A sequência de correções desta rodada permanece sem `cargo test`, `cargo run` ou QA Windows, conforme restrição do projeto.


## Checkpoint 77 — bloco + fluido no mesmo voxel [CÓDIGO; CI NÃO EXECUTADA]

- 0d32627fd18fdb5e8cda7a1c8761fe7e909e70d5 remove a rejeição do formato de save que impedia bloco e fluido no mesmo índice. O runtime já mantém canais independentes de bloco e fluido no `VoxelChunk`, então o save/load agora preserva os dois.
- bb9b49e301c44f64533a4e22806e2abbdffb80a6 faz a malha de fluido considerar simultaneamente a máscara Chisel do próprio voxel e a máscara do vizinho. Assim, quando bloco e fluido coexistem, o fluido só ocupa/renderiza microcélulas abertas; um bloco vizinho também fecha os microvãos correspondentes.
- 530f11a278f8c9ec25267948cfdefc2e2831cde7 adiciona regressão de runtime para bloco e fluido compartilharem o mesmo voxel sem perder nenhum canal.
- e61def3c22e94fea9304ff02c9dc5b71937220db atualiza VERSION para 0.22.8.
- Não executei `cargo test`, `cargo run` ou QA Windows.


## Checkpoint 78 — CI corrigido após leitura do run 3122

- Run CI `35285853346` (run 3122), associado ao PR #7, falhou no Clippy por três usos antigos de `move_axis() -> bool` em `src/player/movement/flight.rs`; o log confirmou `expected bool, found MoveAxisResult`.
- d48a0dca7b19bd58b4c31a91ebc42370305cf4c2 atualiza os três eixos do voo para tratar `MoveAxisResult::Blocked | MoveAxisResult::Stepped(_)`.
- As correções anteriores de `walking.rs` e `swimming.rs` já estão no `develop` atual.
- Não executei `cargo test`, `cargo run` ou QA Windows; a validação foi baseada diretamente no log do CI.


## Checkpoint 79 — segunda correção objetiva da CI do step-up [CÓDIGO; CI PENDENTE]

- Run CI `35285959254` (run 3125) falhou no Clippy depois da correção de `flight.rs`. O log mostrou exatamente dois `clippy::collapsible_if` em `src/player/movement/walking.rs:103` e `:115`, nos testes de movimento dos eixos X/Z.
- `4e8b159f5d54c8db618a0c5baa12b02819afb4ca` colapsa os dois `if` aninhados para as condições compostas sugeridas pelo próprio Clippy, sem alterar a lógica de bloqueio/step-up.
- O CI executa Rust 1.98.1 com `cargo clippy --locked --all-targets --all-features -- -D warnings`; portanto o warning é erro de build e foi tratado diretamente, sem supressão.
- Não executei `cargo test`, `cargo run` ou QA Windows. É necessário aguardar a próxima CI para confirmar que não há outro erro no conjunto integrado.

Próximo passo imediato: consultar a nova CI do HEAD e corrigir todos os erros objetivos restantes antes de iniciar outro item funcional.


## Checkpoint 80 — alinhar código ao novo slime.json/PNG [CÓDIGO; CI PENDENTE]

- O branch develop atual já contém o novo data/creatures/slime.json (asteria:slime) e o novo assets/textures/creatures/slime.png; não havia mais meadow.json/ember.json no tree atual.
- O código de produção já é data-driven e aceita exatamente o schema atual: textures por material, materialTints, animations, collider e parâmetros de movimento; visual.rs resolve nomes de materiais/clips a partir do JSON e motion.rs usa os parâmetros da definição.
- A única inconsistência encontrada era um teste de validação em src/content/creature.rs que ainda esperava o caminho antigo textures/creatures/meadow_slime.png. db8f3f1d2c6c72b8291c279ae0be05a9040bd043 atualiza o teste para textures/creatures/slime.png.
- O VERSION já está em 0.22.8; não foi incrementado porque esta correção apenas alinha uma regressão de teste ao asset já adotado, sem mudança de comportamento do jogo.
- Não executei cargo test, cargo run ou QA Windows. CI pendente para este commit.

Próximo passo imediato: consultar a CI do novo HEAD e corrigir qualquer erro objetivo restante antes de avançar.


## Checkpoint — 2026-09-17: entity damage feedback and data-driven player health
- Shared `src/entity.rs` now owns `EntityHealth` and the generic damage-flash lifecycle.
- Damage starts a 0.15s red material flash using `Color::srgba(1.0, 0.0, 0.0, 0.5)` and restores the original material afterward; the system walks entity descendants so it also applies to a future player model.
- Creature attacks keep using the block-break viewmodel animation and now switch to `death` when health reaches zero instead of despawning immediately, allowing a future Death clip to play. Dead creatures no longer move.
- Added data-driven `data/entities/player.json` and `content::player::PlayerDefinition`; player spawn initializes the shared `EntityHealth`.
- Slime JSON already declares `hurt` and `death` animation states; the current asset can omit those clips until they are authored, and the animation resolver simply skips missing clips.
- CI runs 3195/3194 failed on the pre-refactor private `EntityHealth` import and the Bevy 16-component player tuple; both were corrected. Latest CI runs 3197/3199 are currently in progress; no local `cargo test`/`cargo run` was executed.


## Checkpoint — 2026-09-17: hurt animation replaces red flash
- Removed the generic red damage flash entirely; damage feedback is now animation-driven (`hurt`), as requested.
- The shared `EntityHealth` remains for data-driven health/death state, without visual flash state.
- Slime already declares `hurt` and `death` animation clips in `data/creatures/slime.json`; next asset work is to author those clips in `models/creatures/slime/slime.glb`.


## Checkpoint — 2026-09-17: slime health and per-material skins
- `data/creatures/slime.json` now explicitly declares `health: 10.0`.
- Slime material textures are now data-driven per mesh material: `SlimeShell -> textures/creatures/slime/shell.png`, `SlimeCore -> .../core.png`, and `SlimeFace -> .../face.png`.
- Updated `assets/models/creatures/slime/generate_slime.py` to generate/use the three standalone skins and add a dedicated `SlimeFace` mesh/material to newly generated GLBs.
- The checked-in `slime.glb` is binary and was not regenerated through the GitHub text-file API in this checkpoint; the generator is the source of truth for the updated GLB layout.


## Checkpoint 81 — 2026-09-18
- Fixed creature hit animation retriggering: CreatureAnimationState now carries a revision counter, so repeated hurt hits restart the non-looping clip even when the state name remains hurt.
- Replaced the HUD's placeholder player health bar with live EntityHealth data and added the same live health bar to the targeted-creature card. Both show current/max values and update as health changes.
- Current branch head before this handoff commit: 4df1c97efb16fc517fd64f66e785551dabf9aa3a.
- CI for the current code is running; no local Cargo test/run was executed.

## Checkpoint 82 — 2026-09-18: knockback, death cleanup and slime face alignment [CÓDIGO + CI VERDE]

- O ataque data-driven agora aplica knockback com impulso visível: a força configurada é convertida em velocidade horizontal inicial e amortecida pelo movimento da criatura, sem atravessar AABB sólido.
- Criaturas que chegam a zero de vida continuam tocando a animação Death por 0,75s e então são despawnadas do ECS. Isso elimina o estado anterior em que o visual sumia mas o root da entidade permanecia contado como spawn.
- O mesh Face do slime estava com deslocamento vertical duplicado: o centro da geometria já continha Y=0,5 enquanto o BodyPivot também fornecia Y=0,5. O gerador foi corrigido para usar centro Y=0; como o GLB binário existente ainda pode conter a versão antiga, o carregamento também normaliza o Transform do node Face para Y=-0,5, corrigindo o asset atual sem sobrescrever arte autoral.
- CI 35292988076 (run 3332), no HEAD a4ac9989eaa2b7921d2b6507929b6b9ccd598dee, concluiu success em auditoria de localizações, Clippy rigoroso e cargo check --locked. O commit posterior 77b847822b6daa5ac5ba3018c0028e3f1ae655bb teve a mesma correção de visibilidade do construtor do timer e está com CI em andamento; a run verde confirmada ainda é a anterior ao último ajuste de visibilidade.
- Não executei cargo test, cargo run nem QA Windows.

Próximo passo imediato: aguardar a CI do HEAD 77b847822b6daa5ac5ba3018c0028e3f1ae655bb; depois validar em gameplay o deslocamento do slime, a remoção após Death e o knockback.

## Checkpoint 83 — knockback 3x + face/material override do slime [CÓDIGO; CI PENDENTE]

- O knockback do ataque foi multiplicado de 8x para 24x sobre a força configurada, deixando o mesmo ataque aproximadamente 3x mais forte sem alterar o JSON de gameplay.
- A correção anterior da face aplicava Y=-0.5 no nó Face; isso contradizia o GLB gerado atualmente, em que Face já é filho de BodyPivot e sua geometria está centrada em Y=0. O runtime agora zera translation/rotation/scale do nó Face, eliminando deslocamento/inclinação residual de GLBs antigos.
- O material do slime agora é forçado em runtime para metallic=0, roughness=0.85, reflectance=0, emissive preto, alpha 1 e AlphaMode::Opaque, inclusive quando o material vem do GLB antigo. A textura externa continua sendo aplicada normalmente.
- Não executei cargo test/cargo run nem QA Windows. CI precisa validar a compilação; depois disso a validação visual local deve confirmar face e aspecto fosco.


## Checkpoint 84 — target highlight vence layers de textura [CÓDIGO; CI PENDENTE]

- `303bc3d32633121976f0311398f8617c9c515d4a` aumenta o `StandardMaterial.depth_bias` do highlight de `0.1` para `4.0`.
- O objetivo é superar diferenças reais de profundidade introduzidas por texture/parallax layers, não apenas evitar z-fighting. No Bevy 0.19, `depth_bias` positivo aproxima a profundidade do mesh da câmera e pode ser usado para forçar a ordem de renderização entre superfícies próximas.
- Mantida a escala `1.025` e o depth test: o highlight continua respeitando geometria realmente distante, mas passa a ficar à frente de layers muito próximas da face selecionada.
- Não executei `cargo test`, `cargo run` ou QA Windows. CI/validação visual local ainda pendentes.


## Checkpoint 85 — highlight em câmera de overlay dedicada [CÓDIGO; CI PENDENTE]

- `c7e116a570bd8187e2e09e589915c1c93597fdfc` reserva a ordem de câmera 1 para o highlight, deslocando viewmodel para 2 e HUD para 3.
- `cc025db9e689098937fc1319cda287b48156da83` cria uma Camera3d filha da câmera do Player, com `ClearColorConfig::None`, ordem 1 e RenderLayer 2. Ela usa exatamente a mesma Transform herdada da câmera principal.
- `e86b2a784541ba3c0241b9e6e5397b50dfc135ad` coloca o TargetHighlight e o ChiselPlacementGhost no RenderLayer 2. Assim o overlay é composto depois do mundo, em câmera separada, sem depender de `depth_bias` para atravessar layers de textura/parallax.
- A abordagem anterior (`depth_bias = 4.0`) não resolveu o problema visual reportado; por isso o mecanismo foi trocado para composição por câmera/layer, que é mais determinístico para overlay.
- Não executei `cargo test`, `cargo run` ou QA Windows. CI/validação visual local pendentes.


## Checkpoint 86 — 2026-09-18: rollback da câmera de overlay e ajuste somente do highlight

- Removida a câmera adicional de highlight; o player voltou ao modelo de uma única câmera de gameplay. O viewmodel continua usando sua câmera própria já existente.
- Removidos também o RenderLayer exclusivo e a ordem de câmera reservada para o highlight.
- O TargetHighlight voltou a ser renderizado pela câmera principal, como antes.
- O ajuste ficou isolado no próprio highlight: escala 1.02 e deslocamento de 0.02 na normal da face atingida, para colocar o shell ligeiramente à frente da superfície/layer selecionada sem criar outra câmera.
- O ChiselPlacementGhost também voltou ao layer normal; sua lógica de posicionamento não foi alterada.
- Não executei cargo test, cargo run nem QA Windows. A CI do novo HEAD ainda precisa validar a compilação.


## Checkpoint 87 — 2026-09-18: correção do erro de compilação do rollback

- CI 35295095691 (run 3377) falhou em `src/player/mod.rs` porque o rollback deixou `.insert(...)` como expressão final da função, retornando `&mut EntityCommands` em vez de `()`.
- Corrigido adicionando o ponto-e-vírgula no `.insert(...)`.
- Não executei cargo test/cargo run nem QA Windows.


## Checkpoint 88 — 2026-09-18: unificação do design system, settings e correção de versionamento [CÓDIGO + CI VERDE]

- O toggle de HUD teve o thumb corrigido para manter espaçamento interno simétrico entre esquerda e direita considerando a borda de 2 px; a geometria não usa mais o erro anterior que deixava o thumb colado à direita.
- O card principal de settings voltou a usar padding simétrico de 18 px em todos os lados; a remoção anterior isolada do padding direito foi revertida.
- A ordem de navegação de settings agora é Graphics → HUD → Languages.
- O dropdown de idioma passou a registrar efetivamente o sistema de fechamento por clique fora; os dropdowns de HUD e biome já usam a mesma regra de fechamento.
- O sistema de botões foi consolidado em um único construtor canônico `ui::button::button(...)`, parametrizado por largura, altura e `ButtonVariant`. Foram migrados settings navigation, game rules, new world, game mode, pause menu, starting screen e world selection. Helpers paralelos como `standard_button`, `standard_button_with_marker`, `compact_control_button`, `button_with_marker`, `primary_button` e `danger_button` foram removidos do design system.
- A limpeza do sistema único também removeu os tokens mortos `theme::BUTTON_TEXT` e `typography::button_label`, apontados pelo Clippy após a consolidação.
- Commits principais desta rodada: `3769544708085555c33c445a6b31f6dda6a0ed65` (padding direito removido durante investigação), `70303cffb1c405f991d787ce892272f42dfb4103` (restauração do modelo original do thumb), `5fd6c0c49b187b7c4227a353ccdf3e6919f07743`, `0f823ac5dc84cc4e655f5a23895a0b0eb6a66c9f`, `8aa201d85f3ee82d87776bec7ec661f2ff2e67f2`, `842288e09f51ba7c110a4c6d3879cd2cdcfeea63`, `a109f0efd5959d48410f0c40df879ebab5c1260d`, `f17ac93517327694ead704e80bb83086ccf7adbc`, `3dcd6ecc7e19f903a96a65e402b6b7cd34766211`, `c5c36a1b2d39fd24085da0c90fb9cec8e0e867e9`, `3d062436a481326dae2ee68698bed6839b5af406`, `f6881402b066e5fc6bcd44086e0b1e4e61f66783`, `2e2072e6fc4ef681272f80822967b6c4cca07dad`, `aa6fa75d418157c435cd9e46ef21c3fde8b4ca9c`, `4be35a6ae78e77f34ad7a2faa51c0f65cfbd5465` e `05e6eb2ad7e31de2db63bf0d187b1e9fbdaa41bd`.
- A auditoria de telas identificou que os botões de ação estavam fragmentados em vários helpers locais/semânticos apesar de pertencerem ao mesmo componente visual; esse é o motivo da consolidação. Controles que não são botões de ação — dropdowns, text/numeric inputs, sliders e toggle — permanecem componentes próprios do design system ou candidatos a extração quando ainda estiverem implementados localmente.
- Revisão retroativa de versão: `0.22.8` cobria o último bloco de Chisel/fluidos. Os checkpoints posteriores de EntityHealth, hurt/death, HUD de vida, knockback e cleanup de criaturas formam o bloco funcional `0.23.x`. Como esses bumps não foram feitos na época, o histórico não foi reescrito; o handoff registra a lacuna e a versão atual avança diretamente para `0.24.0` para o bloco de unificação de UI/design system.
- `06a5662853108641f331a22ef66bc70f9d5ea1d7` atualiza `VERSION` para `0.24.0`. `Cargo.toml` continua em `0.10.16` por design para não invalidar fingerprints do Cargo em bumps funcionais.
- CI: runs 3756/3758 expuseram erros objetivos da migração (assinaturas antigas no pause menu e depois símbolos mortos do botão antigo), corrigidos em `aa6fa75d...`, `4be35a6a...` e `05e6eb2a...`. O HEAD funcional `06a56628...` foi validado com **success** no run 3764 (push, `35369262539`) e no run 3765 (PR, `35369266427`).
- **Regra operacional adicionada ao handoff:** a cada novo passo, o usuário deve receber feedback imediato do que está acontecendo e do que será verificado; não trabalhar em silêncio durante uma sequência longa.

Próximo passo imediato: concluir a auditoria das telas/HUD procurando componentes visuais locais que duplicam primitives existentes em `src/ui`, com feedback ao usuário antes de cada novo passo.

## Checkpoint 89 — 2026-09-18: auditoria completa de telas/HUD e refatoração do design system [CÓDIGO + CI VERDE]

- A auditoria cobriu as 5 telas registradas por `ScreensPlugin` (loading, pause, settings, starting e world selection), os módulos de Settings e os 13 módulos registrados pelo HUD. O objetivo foi verificar ownership visual/interativo contra `src/ui`, sem transformar semelhança superficial de `Node` em abstração genérica.
- Resultado da auditoria: botões de ação já estavam consolidados no checkpoint 88; `numeric_input`, scrollbar, typography, transitions e visibility estavam corretamente centralizados. As duplicações reais restantes eram (1) estados/cores de controles selecionáveis alojados em `surface.rs`, (2) dropdowns completos repetidos em Language, Target Block Position e Spawn Biome, (3) toggle implementado dentro de `hud_section.rs`, e (4) styling de campos editáveis repetido em World Name, Spawn Biome Search, Inventory Search, Chat e numeric input.
- `src/ui/selectable.rs`, antes vazio, passou a ser o owner de cores/estados normal, hover, pressed, selected e danger, além da aplicação change-aware de background/border. Hotbar, target HUD, entity card, Inventory e os dropdowns de Settings foram migrados para esse owner. `surface.rs` voltou a cuidar somente de superfícies/containers e os helpers mortos `modal_panel`/`pause_panel` foram removidos.
- `src/ui/dropdown.rs` agora possui a geometria canônica de root/control/panel/options, chevron, `DropdownState<M>` tipado e detecção compartilhada de clique fora. Language e Target Position usam diretamente o shell simples; Spawn Biome reutiliza root/control/state/option/panel base, mas mantém busca, filtro, lista e side effects no domínio da tela. Essa separação é deliberada.
- `src/ui/toggle.rs` foi criado como primitive canônico. O toggle de Display Tooltips usa esse primitive e preserva exatamente a geometria aprovada: 52×30, thumb 20, borda 2 e inset 3, incluindo o cálculo simétrico do thumb habilitado.
- `src/ui/text_input.rs` passou a possuir `frame_surface(...)` e `editor_style(...)`, centralizando fill, focus border, cursor, fonte/cor e no-wrap. Foram migrados numeric input, World Name, Spawn Biome Search, Inventory Search e Chat. Foco, validação, buffer, busca e lifecycle continuam no módulo de domínio correspondente.
- O slider de render distance **não** foi extraído para um novo primitive: existe um único slider concreto e não há repetição de invariant/behavior suficiente para justificar abstração. O world banner, crosshair e layouts de HUD também permaneceram locais quando a semelhança era apenas estrutural/visual específica.
- `ARCHITECTURE.md` foi atualizado para declarar explicitamente os owners: action buttons → `ui::button`; selectable states → `ui::selectable`; containers → `ui::surface`; dropdown shell/state → `ui::dropdown`; toggle geometry/state visuals → `ui::toggle`; editable-field visuals → `ui::text_input`. Regras de domínio continuam nos módulos consumidores.
- Commits estruturais principais: `f0163926...` (selectable), `736cda53...` (surface), `68c247ca...` (dropdown), `02b0752c...` + `57d9cfb9...` (toggle), `8e247727...` (text input), com migrações subsequentes das telas/HUD. `36a955fa...` atualiza o contrato arquitetural e `2e5e5557...` sobe `VERSION 0.24.0 → 0.24.1`.
- Validação: o refactor de código em `d2f60227427f06d9e6d6b2e643edd4681efaf1bd` passou no run `35370715741` (3824). O HEAD funcional/versionado `2e5e5557bc07126c4c559b942b18b7f3e5aaadb0` passou no run `35370808195` (3828), incluindo auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows. Não houve alteração de gameplay/world logic neste checkpoint.

Próximo passo imediato: QA visual Windows das telas afetadas (Settings dropdowns/toggle/text fields, New World biome/name, Inventory search/selection e Chat) para confirmar equivalência visual/interativa após a refatoração. Depois retomar as pendências funcionais/QA já registradas no handoff.

## Checkpoint 90 — 2026-09-18: correções priorizadas de UI, targeting e surface tunnels [CÓDIGO + CI VERDE]

- Pacote priorizado a partir do feedback do usuário: (1) clique interno de dropdown não pode fechar o dropdown; (2) Hotbar/Player HUD não podem aparecer no Pause/Settings; (3) creature com vida zero não pode continuar clicável/targetável; (4) delete em Load World deve remover o item imediatamente; (5) Load World deve usar header/footer canônicos e ações por mundo; (6) Leave World deve ser a terceira linha do Pause Menu; (7) Create World deve ter spacing uniforme; (8) túneis inúteis de superfície sem conexão com cavernas devem ser descartados.
- Dropdown: `ui::dropdown` agora possui `DropdownInside<M>`. O clique fora só fecha quando o mouse foi pressionado e nenhum elemento interno está `Hovered`/`Pressed`. Language e Target Position usam o marker no control/panel/options; Spawn Biome marca também search frame, editor, options frame/list e scrollbar. Portanto clicar/arrastar no scrollbar ou na busca não é mais tratado como outside click.
- HUD no pause: Hotbar e Player HUD deixaram de depender de pares de `OnEnter` que podiam se sobrescrever. A visibilidade agora é derivada dos estados autoritativos atuais. Hotbar fica oculto com Pause, Settings ou Inventory abertos; Player HUD fica oculto com Pause ou Settings abertos.
- Targeting de entities: `update_targets` consulta `EntityHealth` e ignora qualquer creature com `is_dead()`. Uma entity em death timer pode continuar existindo visualmente até despawnar, mas deixa imediatamente de produzir `TargetedCreature` e de receber interação.
- Pause Menu: ordem visual agora é linha 1 Resume; linha 2 World Settings + Game Settings; linha 3 Leave World; linha 4 Exit Game.
- Layout canônico: criado `ui::screen` para header/body/content/footer compartilhados e `ui::settings` para ritmo vertical. Settings foi migrado para `ui::screen`; Create World usa 18 px entre grupos de settings e 8 px dentro de cada setting. O feedback vazio de World Name usa `Display::None` e não reserva mais uma linha invisível.
- Load World foi refeito sobre `ui::screen`, com o mesmo chrome canônico e cosmic background das telas principais. O antigo modelo de selecionar um mundo + botões globais foi removido. Cada card possui seu próprio Load e Delete, ambos usando `ui::button::button(...)` e carregando o id do save na própria action. O footer mantém somente Return/Back.
- Delete World: após `delete_world(id)` retornar sucesso, o save é removido de `WorldSelectionState.worlds` e o `WorldListEntry` correspondente é despawnado no mesmo fluxo. Ao remover o último mundo, o status de lista vazia reaparece. Status e erro vazios usam `Display::None` para não gerar spacing fantasma.
- Surface tunnels: `FeatureGraph` ganhou amostragem 3D com margem. `SurfaceCarverResolveContext` recebe o grafo de cavernas ancoradas e cada candidato de surface tunnel só é aceito se o volume do túnel intersectar esse grafo. Sem cave graph, ou sem interseção, o túnel é descartado antes do density pass. A checagem de suporte de structures usa a mesma regra, evitando inconsistência entre terreno gerado e ground fitting.
- `ARCHITECTURE.md` foi atualizado com ownership de `ui::screen`/`ui::settings`, a área interna tipada de dropdown, dead entities fora do targeting e surface carvers subordinados à conectividade de cavernas.
- `VERSION` avançou de `0.24.1` para `0.24.2`. `Cargo.toml` permanece `0.10.16` por design.
- CI intermediária expôs problemas objetivos durante a integração (helper morto/argument count, segundo inicializador de SurfaceCarverResolveContext e type complexity do feedback de Load World); todos foram corrigidos sem relaxar lints. O HEAD funcional final `2ebe1324ad690e9a198ee39fb830e103ae485528` passou no run `35372599722` (3891) com auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows focado em Load World (cards/scroll/delete/load), dropdown de Spawn Biome (busca + scrollbar sem fechar), Create World spacing, Pause Menu/HUD e targeting durante death timer; também validar visualmente em seed nova que surface tunnels isolados deixaram de aparecer.

## Checkpoint 91 — 2026-09-18: correção estrutural da face da slime [CÓDIGO/ASSET + CI VERDE]

- Investigação do GLB confirmou que o problema não era o binding JSON: `data/creatures/slime.json` já mapeava corretamente `SlimeFace -> textures/creatures/slime/face.png`.
- Causa 1: a mesh `square_pixel_face` usava UVs apenas no intervalo aproximado `0.008..0.242` em ambos os eixos. O `face.png` 64×64 possui olhos/boca em aproximadamente x=8..55/y=20..39, então o mesh amostrava essencialmente uma região branca da textura; sob iluminação isso aparecia como a placa clara/amarelada.
- Causa 2: a face havia sido exportada como uma caixa fina de 24 vértices/36 índices, com seis faces, em vez de uma quad frontal. Isso criava volume lateral visível e reforçava o aspecto de placa.
- Causa 3: a geometria do Face ficava em y local 0.36..0.64 dentro de `BodyPivot`, cujo shell ocupa -0.45..0.45. O node precisava de translation y=-0.5 para centralizar a quad no corpo.
- Causa 4: `configure_loaded_scene` zerava à força o `Transform` de todo node chamado `Face`, anulando qualquer posição autorada correta no GLB.
- O `slime.glb` foi corrigido diretamente: Face usa 4 vértices e 6 índices da face frontal, UVs completos `(0,1) (1,1) (1,0) (0,0)`, node translation `[0,-0.5,0]` e material `SlimeFace` em `MASK` com cutoff 0.5.
- `face.png` agora mantém olhos/boca e torna transparente o fundo branco, eliminando a placa retangular sobre o shell.
- O visual loader deixou de resetar transforms autorados do node Face. Materiais tintados de corpo continuam forçados a opaco; materiais apenas texturizados preservam o alpha mode definido pelo GLB, permitindo decals/cutouts.
- `ARCHITECTURE.md` registra agora que transforms de nodes GLB pertencem ao asset autorado e não devem ser sobrescritos pelo loader genérico; overrides texture-only preservam alpha mode.
- Commits principais: `47aaa863...` (runtime respeita transform/alpha), `34f4f156...` (GLB + face.png corrigidos), `b6daa99d...` (comentário alinhado), `9ec5a52b...` (contrato arquitetural) e `1fe68fa6...` (`VERSION 0.24.4`).
- Validação: HEAD funcional/versionado `1fe68fa6da76658adf1a8933498ece6638f8be1f` passou no run `35374517912` (3921), incluindo auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA visual Windows da slime para confirmar posição, orientação, recorte da textura e ausência de placa/reflectance residual durante idle, jump, hurt e death.

## Checkpoint 92 — 2026-09-18: Spawn Biome força região inicial em vez de procurar ocorrência natural [CÓDIGO + CI VERDE]

- O panic reportado em `world/setup/bootstrap.rs` ao escolher `asteria:overworld/wasteland` confirmou que procurar uma ocorrência natural seca do biome dentro de um raio arbitrário era o modelo errado para a feature.
- Decisão funcional: **Spawn Biome significa que a região inicial do mundo deve nascer naquele biome**, e não que o bootstrap deve percorrer o seed procurando uma ocorrência distante.
- `BiomeField` agora possui um override opcional de surface biome para uma região retangular inicial. Dentro do core, o biome selecionado recebe peso 1.0; fora do core há transição usando a largura canônica de borda do biome field, em vez de um corte completamente desconectado do sistema de influences.
- A mesma região força continentalness em direção a terreno continental via `BiomeField::climate_at`. Com isso, Coast/Ocean não podem substituir o biome selecionado dentro dos chunks iniciais e a hydrology usa a mesma identidade terrestre que terrain/visuals.
- O core forçado corresponde aos **9×9 chunks horizontais canônicos do bootstrap** em torno do spawn padrão (raio 4 chunks). A região é fixa e não depende da render distance do usuário, para que o mesmo seed/save tenha a mesma identidade inicial em qualquer sessão.
- `find_forced_spawn_column`, coarse search, local search e o raio de 2048 blocos foram removidos. O bootstrap não compara mais candidatos contra `sample_surface.primary_id` para localizar o biome selecionado.
- A única busca restante é por **uma coluna fisicamente seca dentro da região que já foi forçada ao biome escolhido**. Ela continua usando `supported_water_at(...)`, portanto o player não é colocado dentro de água real.
- Para mundos sem Spawn Biome selecionado, o comportamento de procurar uma coluna seca perto do spawn padrão permanece.
- `spawnBiome` agora é metadado persistente do mundo: `InMemoryWorldSave` guarda o valor; `WorldSnapshot` serializa `spawn_biome` com `serde(default)` para manter compatibilidade com saves antigos; `WorldSaveContext` captura o valor; Load World restaura o valor antes do bootstrap.
- Ao carregar um save que possui `spawnBiome`, o `BiomeField` reconstrói o mesmo override inicial. Isso evita que chunks iniciais já gerados sejam identificados visual/hidrologicamente como outro biome após reload.
- Wasteland continua com `canGenerateLake=false` e `canGenerateRiver=false`; Caverns continua com ambos `true`. Os nomes do schema permaneceram `canGenerateLake` / `canGenerateRiver`.
- `ARCHITECTURE.md` registra agora que Spawn Biome é um override determinístico da região inicial e nunca uma busca por ocorrência natural distante.
- Commits principais: `4ab5b583...` (override no BiomeField), `5f20bee4...` (blend no surface sampling), `1072db5f...` / `242bfaac...` / `ff2d2ff5...` / `38db5ee0...` (persistência/restauração), `df12b6d3...` (bootstrap sem busca forçada), `6e92f37d...` (arquitetura) e `58e2a2ef...` (`VERSION 0.24.5`).
- Validação intermediária do código funcional: run `35375472761` (3936) **success**.
- Validação canônica do HEAD versionado `58e2a2efaa1d829f4fd645ec6acb2ecebd619b9f`: run `35375607533` (3939) **success**, incluindo auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows criando mundos com Wasteland, Plains e outro biome selecionado, confirmando spawn dentro dos chunks iniciais forçados, ausência do panic e persistência da identidade do biome após salvar/sair/carregar.

## Checkpoint 93 — 2026-09-18: chunks são estado autoritativo + Spawn Biome com shape válido + Button Title Case [CÓDIGO + CI VERDE]

### Persistência/runtime de chunks

- Auditoria confirmou o problema: o modelo anterior gravava apenas `dirty_chunks` e assumia que terreno gerado mas não editado era derivável do seed. `archive_chunk()` inclusive esquecia chunks não editados, removendo-os de `generated_chunks`; revisitar uma área podia rodar worldgen novamente.
- Novo contrato: **worldgen é um evento de criação única por coordenada**. Depois que um chunk foi gerado uma vez, blocos/fluidos/propriedades daquele resultado passam a ser estado autoritativo do mundo.
- `VoxelWorld::archive_chunk()` agora sempre arquiva o chunk e mantém sua coordenada em `generated_chunks`. Runtime unload deixa de apagar a existência do chunk.
- Ao revisitar a área na mesma sessão, `has_generated_chunk()` leva obrigatoriamente a `restore_chunk()`; worldgen não é disparado novamente para aquela coordenada.
- `dirty_chunks` foi removido. A revisão persistente do mundo agora avança tanto ao gerar um chunk pela primeira vez quanto em mutações de bloco/fluid. Lighting, archive e restore continuam derivados/não persistentes e não avançam essa revisão.
- `save_modified_chunks()` foi substituído por `save_generated_chunks()`: o snapshot serializa **todo chunk já gerado**, residente ou arquivado, inclusive chunks vazios. Chunk vazio registrado também é resultado autoritativo e não pode ser esquecido.
- `VoxelWorld::from_saved_chunks()` reconstrói o registro de chunks gerados em estado arquivado. Esses chunks só são restaurados quando necessários; não são recriados do seed.
- Durante `WorldLoadMode::Load`, o bootstrap agora filtra sua lista para coordenadas que já estão em `existing_world.has_generated_chunk(...)`. **Loading não gera chunks ausentes.** Coordenadas nunca existentes só podem entrar em worldgen depois, pelo streaming normal em Gameplay.
- Corrigido race do primeiro frame de um mundo carregado: como streaming roda antes do autosave em `Last`, um chunk realmente novo poderia ser gerado e absorvido como “baseline já salvo”. Agora, se a revisão persistente já avançou nesse primeiro frame, o snapshot é persistido imediatamente.
- Meshes e lighting permanecem derivados e são reconstruídos a partir do conteúdo salvo do chunk; isso não é worldgen.
- Limitação histórica inevitável: snapshots anteriores a este modelo só continham chunks modificados e **não registraram as coordenadas de chunks gerados mas nunca editados**. Esses dados ausentes não podem ser recuperados retroativamente. A garantia integral “gerou uma vez, nunca regenera” vale para chunks registrados/salvos pelo novo modelo.

### Spawn Biome

- O override 9×9 quadrado introduzido no checkpoint anterior foi removido por não respeitar o contrato de tamanho/forma do biome.
- `DimensionBiomeSize.x/z.min..max` são tratados como raios autorados. O Spawn Biome agora escolhe deterministicamente `radiusX` e `radiusZ` dentro desses intervalos usando seed + biome ID.
- A região forçada é uma elipse centrada no spawn padrão, não um retângulo alinhado a chunks.
- A borda passa pelo mesmo `warp_surface_position`/smooth fade do `BiomeField`, evitando contorno quadrado/artificial.
- Dentro do core a influência do biome selecionado é 1.0; na borda ela transiciona para o campo natural.
- Continentalness/hydrology continuam lendo o mesmo override, portanto Coast/Ocean não sobrescrevem o Spawn Biome dentro da região forçada.
- A busca restante continua sendo apenas por coluna seca dentro desse core já criado; não existe busca por ocorrência natural distante.
- O override continua determinístico a partir de seed + `spawnBiome`, portanto reload reconstrói a mesma região sem persistir uma geometria paralela.
- Observação para próxima auditoria: o gerador **natural** de surface biomes ainda usa `size.min` principalmente para site spacing e não aplica `size.max` como limite explícito da região. Isso não foi alterado silenciosamente neste patch; o Spawn Biome passa a respeitar min/max explicitamente.

### UI Button Title Case

- `ui::button::button(...)` agora centraliza a regra de casing: labels são exibidos em **Title Case**, com inicial maiúscula para cada palavra separada por whitespace.
- A regra pertence ao design system; screens não devem implementar casing local.
- `worldSelection.load` foi alinhado na fonte para `Load World`, `Carregar Mundo` e `Cargar Mundo`.
- `ARCHITECTURE.md` registra que action buttons usam exclusivamente o primitive canônico e que o casing também pertence a ele.

### Validação

- Run funcional/documentado antes do bump: `35376713700` (3964) **success**.
- HEAD funcional/versionado canônico: `7b924a2648360a0c7a354480452f5d9d994fc386`.
- Run canônico: `35376804377` (3966) **success**, incluindo auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows focado em (1) gerar área, sair do range e voltar sem worldgen/recriação; (2) salvar/sair/carregar e confirmar que apenas chunks registrados são restaurados; (3) explorar chunk realmente novo e confirmar persistência posterior; (4) criar Wasteland e observar a região além do render inicial para validar raios/shape/transição; (5) verificar Title Case em todos os action buttons.

## Checkpoint 94 — 2026-09-18: Player HUD no Pause + save compacto/assíncrono + ocean bathymetry [CÓDIGO + CI VERDE]

### Player HUD / Pause

- Bug reportado: o Player HUD continuava aparecendo sobre o Pause Menu apesar do sync anterior.
- Causa arquitetural: `sync_player_hud_visibility` dependia de `State<PauseState>::is_changed()` / `SettingsState::is_changed()` e de `Single<PlayerHudRoot>`. Isso tornava a reconciliação sensível ao timing de state transition e à cardinalidade do root.
- Correção: enquanto `GameState::Gameplay` está ativo, o sistema calcula a visibilidade autoritativa a cada frame a partir de `PauseState` + `SettingsState` e aplica a todos os `PlayerHudRoot` via `Query`.
- Não existe handler paralelo OnEnter/OnExit; o dono continua sendo um único sync idempotente.
- Commit principal: `d3282178...`.

### Persistência / tamanho de save / autosave

- Após tornar todo chunk gerado autoritativo em `0.24.6`, Leave World expôs a limitação do formato antigo: `DiskChunk` repetia ID/rotation/orientation/properties em um objeto JSON por voxel ocupado e o snapshot rapidamente atingia o limite de tamanho.
- Novo encoding de `DiskChunk`: **palette de estados + runs contíguos** para blocos e fluidos.
  - Estados repetidos são descritos uma vez na palette.
  - Ocupação é gravada como `start/len/state`.
  - Chunks vazios continuam persistidos apenas pela coordenada/registro.
  - Campos legacy `blocks`/`fluids` continuam desserializáveis com `serde(default)`; snapshots antigos permanecem legíveis.
- `VoxelWorld` passou a ser clonável de forma shallow para save: `VoxelChunk` já compartilha arrays por `Arc`; `archived_chunks` agora usa `Arc<ArchivedChunk>`. Tirar um snapshot estrutural não duplica milhares de voxels no frame.
- O autosave periódico de 60 s não executa mais compactação, serialização JSON e `fsync` no main thread.
  - Main thread captura metadados + clone estrutural compartilhado.
  - `AsyncComputeTaskPool` faz `WorldSnapshot::capture`, compactação dos chunks e publicação em disco.
  - Apenas uma task de autosave pode ficar em voo; se ela ainda estiver gravando, outra não é iniciada.
  - Ao concluir, o baseline é atualizado com exatamente o estado salvo. Falha não avança baseline.
- O worker não força `Clone` em `ToolRegistry`, `DimensionRegistry` ou `DayNightCycleRegistry`. Ele usa o `PruneRegistries` owned já existente, contendo apenas os dados necessários para validação/publicação/limpeza.
- Leave World/Exit continuam fazendo commit final síncrono para não abandonar o mundo antes de uma publicação durável. O encoding compacto reduz fortemente o custo/tamanho desse commit, mas QA runtime ainda é necessário para medir latência real em mundos grandes.
- Commits principais: `4d0eea57...` (palette+runs), `b3464ef5...` / `f03f66e0...` (archives compartilhados), `2e7452ad...` (autosave fora do main thread), `87ebb0e9...` / `1aff3e84...` (owned validation data).

### Ocean / transição com Spawn Biome

- Bug reportado: oceano adjacente a Plains/Wasteland produziu faixa de terreno “cortado”.
- Causa encontrada no override de Spawn Biome: `BiomeField::climate_at` empurrava continentalness até 1.0 conforme o peso forçado. Ao encontrar oceano natural na borda, o gradiente era artificialmente comprimido.
- Correção: o override agora **suprime apenas a força do oceano necessária**. `suppress_ocean_continentalness` mantém continentalness natural quando já é terra e, em água, move o valor apenas em direção ao threshold oceânico conforme o peso do Spawn Biome. Não existe mais “continente máximo” artificial na região forçada.
- Commits principais: `ef59978e...`, `1eada322...`, `e39060bf...`.

### Ocean floor bathymetry

- Bug reportado: fundo do oceano era praticamente flat; variação visual vinha quase só de caves/tunnels.
- Causa: em mar aberto `ocean_strength -> 1`, o floor target convergia para `sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH`, esmagando a variação de altura original.
- Correção: open ocean possui bathymetry determinística própria:
  - broad noise scale `0.012`,
  - detail noise scale `0.041`,
  - variação máxima configurada em `8.0` blocos,
  - amplitude multiplicada por `ocean_strength`, portanto o relevo nasce suavemente a partir da costa e fica completo em mar aberto.
- `HydrologyRegion::ocean_floor_target` é agora a única fonte de verdade do fundo oceânico.
  - density carving usa esse target;
  - `supported_water_at` / água física usam o mesmo target;
  - o bed físico e o terreno cavado não podem divergir.
- `HydrologyRegion` agora carrega seed para bathymetry determinística; fixtures de hydrology foram atualizadas.
- Commits principais: `c83a7fb7...`, `a8cb2259...`, `8a9a616d...`, `4f9be32d...`, `3f8457d7...`, `aff18721...`, `bd07812b...`.

### Contrato arquitetural

- Modal HUD visibility deve reconciliar diretamente states autoritativos; não depender de `is_changed()`/handler episódico.
- Chunk persistence usa formato compacto e autosave periódico fora do main thread, uma task por sessão.
- Spawn Biome não maximiza continentalness; atenua apenas ocean strength conforme influência.
- Ocean floor deve possuir relevo bathymétrico determinístico e density/water devem compartilhar exatamente o mesmo target.
- `ARCHITECTURE.md` atualizado em `4fc4d36b...`.

### Validação

- HEAD funcional/versionado canônico: `1a7126de97fd9982553d718b133f46e4b0da754b`.
- CI canônica: `35378340083` / run 4010 — **success**.
- Passaram: auditoria de localizações, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows focado em (1) abrir Pause e confirmar Player HUD invisível; (2) deixar autosave de 60 s acontecer sem hitch perceptível e verificar criação de snapshot válido; (3) Leave World num mundo com vários chunks e conferir tamanho/tempo do save compacto; (4) explorar costa entre Spawn Biome e oceano natural para verificar ausência da faixa cortada; (5) observar fundo de oceano em mar aberto para validar bathymetry sem depender de tunnels.

## Checkpoint 95 — 2026-09-18: surface tunnels sem paredes retas no terreno [CÓDIGO + CI VERDE]

- Bug reportado: surface tunnels ainda podiam produzir paredes visualmente retas/verticais ao cortar o terreno.
- Causa 1: o Bezier do tunnel era discretizado com apenas **5 pontos**. Com lengths de até ~170 blocos, isso transformava a curva em cápsulas retas de dezenas de blocos, deixando laterais/trechos claramente lineares.
- Correção: `TUNNEL_PATH_SAMPLES` passou de 5 para **13**, reduzindo o comprimento dos segmentos e preservando a curvatura visível do path sem trocar o modelo de conectividade.
- Causa 2: o perfil circular do tubo era levado intacto até a superfície. Quando a seção intersectava uma encosta, a lateral cilíndrica podia chegar ao topo como parede quase vertical.
- Correção: `surface_carver_density_delta` agora recebe a altura real da superfície da coluna.
- Nos últimos **12 blocos abaixo da superfície**, o perfil ganha um **flare horizontal suave**:
  - o raio vertical permanece o raio autorado do tunnel;
  - somente o raio horizontal cresce progressivamente;
  - flare máximo atual = **0.85 × radius** adicional;
  - o fator usa smoothstep por profundidade;
  - abaixo dessa faixa o perfil continua circular exatamente como antes.
- A distância do tunnel deixou de ser apenas distância Euclidiana/radius e passou a usar distância normalizada anisotrópica perto da boca: horizontal dividido pelo raio flareado, vertical dividido pelo raio original.
- `structures/support.rs` usa o mesmo `raw_surface_height` ao avaliar carvers, mantendo suporte de estruturas consistente com a geometria realmente gerada.
- `tunnel_connects_to_cave` agora aceita slice de pontos em vez de array fixo de `TUNNEL_PATH_SAMPLES`, evitando acoplamento dos testes/conectividade a um número específico de amostras.
- O requisito anterior continua valendo: surface tunnel só existe se intersectar o cave connector graph; esse fix altera apenas shape/rasterização, não a regra de conectividade.
- `ARCHITECTURE.md` agora proíbe discretizar tunnels longos em poucos segmentos retos e exige flare suave da boca ao encontrar a superfície.
- Commits principais: `8a75f95b...` (curva + mouth flare), `cb05c63e...` (surface height no density pass), `d625332a...` (structure support), `1f3c84ef...` (connectivity slice), `223093e4...` (contrato arquitetural), `c7cfd172...` (`VERSION 0.24.8`).
- Primeira validação 4015 expôs apenas integrações mecânicas faltantes (call de structure support e arrays de teste fixos), corrigidas antes da validação final.
- HEAD funcional/versionado canônico: `c7cfd172c6302341006850c7bf583ce872a84dc8`.
- CI canônica: `35379295792` / run 4022 — **success**.
- Passaram: auditoria de localizações, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA visual Windows em Mountains, procurando entradas de surface tunnels em encostas altas e baixas para confirmar (1) ausência de trechos longos retos, (2) boca com talude suave sem parede cilíndrica vertical, (3) subterrâneo mantendo seção circular e (4) conectividade com caves preservada.

## Checkpoint 96 — 2026-09-18: surface tunnels com margem gradual lake-style [CÓDIGO + CI VERDE]

- Bug reportado após `0.24.8`: apesar de curva mais densa e flare da boca, surface tunnels ainda podiam produzir paredes retas porque o terreno intacto começava imediatamente fora do core escavado.
- Diagnóstico comparando com lakes:
  - lakes possuem um **core** de carve;
  - depois possuem uma **margem externa de grading**;
  - nessa margem, a altura do terreno é gradualmente aproximada da borda do feature e a força cai com `smoothstep` até zero.
- O surface tunnel agora segue o mesmo modelo em duas fases:
  1. **core 3D** continua sendo escavado pelo perfil tubular/flare já existente;
  2. **margem horizontal externa seca** gradua o terreno da boca até o terreno natural.
- `SurfaceCarverColumn` agora guarda `margin_density_delta` resolvido uma vez por coluna, em vez de recalcular a margem para cada voxel.
- Para cada tunnel próximo da superfície:
  - calcula a distância horizontal da coluna até o eixo amostrado do tunnel;
  - interpola a altura `path_y` do eixo no ponto horizontal mais próximo;
  - calcula quão próximo o topo do tunnel está da superfície;
  - se estiver dentro de `TUNNEL_MOUTH_BLEND_DEPTH = 12`, ativa a margem proporcionalmente com `smoothstep`;
  - o core horizontal considera o flare atual da boca;
  - a margem começa em normalized distance `1.0` e termina em `1.35`.
- Dentro do core, a própria abertura reduz a contribuição da margem via `(1 - opening)`, equivalente ao princípio usado por lake shore grading: o carve domina o centro; a margem domina a borda.
- Na borda do core, o alvo de superfície é `path_y + 0.5`. Assim o terreno externo desce em talude em direção ao eixo do tunnel em vez de terminar numa parede vertical.
- Fora da normalized distance `1.35`, o delta da margem é exatamente zero e o terreno natural permanece intacto.
- A margem nunca eleva terreno: `height_delta` é limitado a valores negativos, servindo apenas para suavizar/cortar a encosta.
- O candidate reach horizontal foi ampliado para considerar **flare máximo × margem externa**, evitando truncar o grading em fronteiras de células/chunks.
- O culling vertical também foi expandido pelo `TUNNEL_MOUTH_BLEND_DEPTH`, para que chunks superiores onde só a margem atua não sejam descartados antes do grading.
- `resolve_surface_carver_column` agora recebe a altura real da superfície. Tanto `generation/density.rs` quanto `structures/support.rs` passam a mesma altura usada pelo terrain/support sampling, mantendo a geometria consistente.
- O perfil anterior de `0.24.8` permanece:
  - 13 amostras na curva;
  - flare horizontal perto da superfície;
  - seção circular no subterrâneo profundo;
  - conectividade obrigatória com cave graph.
- `ARCHITECTURE.md` registra agora explicitamente **core + margem externa gradual**, proibindo voltar ao modelo em que terreno intacto começa imediatamente após o carve.
- Commits principais: `a517bc34...` (grading lake-style), `3831b082...` / `53b3377d...` (surface height nos consumidores), `fd72bd2c...` (contrato arquitetural), `b271d4db...` (`VERSION 0.24.9`).
- Validação funcional: run `35379735852` (4029) — **success**.
- HEAD funcional/versionado canônico: `b271d4db080ef3e6c6d156cb05a2a31c3cc24d6e`.
- CI canônica: `35379845621` / run 4033 — **success**.
- Passaram: auditoria de localizações, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA visual Windows em Mountains, verificando principalmente as laterais externas da boca dos surface tunnels. A transição esperada agora é: tunnel core -> talude/margem gradual -> terreno natural, sem parede reta entre core e terreno intacto.

## Checkpoint 97 — 2026-09-18: tunnel mouth basin + river proibido em Wasteland [CÓDIGO + CI VERDE]

- Surface tunnels ainda produziam paredes retas porque, mesmo com flare e margem externa, o **core da superfície** continuava dependendo do carve 3D tubular. A encosta podia continuar herdando a lateral do cylinder.
- Correção estrutural: a boca do tunnel agora usa um **surface basin 2D lake-style**.
  - `TUNNEL_PATH_SAMPLES`: 13 -> **25**.
  - O centro do core gradua a superfície em direção ao interior do tunnel.
  - A borda do core converge para o **roof/topo do tunnel**, não para o eixo.
  - Fora do core existe margem externa fixa de **14 blocos** com `smoothstep` até terreno natural.
  - O grading de superfície é somado independentemente do carve 3D; o carve continua responsável pelo vazio subterrâneo, não pela forma da encosta.
  - Deep underground continua circular.
- Isso substitui a abordagem anterior em que a margem era estreita/proporcional ao raio e mirava o eixo do tunnel, que ainda podia criar taludes agressivos ou paredes.

- River em Wasteland: o conteúdo já tinha `canGenerateRiver=false`, mas a regra era aplicada apenas na **seleção de source cells**.
- `keep_only_complete_downstream_paths()` agora encerra/rejeita qualquer river path que atravesse land cell com `canGenerateRiver=false`.
- A geração de edge possui defesa adicional: downstream land cell com river desabilitado é rejeitado.
- A curva contínua final também é amostrada ao longo dos segmentos. Se qualquer ponto atravessar biome de superfície com `canGenerateRiver=false`, a edge é rejeitada.
- Oceano físico continua sendo destino válido mesmo se o biome de superfície subjacente tiver rivers desabilitados.
- Isso garante que Wasteland não funcione só como “não nasce river aqui”; ele passa a ser **barreira física para river channel**.

- Commits principais:
  - `bafaff52...` — tunnel mouth como graded surface basin;
  - `59ff805c...` — bloqueio de downstream path por biome;
  - `ecf941e9...` — validação contínua da curva;
  - `0a512eb4...` — edge guard;
  - `6d969fce...` — integração final;
  - `1a659a99...` — arquitetura;
  - `3ebe9df9...` — `VERSION 0.24.10`.
- Validação funcional anterior: run `35381097948` (4048) — **success**.
- HEAD funcional/versionado canônico: `3ebe9df95c784b3729ad6cb059168bcf02b56230`.
- CI canônica: `35381226764` / run 4052 — **success**.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows verificando (1) mouths de surface tunnels em Mountains para confirmar transição basin -> margem -> terreno natural sem parede reta; (2) Wasteland adjacente a Plains/Mountains para confirmar que rivers param/contornam e não atravessam o biome.

## Próxima feature planejada — Ocean Mountains

- Feature futura a trabalhar em breve: **geração de montanhas no oceano**.
- Ainda não implementada.
- Ainda não há contrato fechado para frequência, altura, relação com bathymetry, Ocean/Coast, ilhas expostas ou integração com o sistema de terrain modifiers.
- Ao iniciar essa feature, primeiro definir a arquitetura para que montanhas oceânicas participem do mesmo terrain/hydrology pipeline sem reintroduzir cortes abruptos entre fundo oceânico, costa e terreno elevado.

## Checkpoint 98 — 2026-09-18: tunnel conectado de verdade + river/lake blend + hydrology materials por biome + diversidade regional [CÓDIGO + CI VERDE]

### Surface tunnels: ownership e conexão reais

- Relato consolidado: ainda existiam paredes flat e surface tunnels que apenas escavavam a superfície e morriam sem chegar a uma cave.
- Causa arquitetural encontrada: havia **dois donos de abertura terrestre**.
  - `surfaceCarver` criava a boca/tunnel.
  - o `anchored cave connector graph` também adicionava `surface_cave_entrance` e podia abrir a superfície de forma independente.
- Correção: em terra, **somente `surfaceCarver` pode abrir a superfície**.
  - `surface_cave_entrance` foi removido do connector graph terrestre.
  - Ocean cave opening continua como exceção hidrológica separada.
- Um surface tunnel candidate não é mais aceito só porque “algum ponto toca o cave graph”.
  - encontra uma boca real em relação à superfície do terrain;
  - testa os dois sentidos do path;
  - exige interseção com o connector graph com roof pelo menos **6 blocos abaixo da superfície**;
  - escolhe o corredor válido/mais curto;
  - **trunca no primeiro ponto de conexão profunda**;
  - todo trecho depois da conexão é descartado.
- Isso elimina o caso em que metade do tunnel raspava uma cave enquanto outro trecho solto continuava cortando a montanha e terminava em lugar nenhum.
- Structure support usa o mesmo `DimensionDefinition`/surface geometry do resolver.
- Commits principais: `a3d330fc...`, `5c635a15...`, `ea9e8bd4...`, `d6d643ff...`, `c25d4f78...`, `ddab4f25...`.

### River entrando em Lake

- River e lake estavam somando carve/headroom independentes; um river podia reescavar uma vala dentro do basin da lake.
- O lake basin agora é autoritativo conforme sua opening strength cresce.
- `density_deltas_for_column` multiplica o channel carve por `1 - lake_opening`; no interior forte da lake o river carve converge a zero.
- `supported_river_surface_at` também atenua river headroom pela opening de lakes fisicamente suportadas.
- Resultado esperado: river continua abrindo a entrada/saída da lake, mas não recorta o interior do basin com uma trench independente.
- Commits: `66b2c491...`, `e6c6c695...`.

### Hydrology block materials pertencem aos biomes

- Pedido: `riverBedBlock`, `lakeBedBlock`, `oceanBedBlock` etc. não devem pertencer à Dimension.
- `DimensionHydrology` agora contém somente estado global:
  - `waterFluid`;
  - IDs de `coastBiome` / `oceanBiome`;
  - `riverWeight` / `lakeWeight`.
- `BiomeHydrology` agora possui:
  - `shoreBlock`;
  - `riverBedBlock`;
  - `lakeBedBlock`;
  - `oceanBedBlock`;
  além das regras/chances já existentes.
- Para não carregar Strings no hot path de drainage, foi criado `BiomeHydrologyRules` (`Copy`) com apenas `canGenerateLake`, `canGenerateRiver`, `lakeChanceMultiplier` e `riverWidthMultiplier`.
- Validação de referências de blocks migrou para o próprio biome.
- Material pass resolve:
  - river/lake/inland shore a partir do **surface biome dominante da coluna**;
  - coast shore a partir do Coast biome configurado;
  - ocean bed a partir do Ocean biome configurado.
- O material do ocean floor passou a usar o mesmo `ocean_floor_target` bathymétrico de density/water, removendo mais uma fonte de divergência.
- JSON do Overworld migrado:
  - Dimension perdeu todos os block IDs hidrológicos;
  - Plains, Witchwood, Enchanted Forest e Mountains receberam materiais inland;
  - Coast recebeu `shoreBlock`;
  - Ocean recebeu `shoreBlock` + `oceanBedBlock`.
- Commits principais: `5409b3e6...`, `f2241093...`, `e3a2c71c...`, `e434de56...`, `1c1fa42d...`, `4721dd81...`, `ec2e9a5b...`, `deb782df...`, `53e2bc63...`, `0af45f0a...`, `ebc83dce...`, `20ce9d9e...` e migração dos JSONs subsequente.

### Witchwood / Enchanted Forest / Wasteland / Plains

- Relato: Witchwood e Enchanted Forest estavam muito raros; depois foi confirmado que até Wasteland era difícil de achar, com trechos de ~2k blocos praticamente só de Plains.
- A tentativa intermediária de remover `avoidNear` do Wasteland foi **revertida**. O `avoidNear` Wasteland ↔ Witchwood/Enchanted continua sendo requisito.
- Causa real: `avoidNear` era testado contra **qualquer um dos 8 raw neighbor site draws**.
  - Wasteland conflita com dois biomas mágicos;
  - com pesos regionais próximos, quase todo Wasteland candidate encontrava pelo menos um raw Witchwood/Enchanted entre 8 neighbors e era descartado;
  - Witchwood/Enchanted também eram frequentemente descartados ao encontrar raw Wasteland;
  - Plains não tinha conflito e virava o fallback estatístico dominante.
- Correção: `avoidNear` agora é avaliado contra uma **região vizinha dominante**, usando a regra já existente de maioria real (>=5/8 neighbors), e não contra um raw site isolado.
- Isso preserva a separação Wasteland↔mágicos sem transformar a constraint em um multiplicador oculto de raridade.
- Pesos atuais foram mantidos:
  - Plains 1.0;
  - Wasteland 1.30;
  - Witchwood 1.25;
  - Enchanted Forest 1.25.
- Commits principais: `736986f0...` (restaura `avoidNear`), `fb7eabc0...` (semântica dominante), `d7f28da2...` (integração/test compile), `5f86cc02...` (arquitetura).

### Estado dos pedidos desta sequência

- **Tunnel paredes flat / dead-end:** código corrigido estruturalmente; QA Windows ainda necessário.
- **River em Wasteland:** já corrigido no checkpoint 97; continua valendo.
- **River carving dentro de lake:** corrigido neste checkpoint; QA visual pendente.
- **Witchwood/Enchanted muito raros + Plains dominante:** causa de seleção corrigida sem remover `avoidNear`; QA de distribuição pendente.
- **Hydrology bed/shore blocks na Dimension:** migrados para biomes.
- **Ocean Mountains:** permanece **feature futura planejada**, ainda não implementada neste bloco.

### Validação

- CI funcional antes da consolidação: run `35384008079` (4136) — **success**.
- HEAD funcional/versionado/documentado: `368b805306190dc6d0655fd592f12f43f0f380fc`.
- CI canônica: `35384155361` / run 4139 — **success**.
- Passaram: auditoria de localizações, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows focado em (1) surface tunnel com boca → cave real sem parede flat/dead-end; (2) river entrando/saindo de lake sem trench no basin; (3) navegar alguns quilômetros e confirmar frequência saudável de Wasteland/Witchwood/Enchanted preservando `avoidNear`; (4) confirmar materiais de river/lake/coast/ocean após a migração de ownership. Ocean Mountains continua no backlog logo após estabilizar esses itens.

## Checkpoint 99 — 2026-09-18: avoidNear bloqueia somente fronteira real entre biomas [CÓDIGO + CI VERDE]

- Correção de semântica solicitada pelo usuário: `avoidNear` **não significa distância mínima, raio de exclusão, vizinho dominante ou região proibida ao redor de outro biome**.
- Significado canônico agora: dois biomas conflitantes via `avoidNear` podem existir próximos, mas **não podem compartilhar uma fronteira de região**.
- Implementação:
  - cada site mantém normalmente o biome sorteado pelo peso/climate;
  - os 8 sites imediatos ainda são usados apenas como candidatos geométricos de possível contato, não como veto por proximidade;
  - para um par conflitante, `surface_sites_share_border(...)` verifica geometricamente se os dois sites realmente possuem uma aresta Voronoi compartilhada;
  - a checagem usa a interseção das restrições lineares na bissetriz entre os dois sites, considerando os sites relevantes ao redor;
  - se não há aresta compartilhada, `avoidNear` não interfere, mesmo que o outro biome esteja relativamente perto;
  - se há fronteira compartilhada e os biomas conflitam simetricamente, somente então o raw biome é rejeitado e o site é rerrolado entre biomas permitidos.
- O raw biome é preservado sempre que sua fronteira é válida; isso evita que `avoidNear` distorça frequência regional por simples presença de neighbors.
- A implementação anterior intermediária baseada em “dominant neighbor >=5/8” foi removida por ainda representar uma heurística de proximidade/região ao redor, diferente da intenção do usuário.
- `ocean_biome_id` foi removido do `BiomeField` porque existia apenas para a antiga lógica de proximidade do selector; `ocean_weight` continua para hydrology/Spawn Biome.
- `ARCHITECTURE.md` registra explicitamente: `avoidNear` = **não compartilhar boundary**, nunca exclusion radius.
- Commits principais:
  - `c7b8bda4...` — detecção de fronteira Voronoi;
  - `93edbcde...` — preserva raw biome salvo conflito real;
  - `ca685354...` / `b48cf39e...` / `b3912fb3...` — limpeza de integração/Clippy;
  - `f3d6a926...` — contrato arquitetural;
  - `abe31483...` — `VERSION 0.24.12`.
- Validação funcional pré-bump: run `35385205233` (4149) — **success**.
- HEAD funcional/versionado canônico: `abe31483e2a7886d02d1b222f94df1b2e21b43b9`.
- CI canônica: `35385314155` / run 4152 — **success**.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows de distribuição regional, navegando vários quilômetros e confirmando simultaneamente (1) Plains não domina trechos absurdos, (2) Wasteland/Witchwood/Enchanted aparecem com frequência compatível com os pesos atuais e (3) Wasteland nunca compartilha fronteira direta com Witchwood/Enchanted, embora possam existir próximos separados por outro biome.



## Checkpoint 100 — 2026-09-18: Biome Size Multiplier, sliders editáveis, rios no oceano, surface carvers data-driven e HUD no Pause [CÓDIGO + CI VERDE]

### Biome Size Multiplier

- Novo campo no fluxo **New World**: `Biome Size Multiplier`.
- Faixa válida: **0.5x até 5.0x**, em passos exatos de **0.1**.
- Default: **1.0x**.
- O valor é mantido internamente em décimos inteiros para evitar drift de ponto flutuante na regra de passo.
- Durante a construção do `BiomeField`, os `size.x/z/y.min/max` definidos pela Dimension são multiplicados pelo valor configurado e **arredondados** antes de serem usados pelo worldgen.
- O multiplicador é persistido em snapshot/manifest/in-memory save e restaurado no load. Saves antigos que não possuem o campo usam **1.0x** via default compatível.
- O multiplicador participa da identidade do snapshot para impedir que um mundo carregado volte silenciosamente a outro tamanho de biome ao gerar chunks novos.
- O `SAVE_FORMAT_VERSION` não foi alterado; a compatibilidade é mantida por campo serde defaultado.

### Regra de UI para sliders

- Foi criado um primitive compartilhado em `src/ui/slider.rs`.
- Todo slider existente agora possui **number input ao lado**, sincronizado nos dois sentidos.
- Hoje isso cobre:
  - Render Distance;
  - Biome Size Multiplier.
- Render Distance mantém sua faixa inteira original.
- Biome Size Multiplier aceita somente o domínio 0.5–5.0 em décimos; estados parciais de digitação podem existir apenas enquanto o usuário compõe o valor, sem aplicar configuração inválida.
- O numeric input ganhou suporte decimal reutilizável sem duplicar o design system.

### Rivers não continuam dentro do oceano

- Bug observado em QA exploratória: river edges podiam continuar sendo rasterizadas depois de entrar em água oceânica física.
- O oceano continua sendo um **destino válido** para o rio, mas agora funciona como destino terminal.
- O path é amostrado até encontrar a primeira entrada em oceano físico; a fronteira é refinada e o rio termina ali, com a mouth no nível do mar.
- A água depois desse ponto pertence somente ao oceano; não existe mais channel de river atravessando o fundo oceânico.

### Surface carvers / entradas de cavernas data-driven

- `BiomeDefinition` agora possui `allowSurfaceCarvers`.
- Semântica canônica:
  - o **volume biome cavernoso** é dono das definições `surfaceCarvers`;
  - o **surface biome** decide apenas se aceita ou não ser perfurado por esses carvers.
- Configuração atual do Overworld:
  - Plains: `allowSurfaceCarvers=true`;
  - Mountains: `true`;
  - Witchwood: `true`;
  - Enchanted Forest: `true`;
  - Wasteland: `true`.
- O tunnel carver que antes estava em Mountains foi movido para `asteria:overworld/caverns`.
- Validação de conteúdo impede surface biomes de serem donos de `surfaceCarvers` e impede volume/hydrology biomes de habilitarem `allowSurfaceCarvers`; volume carvers só são permitidos em volume biome com `densityModifier=cavern`.
- A tentativa intermediária de criar uma entrada terrestre diretamente no connector graph foi **revertida** antes do estado final. O contrato anterior permanece: em terra, somente `surfaceCarver` pode abrir a superfície; ocean cave openings continuam sendo a exceção hidrológica separada.

### Correção estrutural da entrada de cavernas

- A investigação mostrou por que o jogador podia explorar longas distâncias sem achar entrada:
  - `seaLevel` do Overworld é **90**;
  - o carver tinha `elevation = 10..46`;
  - o código antigo calculava `sea_level + elevation`, posicionando o eixo em **Y 100..136**;
  - Caverns existe em **Y 8..64**.
- Isso tornava a conexão real com o cave graph subterrâneo praticamente impossível.
- Semântica corrigida:
  - `elevation` é agora um **Y absoluto subterrâneo alvo**;
  - a mouth nasce na **altura real do terreno**;
  - o path usa Bezier curvo da superfície até o alvo subterrâneo;
  - o candidate só sobrevive se encontrar o connector graph com profundidade de teto suficiente;
  - o path é truncado na primeira conexão subterrânea válida.
- A regra visual gradual foi preservada:
  - o tunnel profundo continua circular;
  - perto da superfície a boca usa o basin 2D lake-style existente;
  - a borda converge para o roof do tunnel;
  - a margem externa fixa de **14 blocos** usa `smoothstep` até o terreno natural;
  - portanto a entrada não deve voltar a produzir uma parede reta/cilíndrica na encosta.
- `ARCHITECTURE.md` registra tanto o ownership data-driven quanto a nova semântica absoluta de `elevation`.

### Player HUD no Pause

- Causa real encontrada: `EntityCard` ativo usava `Visibility::Visible`, podendo furar a visibilidade herdada do `PlayerHudRoot`.
- Correção: card ativo usa `Visibility::Inherited`; card sem entidade continua `Hidden`.
- O `PlayerHudRoot` continua derivando sua visibilidade diretamente de `PauseState` e `SettingsState`, de forma idempotente no Update.
- Handlers one-shot redundantes de Pause/Settings que foram tentados durante a investigação foram removidos antes do estado final, preservando o contrato arquitetural de visibilidade state-driven.

### Versionamento e validação

- `VERSION`: **0.24.12 → 0.25.0** por introdução de nova configuração persistente de worldgen e novo contrato data-driven de surface carvers.
- Commit de bump: `cb1a70b363bbffc2a41d06e109f31a5eb64d1fbd`.
- O ajuste do HUD já havia passado no run `35388452658`.
- A sequência do carver descendente expôs apenas integrações mecânicas antigas de campos `sea_level` em structure support e fixture de fluids; ambas foram removidas.
- HEAD funcional validado: `92f557060a6e2a9185b2854c66aeeaea1ddec74f`.
- CI final: `35388784039` — **success**.
- Passaram:
  - auditoria de localizações EN/PT-BR/ES;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows em mundo novo verificando (1) Biome Size Multiplier em 0.5/1.0/5.0 e persistência após reload; (2) number inputs de ambos sliders; (3) rivers terminando na primeira entrada física no oceano; (4) entradas de caverna aparecendo em todos os surface biomes atualmente habilitados, incluindo Wasteland; (5) mouth gradual sem parede reta e conexão real com Caverns; (6) Player HUD completamente oculto durante Pause e Settings.



## Checkpoint 101 — 2026-09-18: performance do worldgen + biomas independentes Gorge / Alps / Mountain Belt / Volcano [CÓDIGO VALIDADO; VERSION 0.26.0; QA WINDOWS PENDENTE]

### Correção de performance após surface carvers data-driven

- Relato do usuário: geração de mundo ficou **extremamente lenta** após as mudanças recentes.
- Causa estrutural encontrada no hot path:
  - o surface carver de Caverns passou a poder ser avaliado em praticamente todo surface biome habilitado;
  - os mesmos tunnel candidates eram reconstruídos/conectados repetidamente para cada coluna do chunk;
  - o path descendente novo ainda carregava rescans legados da versão antiga do tunnel.
- Correções aplicadas:
  - cache determinístico de surface tunnel candidate por célula durante o density pass do chunk;
  - candidates fora do alcance da coluna são descartados antes da resolução cara;
  - geometria/conexão do mesmo tunnel é reutilizada pelas colunas do chunk;
  - path descendente considera o primeiro ponto como mouth autoritativa e não procura novamente a boca ao longo dos 25 samples;
  - `surface_height()` para validar profundidade de conexão só é calculado depois de um sample realmente tocar o cave graph.
- O objetivo é preservar exatamente a frequência/semântica das entradas, removendo recomputação redundante.
- Commits principais da correção: `58ab3fe4...` e sequência anterior do cache/rescan.
- Um CI intermediário da correção de performance fechou verde antes da sequência de biomas; depois o bloco foi reaberto pelas mudanças arquiteturais abaixo.

### Correção de arquitetura: Mountain-family são biomas próprios

- Solicitação corrigida pelo usuário: **Gorge, Alps, Mountain Belt e Volcano NÃO devem herdar Mountains nem existir como overlays/modifiers do biome Mountains**.
- A tentativa intermediária de introduzir `BiomeKind::TerrainOverlay` + `parentBiome` foi **abandonada e removida**.
- Contrato atual:
  - `Mountains`, `Gorge`, `Alps`, `Mountain Belt` e `Volcano` são **surface biomes independentes**;
  - cada um possui sua própria distribution;
  - cada um possui seu próprio `BiomeTerrain`;
  - cada um possui seus próprios `surfaceLayers`;
  - cada um possui sua própria hydrology;
  - cada um possui suas próprias structures/visuals/allowSurfaceCarvers explicitamente;
  - nenhuma regra de geração pode depender de branch hardcoded pelo ID do biome.
- Novos terrain generators no enum `BiomeTerrain`:
  - `gorge`;
  - `alps`;
  - `mountain_belt`;
  - `volcano`.
- A força já calculada pela macro-distribution passa junto na `BiomeInfluence.terrain_strength` para os terrains cuja forma depende diretamente da distribuição.
  - Gorge usa essa força para graduar parede → fundo;
  - Volcano usa essa força para formar cone → cratera.
- Essa força é **reutilizada** do `sample_surface()`; o terrain não reamostra a macro-distribution por coluna, evitando reintroduzir custo desnecessário no worldgen.

### Gorge

- `asteria:overworld/gorge` é surface biome completo.
- Distribution: `noise_band`.
- Terrain próprio `type: "gorge"`:
  - `baseHeight: 18`;
  - `depth: 42`;
  - `wallHeight: 20`.
- Surface material atual: stone.
- `allowSurfaceCarvers=true`.
- Hydrology é declarada no próprio JSON; nenhuma permissão é herdada de Mountains.
- Possui placement próprio de boulders.

### Alps

- `asteria:overworld/alps` é surface biome completo.
- Distribution: `mountain_peak`.
- Terrain próprio `type: "alps"`:
  - base elevada;
  - ridge principal mais alto/agudo;
  - detalhe secundário jagged próprio.
- Surface material atual: stone.
- `allowSurfaceCarvers=true`.
- Hydrology e boulders são declarados explicitamente no próprio biome.
- Não depende de `Mountains.terrainModifiers`.

### Mountain Belt

- `asteria:overworld/mountain_belt` é surface biome completo.
- Distribution: `mountain_belt`.
- Terrain próprio `type: "mountain_belt"`, com ridge broad + detalhe condicionado ao ridge.
- Surface material atual: stone.
- `allowSurfaceCarvers=true`.
- Hydrology e structures são declarados explicitamente.

### Volcano

- `asteria:overworld/volcano` é surface biome completo.
- Distribution: `mountain_peak`, mais rara/espaçada.
- Terrain próprio `type: "volcano"` com cone e cratera:
  - `baseHeight: 12`;
  - `height: 72`;
  - `craterDepth: 38`;
  - `craterRadius: 0.22`.
- Material autoral existente confirmado no content registry: **`asteria:bassalt`** (ID possui dois “s” e deve ser preservado).
- Surface layers:
  - 6 blocos de `asteria:bassalt`;
  - stone abaixo.
- Regra explicitamente solicitada/confirmada:
  - `canGenerateRiver=false`;
  - `canGenerateLake=false`;
  - multipliers de river/lake = 0.
- Portanto Volcano **não herda** a hydrology de Mountains.

### Dimension / tamanhos

Os quatro novos surface biomes estão registrados no Overworld com sizes próprios, portanto participam da seleção normal de surface biome e também respondem ao Biome Size Multiplier:

- Gorge: X 80–180 / Z 140–360;
- Alps: X/Z 140–320;
- Mountain Belt: X/Z 160–380;
- Volcano: X/Z 120–260.

Esses valores ainda são tuning inicial e precisam de QA visual/distributiva em jogo.

### Limpeza da arquitetura intermediária

Foram removidos do caminho final:

- `BiomeKind::TerrainOverlay`;
- `parentBiome`;
- runtime `TerrainOverlayEntry`;
- aplicação aditiva de terrain overlay no `surface_height`;
- material blending de overlay;
- validações de parent overlay;
- modifier intermediário `volcanicCone`.

`ARCHITECTURE.md` foi corrigido para registrar que a família montanhosa é composta por **surface biomes independentes**, sem herança implícita de Mountains.

### Estado de CI / versionamento neste ponto

- O código funcional do bloco ficou em `b696cfe5738911729f566feb58dd8588849cbc9f`.
- CI de push `35392816751` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- `8ebf13a4db55ff20ccc03f31fc869e7cf1131107` sobe `VERSION 0.25.1 → 0.26.0` pelo novo bloco funcional de biomas/terrains montanhosos independentes.
- No momento desta atualização, o commit isolado de bump ainda não tinha workflow observável pelos checks disponíveis; portanto a evidência canônica de compilação continua sendo a run verde do código imediatamente anterior.
- Failures anteriores desta sequência foram estados intermediários da refatoração e incluíram:
  - referência residual a `TerrainOverlay/parentBiome`;
  - fixtures de `BiomeInfluence` sem o novo campo `terrain_strength`;
  - ambos já foram corrigidos antes da run verde.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### Próximo passo imediato

1. Confirmar a CI do commit de bump `8ebf13a4...` quando a run ficar observável e, se verde, tratá-lo como HEAD canônico versionado.
2. QA Windows/worldgen necessária para:
   - confirmar que a regressão severa de velocidade foi eliminada;
   - confirmar Gorge com paredes abruptas/desfiladeiro reconhecível;
   - confirmar Alps visualmente mais altos/agudos;
   - confirmar Mountain Belt formando cadeias coerentes;
   - confirmar Volcano com cone/cratera e `bassalt`;
   - confirmar ausência de rivers/lakes em Volcano;
   - avaliar frequência/tamanho dos quatro biomes e ajustar tuning sem alterar a arquitetura.



## Checkpoint 102 — 2026-09-18: Volcano e Gorge não ficam flat quando o spawn biome é forçado [CÓDIGO + CI VERDE; VERSION 0.26.1; QA WINDOWS PENDENTE]

- Relato do usuário: terrain generation de Volcano estava totalmente flat; Gorge apresentava o mesmo problema.
- Causa objetiva:
  - o fluxo de New World pode forçar o surface biome inicial para o biome escolhido;
  - dentro do core da região forçada, `forced_weight` é 1.0 constante;
  - `sample_surface()` usava esse mesmo valor também como `BiomeInfluence.terrain_strength`;
  - `BiomeTerrain::Volcano` e `BiomeTerrain::Gorge` dependem diretamente de `terrain_strength` para formar cone/cratera e parede/fundo;
  - portanto o biome era corretamente Volcano/Gorge em identidade/material, mas sua força geométrica virava constante em toda a região forçada, achatando o terrain.
- Correção em `db05e21ed9a28f25b23202c5b5dbfe1741194319`:
  - separa definitivamente **peso/seleção do biome** de **força geométrica do terrain**;
  - macro-biome selection ainda usa `distribution_strength * biome.weight` para competição/frequência, mas `terrain_strength` passa a preservar a força crua da distribution sem ser distorcida pelo weight;
  - forced `mountain_peak` recebe perfil radial dentro da região forçada, permitindo Volcano formar cone e cratera;
  - forced `noise_band` recebe perfil transversal pelo eixo curto da região, permitindo Gorge formar paredes e fundo em vez de um plano;
  - forced `mountain_belt` também recebe perfil transversal coerente para não carregar a mesma armadilha caso passe a consumir terrain strength no futuro;
  - biomes regionais mantêm strength 1.0; Alps/Mountain Belt atuais continuam usando seus próprios noises e não dependem desse strength para a altura.
- A correção é genérica no caminho de `BiomeField::sample_surface()`; não há branch hardcoded por ID de Volcano ou Gorge.
- CI funcional: run de push `35393716818` — **success**:
  - auditoria de localizações;
  - Clippy `--locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- `9367da25dc6b15b4ddcd1175fd5ae3dbb0a1909c` sobe `VERSION 0.26.0 → 0.26.1` por bugfix de worldgen. Run de push do bump `35393788708` estava **queued** no momento deste registro.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: QA Windows criando mundo com Spawn Biome = Volcano e Spawn Biome = Gorge. Confirmar que Volcano possui slope radial + cratera visível e que Gorge possui seção transversal com paredes/fundo, sem regressão de identidade, material `asteria:bassalt`, hydrology ou transição para biomes vizinhos. Também validar um Volcano/Gorge natural fora da região forçada para confirmar equivalência visual aproximada entre spawn forçado e ocorrência natural.


## Checkpoint 103 — 2026-09-18: surface biomes unificados + mountain separation + Gorge/Volcano tuning + crash/OOM hardening [CÓDIGO FUNCIONAL CI VERDE; VERSION 0.27.0; QA WINDOWS PENDENTE]

### Relatos que motivaram o bloco

- Após caminhar pelo Overworld, o único biome terrestre não-montanhoso encontrado voltava a ser Plains.
- Mountains/Ocean e as novas formações montanhosas estavam efetivamente sendo aplicados por cima da seleção regional, consumindo biomas vizinhos e deixando trechos de poucos blocos.
- Volcano podia nascer imediatamente ao lado de Gorge, produzindo transição visual ruim.
- Gorge tinha fundo totalmente flat.
- A cratera de Volcano estava grande demais e o vulcão parecia circular/perfeito demais.
- Mundo iniciado em Enchanted Forest fechou após exploração sem relatório de crash.

### Ownership único de surface biome

- Foi removido o estágio `strongest_macro` de `BiomeField::sample_surface()`.
- Todo biome físico de superfície agora disputa a mesma região territorial de surface:
  - Plains;
  - Wasteland;
  - Witchwood;
  - Enchanted Forest;
  - Mountains;
  - Gorge;
  - Alps;
  - Mountain Belt;
  - Volcano;
  - Coast;
  - Ocean.
- `distribution` continua controlando elegibilidade/força de biomas não-regionais, mas não existe mais uma segunda fase que sobrepõe Mountains/Gorge/Volcano/etc. em cima de outro biome já selecionado.
- Depois que um biome ganha uma região, sua `distribution_strength` pode continuar alimentando apenas a geometria interna do terrain próprio.
- Isso elimina a arquitetura que podia “comer” um biome regional e reduzi-lo a uma faixa estreita.

### Frequência / Plains dominante

- O selector de sites agora considera todos os surface biomes no mesmo weighted draw.
- Regional continua tendo distribution strength 1.0.
- Biomas de distribuição especial competem somente onde a distribution própria possui força.
- Plains/Wasteland/Witchwood/Enchanted e todos os biomas terrestres atuais recebem faixa de continentalness terrestre (`min=0.36`).
- Ocean usa faixa oceânica (`0.0..0.32`) e Coast usa faixa de transição (`0.28..0.48`).
- Isso impede biomas terrestres genéricos de continuarem competindo normalmente no miolo oceânico e reduz o viés estrutural que deixava Plains como fallback visual dominante.

### Ocean e Coast são Surface

- `asteria:overworld/ocean` e `asteria:overworld/coast` foram migrados de `kind: hydrology` para `kind: surface`.
- Ambos agora possuem:
  - terrain próprio;
  - surface layers;
  - size X/Z na Dimension;
  - climate continentalness;
  - hydrology/materials próprios.
- Ocean size atual: X/Z 180–520.
- Coast size atual: X/Z 90–220.
- Hydrology não entra mais no `CurrentBiome.influences` geral; Coast/Ocean permanecem como diagnóstico hidrológico separado, enquanto a identidade principal vem do surface biome real.
- Ocean carving/water usa continentalness efetiva atenuada pela influência surface de Ocean/Coast; low continentalness sozinho não deve mais cavar oceano dentro de um biome terrestre sem ownership oceânico.
- Forced Spawn Biome de land continua suprimindo oceano proporcionalmente ao override; forced Ocean/Coast não se auto-suprime.

### `avoidNear` entre formações montanhosas

- Mountains, Gorge, Alps, Mountain Belt e Volcano declaram `avoidNear` mutuamente.
- Antes deste bloco isso seria ineficaz para macro-biomes porque `avoidNear` só participava do grafo regional.
- Como todos agora disputam a mesma seleção de surface site, a regra passa a valer de verdade:
  - formações conflitantes podem existir próximas;
  - mas não podem compartilhar uma fronteira Voronoi direta;
  - um biome surface compatível deve separar as regiões.

### Gorge

- `BiomeTerrain::Gorge` agora possui:
  - `floorAmplitude`;
  - `floorScale`.
- Tuning atual:
  - `floorAmplitude = 6`;
  - `floorScale = 0.032`.
- O relief é aplicado principalmente no interior do desfiladeiro e desaparece em direção às paredes.
- Portanto o fundo não deve mais ser uma chapa plana.

### Volcano

- `craterRadius`: **0.22 → 0.14**.
- O cone/cratera ganhou parâmetros data-driven de irregularidade:
  - `irregularity = 0.13`;
  - `irregularityScale = 0.009`;
  - `detailIrregularity = 0.055`;
  - `detailScale = 0.031`;
  - `craterIrregularity = 0.08`.
- A deformação broad/detail atua principalmente na meia-encosta, preservando centro e borda externa.
- A borda da cratera recebe noise separado, evitando crater rim perfeitamente circular.
- Continua usando `asteria:bassalt` e continua com river/lake desabilitados.

### Crash seco / pressão de memória durante exploração

Foram encontrados dois problemas concretos capazes de gerar grande pressão de memória sem necessariamente produzir Rust panic:

1. **Chunks nunca editados eram arquivados para sempre em RAM.**
   - Cada chunk gerado entrava em `generated_chunks`.
   - Ao sair da janela de streaming, o chunk era transformado em `ArchivedChunk` e mantido indefinidamente.
   - Caminhar continuamente fazia memória crescer com a distância explorada.

2. **Autosave clonava o `VoxelWorld` residente inteiro.**
   - `capture_owned()` fazia `self.world.clone()`;
   - isso duplicava toda a janela residente no momento do autosave, além dos chunks arquivados.

Correção:

- Terrain determinístico e nunca editado agora é derived state:
  - ao sair da retenção de streaming, ele é descartado completamente;
  - se o jogador voltar, é regenerado pelo mesmo seed.
- A primeira mutação persistente de block/fluid marca o chunk como `persistent_chunks`.
- Apenas chunks persistentes são arquivados e serializados no save.
- Chunks vindos de saves existentes são tratados conservadoramente como persistentes.
- Geração determinística por si só não incrementa mais a revisão persistente do mundo.
- Autosave não clona mais `VoxelWorld`:
  - o main thread captura diretamente um `WorldSnapshot` serializável;
  - somente chunks persistentes entram no snapshot;
  - o worker recebe o snapshot pronto para validar/publicar.
- Session logging agora marca a sessão anterior com `UNCLEAN SHUTDOWN DETECTED` no próximo boot quando não encontra:
  - `CLEAN SHUTDOWN`;
  - `RUST PANIC`;
  - `WINDOWS NATIVE EXCEPTION`.
- Esse marcador não inventa a causa do término, mas impede que um kill/OOM/fail-fast silencioso pareça uma sessão normal sem evidência.

### Arquitetura / documentação

- `ARCHITECTURE.md` atualizado para registrar:
  - ownership territorial único de surface biomes;
  - Ocean/Coast como surface identities;
  - hydrology sem substituir surface identity;
  - `avoidNear` para qualquer surface region;
  - descarte/regeneração de terrain determinístico não editado;
  - autosave proibido de clonar o `VoxelWorld`.

### Commits principais

- Gorge floor relief: `61c08296...`, `e2fb1f32...`, `c062276a...`.
- Volcano crater/irregularidade: `b6410ecd...`, `c01a778c...`, `b2e5e3be...`, `c311a203...`.
- Mountain `avoidNear`: `dfceb6d6...`.
- Surface ownership unificado: `24f4f04b...`, `1bc22fcb...`, `da9c0e1f...`, `e832bfcf...`, `10f5c3e5...`.
- Ocean/Coast Surface + climate: `62735891...`, `7d5eb8db...`, `5bba770c...` e sequência de climate commits.
- Hydrology/identity ownership: `a496d85b...`, `61f53854...`, `df77645b...`, `8cc8b956...`, `28242873...`, `d2ff35e2...`.
- Memória/persistência/autosave: `e9ef8150...`, `076250f3...`, `ae50a577...`.
- Crash-log unclean marker: `a1399a03...`.
- Contrato arquitetural: `41d8acf1...`.
- Version bump: `27e6249f1390bf0401fbc739b0d499be27869112` — `0.26.1 → 0.27.0`.

### CI / validação

- Código funcional validado em `004068bd7be914afa85081cc7a10b2a2c89506b3`.
- Run de push `35395177274` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- O cleanup `8b4a0d44...` expôs duas assignments mortas de `surface_weight` no Clippy; esse estado intermediário falhou.
- `4778ac925d95074a0b2b8606f6c64b2da3e4a156` corrige a normalização diagnóstica sem writes mortos.
- `VERSION 0.27.0` está em `27e6249f1390bf0401fbc739b0d499be27869112`.
- CI canônica final: run de push `35395484913` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### Próximo passo imediato

QA Windows em mundo novo, prioritariamente:

1. Spawn Biome = Enchanted Forest, caminhar continuamente além de pelo menos uma janela de unload e atravessar um autosave; confirmar ausência de fechamento seco e observar o log se houver término anormal.
2. Navegar vários quilômetros e confirmar diversidade real entre Plains/Wasteland/Witchwood/Enchanted + formações montanhosas, sem biomes terrestres reduzidos a tiras por overlays.
3. Confirmar Ocean/Coast como regiões surface coerentes e água oceânica restrita ao ownership oceânico/transição.
4. Confirmar que Mountains/Gorge/Alps/Mountain Belt/Volcano não compartilham fronteira direta quando conflitam por `avoidNear`.
5. Gorge: fundo visivelmente irregular, mantendo paredes/desfiladeiro.
6. Volcano: cone não circular perfeito, cratera menor porém ainda grande, irregularidade natural e `bassalt`.
7. Reabrir/revisitar chunks não editados e confirmar regeneração determinística; editar um chunk, sair da área, voltar e confirmar que a edição persiste.


## Checkpoint 104 — 2026-09-18: crash ao iniciar mundo após Ocean/Coast virarem Surface [FIX + CI VERDE; VERSION 0.27.1]

- Crash reportado:
  - panic em `src/content/dimension_hydrology.rs:70`;
  - mensagem: `hydrology.oceanBiome must reference a hydrology biome`;
  - `asteria:overworld/ocean` já era corretamente `BiomeKind::Surface` após o checkpoint 103.
- Causa:
  - a migração de Ocean/Coast para surface biome real foi feita no conteúdo/runtime;
  - `DimensionHydrology::validate_references()` manteve a invariância antiga que exigia `BiomeKind::Hydrology`;
  - portanto o conteúdo novo era rejeitado antes do worldgen iniciar.
- Correção em `15b569b30a81c6c753cb76bcbe3d53bcb3fabaa7`:
  - `hydrology.oceanBiome` e `hydrology.coastBiome` agora **devem referenciar `BiomeKind::Surface`**;
  - a mensagem de validação foi atualizada para refletir o contrato arquitetural novo;
  - a validação continua estrita: referência ausente ou biome de kind incorreto ainda produz erro explícito.
- CI funcional: run de push `35395972922` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- `5e7dbdad861e4b8e877fcab47c039651525f4536` sobe `VERSION 0.27.0 → 0.27.1`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo imediato: repetir criação/entrada no Overworld e confirmar que o bootstrap passa da validação de `DimensionHydrology`. Depois continuar a QA do checkpoint 103: diversidade de surface biomes, Ocean/Coast, Gorge, Volcano, mountain `avoidNear` e exploração prolongada/autosave.


## Checkpoint 105 — 2026-09-18: `requireNear` data-driven para adjacência obrigatória de surface biomes [FEATURE + CI VERDE; VERSION 0.28.0]

- Novo campo em `DimensionBiome`: `requireNear: Vec<String>`.
- Semântica: OR. Um surface biome com lista não vazia só pode ganhar uma região se compartilhar uma borda Voronoi real com pelo menos um dos biomes listados.
- A mesma geometria de fronteira usada por `avoidNear` é reutilizada; não é distância aproximada nem raio.
- Validação de conteúdo:
  - `requireNear` só é permitido em `Surface`;
  - IDs não podem repetir nem apontar para o próprio biome;
  - alvo precisa existir, estar na mesma dimensão, ser `Surface` e ter `weight > 0`;
  - o mesmo alvo não pode aparecer simultaneamente em `avoidNear`.
- Overworld:
  - Coast agora possui `requireNear: ["asteria:overworld/ocean"]`;
  - portanto Coast só é válido se tocar Ocean diretamente.
- Spawn Biome:
  - biomes com `requireNear` deixam de aparecer como opção de spawn forçado;
  - bootstrap também rejeita config/save que tente forçar isoladamente um biome com dependência de adjacência.
- Commits principais:
  - `2b353b60...` adiciona o campo/validação;
  - `41c3af79...` leva a regra ao `BiomeField`;
  - `e6f40ace...` aplica a borda obrigatória na seleção;
  - `fac23e66...` configura Coast → Ocean;
  - `a0e1e302...` / `5859ae19...` protegem Spawn Biome;
  - `c409327e...` rejeita targets desativados;
  - `74b7d6bf...` documenta a arquitetura;
  - `5acad2c8...` sobe `VERSION 0.27.1 → 0.28.0`.
- CI funcional: run de push `35396733423` — **success**:
  - auditoria de localizações;
  - Clippy `--locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

Próximo passo: criar fluido de lava como feature separada: emissivo, opaco, laranja, spread menor e mais lento que água.


## Checkpoint 106 — 2026-09-18: lava emissiva + crater lake/spill channels no Volcano [FEATURE + CI VERDE; VERSION 0.29.0]

### Fluido lava

- Novo fluido: `asteria:lava` em `data/fluids/lava.json`.
- Autoramento atual:
  - HSI laranja: hue 28, saturation 0.96, intensity 0.72;
  - opacity 1.0;
  - roughness 0.34;
  - metallic 0.0;
  - lightDampening 15;
  - lightEmission 15;
  - spreadSpeed 2.0;
  - maxSpread 3.
- Comparação com água atual:
  - água: `spreadSpeed=12.0`, `maxSpread=7`;
  - lava portanto espalha bem mais devagar e por menos blocos.

### Emissão de luz de fluidos

- `FluidDefinition` ganhou `lightEmission`, default 0.
- Validação exige 0..15.
- Emissão usa a própria cor HSI do fluido.
- Intensidade luminosa escala com o nível da célula de fluido:
  - camada fina emite menos;
  - célula cheia emite o valor authored completo.
- Luz de fluido entra no mesmo pipeline de block light colorido usado pelos blocos.
- Alteração de nível do fluido agora também invalida lighting, não apenas troca de fluid ID.
- Fluido com `opacity >= 1.0` usa material `AlphaMode::Opaque`; fluidos translúcidos continuam `Blend`.

### `surfaceFluid` data-driven em biome

- Novo autoramento opcional em `BiomeDefinition`: `surfaceFluid`.
- Regra atual implementada: `type: "volcano_crater"`.
- Não existe branch por biome ID; a geração decide pela regra + terrain type.
- Validação:
  - só surface biome pode possuir `surfaceFluid`;
  - `volcano_crater` exige terrain `Volcano`;
  - fluid referenciado precisa existir;
  - `minimumStrength`, spill strengths/width/level e scale são validados;
  - spillMaximumStrength não pode invadir a faixa do crater lake;
  - crater `levelOffset` deve ficar abaixo de `craterDepth`.

### Volcano lava lake + spills

`asteria:overworld/volcano` agora declara:

- `fluid = asteria:lava`;
- `minimumStrength = 0.91`;
- `levelOffset = 8`;
- `spillMinimumStrength = 0.56`;
- `spillMaximumStrength = 0.90`;
- `spillScale = 0.012`;
- `spillWidth = 0.055`;
- `spillLevel = 6`.

Comportamento:

- O lago da cratera usa nível horizontal derivado da geometria authored do Volcano:
  `seaLevel + baseHeight + height - craterDepth + levelOffset`.
- Só colunas internas com terrain strength suficiente recebem o crater lake.
- Spill channels ficam fora do lago, na faixa de strength da encosta.
- Os spill points/canais vêm de contornos determinísticos de noise; não existe lava uniformemente vazando por toda a borda.
- O passe de geração mantém `primary_surface_index` e `primary_terrain_strength` por coluna para fazer essa decisão sem resample extra do biome field.
- O fluido authored do terrain tem prioridade no passe de fluidos daquela coluna; hydrology continua independente.

### Arquitetura

- `ARCHITECTURE.md` registra:
  - comportamento de fluidos pertence a `FluidDefinition`;
  - emissão colorida é HSI/data-driven;
  - opacity 1 usa material opaco;
  - `surfaceFluid` é feature do surface biome, não overlay hidrológico;
  - Volcano crater fluid não pode depender de biome ID hardcoded.

### Commits principais

- `19841e28...` — `FluidDefinition.lightEmission`.
- `d98ce251...` — material opaco para fluidos opacos.
- `6862ca6e...` — relight em mudança de nível.
- `2c0e20f2...` / `125f8253...` — emissão HSI de fluidos e propagação.
- `c62becd8...` — `asteria:lava`.
- `64be6654...`, `4597cc2b...`, `716999ff...`, `bd5e551d...` — infraestrutura/validação `surfaceFluid`.
- `be41aaf4...` — terrain strength primário por generation column.
- `388ceb37...` / `a85b3918...` — rasterização de crater fluid/spill channels.
- `2b26d71f...` — Volcano passa a usar lava.
- `a4444c37...` — fixture final de lighting.
- `14172b88...` / `bfc41995...` — invariantes finais crater/spills.
- `54350ffc...` — contrato arquitetural.
- `e848e9e918a87527764ed79616ac3d8134c0830a` — `VERSION 0.28.0 → 0.29.0`.

### CI / validação

- CI funcional final: push run `35397375586` em `bfc41995...` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- `VERSION 0.29.0` em `e848e9e9...`; run de push `35397447773` — **success**.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA imediata

1. Spawn Biome = Volcano em mundo novo.
2. Confirmar crater lake laranja/opaco dentro da cratera.
3. Confirmar emissão de luz laranja à noite/interior sombreado.
4. Confirmar alguns spill channels na encosta, sem vazamento uniforme em todo o rim.
5. Confirmar que a lava, quando entra no solver dinâmico, espalha mais devagar e menos longe que água.
6. Verificar visual das transições de mesh em degraus fortes da encosta; os spills são rasterizados sobre a superfície e podem precisar de tuning visual após QA Windows.


## Checkpoint 106 — 2026-09-18: lava emissiva + lago/derrames no Volcano [FEATURE + CI VERDE; VERSION 0.29.0]

### Fluido `asteria:lava`

- Novo fluido data-driven em `data/fluids/lava.json`.
- Visual:
  - laranja HSI: `hue=28`, `saturation=0.96`, `intensity=0.72`;
  - `opacity=1.0`: renderiza no caminho opaco em vez de Alpha Blend;
  - `roughness=0.34`, `metallic=0`.
- Luz:
  - `FluidDefinition` ganhou `lightEmission` (0..15);
  - lava usa `lightEmission=15`;
  - emissão usa a cor HSI do próprio fluido e escala com o nível da célula;
  - iluminação mistura a emissão de bloco e fluido e propaga a mais forte;
  - mudanças de nível/estado do fluido invalidam iluminação, não apenas troca de `fluid_id`.
- Dinâmica:
  - água continua `spreadSpeed=12`, `maxSpread=7`;
  - lava usa `spreadSpeed=2`, `maxSpread=3`;
  - portanto lava espalha significativamente mais devagar e por distância menor.
- `lightDampening=15`: lava é um meio opaco para transmissão de luz, mas emissivo.

### Surface fluid data-driven

- Novo `BiomeSurfaceFluid`, atualmente com pattern `volcano_crater`.
- `BiomeDefinition` ganhou `surfaceFluid`.
- Campos authored do pattern:
  - `fluid`;
  - `minimumStrength`;
  - `levelOffset`;
  - `spillMinimumStrength`;
  - `spillMaximumStrength`;
  - `spillScale`;
  - `spillWidth`;
  - `spillLevel`.
- O conteúdo valida:
  - ID de fluido não vazio e existente no `FluidRegistry`;
  - surface fluid só em Surface biome com terrain Volcano;
  - ranges 0..1 coerentes;
  - `spillMaximumStrength <= minimumStrength`, mantendo spill fora da faixa do lago;
  - `levelOffset < craterDepth`, impedindo lago acima do rim;
  - `spillLevel` dentro de `MAX_FLUID_LEVEL`.

### Volcano

`asteria:overworld/volcano` agora declara:

- `surfaceFluid.type = volcano_crater`;
- `fluid = asteria:lava`;
- `minimumStrength = 0.91`;
- `levelOffset = 8`;
- `spillMinimumStrength = 0.56`;
- `spillMaximumStrength = 0.90`;
- `spillScale = 0.012`;
- `spillWidth = 0.055`;
- `spillLevel = 6`.

Worldgen:

- `GenerationColumnSample` preserva `primarySurfaceIndex` e `primaryTerrainStrength`.
- O fluid pass resolve o `surfaceFluid` do surface biome proprietário; não há hardcode no ID do Volcano.
- O lago da cratera usa um nível estável derivado da geometria authored do cone:
  - `seaLevel + baseHeight + height - craterDepth + levelOffset`;
  - só preenche ar acima do terreno da coluna;
  - só atua a partir de `minimumStrength`.
- Derrames usam noise-band determinístico, seedado pelo mundo + biome, restrito à faixa de strength authored da encosta.
- Os derrames são fontes parciais (`spillLevel=6`) em canais estreitos, evitando uma cobertura uniforme da montanha.
- Surface fluid é rasterizado antes da hydrology normal; Volcano continua com river/lake natural desabilitados.

### Arquitetura / commits

- `19841e28...`: campo `lightEmission` em fluidos.
- `d98ce251...`: fluidos `opacity=1` renderizam como opaque.
- `6862ca6e...`: relight em mudanças de estado/nível do fluido.
- `2c0e20f2...` / `125f8253...`: emissão HSI de fluido integrada à propagação.
- `c62becd8...`: conteúdo base da lava.
- `64be6654...` / `4597cc2b...`: `BiomeSurfaceFluid` + `surfaceFluid`.
- `bd5e551d...`: valida referência de fluido.
- `be41aaf4...`: preserva terrain strength por coluna.
- `388ceb37...` / `a85b3918...`: geração de crater fluid e spill channels + contexto.
- `2b26d71f...`: autoria do Volcano com lava.
- `ad5f13be...` / `a4444c37...`: fixtures de compilação atualizadas.
- `14172b88...` / `bfc41995...`: validações de separação spill/lago e nível abaixo do rim.
- `54350ffc...`: contrato arquitetural de emissão/surface fluid.
- `e848e9e9...`: `VERSION 0.28.0 → 0.29.0`.

### CI / QA

- CI canônica versionada: push run `35397447773` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

QA Windows prioritária:
1. Abrir/criar mundo e confirmar que `asteria:lava` renderiza laranja e completamente opaca.
2. Confirmar emissão de luz visível em noite/caverna e atualização ao alterar/remover lava.
3. Confirmar spread bem mais lento/curto que água.
4. Forçar/encontrar Volcano e verificar lago de lava dentro da cratera, abaixo do rim.
5. Verificar poucos derrames/canais na encosta, sem cobrir o Volcano inteiro e sem nascer dentro do lago.
6. Confirmar que Volcano continua sem river/lake natural e mantém superfície de `asteria:bassalt`.
