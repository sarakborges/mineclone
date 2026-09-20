# HANDOFF — Asteria / Mineclone

**ESTADO AUTORITATIVO ATUAL — 2026-09-20:** `develop`, Rust + Bevy 0.19.1, `VERSION 0.40.0`. HEAD funcional `7311587a3157cd6c5d6cbde0643df3a551128276`. CI push `35536185243` e PR `35536186909`: **success** em auditoria de localizações, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`. Streaming de chunks novos agora usa coorte coerente por `generation_region`: integra todos os chunks desejados daquela region, mantém-os staged/invisíveis, só então executa full fluid settling até fixed point, seguido de lighting/ready/primeiro mesh. Ordem de conclusão das async generation tasks não decide mais quais vizinhos existiam durante o spread inicial. World Selection mantém botão para abrir a pasta canônica `worlds` no Explorer. Não houve `cargo test`, `cargo run` nem QA Windows.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.34.0`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. **Estado mais recente em `develop`: comparação/correção do save game em andamento; HEAD funcional ainda não versionado `bd713963b9bbb8f69eeeb6df6b7bf7aa2773152f`.** CI `35413383474` passou com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. Já foram corrigidas persistência de scheduled fluid work, health do player, creatures e lock cross-process do diretório do mundo. Ainda NÃO foram implementados selected hotbar slot, rotação/look do player, autosave disparado somente pela passagem do clock, nem a migração do snapshot global para storage incremental por chunk/region. **Não houve bump de VERSION neste checkpoint porque o bloco de save foi interrompido antes do fechamento completo.** Não houve `cargo test`, `cargo run` ou QA Windows.

**Fonte ativa:** `sarakborges/mineclone`, branch **`develop`**, Rust + Bevy 0.19.1. **Versão raiz atual `VERSION`: `0.34.0`**. `Cargo.toml` permanece em `0.10.16` deliberadamente e NÃO é a versão funcional do jogo. **HEAD funcional/versionado atual:** `b6e67ff852a3e0f021948537e596263ec05a69bd`. O runtime de chunks/lighting agora separa residência de conteúdo, readiness de iluminação e render dirtiness, inspirado na separação `INITIALIZE_LIGHT → LIGHT → FULL` e no dirty-state de render sections do Minecraft. A fila de luz rastreia work pendente por section 16³; o primeiro mesh aguarda o halo 3×3×3 estabilizar; background terrain/lighting remesh não captura halo ainda em propagação; light changes são coalescidas antes de rebuild; load/unload vertical invalida skylight das sections residentes abaixo na mesma coluna. Fluid remesh e geometry edits interativos continuam responsivos/independentes. CI final `35412334182` passou com auditoria de localizações, Clippy `-D warnings` e `cargo check --locked`. O CSM direcional do sol (1024, 3 cascades, ~8 chunks) foi identificado como um sistema extra em relação ao modelo de terreno/lightmap do Minecraft, mas NÃO foi alterado neste bloco. Não houve `cargo test`, `cargo run` ou QA Windows.

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


## Checkpoint 107 — 2026-09-18: Coast deixa de ser biome; Ocean ganha `surfaceMargin` data-driven [FEATURE + CI VERDE; VERSION 0.30.0]

### Mudança conceitual

- `asteria:overworld/coast` deixou de ser um `Surface` biome.
- O arquivo `data/dimensions/overworld/biomes/coast.json` foi removido.
- `DimensionHydrology` não possui mais `coastBiome`.
- `Coast` não disputa mais `weight`, `size`, clima ou território no grafo de surface biomes.
- `Ocean` continua sendo um `Surface` biome real.
- A faixa costeira passa a ser uma margem de borda do Ocean, sem identidade própria.

### Novo `surfaceMargin`

- `BiomeDefinition` ganhou `surfaceMargin` opcional.
- Estrutura atual:
  - `width`: largura física da margem;
  - `surfaceLayers`: materiais aplicados nessa faixa.
- `surfaceMargin`:
  - só é permitido em `Surface` biomes;
  - exige `width > 0` finito;
  - exige layers válidas;
  - valida referências de blocos no carregamento.
- A margem é resolvida pela distância até a **fronteira Voronoi real** entre o surface biome atual e um vizinho que declara `surfaceMargin`.
- Ela não é baseada em proximidade genérica, clima ou peso de blend.
- A margem só atua quando o vizinho que define a fronteira efetiva é o biome proprietário da margem.

### Overworld / Ocean

`asteria:overworld/ocean` agora declara:

- `surfaceMargin.width = 32`;
- layers da margem:
  - `asteria:sand`, depth 4;
  - `asteria:stone` como fallback profundo.
- O `continentalness.max` do Ocean foi ajustado de `0.32` para `0.38` para eliminar a lacuna climática que antes era coberta pelo Coast biome e permitir competição/transição direta Ocean ↔ terra.

### Identidade e materiais

- A margem **não substitui `CurrentBiome`**.
- Praia junto de Plains continua Plains.
- Praia junto de Witchwood continua Witchwood.
- A coluna terrestre dentro da margem usa as `surfaceLayers` authored pelo Ocean.
- Fora da margem, volta integralmente às layers do biome terrestre.
- No lado oceânico, o próprio Ocean continua fornecendo seus materiais normais.
- `HydrologyMaterialSet.coast_shore_block` foi renomeado para `ocean_shore_block`.
- O material de shoreline oceânico agora é resolvido diretamente do `BiomeHydrology` do Ocean.

### Hydrology

- `HydrologyBiomeOverlay` agora expõe apenas Ocean; Coast deixou de existir como camada diagnóstica.
- `HydrologyField` e `WorldFeatureFields` não recebem mais `coast_weight`.
- O fator físico de ocean no generation path depende apenas da influência do surface biome Ocean.
- Baixa continentalness sozinha continua proibida de cavar Ocean fora do ownership/transição do Ocean.
- `Spawn Biome` não possui mais exceção para Coast; somente forced Ocean preserva continentalness oceânica.

### `requireNear`

- A infraestrutura `requireNear` permanece disponível e válida para futuros surface biomes que realmente precisem de adjacência territorial obrigatória.
- Shoreline não usa mais `requireNear`; essa relação é modelada por `surfaceMargin`.

### Commits principais

- `abf8c558...`: tipo data-driven `BiomeSurfaceMargin`.
- `9d9057fa...` / `be4c18b3...`: registro/autoria em `BiomeDefinition`.
- `1f857968...` / `c58cc3c4...`: validação de margin e referências de blocos.
- `de83e774...` / `54604011...`: amostra de fronteira real e distância Voronoi.
- `95059cc7...`: resolução de margem por coluna.
- `06805d07...` / `8e17c217...`: aplicação da margem no material pass.
- `843699cf...` / `f777bd7f...`: remoção de `coastBiome` e special-case de Coast no `BiomeField`.
- `76c5fbe9...` / `c9085acc...` / `c53915c0...`: hydrology identity Ocean-only.
- `453b6799...` / `ae1bf724...` / `3e54d7f6...`: remoção de `coast_weight` e gating físico somente por Ocean.
- `475a407e...` / `6aa1aee7...` / `50ebf945...`: shoreline material vem do Ocean e naming interno atualizado.
- `2f939922...`: Coast removido da dimensão.
- `669dee8b...`: Ocean recebe `surfaceMargin`.
- `06bb39ba...`: `coast.json` removido.
- `2427ab82...` / `a7477504...` / `42b24360...`: fixtures/test compilation atualizados.
- `2a4da3a2...`: validação da dimensão deixa de referenciar Coast.
- `1624730c...`: arquitetura atualizada para shoreline margin.
- `c0794cb0...`: `VERSION 0.29.0 → 0.30.0`.

### CI / QA

- CI funcional canônica: push run `35398588362` — **success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

QA Windows prioritária:
1. Gerar mundo novo e localizar Ocean.
2. Confirmar transição Ocean ↔ Plains/Witchwood/Enchanted/etc. sem uma região identificada como Coast.
3. Confirmar faixa de areia de aproximadamente 32 blocos no lado terrestre da fronteira oceânica.
4. Confirmar que `CurrentBiome` continua reportando o biome terrestre na praia.
5. Confirmar que a água/floor do Ocean permanece restrita ao ownership/transição do Ocean e não invade terra distante.
6. Confirmar que rios continuam podendo desembocar no Ocean e que shoreline material continua coerente.

## Checkpoint 108 — 2026-09-18: fluidos runtime gravity-first com busca downhill [FEATURE + CI VERDE; VERSION 0.31.0; QA WINDOWS PENDENTE]

### Objetivo

- Solicitação do usuário: fazer água/lava **fluírem de forma correta**, adotando essencialmente o modelo discreto de fluidos do Minecraft, sem simulação volumétrica/pressão real.
- O solver continua único e data-driven para todos os fluidos. Não existe branch por ID de água ou lava.
- `spreadSpeed` continua controlando a cadência temporal e `maxSpread` continua controlando o alcance horizontal authored de cada fluido.

### Gravidade e alcance horizontal

- Queda vertical tem prioridade.
- Quando uma célula recebe fluido vindo de cima, ela vira uma célula dinâmica cheia e seu `spread_distance` é resetado para **0**.
- Consequência: cada queda vertical inicia um novo trecho horizontal.
  - Água pode percorrer até seu `maxSpread=7`, cair, e voltar a ter até 7 blocos de alcance no novo patamar.
  - Lava pode percorrer até seu `maxSpread=3`, cair, e voltar a ter até 3 blocos no novo patamar.
- Isso permite escorrer por encostas longas sem aumentar artificialmente `maxSpread`.

### Cachoeiras não abrem lateralmente no ar

- Célula dinâmica em queda não pode iniciar spread horizontal enquanto estiver sem apoio sólido.
- Ela continua descendo até alcançar terreno.
- Uma source exposta no topo de uma coluna ainda pode derramar pela borda, preservando o comportamento de fonte.

### Busca da queda mais próxima

- Em terreno suportado, antes de espalhar radialmente, o solver procura uma abertura para queda dentro do **alcance horizontal restante** daquela célula.
- A busca usa BFS curta e limitada por `maxSpread - spread_distance`.
- Se houver queda alcançável:
  - o solver mantém apenas as direções iniciais que pertencem a um caminho horizontal mínimo até a queda mais próxima;
  - empates preservam todas as direções de menor distância.
- Se não houver queda alcançável dentro do range, o comportamento volta ao spread radial normal.
- A busca não atravessa blocos sólidos, fluidos de outro tipo ou chunks não carregados.
- O caminho não pode voltar pelo próprio voxel de origem para fabricar atalhos.

### Arquitetura / persistência

- `ARCHITECTURE.md` registra o contrato gravity-first/downhill em `5cf52abedff0b4288aece2503d82b6538d9ec847`.
- Hidrologia natural determinística continua fora do solver runtime; Ocean/River/Lake não são bulk-enqueued.
- Não houve mudança no schema de `FluidDefinition`.
- Não houve mudança no formato de save: `FluidCell` já persistia `spread_distance`.
- Worldgen de crater lake/spill channels continua separado da simulação dinâmica.

### Cobertura e commits

- `34221ecce2dae814b35bf06f995d7390189ef24b` — implementação principal:
  - queda vertical reseta distância;
  - bloqueio de lateral em células dinâmicas airborne;
  - BFS downhill por range restante;
  - fallback radial quando não há queda.
- Cobertura de regressão adicionada no próprio solver para:
  - range horizontal independente de level;
  - reset de distância após queda;
  - cachoeira dinâmica sem spread lateral;
  - preferência pela queda alcançável mais próxima;
  - spread radial em superfície sem queda.
- `b6f685c237737ff6331f738cf1f1733aa743a206` — corrige apenas o Clippy do helper `FluidCell::flowing` usando-o novamente na cobertura de regressão.
- `f272b6cc89aff06a04ddd77a461c2a8faf221306` — `VERSION 0.30.0 → 0.31.0`.

### CI

- CI intermediária `35399480357` falhou somente em Clippy por `FluidCell::flowing` ter ficado sem uso após a troca do teste antigo; nenhuma falha funcional/typing do solver foi reportada.
- Correção: `b6f685c2...`, sem mudança de semântica.
- **CI final `35399569196` — success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA imediata

1. Criar/usar uma fonte dinâmica de água sobre plataforma com uma queda próxima e confirmar que o fluxo prefere a borda em vez de abrir igualmente para todos os lados.
2. Confirmar que, sem queda dentro de `maxSpread`, água continua abrindo radialmente.
3. Confirmar waterfall: coluna cai verticalmente sem “asas” laterais no meio do ar.
4. Confirmar landing: depois de tocar o chão, a água recebe novamente até 7 blocos de alcance horizontal.
5. Repetir com lava e confirmar a mesma geometria usando somente seus parâmetros authored: spread bem mais lento e no máximo 3 blocos por patamar.
6. Em Volcano, provocar topology update perto da lava/surface fluid e observar se o solver dinâmico acompanha a encosta sem interferir na rasterização determinística inicial.
7. Observar frame time em um cenário com várias frentes simultâneas; a BFS é limitada por `maxSpread`, mas QA runtime ainda precisa validar custo real.

## Checkpoint 109 — 2026-09-18: todo fluido gerado ativa o solver runtime [FIX + CI VERDE; VERSION 0.31.1; QA WINDOWS PENDENTE]

### Correção de contrato

- Após QA do checkpoint 108, o usuário confirmou que os fluidos não estavam fluindo.
- Causa: o solver gravity-first/downhill estava implementado, porém o caminho de carregamento de chunks só retomava fluido **dinâmico** já existente em bordas. Fluidos gerados como `FluidCell::source` — Ocean, River, Lake, água subterrânea, crater lake e spill channels de Volcano — não recebiam o primeiro trabalho no solver.
- Regra autoritativa corrigida: **todo fluido gerado deve poder fluir depois de gerado, independentemente de biome ou sistema de origem**.
- Não existe distinção de ativação entre água de hydrology e lava de `surfaceFluid`.

### Ativação genérica

- `PendingFluidUpdates::enqueue_loaded_fluid_frontier()` agora:
  - varre o conteúdo fluido do chunk que acaba de ficar residente;
  - para cada célula de fluido, semeia apenas destinos adjacentes atualmente vazios;
  - prioriza o destino abaixo e também considera os quatro horizontais;
  - não coloca voxels já preenchidos de fluido na fila como trabalho redundante.
- Quando um chunk novo chega, as faces de chunks vizinhos já residentes também são revisitadas.
  - Isso permite que qualquer source ou fluxo dinâmico atravesse uma seam que antes estava descarregada.
  - A regra vale igualmente para Ocean/River/Lake/Volcano e futuros geradores.
- O solver continua responsável por decidir se o alvo realmente recebe fluido; a etapa de load apenas torna o alvo elegível para processamento.

### Limpeza arquitetural

- A otimização antiga `boundary_dynamic_fluid_count` foi removida.
- Ela codificava a distinção “somente dynamic fluid pode retomar/atravessar boundary”, que deixou de ser válida.
- `PendingFluidUpdates::reserve()`, usado apenas por esse caminho antigo, também foi removido.
- O chunk continua mantendo metadata geral de occupancy/fluid boundary, que serve ao novo caminho universal.
- `ARCHITECTURE.md` agora define fluidos gerados como **estado inicial de fluido**, não decoração estática:
  - hydrology, `surfaceFluid` e futuros geradores podem todos alimentar o mesmo solver;
  - somente targets vazios expostos são semeados, evitando bulk-enqueue do volume preenchido inteiro.

### Commits

- `b0f7e17958a0582f4be13db12df2c0c328897fd5` — ativa frontiers de todos os fluidos residentes.
- `a65762da5711735061529daeb4aa9b2ed365ed5d` — atualiza o contrato arquitetural.
- `1fda89c7263b16d696de0958526907c3fac978f2` — `VERSION 0.31.0 → 0.31.1`.
- `fda74f7c...` / `03119e95...` / `0db60e4cfa7f78aa6b72e73cf0490890f406c3c9` — limpeza de import/helper/metadata dinâmica obsoleta.

### CI

- CI intermediária `35400293204` falhou em Clippy apenas por código morto/unused deixado pela remoção da antiga otimização:
  - import `VoxelChunk`;
  - `PendingFluidUpdates::reserve()`;
  - `VoxelChunk::boundary_dynamic_fluid_count()`.
- O código morto foi removido em vez de receber suppressions.
- **CI final `35400470376` — success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA imediata

1. Mundo novo ou chunk recém-gerado com Volcano: confirmar que crater/spill lava começa a escorrer sem nenhuma edição manual.
2. Confirmar que água de Ocean/River/Lake também reage a espaço vazio adjacente e flui pelo mesmo solver.
3. Quebrar terreno ao lado/abaixo de qualquer fluido gerado e confirmar que a atualização continua propagando.
4. Atravessar boundary de chunk com fluido e confirmar que o fluxo continua quando o chunk vizinho fica residente.
5. Confirmar diferenças authored: água `spreadSpeed=12/maxSpread=7`; lava `spreadSpeed=2/maxSpread=3`.
6. Observar custo de integração em chunks com grandes volumes de Ocean; somente targets vazios entram na fila, mas o scan de fluidos ocorre uma vez por residência de chunk e deve ser avaliado em QA runtime.

## Checkpoint 110 — 2026-09-18: corrigida starvation de fluidos e perda de prioridade entre cadências [FIX + CI VERDE; VERSION 0.31.2; QA WINDOWS PENDENTE]

### Reprodução que revelou o problema

- Após o checkpoint 109, o usuário confirmou que:
  - fluido gerado continuava sem fluir;
  - remover manualmente o bloco diretamente abaixo de um fluido também não fazia o fluido cair.
- O caminho de topology edit foi rastreado:
  - left-click normal usa `VoxelTopologyRuntime::set_block()`;
  - a remoção realmente chegava a `PendingFluidUpdates::enqueue_voxel_edit()`;
  - `process_fluid_updates` estava registrado em Gameplay e condicionado a ticks reais;
  - `desired_fluid()` já tinha a regra correta para queda vertical.
- Portanto o problema estava no scheduling da fila, não na regra de gravidade.

### Causa real

- Havia **uma única fila compartilhada** para trabalho de todos os fluidos.
- `update_ready_steps()` mantém cadência separada por definição:
  - água: `spreadSpeed=12`;
  - lava: `spreadSpeed=2`.
- Porém, quando qualquer fluido rápido ficava ready, a fila global era consumida.
- Se um target de lava fosse retirado num step em que lava ainda não estava ready:
  - o solver identificava corretamente que o target pertencia à lava;
  - mas re-enfileirava esse target no **fim da mesma fila global**.
- Com muitos targets gerados, especialmente água/hydrology, isso destruía a prioridade:
  - um bloco recém-quebrado abaixo de lava podia ser promovido para a frente;
  - um tick de água o retirava;
  - como lava ainda não estava ready, ele voltava para o fim;
  - quando chegava o tick da lava, o target já estava soterrado por grande backlog.
- Frontiers geradas sofriam o mesmo problema.

### Nova estrutura de scheduling

- `PendingFluidUpdates` agora possui:
  - uma fila genérica de **topology edits** cujo fluido ainda precisa ser resolvido pelo estado atual do mundo;
  - filas separadas por `FluidId` para frontiers e continuação da propagação;
  - acumuladores de `spreadSpeed` continuam separados por fluido;
  - cursor round-robin evita que uma fila ready monopolize as demais dentro do mesmo step/frame.
- Frontiers de chunk agora conhecem o `fluid_id` da célula doadora e semeiam diretamente a fila daquele fluido.
- Se um topology target for avaliado antes do seu fluido estar ready:
  - ele é promovido para a **fila prioritária daquele fluido**;
  - não perde prioridade para água/lava de outra cadência.
- Topology edits genéricos são avaliados antes do backlog normal de frontiers.
- Quando uma mutação de fluido acontece, seus vizinhos/centro continuam diretamente na fila do mesmo fluido, respeitando o próximo step authored.

### Block edits

- A notificação do solver saiu do wrapper específico `VoxelTopologyRuntime` e foi movida para `VoxelMutationRuntime::set_block()`.
- Consequência: **toda mutação runtime de bloco** que usa o runtime comum notifica fluidos.
- Isso inclui o caminho normal de quebrar/colocar bloco e também Chisel, que antes usava `VoxelMutationRuntime` sem ativar fluidos.
- `VoxelTopologyRuntime` permanece como wrapper de API, mas não possui uma segunda `PendingFluidUpdates`; não há enqueue duplicado.

### Worldgen / hydrology

- Todo fluido gerado continua elegível para runtime, independente da origem.
- O volume preenchido pelo worldgen continua autoritativo como estado inicial.
- Não se bulk-enqueue todos os voxels preenchidos.
- Apenas targets vazios expostos abaixo/laterais são semeados.
- `ARCHITECTURE.md` foi corrigido para remover a antiga interpretação de que natural hydrology era sempre runtime-static.
- Work de frontier/cadência é data-driven e não contém IDs hard-coded de água/lava.

### Commits principais

- `c893d145b5cbbbaa7c61b23c00f94b7226454aad` — separa pending work por cadência/`FluidId`.
- `ee69b43ab81e83abcfa78fcbe05ee07dd5e4d7bc` — frontiers geradas são roteadas pela identidade do fluido.
- `8b5460d44cc9133c5604f6f30879b9af5af8ad86` — toda mutação runtime de bloco notifica fluidos.
- `43510ac833303059362ff62349a6243c3256b6a5` — topology priority é preservada através da espera pela cadência.
- `bb32dc62c7732baa3876436b0f19d087d6a9345c` — completa passagem de `fluid_id` em todos os scans de frontier.
- `6532d29a7ed990188d38fc2c150bfc9802c50606` — ajuste Clippy-clean da snapshot de filas ready.
- `f2e733cc941eba2d0a5608e22c419815e65b5360` — contrato arquitetural atualizado.
- `9e81d703743e69fc3a9d64115da67709dfead14f` — `VERSION 0.31.1 → 0.31.2`.

### CI

- CI intermediária `35401168767` falhou porque dois call sites de frontier ainda chamavam o helper sem o novo argumento `fluid_id`.
- Corrigido em `bb32dc62...`.
- **CI final `35401298308` — success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA imediata obrigatória

1. Com uma source de lava apoiada em um bloco, quebrar **o bloco diretamente abaixo**.
   - Esperado: em no máximo um step de lava (~0,5 s com `spreadSpeed=2`), a célula vazia abaixo recebe lava e a coluna continua descendo.
2. Repetir com água.
   - Esperado: reação perceptivelmente mais rápida conforme `spreadSpeed=12`.
3. Observar lava gerada no Volcano sem nenhuma edição manual.
   - Targets expostos devem iniciar fluxo mesmo com hydrology/water presente em outros chunks.
4. Confirmar que fluxo de água e lava simultâneos não fazem a lava “sumir” da agenda quando água está ready.
5. Confirmar Chisel: remover suporte/topologia perto de fluido deve acordar o solver.
6. Se world state mudar mas mesh ainda parecer estático, o próximo alvo é starvation no `ChunkRemeshQueue`; este checkpoint corrige especificamente scheduling/mutação do solver.

## Checkpoint 111 — 2026-09-18: impedir starvation visual do remesh de fluidos [FIX; VERSION 0.31.3; RETESTE VISUAL OBRIGATÓRIO]

### Contexto

- Após o checkpoint 110, o usuário confirmou novamente que o fluido ainda parecia não fluir, inclusive ao remover um bloco diretamente abaixo de um fluido estático.
- O caminho de world state foi rechecado:
  - fluidos gerados por hydrology e `surfaceFluid` são `FluidCell` reais armazenados no `VoxelWorld`;
  - não existe uma camada visual separada/fake para água/lava geradas;
  - topology edit chega na fila de fluidos;
  - `desired_fluid()` tem queda vertical direta quando há fluido acima;
  - `set_fluid_at()` é o único mutation path runtime.
- Isso deixou uma segunda classe de falha possível: **estado mutando, mas mesh de fluido não acompanhando visualmente**.

### Causa encontrada no pipeline visual

- `ChunkRemeshQueue::dispatch_remesh_tasks()` tinha prioridade fixa:
  1. Lighting
  2. Geometry
  3. Fluid
- Toda mutação de fluido chama `PendingLightingUpdates::enqueue_medium_edit()`.
- `process_dynamic_lighting()` pode continuar produzindo lighting work e também re-enfileira fluid remeshes para atualizar iluminação de faces.
- Com fluxo ativo, especialmente lava emissiva, isso permite starvation:
  - cada nova célula de fluido gera lighting;
  - Lighting sempre ganha do Fluid;
  - o estado do `VoxelWorld` pode avançar enquanto o mesh exibido continua representando a source antiga.
- Água sofre a mesma classe de problema porque mudança de medium também afeta iluminação/dampening, mesmo sem emissão.

### Correção

- Background remesh agora usa **round-robin** entre:
  1. Fluid
  2. Lighting
  3. Geometry
- `next_background_kind` preserva o próximo tipo entre frames/dispatches.
- Se um tipo não tem trabalho renderizável, o dispatcher tenta os demais no mesmo ciclo.
- Fluid recebe oportunidade garantida mesmo com lighting contínuo.
- Lighting não é descartado:
  - o fluid mesh pode aparecer com a geometria/estado atual enquanto lighting ainda converge;
  - a própria convergência de lighting continua podendo agendar follow-up fluid remesh para iluminação final correta.
- Immediate geometry de block edit continua separado e não foi alterado.

### Arquitetura

- `ARCHITECTURE.md` agora explicita:
  - background remesh deve ser fair entre Fluid/Lighting/Geometry;
  - lighting contínuo não pode impedir mudança de estado de fluido de se tornar visualmente observável.

### Commits

- `e1399c96e5419af21501a8fce716f22b26ba560b` — round-robin de background remesh; remove prioridade fixa Lighting→Geometry→Fluid.
- `3324880065560c26fc3c2fac5fb6a3da5f916544` — contrato arquitetural de remesh fairness.
- `0c35a080b6012c95e4da4b55fb071d0ecb28b22c` — `VERSION 0.31.2 → 0.31.3`.

### CI

- Commit funcional `e1399c96...`: **CI `35401619817` success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### Reteste que decide o próximo passo

1. Pegar uma source de água/lava visível.
2. Remover o bloco diretamente abaixo.
3. Observar se uma nova célula de fluido aparece abaixo.
4. Se ainda não aparecer visualmente em `0.31.3`, o próximo passo NÃO é mais refatorar por hipótese:
   - adicionar diagnóstico runtime explícito para registrar, naquele voxel, `queued → desired → set_fluid_at result → chunk remesh scheduled/applied`;
   - usar o log do caso reproduzido para identificar a etapa exata que não acontece.

## Checkpoint 112 — 2026-09-18: fluidos residentes acordam no Gameplay e fluid mesh tolera churn de lighting [FIX + CI VERDE; VERSION 0.31.4; QA WINDOWS PENDENTE]

### Reprodução e conclusão

- O usuário confirmou um sinal decisivo: ao remover manualmente o bloco abaixo de um fluido, o fluido finalmente propagava, porém com flicker.
- Isso provou duas coisas:
  - o solver e `set_fluid_at()` eram capazes de produzir propagação quando recebiam trabalho;
  - depender de uma topology edit para acordar fluidos gerados era incorreto.
- Requisito autoritativo reafirmado: **todo fluido residente deve começar/continuar sua simulação sem depender de o jogador alterar blocos próximos**.

### Ativação determinística no lifecycle

- Foi adicionado `reseed_loaded_fluid_frontiers()`.
- Ao entrar em `GameState::Gameplay`:
  - `PendingFluidUpdates` é reconstruído;
  - todos os chunks já residentes são varridos uma única vez;
  - cada fluido residente semeia seus targets vazios expostos;
  - o resultado não depende mais da ordem em que os chunks terminaram geração/restauração durante Loading.
- Streaming depois da entrada em Gameplay continua usando o caminho incremental:
  - o chunk recém-residente semeia sua frontier;
  - boundaries de vizinhos já carregados são revisitadas para liberar fluxo cross-chunk.
- O reseed inicial usa um scan por chunk, sem repetir o scan de neighbors para todos os chunks do bootstrap.
- Targets de frontier gerada entram como priority work na fila do próprio `FluidId`, evitando que o primeiro passo fique soterrado por backlog do mesmo fluido.

### Flicker / freshness do remesh

- O pipeline async de remesh antes tratava **freshness de conteúdo** e **freshness de lighting** como uma condição única.
- Como cada mutação de fluido altera medium/lighting, um fluid mesh podia:
  - capturar um estado de fluido válido;
  - terminar o build;
  - ser rejeitado só porque lighting mudou enquanto ele era construído;
  - repetir esse ciclo durante propagação ativa.
- `ChunkRemeshDependencies` agora separa:
  - `content_is_current(world)`;
  - `lighting_is_current()`.
- Terrain/Lighting mesh continuam exigindo lighting atual.
- Fluid mesh:
  - **nunca publica conteúdo voxel/fluid stale**;
  - pode publicar geometria quando o conteúdo capturado ainda é atual mesmo se lighting mudou durante o build;
  - nesse caso, mantém/agende um follow-up fluid remesh para convergir a iluminação final.
- Isso complementa o round-robin Fluid/Lighting/Geometry do checkpoint 111 e remove a dependência visual de a iluminação “parar de mudar” antes de o fluxo aparecer.

### Commits principais

- `00a56bf5b17aaa3c60d46a1de95782752cb5ce8f` — separa freshness de conteúdo e lighting.
- `85ba190a7971276b0a5accec1b79f3b823f7fed2` — publica fluid geometry atual mesmo durante lighting churn.
- `ff9eaf117c5582e17ed618b91556ce155b9ae2f3` — prioriza ativação de frontiers geradas.
- `cba62583565db57b81ff33843eb811d1621ee56d` / `cd3567b3f4d9bd8f70ed02c1d7a0d49727001b7d` — reseed dos fluidos residentes na entrada de Gameplay.
- `87a9704ee33b2040bc98eb4a13aa7db8a0a32895` / `cb616e32ce3c7e3a5d19448faba5e9cdc6473b0b` — scan inicial sem duplicar neighbor work.
- `df43120627c21e4302a50181c8d70144991c31f9` — corrige wrapper recursivo introduzido durante a limpeza.
- `1a760c9a076ce210a59a93c6ad0c224ae89aa6f4` / `f9fa39ab8a2720b2e0b9851bd007d2b56a8127fe` — limpeza e atualização do teste de freshness.
- `632f03d7b8c7b5fb7443a4f3f68d639b879522b7` — arquitetura atualizada.
- `bf6337fc13250374be668e8f6c52b70ce3665e22` — `VERSION 0.31.3 → 0.31.4`.

### CI

- Houve CI intermediária falhando apenas por helper morto/teste ainda usando a semântica antiga de freshness combinada; ambos foram corrigidos sem suppressions.
- **CI funcional final do bloco: `35402331352` — success**:
  - auditoria de localizações;
  - Clippy `--locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA obrigatória

1. Entrar em mundo com lava de Volcano e **não tocar em nenhum bloco**: a frontier exposta deve começar a propagar sozinha.
2. Repetir com água gerada.
3. Quebrar o bloco imediatamente abaixo de uma source: deve reagir conforme `spreadSpeed`, sem depender disso para “iniciar” a simulação.
4. Observar flicker durante queda/espalhamento; a geometria não deve desaparecer/reverter enquanto lighting converge.
5. Se ainda não auto-iniciar em `0.32.0`, o próximo passo deve ser instrumentação runtime explícita `queued → desired → set_fluid_at → remesh scheduled → remesh applied`, não outro refactor por hipótese.

## Checkpoint 113 — 2026-09-18: surfaceMargin do Ocean possui identidade efetiva Ocean [FEATURE + CI VERDE; VERSION 0.32.0; QA WINDOWS PENDENTE]

### Regra alterada pelo usuário

- O usuário definiu explicitamente: **“a margem do oceano precisa ser bioma oceano”**.
- A regra anterior — shoreline material do Ocean sobre terreno que ainda reportava Plains/Witchwood/etc. — foi removida.
- `surfaceMargin` continua não sendo uma segunda seleção territorial de terrain, mas agora pode possuir a **identidade superficial efetiva** dentro da largura authored.

### Separação de terrain owner e identity owner

- `BiomeFieldSample` agora distingue:
  - `primary_surface_index`: owner regional usado pela forma/blend do terreno;
  - `surface_margin_index`: biome que possui a margem ativa;
  - `identity_surface_index`: biome efetivo daquela posição;
  - `primary_id`: ID correspondente à identidade efetiva;
  - `influences`: influências regionais de terreno, preservadas para continuidade geométrica.
- `BiomeFieldEntry` mantém `surface_margin_width` em runtime para resolver a margem diretamente no sampling.
- Quando a posição está do lado terrestre da boundary dentro de `Ocean.surfaceMargin.width`:
  - terrain continua usando o blend/região de land para não criar um degrau artificial;
  - identity passa a **Ocean**.

### Propagação da identidade efetiva

Dentro da margem do Ocean, agora usam Ocean:

- `CurrentBiome.surface_id`;
- `CurrentBiome.surface_influences`;
- identidade final de `CurrentBiome` antes de volume biome;
- grass/leaf/foliage/water visuals;
- permissões de hydrology;
- ownership de materiais de hydrology;
- lookup de `surfaceFluid`;
- `allowSurfaceCarvers` / surface-carver ownership;
- structure candidate ownership, porque structures já validam `sample_surface().primary_id`;
- structure-support carver sampling.

A forma regional do terreno continua separada da identidade efetiva, para a shoreline permanecer suave.

### Limpeza estrutural

- O antigo `nearest_boundary` exposto no `BiomeFieldSample` deixou de ser necessário após a resolução direta de `surface_margin_index` e foi removido.
- `GenerationColumnSample.primary_surface_index` também foi removido:
  - terrain já usa `surface_influences` / o sample regional;
  - comportamento usa `identity_surface_index`.
- Comentários no `BiomeFieldSample` documentam explicitamente a diferença entre terrain owner e margin identity para evitar que os dois voltem a ser conflados.

### Arquitetura

- `ARCHITECTURE.md` foi corrigido:
  - remove a afirmação histórica de que uma beach do Ocean continuava Plains/Witchwood;
  - define `surfaceMargin` como boundary modifier que pode substituir identidade/comportamento dentro da width;
  - deixa explícito que isso **não troca o terrain generator regional**.
- Ocean shoreline passa a ser: **Ocean biome identity + shoreline boundary geometry/material**, não um pseudo-Coast nem land biome com areia.

### Commits principais

- `f574748668f959531fd36c1b331e83d60205f0de` / `a9077d6c6d96e4e041eef2cd7fc99da819ac83d2` — runtime metadata e resolução da identidade efetiva de margin.
- `ca750c4fa77e574d016482d221cd211e421ac159` / `a999e28c7eed2df72e47cb591166a033481e5722` — CurrentBiome efetivo.
- `204743a9d2dcc9471cabc24db6b741795d611eef` — visuais do margin owner.
- `61db4b971cd99ef6c90a34ce5956990dae34aab7` — carrega identity owner no generation column.
- `fa758f7f4670fec97b2c14dc5d61a60a43c5094a` — `surfaceFluid` obedece identidade efetiva.
- `9f8cc1cef079501092fe7805b8b1a18f4943c407` / `b6bd7f080fa895ceff73780d3b97db8b3a28abb8` — materiais/permissões de hydrology.
- `e232785ace04a7258d17af963004d137d1400d0b` / `331aa9a4f8a4c0bf347f5bcd5677e0dd4639fb4c` / `6ca4993a0c3cd86e99faca8b0464488377d4d8ab` — surface carvers e suporte de estruturas.
- `09f7c642baa39b6e2a4c72083f99dcc2de1bf653` — contrato arquitetural atualizado.
- `2cdc67a7c2dd5c71979d59e2438c1582be33ff3d` / `d3a7c287753f6b5b74e9e95b65bfc99ecb8881e1` — remoção de metadata redundante.
- `0a4ec6077dfef0f4e4ca184d4a9703b888b67efd` — documentação inline da separação terrain/identity.
- `1b40f1001c680496b7cdb1a5d01aeb70464212ab` — último fix funcional antes do bump.
- `f0448359abe274a73f1c3d712d8839043df67202` — `VERSION 0.31.4 → 0.32.0`.

### CI

- CIs intermediárias localizaram:
  - call site de surface-carver em structure support sem o novo identity index;
  - metadata antiga que ficou dead code;
  - uma referência local antiga a `primary_surface_index`.
- Todos foram corrigidos sem suppressions.
- CI funcional antes do bump: `35402981026` — success.
- **CI final versionada: `35403045031` — success**:
  - auditoria de localizações;
  - Clippy `--locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA obrigatória

1. Caminhar da terra para a faixa de areia do Ocean: HUD/CurrentBiome deve mudar para **Ocean** já na margin.
2. Confirmar que a costa continua geometricamente suave e não vira um degrau de Ocean terrain no início da margin.
3. Confirmar visuais/cores do Ocean dentro da margin.
4. Confirmar que Ocean `canGenerateRiver=false` / `canGenerateLake=false` e `allowSurfaceCarvers=false` são respeitados na faixa.
5. Confirmar que structures de land não nascem dentro da margin Ocean.

## Checkpoint 114 — 2026-09-18: investigação profunda e correção estrutural do scheduler de fluidos [FIX + CI VERDE; VERSION 0.32.1; QA WINDOWS PENDENTE]

### Por que os fixes anteriores ainda produziam comportamento errado

Após o usuário confirmar novamente que fluidos ainda não funcionavam corretamente e pedir investigação profunda antes de qualquer nova resposta, o pipeline completo foi auditado de novo: geração/source → frontier → cadence → queue → solver → mutation → requeue → downhill routing → remesh.

Foram encontrados **quatro bugs reais e independentes** no runtime, todos capazes de produzir os sintomas já observados — fluido que parece parado até uma edição, propagação aos trancos e comportamento/flicker estranho.

#### 1. A “frontier congelada” não estava congelada

O código fazia snapshot do **tamanho** da fila no início de um fluid step, mas após cada mutação chamava `enqueue_with_neighbors_priority()`.

Isso colocava centro/vizinhos recém-descobertos **na frente da mesma fila que ainda estava sendo processada**.

Consequência:

- um batch iniciado com N posições não processava necessariamente aquelas N posições;
- o primeiro voxel que mudava podia inserir novos targets no front;
- esses targets novos eram processados ainda no **mesmo logical step**;
- uma única frente podia avançar vários blocos num tick, enquanto targets originais eram empurrados para depois;
- a ordem da fila passava a alterar a física visível.

O contrato agora é real:

- cada lane inicia um batch com `batch_remaining = queue.len()`;
- trabalho produzido durante aquele batch entra **no final** da fila;
- priority work recebido enquanto um batch está ativo também é degradado para enqueue normal;
- nada descoberto depois do snapshot entra no logical step atual;
- só o batch seguinte pode consumir esse trabalho.

Foi adicionado `VoxelUpdateQueue::enqueue_with_neighbors()` sem promoção para suportar isso.

#### 2. O frame budget consumia o step, mas não preservava o batch

O sistema anterior guardava `fluid_batch_remaining` em `Local<Vec<usize>>` recalculado a cada execução.

Quando o budget de 1 ms / 512 items acabava no meio de um fluid step:

- parte da frontier ficava na fila;
- o crédito temporal daquele step já tinha sido consumido pelo accumulator;
- o sistema só rodava quando `world_ticks_advanced`;
- para fluidos lentos, especialmente lava, o resto do **mesmo** logical step podia ficar esperando outro intervalo de `spreadSpeed`;
- com uma frontier gerada grande, isso transformava backlog em pausas artificiais;
- um block edit funcionava melhor porque era promovido para o front, explicando por que interação manual parecia “acordar” o fluido.

Agora cada `FluidUpdateLane` possui estado persistente:

- `queue`;
- `accumulated_steps`;
- `ready_steps`;
- `batch_remaining`.

Se o frame budget acaba:

- `batch_remaining` permanece;
- o sistema de fluidos continua executando nos frames seguintes mesmo quando não houve novo world tick;
- o batch em andamento termina antes de iniciar outro logical step;
- `spreadSpeed` continua apenas criando créditos de novos steps;
- até 4 steps podem ficar acumulados para catch-up, sem liberar propagação recursiva dentro do mesmo step.

O sistema portanto roda todo PostUpdate em Gameplay, mas a **cadência física** continua vindo somente de `WorldTickClock`.

#### 3. A queue por fluido perdia o próprio FluidId ao fazer pop

As lanes eram separadas por `FluidId`, porém `pop_ready_fluid()` retornava somente `IVec3`.

Depois o código recalculava o fluido pelo estado atual e usava:

`current.or(desired)`

Isso tinha duas falhas:

- o scheduler não sabia mais de qual lane aquela posição veio;
- numa substituição de fluido, `current.or(desired)` escolhia o fluido **antigo**, podendo aplicar/re-enfileirar uma transição usando a cadência errada.

Agora:

- `pop_runnable_fluid()` retorna `(FluidId, IVec3)`;
- a mutação calcula o fluido que realmente governa a **transição**:
  - se há desired, a cadência é do fluido de destino;
  - se é remoção, a cadência é do fluido atual;
- se o FluidId da transição não corresponde à lane que fez o pop, a posição é reroteada e **não é mutada naquele tick**;
- quando uma mudança troca IDs, as vizinhanças dos fluidos antigo e novo são invalidadas separadamente.

Isso remove dependência acidental de água/lava compartilharem uma posição e impede um fluido rápido de executar estado de outro fluido.

#### 4. O downhill pathfinder esquecia a queda assim que ela era preenchida

`can_fall_from()` considerava queda apenas quando a célula abaixo estava totalmente vazia.

Então:

1. a BFS encontrava uma abertura;
2. o fluxo seguia até ela;
3. a coluna vertical começava a ser preenchida;
4. no step seguinte a célula abaixo já continha fluido;
5. a BFS concluía que a queda “sumiu”;
6. o fluxo podia voltar ao fallback radial ou mudar de direção.

Isso explicava rota instável/pulsante mesmo quando o scheduler chegava a mutar corretamente.

Agora uma queda continua sendo reconhecida quando:

- abaixo está vazio; ou
- abaixo contém o **mesmo fluido dinâmico vertical**, identificado por `!source && spread_distance == 0`; ou
- o node atual está vazio e abaixo já existe o mesmo fluido, permitindo reconhecer um ledge que desemboca em um lower pool já preenchido.

Ao mesmo tempo, source columns já preenchidas dentro de Ocean/Lake não viram falsos “drops” só por terem source do mesmo fluido embaixo.

### Frontier gerada

- Reseed ao entrar em Gameplay continua autoritativo e independente de block edit.
- Ordem dos chunks residentes agora é determinística antes do scan.
- Target diretamente abaixo de source continua priority.
- Targets horizontais gerados voltaram a enqueue normal:
  - gravity continua favorecida;
  - scan de muitas sources não transforma todos os targets horizontais em uma pilha LIFO arbitrária.
- Streaming continua reativando seams quando o chunk vizinho fica residente.

### Cobertura adicionada

Sem executar `cargo test`, foram adicionados testes compilados por `cargo clippy --all-targets` para as invariantes novas:

- trabalho descoberto por uma mutação não entra no logical step já congelado;
- priority work recebido durante um batch ativo também espera o próximo step;
- batch incompleto continua runnable sem exigir novo cadence credit;
- replacement escolhe a lane do fluido de destino;
- downhill continua preferindo uma waterfall depois que a coluna vertical já foi preenchida;
- empty ledge sobre lower pool do mesmo fluido continua sendo reconhecido como drop.

### Commits principais

- `bf26360f81973d53b88099f9119f7de383512153` — enqueue de vizinhança sem priority.
- `a61413d88db5e7d2751e2e0151afca670526e4af` — lanes persistentes, batch state, transition ownership e continuation across frames.
- `89a207cc411787141ff3d36b8295a53921e6549d` — generated frontier: downward priority / horizontal normal.
- `5d02f18b4e1ace9a906acf1141c901d0413db074` / `d8c13a029a5f0d4687127008abe0e76d035c2fe1` — fluid processor continua batches entre frames; condição antiga de world-tick removida.
- `afb778cf32a1ee7128da1576bec334540231a9a5` — waterfall preenchida permanece um downhill sink.
- `d8f3bfbdb01982f4f39026f87e0301a800ebb660` — cobertura da freeze semântica inclusive com priority work.
- `2fa2d66d5176515b75254bb2fddb97250a4a83ac` — contrato arquitetural do scheduler.
- `3912620010c6c1aee6b849f5377f6ddc7e39ee4e` — `VERSION 0.32.0 → 0.32.1`.
- `fa0b7d6dbe9ee6c7e7028fbe8e9738f05c10449d` — lower same-fluid pool também permanece downhill destination.
- `c65120f064b97a77bab3868f0dc97331146ff76b` — contrato final de persistent downhill sinks.

### CI

- O scheduler principal já ficou green em `35404415710`.
- A cobertura extra da frozen frontier ficou green em `35404524994`.
- **CI funcional final após o último ajuste do solver: `35404758584` — success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows, conforme restrição permanente.

### QA que valida este fix

1. **Sem editar blocos:** entrar em mundo com lava/água gerada e observar uma frontier exposta. O primeiro movimento deve ocorrer pelo próprio `spreadSpeed`.
2. Source sobre um buraco: água cai; lava cai com cadência menor; nenhuma precisa de topology edit para iniciar.
3. Plataforma com cliff dentro de `maxSpread`: o fluxo deve escolher a borda, formar waterfall e **continuar preferindo a mesma borda depois que a coluna vertical existir**.
4. Sem drop dentro do range: spread radial normal.
5. Durante uma única onda de água/lava, a frente deve avançar **um logical step por cadence**, sem cascata recursiva de vários blocos causada por enqueue priority.
6. Sob carga/frontier grande: se o budget cortar o frame, o restante da mesma onda deve continuar no frame seguinte, sem congelar até o próximo intervalo do fluido.
7. Água e lava simultâneas: cada uma deve manter sua própria cadence; posição reroteada para outro fluido não pode ser executada pela lane errada.
8. Observar mesh: a propagação não deve mais oscilar por mudança de rota quando a waterfall é preenchida; o anti-flicker/remesh dos checkpoints 111/112 permanece ativo.

## Checkpoint 115 — 2026-09-19: throughput de fluidos + tuning de cadência [PERFORMANCE + TUNING + CI VERDE; VERSION 0.32.2; QA WINDOWS PENDENTE]

### Sintoma

- Após o checkpoint 114, o usuário confirmou que os fluidos finalmente fluíam corretamente.
- Novo problema: a propagação estava **visivelmente lenta demais**.
- Investigação separou duas causas:
  1. throughput real do solver não conseguia acompanhar a cadência authored quando havia frontier grande;
  2. a cadência authored da lava também estava conservadora demais mesmo sem backlog.

### Gargalo 1 — BFS demais por target

Antes:

- cada target horizontal inspecionava até 4 vizinhos fluidos;
- para **cada candidato**, o solver podia executar uma BFS downhill completa;
- só depois da BFS comparava força/level/spread distance/fluid ID;
- no pior caso eram até 4 searches para decidir um único target.

Agora:

1. candidatos baratos são coletados primeiro;
2. ordenados pela mesma regra que já definia o vencedor:
   - maior level;
   - menor spread distance;
   - menor `FluidId` em empate;
3. a BFS é executada nessa ordem;
4. o primeiro candidato que passa no routing downhill é o resultado.

Como a ordenação é a mesma da seleção antiga, a semântica não muda. No caso normal, cai de múltiplas BFS para **uma BFS por target**.

### Gargalo 2 — alocação e queue overhead da BFS

Antes cada search criava:

- novo `HashMap<IVec3, ...>`;
- novo `VecDeque`.

Agora existe `FluidSolverScratch` persistente como `Local` do sistema:

- `visited` é limpo e reutiliza capacidade;
- queue de BFS é uma `Vec` reutilizada com cursor, em vez de `VecDeque::pop_front()`;
- usa `bevy::platform::collections::HashMap` em vez do `std::HashMap` genérico;
- topology classification e runtime propagation compartilham o mesmo scratch no frame.

Isso remove alocação repetitiva no hot path e reduz overhead de queue/hash.

### Gargalo 3 — generated frontier colocava trabalho impossível na fila

A ativação universal continuava correta, mas o seeding inicial era mais largo do que o solver real.

Exemplo crítico: parede vertical de Ocean/Lake.

- várias source layers podem estar expostas lateralmente;
- uma layer coberta por fluido acima e sem apoio sólido **não pode espalhar lateralmente** pela regra do solver;
- mesmo assim a frontier antiga colocava seus targets vazios na queue;
- depois o solver gastava budget só para rejeitá-los.

Agora frontier e solver compartilham `can_spread_horizontally_from()`:

- target abaixo continua considerado independentemente;
- target horizontal só é seeded se a source realmente puder fazer side-flow;
- a checagem de eligibility é **lazy**: interior de Ocean com todos os vizinhos preenchidos não paga essa consulta;
- isso reduz especialmente backlog de Ocean/Lake sem transformar hidrologia em estática.

### Gargalo 4 — invalidation maior que o necessário

Após cada mutação o solver re-enfileirava:

- centro;
- 6 cardinais.

Para uma **mudança de fluido**, o voxel acima não depende do fluido abaixo para nenhuma regra que precise ser recalculada.

Agora runtime fluid mutation invalida somente:

- centro;
- abaixo;
- 4 horizontais.

Block/topology edits continuam usando o caminho amplo/prioritário, porque alteração de bloco pode mudar apoio e outras condições.

### Catch-up adaptativo

Budget normal continua conservador:

- 1 ms;
- mínimo de 64 items antes de consultar relógio;
- máximo 512 updates/frame.

Quando existe backlog real:

- queue total >= 512; **ou**
- alguma lane acumulou mais de um logical step de debt;

o sistema usa catch-up:

- até 3 ms;
- mesmo mínimo seguro de 64;
- máximo 2048 updates/frame.

Importante:

- catch-up aumenta **throughput**, não `spreadSpeed`;
- logical step/cadence continuam iguais;
- batch freeze do checkpoint 114 permanece;
- o limite mínimo não foi aumentado para 128, evitando obrigar um frame a processar trabalho caro demais antes de respeitar o relógio.

### Cadência authored

Depois de corrigir performance, a velocidade nominal também foi ajustada separadamente:

- água: `spreadSpeed 12 → 16`;
- lava: `spreadSpeed 2 → 4`;
- `maxSpread` permanece:
  - água 7;
  - lava 3.

Consequência sem backlog:

- água: ~62,5 ms por logical step;
- lava: ~250 ms por logical step;
- lava continua 4× mais lenta e com alcance horizontal menor, mas não leva mais ~500 ms por voxel.

### O que NÃO foi alterado

- gravity-first;
- reset de `spread_distance` após queda;
- BFS para nearest reachable drop;
- fallback radial sem drop;
- waterfall/lower-pool sink persistence;
- lane ownership por `FluidId`;
- frozen logical batches;
- remesh/lighting semantics;
- universal activation de fluidos gerados.

### Commits principais

- `1d90a12fcf6448f027bbc226b4ce80e7b82ed7ed` — candidate ranking antes da BFS + scratch reutilizável.
- `a4d0d47ccc2bd099e12bb5ff198cbf0187f36c36` / `b14d09bb37ac3047f6078a7a1bf2d1b6aa3f92ea` — scratch integrado ao runtime/visibilidade corrigida.
- `70d93eb9332fec9ff91358e1dcfb9819f4a83b5c` — catch-up adaptativo + vizinhança de recompute menor.
- `452163891e3545cf1f01be023e14d5db040ddc45` — remove helper genérico que deixou de ser necessário.
- `deef998a201b6087bfd2395f794d6dc505bf6428` / `863f0c26471aadbbf0d2d821f6603ff4caa516b4` / `37e3f2316f0bc700b4c40ad4539f8e7b09080861` / `296984f8f0ab164e876575bb99b3f4eab27b0dc7` — frontier gerada filtra side-flow impossível e faz eligibility lazy.
- `de8e88a1fa51d012f4f4fb01b4c4dc9f0ce31683` — catch-up mantém o mínimo de 64 para proteger frame time.
- `208542ed165c35a0ec7340eccd5fbef8109b89e8` — contrato arquitetural de throughput.
- `abca44a6d9db7588d4ba512fbe206276dcf7200a` — `VERSION 0.32.1 → 0.32.2`.
- `0ba1e9d6bedd2b0b23f817f518bc8d6a24570b8d` — água `12 → 16`.
- `d8a3fc1b22f612f11fcd94e49019e11467975455` — lava `2 → 4`.
- `4912aeeff5213609638de12c1404a766ef7b83bc` / `ba8efed288924714ebc3c854c22a537eae8ae50e` — BFS usa HashMap de plataforma + Vec/cursor reutilizável.

### CI

- CI do bloco principal antes do micro-opt final: `35410125356` — success.
- CI com tuning de água/lava: `35410249991` — success.
- A primeira tentativa da queue vetorial falhou apenas porque um seed ainda usava `push_back`; corrigido sem mudança semântica.
- **CI final `35410401754` — success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Água em terreno plano sem drop: a frente deve abrir perceptivelmente mais rápido, mantendo alcance 7.
2. Água procurando cliff: routing downhill deve continuar igual, só com menor latência.
3. Lava no Volcano: deve continuar claramente mais lenta que água, mas sem pausas de ~0,5 s por voxel.
4. Ocean/Lake carregados: confirmar que não há atraso grande causado por side targets impossíveis das camadas internas.
5. Cenário com várias frontiers: confirmar que backlog entra em catch-up sem hitch perceptível.
6. Observar frame time durante flow pesado. O catch-up pode usar até 3 ms de CPU no sistema de fluidos somente quando há backlog.
7. Se ainda houver slowdown apenas com **muitas frontiers independentes em muitos chunks**, o próximo gargalo arquitetural é a barreira global por `FluidId`; nesse caso o próximo passo é shard espacial/chunk-local das lanes, não aumentar mais `spreadSpeed` ou budget.

## Checkpoint 116 — 2026-09-19: scheduler de fluidos por voxel inspirado no Minecraft [ARCHITECTURE + FIX; VERSION 0.33.0; CI VERDE; QA WINDOWS PENDENTE]

### Sintoma que motivou a troca

- Após o checkpoint 115, throughput melhorou, mas o usuário reportou que o fluxo ainda não apresentava visualmente cada step:
  - o fluido “piscava”;
  - depois aparecia já avançado;
  - o requisito correto é que cada propagation step seja observável dentro da própria cadência.
- A causa arquitetural era que Asteria ainda organizava simulação como **waves globais por `FluidId`**:
  - uma lane acumulava cadence credit;
  - processava uma frontier/batch inteira;
  - backlog/catch-up e remesh async podiam coalescer estados intermediários.

### Investigação do Minecraft

Foi examinado o modelo moderno de Minecraft Java por mappings/Javadocs Yarn + NeoForge:

- `FlowingFluid` / Yarn `FlowableFluid` possui estado local de fluido:
  - `LEVEL`;
  - `FALLING`;
  - source/still vs flowing.
- O fluxo é organizado por **scheduled fluid ticks do mundo**, não por uma frontier global de “toda água”:
  - `FluidState.tick(...)`;
  - `FlowingFluid.tick(...)`;
  - `getNewLiquid(...)`;
  - `spread(...)`;
  - `spreadToSides(...)`;
  - `getSpread(...)`;
  - `getSlopeDistance(...)`;
  - `getSpreadDelay(...)` / fluid tick delay;
  - `scheduleFluidTick(pos, fluid, delay)`.
- Cada posição atualiza quando o seu tick vence e novos trabalhos são agendados para ticks futuros.
- Water/Lava especializam delay/drop-off/slope distance/source rules, mas o scheduler é genérico.
- Asteria NÃO copiou:
  - infinite-source creation da água;
  - regras dimensionais específicas de delay/drop-off de lava;
  - still/flowing como registros de fluidos separados.
- Asteria manteve seus próprios contratos data-driven (`spreadSpeed`, `maxSpread`, `FluidCell::source/spreading`) e adotou **o modelo de scheduling**, que é a parte relevante para apresentação step-by-step.

### Implementação nova

`PendingFluidUpdates` agora possui:

- `topology_queue: VoxelUpdateQueue`
  - block edits entram aqui como wake-ups de topologia;
- `wake_queue: DeduplicatedQueue<FluidTickKey>`
  - generated/streamed frontiers entram aqui antes de receber due time;
- `scheduled: BTreeMap<u64, VecDeque<FluidTickKey>>`
  - buckets por absolute world tick;
- `scheduled_due: HashMap<FluidTickKey, u64>`
  - dedup/autorização do due tick atual;
- `FluidTickKey = (FluidId, IVec3)`.

As antigas estruturas foram removidas:

- `FluidUpdateLane`;
- `accumulated_steps`;
- `ready_steps`;
- `batch_remaining`;
- global round-robin por fluid lane;
- frozen wave/batch como unidade de cadence.

### Semântica de scheduling

1. Um generated frontier ou topology edit identifica `FluidId + target position`.
2. O target recebe:
   - `due_tick = current_tick + fluid_delay`.
3. `fluid_delay` é derivado data-driven:
   - `round(ticksPerSecond / spreadSpeed)`;
   - mínimo 1 world tick;
   - `spreadSpeed <= 0` não agenda runtime propagation.
4. Quando `due_tick <= current_tick`:
   - target é retirado do scheduler;
   - `desired_fluid()` é recalculado contra o **estado atual**;
   - se não houver mudança, o tick termina;
   - se a transição pertencer a outro `FluidId`, ela é re-agendada sob o fluido correto;
   - se houver mutação:
     - `set_fluid_at`;
     - lighting medium edit;
     - fluid remesh;
     - center/down/4 horizontais são agendados para um **tick futuro**.
5. Descendentes são sempre agendados usando o world tick em que o parent foi **realmente processado**, não o due tick antigo.

Consequência crítica:

- mesmo se um tick ficou atrasado por budget/frame stall, o solver NÃO tenta “recuperar” a cadeia fazendo:
  - step 1;
  - step 2;
  - step 3;
  - tudo no mesmo frame.
- O parent processado em `current_tick=N` só pode criar descendente em `N + delay`.
- Catch-up pode processar vários eventos independentes já vencidos, mas não pula estados intermediários da mesma propagação.

### Deduplicação / ordering

- Scheduler é deduplicado por `(FluidId, position)`.
- Re-agendar mais tarde não altera um tick anterior já pendente.
- Re-agendar mais cedo substitui logicamente o due time;
  - eventual registro stale no bucket antigo é ignorado quando chegar nele.
- Generated frontier continua:
  - abaixo como priority wake;
  - horizontais como normal wake;
  - side-flow impossível filtrado pela mesma eligibility do solver.
- O solver continua gravity-first e a queue futura preserva down-before-horizontal ao semear neighborhoods.

### Física preservada

Não foi alterado:

- source cells permanentes;
- vertical fall reseta `spread_distance`;
- falling dynamic cells não espalham lateralmente no ar;
- nearest-drop BFS;
- waterfall/lower-pool persistence;
- `maxSpread` data-driven;
- candidate ranking antes da BFS;
- scratch reutilizável;
- água `spreadSpeed = 16`;
- lava `spreadSpeed = 4`;
- água `maxSpread = 7`;
- lava `maxSpread = 3`.

Com world tick atual de 40 TPS:

- água 16/s quantiza para **3 ticks** por step;
- lava 4/s quantiza para **10 ticks** por step.

### Experimento descartado

Antes da investigação do Minecraft houve um experimento curto:

- `9ce067e5...` — publish atômico de wave;
- `7d46ea63...` — presentation lock por chunk.

Após confirmar o modelo do Minecraft, esse caminho foi descartado:

- `e9fcba79a63576d601fbebf5763df699986f5dc9` restaura o remesh pipeline anterior;
- a solução final não depende de bloquear chunks esperando uma wave global terminar.

Isso é intencional: presentation lock tratava o sintoma de uma abstração errada; scheduled voxel ticks removem a abstração errada.

### Commits finais

- `e9fcba79a63576d601fbebf5763df699986f5dc9` — remove presentation-lock experiment e restaura remesh pipeline.
- `bb17e7244edc734f869be5711b71031871d3011c` — scheduled propagation por voxel/due tick.
- `44bfbe14c7f4aeb0bcc9baceda74438e189e4795` — delay desacoplado de fixture/definition inteira; scheduler depende somente de `spreadSpeed` + TPS.
- `ba4238b288a1993f2614a5f661ea144d16669b60` — `ARCHITECTURE.md` migra contrato de lanes para scheduled voxel ticks.
- `d2747fa68c89ef593a6e9e58d03a1f6590486f2d` — `VERSION 0.32.2 → 0.33.0`.

### CI

- Scheduler funcional em `44bfbe14...`: CI `35411423074` — success.
- **CI final versionada `35411527300` — success**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Água em superfície plana:
   - cada avanço deve aparecer como um step separado;
   - não deve haver “pisca e aparece vários blocos à frente”.
2. Waterfall:
   - horizontal routing encontra cliff;
   - próxima descida acontece somente no próximo scheduled interval;
   - coluna continua usando o mesmo downhill sink.
3. Lava:
   - cada step deve permanecer claramente observável;
   - intervalo nominal é 10 world ticks.
4. Block edit abaixo de source:
   - não reage recursivamente no mesmo frame;
   - target recebe scheduled tick e cai no devido intervalo.
5. Frame stall/backlog:
   - posições independentes podem fazer catch-up;
   - uma única cadeia nunca pode atravessar dois propagation steps no mesmo frame por causa do atraso.
6. Se o world state estiver step-by-step correto mas o mesh ainda omitir algum step sob carga extrema, o próximo alvo é exclusivamente **latência do async fluid remesh**, não o scheduler de física. Nesse caso adicionar acknowledgment/version de apresentação por chunk, sem reintroduzir global fluid waves.

## Checkpoint 117 — 2026-09-19: chunk/light section lifecycle inspirado no Minecraft [ARCHITECTURE + FIX; VERSION 0.34.0; CI VERDE; QA WINDOWS PENDENTE]

### Comparação arquitetural com Minecraft

A comparação foi feita sobre três partes independentes do Minecraft Java moderno:

1. **Chunk lifecycle**
   - Minecraft possui estágios explícitos de chunk incluindo `INITIALIZE_LIGHT`, `LIGHT` e `FULL`.
   - A consequência útil para Asteria não é copiar a coluna inteira de chunk, e sim tratar iluminação pronta como um estado separado de conteúdo gerado/residente.

2. **Light engine**
   - O engine mantém estado e work de iluminação por section/coluna, com APIs de section status, column enable e propagation.
   - Asteria já usa `CHUNK_SIZE = 16`, portanto um `VoxelChunk` 16³ corresponde naturalmente à granularidade de uma Minecraft light/render section.
   - Não foi adotada a representação nibble/scalar do Minecraft porque Asteria possui block light HSI colorida. Foi adotada a ideia de readiness/dirty state por section.

3. **Render sections**
   - Minecraft mantém render sections explicitamente dirty e só agenda rebuild através do dispatcher; rebuild/upload são separados do estado do mundo.
   - Asteria já possuía async remesh + revisions, mas disparava rebuilds enquanto a luz ainda convergia, criando trabalho descartado e estados visuais intermediários.

### Bug real encontrado — skylight vertical stale

`seed_chunk_direct_lighting()` calcula céu direto usando as sections atualmente carregadas acima.

Antes:

- section baixa podia carregar e receber céu;
- depois uma section mais alta da mesma coluna carregava com terreno que bloqueava esse céu;
- a section baixa **não era automaticamente re-relaxada como parte da mudança da coluna**;
- inversamente, descarregar uma section superior podia abrir céu e deixar inferiores com lighting antigo.

Isso não é tuning: era uma invalidation ausente.

Agora:

- `VoxelWorld::loaded_chunk_coords_below(coord)` expõe sections residentes abaixo na mesma coluna x/z;
- load/restore de section chama `enqueue_loaded_column_below()`;
- unload também chama `enqueue_loaded_column_below()` depois de remover a section;
- todas as sections inferiores residentes são reavaliadas contra a nova coluna de céu.

### Pending lighting por section

`LightingQueue` agora mantém `pending_by_chunk: HashMap<IVec3, usize>`.

A contagem acompanha:

- enqueue background;
- promoção background → interactive sem dupla contagem;
- priority interactive;
- pop;
- deduplicação já existente.

Novas consultas:

- `has_pending_in_chunk(coord)`;
- `has_pending_in_halo(coord)`, cobrindo o halo 3×3×3 usado pelo vertex lighting/AO/meshing.

`PendingLightingUpdates::has_pending_in_halo()` também considera emission edits ainda não materializados na queue.

Isso permite perguntar se uma section está realmente visualmente pronta sem escanear a fila global.

### Initial mesh: lighting antes de FULL/render

Antes:

1. chunk era gerado/restaurado;
2. recebia direct-light seed;
3. full/background relaxation era enfileirada;
4. chunk já podia ser enviado para mesh;
5. propagação terminava depois e disparava remesh.

Agora:

1. chunk fica residente;
2. direct-light seed ocorre;
3. relaxation inicial e invalidation de coluna são enfileiradas;
4. o chunk permanece na ready queue enquanto seu halo 3×3×3 tiver lighting pendente;
5. só depois é criada a primeira mesh task.

Isso elimina a publicação inicial de uma section com lighting apenas parcialmente convergido.

Importante:

- initial/streaming lighting permanece background;
- block edits interativos continuam preemptando background lighting;
- não foi criada uma lane “streaming critical” neste bloco.

### Coalescência de lighting remesh

Antes cada slice de até 2 ms de propagação podia:

- alterar voxels;
- bump revision;
- imediatamente pedir terrain/lighting/fluid remesh;
- no slice seguinte alterar mais luz;
- invalidar ou substituir o resultado anterior.

Agora:

- lighting revisions continuam sendo bumped imediatamente para impedir publicação stale;
- chunks que mudaram entram em `LightingRemeshState.dirty`;
- a notificação de remesh só é publicada quando o halo 3×3×3 daquela section não possui mais work de lighting pendente;
- diagonal AO/face-lighting invalidation continua seletiva por boundary metadata;
- dirty state é coalescido: muitos slices de propagação resultam em um rebuild estável.

### Dispatcher de remesh

Background `Geometry` e `Lighting` remesh agora também verificam section-light readiness **antes de capturar o snapshot**.

Isso evita:

- construir mesh com lighting conhecido como transitório;
- gastar worker;
- terminar task;
- rejeitar por lighting revision stale;
- repetir até convergência.

Exceções deliberadas:

- **Immediate geometry** de edição do jogador continua imediato.
- **Fluid remesh** continua independente do gate de iluminação, preservando a regra de fluid steps visíveis; se a iluminação do fluid mesh estiver stale, o mecanismo existente agenda follow-up para convergir vertex lighting.

### Chunk/render pool

Asteria já tinha boas equivalências ao Minecraft:

- section/chunk de 16³;
- geração/mesh async;
- prioridades de streaming perto do player;
- visibilidade com hysteresis;
- render allocation por section;
- dirty/revision validation.

Minecraft vai além com storage fixo/reutilizado de render sections ao redor da câmera. Asteria ainda cria/destrói allocations ao unload/reload; o retention radius reduz churn, mas isso **não foi migrado** neste checkpoint porque exigiria outra mudança arquitetural e não era a causa direta do lighting/shadow stale identificado.

### Sombras — diferença importante ainda não alterada

A comparação mostrou que Asteria adiciona um sistema de sombra de terreno que não corresponde ao caminho principal de iluminação do Minecraft:

- `DirectionalLight` real para o sol;
- shadow map 1024;
- 3 cascades;
- distância de sombra limitada a ~8 chunks;
- terrain recebe essas sombras além de:
  - voxel skylight;
  - colored block light;
  - AO/face lighting.

Portanto Asteria atualmente soma:

**voxel light/AO + real-time cascaded directional shadows**.

Esse CSM pode produzir:

- cascade transition;
- shimmer ao mover a câmera;
- shadow popping na distância;
- artefatos independentes do chunk/light lifecycle.

Não foi desligado nem alterado neste bloco porque isso muda a direção visual do jogo. O diagnóstico agora fica separado:

- artefato que aparece junto de **chunk load/unload / iluminação convergindo** → section-light lifecycle;
- artefato que acompanha **movimento da câmera, sol ou limite de ~8 chunks** → CSM direcional é o próximo alvo.

### Commits principais

- `b31eae5218bcf0342512018d6c5daedf09431f70` — pending lighting por section.
- `68f1f4deea63dc97ad7e088984e5ab1493272fc3` — sections residentes abaixo na mesma coluna.
- `f6ccb20267daff0de9478b75d328048ac83f83c2` — readiness de halo + column relight.
- `7b865ec55bfe8b75491231dea13cd2169b4224f1` — coalescência de lighting remesh.
- `90234a2c92d3b509e2e3f7def236ab45449b3fcd` — initial mesh aguarda halo iluminado.
- `b8de63524f078abd558220ced694ca2d10b0e078` — skylight inferior invalidado no unload.
- `b6bd23e82e0d1ab792b2a8a18ab02b364f585558` / `792b9b8e6e9abdddc243fddf6962344b0f968a90` — correções de borrow/visibility encontradas pelo CI.
- `3c27834cd5de92aa688d2b98c4ff1fcc1a3d887b` — background remesh espera lighting halo estável.
- `1b3ddf5737739bfb71c0811fb02d2e7e2653675f` — cleanup do scan cache após gating.
- `dbfebbf44a0bc6a6a81a33e9a10ad4dd3439c25e` — contrato arquitetural de section lighting.
- `b6e67ff852a3e0f021948537e596263ec05a69bd` — `VERSION 0.33.0 → 0.34.0`.

### CI

- `35412036134`: falhou apenas por borrow-check no novo estado de coalescência.
- `35412126836`: falhou apenas por visibilidade privada do tipo usado no system signature.
- `35412222407`: **success** no HEAD funcional antes de docs/version.
- **`35412334182`: success no `0.34.0` versionado**:
  - auditoria de localizações;
  - Clippy `--locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Aproximar-se de terreno novo:
   - chunk não deve aparecer escuro/claríssimo e depois corrigir;
   - primeira publicação já deve usar lighting local convergido.
2. Voar verticalmente / carregar terrain acima de sections já visíveis:
   - sections inferiores devem atualizar skylight;
   - não devem manter sombras antigas da coluna.
3. Afastar-se até sections superiores descarregarem:
   - céu inferior deve reabrir corretamente.
4. Colocar/remover bloco emissivo ou opaco:
   - geometria interativa deve responder imediatamente;
   - lighting final deve convergir sem múltiplos rebuilds visuais intermediários.
5. Observar fluidos simultaneamente:
   - gating de terrain/lighting não deve atrasar fluid remesh.
6. Se restar shadow shimmer/pop:
   - observar se acompanha câmera/sol e o limite de distância;
   - se sim, investigar/remover/segmentar CSM de terrain separadamente, sem mexer novamente no voxel light engine.



## Checkpoint 118 — 2026-09-19: comparação do save game com Minecraft + correções de estado persistente [WIP; VERSION ainda 0.34.0; CI VERDE]

### Pedido / direção

O usuário pediu comparar o sistema de save game porque suspeitava que o Asteria ainda tinha peculiaridades que não funcionavam.

A comparação foi feita olhando principalmente para:

- ownership de chunk gerado vs chunk salvo;
- chunk streaming/unload;
- scheduled simulation state;
- player state;
- creature/entity state;
- atomicidade/recovery;
- concorrência entre processos;
- custo estrutural do snapshot global.

### Divergência arquitetural principal encontrada — snapshot global força trade-off errado

O contrato atual de `ARCHITECTURE.md` diz que:

- worldgen determinístico é estado derivado até o primeiro block/fluid mutation persistente;
- chunk gerado e não editado pode ser descartado e regenerado do seed;
- somente chunks persistentes/editados entram no snapshot.

Isso apareceu historicamente para resolver crescimento de RAM/IO e autosave pesado.

Comparação com Minecraft:

- chunk gerado é armazenamento autoritativo depois de existir;
- chunk pode sair da RAM sem depender de reexecutar worldgen;
- persistência de chunk é incremental/segmentada, não um único snapshot completo de todos os chunks persistentes;
- metadata/player/chunks/scheduled work não precisam ser reescritos como um blob global único.

Conclusão importante para o próximo passo arquitetural:

- o problema não deveria ser resolvido escolhendo entre:
  - “salvar todo chunk gerado e explodir snapshot/RAM/IO”; ou
  - “não salvar chunk intacto e regenerar do seed”.
- O caminho correto é separar storage incremental de chunk/region do snapshot global de metadata.
- **Essa migração NÃO foi implementada neste checkpoint.**
- Portanto, chunk gerado não editado ainda pode ser regenerado conforme o contrato atual.
- Consequência prática: mudança de worldgen entre versões ainda pode alterar áreas já exploradas mas nunca editadas.

### Scheduled fluid work agora persiste

Problema anterior:

- `PendingFluidUpdates` era completamente runtime-only;
- salvar entre um topology edit e seu processamento podia perder a atualização;
- scheduled fluid ticks desapareciam no reload;
- `WorldTickClock.tick` absoluto também não era persistido, então salvar `due_tick` bruto seria incorreto.

Implementação atual:

- `SavedFluidUpdates` foi adicionado ao `WorldSnapshot` com `#[serde(default)]` para compatibilidade com saves antigos.
- Persiste:
  - topology wake-ups pendentes;
  - frontier wakes ainda sem due time;
  - scheduled fluid ticks.
- Fluid IDs são salvos como **IDs textuais**, nunca como `FluidId` runtime.
- Scheduled tick é salvo como **`remaining_ticks`**, não `due_tick` absoluto.
- No load:
  - o scheduler novo começa no tick 0;
  - cada evento é reconstruído para o delay restante equivalente.
- Saves antigos sem `fluid_updates` continuam carregando com vazio.

### Bug adjacente corrigido — due tick em chunk descarregado não pode sumir

Foi encontrado um bug independente no scheduler:

- se um scheduled fluid tick vencia quando `world.sample_at(position)` não tinha chunk residente;
- o tick era removido da fila e simplesmente descartado.

Agora:

- ticks vencidos de chunks não residentes entram em `dormant_scheduled` por chunk coord;
- quando o chunk volta a ficar residente:
  - o work dormente é reativado;
  - processa sem ter sido perdido.
- Esse estado dormente também entra no save como scheduled work com delay imediato.

Isso aproxima o lifecycle do modelo esperado de chunk simulation: unload não apaga trabalho autoritativo.

### Player health agora persiste

Problema anterior:

- `SavedPlayer` tinha apenas posição e `creative`;
- `spawn_player_entity()` sempre usava `EntityHealth::new(definition.health)`;
- reload curava o player para max health.

Agora:

- `SavedPlayer.health: Option<f32>` com `#[serde(default)]`;
- saves antigos sem health continuam válidos;
- health salva participa do dirty-state de autosave;
- load restaura health com clamp ao novo max da definition;
- valor inválido/não finito/negativo invalida o snapshot;
- `EntityHealth::restored(max, current)` centraliza restore seguro.

### Creatures agora persistem

Problema anterior:

- `CreatureInstance` era ECS temporária;
- nenhuma creature aparecia no `WorldSnapshot`;
- sair/recarregar o mundo removia todas;
- natural spawner criava uma nova população.

Agora `SavedCreature` persiste:

- `definition_id`;
- posição;
- health.

Semântica deliberada:

- creature morta não entra no snapshot;
- visual/animation/knockback não são persistidos;
- ao voltar, a creature continua no mesmo lugar e com a mesma vida, retomando motion normal;
- IDs desconhecidos ou valores inválidos invalidam aquele snapshot durante validation/fallback;
- saves antigos sem `creatures` continuam válidos por `#[serde(default)]`.

### Creatures fora de chunks residentes ficam dormentes

Uma primeira versão restaurava todas as creatures diretamente no ECS ao entrar em Gameplay.

Isso foi corrigido antes de fechar o bloco:

- `PendingCreatureRestores` mantém creatures cujo voxel/chunk ainda não está carregado;
- `restore_saved_creatures` roda em Gameplay e só materializa a entity quando `VoxelWorld::is_loaded_at(...)` for verdadeiro;
- autosave inclui:
  - creatures ECS ativas;
  - creatures ainda dormentes.
- Assim salvar antes de revisitar uma área não apaga entities daquele local.

### Lock cross-process do diretório do mundo

Problema anterior:

- `WORLD_LOCKS` / `WorldGate` protegia writers/readers apenas **dentro do processo**;
- duas instâncias do Asteria podiam abrir o mesmo mundo e gravar gerações concorrentes no mesmo diretório.

Agora:

- cada world directory possui `session.lock`;
- `WorldDirectoryLock` mantém um `std::fs::File` aberto e bloqueado via `File::try_lock()`;
- o lock é do SO:
  - não depende de flag manual;
  - cai automaticamente quando processo/handle morre;
  - o arquivo pode permanecer sem representar stale ownership.
- create world:
  - cria o diretório;
  - adquire lock antes de entregar a sessão;
  - mantém o handle como Bevy Resource.
- load world:
  - adquire lock antes de decodificar/ativar o mundo;
  - outra instância recebe `WouldBlock`.
- delete world:
  - só prossegue se conseguir adquirir o lock;
  - mundo aberto em outra instância não pode ser deletado.
- `release_world_session` remove `WorldDirectoryLock`, liberando o lock quando volta ao menu.

### Atomicidade/recovery que já estava boa e foi preservada

O sistema atual já tinha pontos fortes:

- snapshot e manifest são publicados como gerações imutáveis;
- snapshot é escrito/syncado antes do manifest commit marker;
- loader tenta gerações mais novas e faz fallback para backups restorable;
- prune só remove gerações antigas depois de validar backups;
- limite de snapshot é aplicado durante serialization;
- load pesado ocorre em worker;
- autosave publication ocorre em worker;
- Leave World/Exit continuam com commit síncrono.

Nada disso foi removido neste bloco.

### Peculiaridades ainda pendentes, já identificadas

1. **Selected hotbar slot**
   - `PlayerHotbar::restore_items()` força `selected_slot = 0`.
   - Minecraft persiste o slot selecionado.
   - Ainda NÃO corrigido.

2. **Player rotation / look direction**
   - snapshot salva position mas não orientation.
   - player/camera é recriado com rotation default.
   - Ainda NÃO corrigido.

3. **Clock-only autosave**
   - `SavedWorldState` deliberadamente exclui clock para não criar full snapshot apenas pela passagem de ticks/tempo.
   - day/tick-in-day são gravados quando outro save acontece, mas passagem de tempo sozinha não torna o save dirty.
   - Esse trade-off é sintoma do snapshot global.
   - Ainda NÃO corrigido.

4. **Generated chunks intactos**
   - ainda são derivados/regeneráveis;
   - mudança de worldgen pode alterar área explorada sem edição.
   - Precisa da migração para chunk/region storage incremental.

5. **Snapshot global**
   - metadata + player + persistent chunks + entities + scheduled work ainda vivem no mesmo documento JSON.
   - Cada autosave relevante reserializa todos os chunks persistentes.
   - Escalabilidade continua inferior ao modelo incremental por chunk/region.

6. **Outros entity state**
   - motion phase, velocity, animation e knockback não são persistidos neste primeiro passo.
   - Isso foi deliberado; não é necessário para preservar identidade/posição/vida básica da creature.

### Commits principais deste checkpoint

Fluid scheduler persistence:

- `6e94d155a911ad8e81918f37d9dd548c9beb6558` — expõe pending deduplicated queue values para persistência.
- `456c881c328753c53c5b4a815ea3fa5a5a2b2cfa` — expõe pending voxel updates.
- `820e99f6c118fc7c9f7dfde44e19bb3b0b0175c5` — persiste scheduled fluid updates + dormant unloaded ticks.
- `8e3b00e7650b56bc6d00409cfd497bc73f3dfb6a` — inclui fluid scheduler no snapshot.
- `c792c1eea7bde2760b13a23e4281989e9f9576ab` — captura fluid ticks com world save.
- `bef3ee36fe73757eae63e10c1d1fc3ad982efc53` — restore no load.
- `0db860153a5198fba81fc3c19c65dea9af0d2419` — correção de integração encontrada pela CI.

Player/creatures:

- `b455036ce306474313c94b070bc7f212ca94df9e` — restore de EntityHealth.
- `dc2185fa48a877f232eef532d077378eb1ef12c3` — health no `SavedPlayer`.
- `017f5abbb81cd24117642efb29d81af4d282962a` — health restaurada no load.
- `3b39447b0d6b311c963a64e322d23ffd1bcc3b37` — modelo + restore de creatures.
- `d8dd9b9259f34f6d64f2aa9b86ce0b9c79511284` — creatures no snapshot/validation.
- `d0ab94b752e1528a1d3861015fae5d329b3d52fe` — creatures dormentes até chunk carregar.
- `9f9e8865046c4d1bf7ba89bb2645173f94b2e326` — autosave preserva também creatures dormentes.

World directory locking:

- `537d40a86ffe80f2a560fedb30d1bf2bc80ef7ed` — storage layer ganha `WorldDirectoryLock`.
- `59fc7e630f2710a29aad8e15b19f52eff8990dfc` — new world mantém lock pela sessão.
- `fbed5ef22c392ae856746231479ba238499b9dbb` — loaded world mantém lock pela sessão.
- `49ff56f79875f55bc615654db94bf324091f0bd4` — release do lock ao sair da sessão.
- `bd713963b9bbb8f69eeeb6df6b7bf7aa2773152f` — cleanup final do `OpenOptions` do lock.

### CI

Runs intermediárias importantes:

- `35413056069` — falhou por `IVec3::to_array` usado como function pointer com assinatura incompatível.
- `35413103158` — falhou somente por APIs antigas de save que ficaram dead code.
- `35413207481` — **success** para fluid ticks + player health + creature persistence/dormancy.
- `35413324281` — falhou somente por import não usado + `OpenOptions` sem truncate policy explícita.
- **`35413383474` — success no HEAD atual `bd713963...`**:
  - auditoria de localizações;
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.

Não executei `cargo test` (proibido sem autorização explícita), `cargo run` nem QA Windows.

### Estado/versionamento

- `VERSION` permanece **0.34.0**.
- Não houve bump neste checkpoint porque o usuário interrompeu o bloco pedindo atualização do handoff antes de finalizar toda a comparação.
- HEAD funcional atual não versionado: `bd713963b9bbb8f69eeeb6df6b7bf7aa2773152f`.

### Próximo passo quando retomar

Continuar exatamente daqui, sem refazer a investigação:

1. persistir selected hotbar slot;
2. persistir player/camera rotation/look;
3. decidir o tratamento correto de clock-only dirty state sem voltar a full snapshot a cada passagem de tempo;
4. desenhar a migração de chunk persistence:
   - chunk gerado passa a ser autoritativo depois de existir;
   - unload escreve/retém chunk em storage incremental;
   - metadata/player continuam commitados separadamente;
   - region/chunk files devem permitir atualização parcial;
   - recovery/backup generation precisa continuar atômico;
   - não reter todos os chunks explorados em RAM;
   - não reserializar todos os chunks persistentes a cada autosave.
5. Só depois fechar o bloco com bump de versão apropriado e QA de roundtrip.



## Checkpoint 119 — 2026-09-20: save somente ao sair + fechamento da janela durável [CÓDIGO APLICADO; CI PENDENTE]

### Pedido / direção

O usuário decidiu remover autosave completamente. A partir deste checkpoint, Asteria só pode publicar save durável quando o jogador:

- usa **Leave World**;
- usa **Exit Game** dentro de um mundo;
- fecha a janela pelo botão do sistema operacional enquanto existe um mundo ativo.

Não deve existir gravação periódica de world snapshot nem checkpoint periódico separado de relógio durante Gameplay.

### Implementação

Commit funcional:

- `667c284d7465c7c925f2aab8bf55a1bbdfb2e19e` — `refactor: save worlds only on exit`.

Mudanças principais:

- `WorldSession` deixou de carregar:
  - timer de autosave;
  - baseline/dirty-state de autosave;
  - task assíncrona;
  - último estado salvo.
- `autosave_world`, `SavedWorldState`, `OwnedWorldSaveCapture` e o caminho de `AsyncComputeTaskPool` para autosave foram removidos.
- `src/world/clock_persistence.rs` foi removido integralmente.
  - não há mais `clock-*.json`;
  - não há mais `fsync` periódico em `Last`;
  - day/tick continuam persistindo no `WorldSnapshot` final.
- `WorldPlugin` não agenda mais autosave nem clock checkpoint.
- `WindowPlugin.close_when_requested = false` para impedir o fechamento automático antes do save.
- Um `WindowCloseRequested` em Gameplay:
  - executa o mesmo commit síncrono/durável usado por Leave/Exit;
  - só envia `AppExit::Success` depois de sucesso;
  - em erro, consome o pedido e mantém a janela/mundo aberto para retry.
- Em menus/loading, fechar a janela continua saindo imediatamente porque não existe snapshot de mundo ativo a publicar.
- `WorldSession::persist` agora é somente a operação final síncrona de captura + publicação; não mantém baseline de autosave depois do commit.

### Contrato arquitetural

`ARCHITECTURE.md` foi atualizado no mesmo commit:

- Gameplay não executa periodic autosave;
- não existe clock-only checkpoint;
- durable writes de mundo pertencem apenas ao lifecycle de saída;
- falha de save impede abandonar o mundo/janela;
- `WorldSnapshot` é o único owner durável de day/tick;
- background periodic publication não pode reaparecer sem decisão explícita de produto/arquitetura.

### Versionamento

- `VERSION`: **0.34.7 → 0.34.8**.
- `Cargo.toml` permanece deliberadamente em `0.10.16`.

### Validação

Neste ponto do handoff:

- commit aplicado em `develop`;
- CI ainda não verificada;
- não executei `cargo test`, `cargo run` nem QA Windows.

### Próximo passo

Seguir a auditoria de performance pela prioridade já definida:

1. remover o scan global por-frame de `dormant_scheduled` nos fluidos e reativar trabalho por evento de chunk carregado;
2. depois otimizar o custo de seed/frontier/lighting no integration path;
3. manter este handoff atualizado a cada bloco antes de avançar.


## Checkpoint 120 — 2026-09-20: fluid dormant work reativado por residency event [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`process_fluid_updates()` chamava `reactivate_loaded_dormant()` em todo frame. Essa rotina:

- percorria todas as chaves de `dormant_scheduled`;
- consultava o `VoxelWorld` para descobrir quais chunks haviam voltado;
- alocava um `Vec`;
- ordenava os coords;
- só então reativava os ticks daquele chunk.

O custo crescia com a quantidade histórica de chunks com fluid work dormente, apesar de o streaming já conhecer exatamente o momento em que um chunk volta a ficar residente.

### Implementação

Commit funcional:

- `9b59188fa7e98b5cf093f5463f972036d0eaef63` — `perf: reactivate dormant fluid work on chunk load`.

Mudanças:

- `PendingFluidUpdates::reactivate_loaded_dormant(world, tick)` foi removido.
- Novo `reactivate_loaded_chunk(coord, current_tick)`:
  - faz lookup direto por coord;
  - remove apenas a lista daquele chunk;
  - reagenda seus ticks para o tick atual.
- `process_fluid_updates()` não faz mais scan global de chunks dormentes.
- O streaming passa `WorldTickClock.current_tick()` pelo integration path.
- `seed_loaded_chunk_lighting()`, que já é o ponto comum de primeira integração de um chunk residente durante Gameplay, reativa o fluid work daquele coord antes de semear a frontier.
- Bootstrap/Loading não ganhou polling nem reativação artificial:
  - a simulação de fluidos não roda nessa fase;
  - `dormant_scheduled` só surge quando um due tick é processado enquanto seu chunk não está residente em Gameplay.
- A regressão unitária interna foi atualizada para testar reativação explícita por residency event; ela NÃO foi executada.

### Arquitetura

`ARCHITECTURE.md` agora registra explicitamente:

- due fluid work dormente é indexado por chunk;
- reativação pertence à transição de residência daquele chunk;
- o loop por-frame da simulação não pode redescobrir chunks carregados via scan global do mapa dormente.

### Versionamento

- `VERSION`: **0.34.8 → 0.34.9**.
- `Cargo.toml` permanece `0.10.16`.

### Validação

Neste ponto:

- código aplicado em `develop`;
- CI do bloco ainda não verificada;
- não executei `cargo test`, `cargo run` nem QA Windows.

### Próximo passo

Próxima prioridade de performance:

1. reduzir o custo de `enqueue_loaded_fluid_frontier()` em chunks com grandes volumes de fluido;
2. revisar o custo de direct skylight seed no integration path;
3. só depois avançar para solver/remesh/streaming queue refinements.


### Correção de integração CI dos checkpoints 119–120

A primeira CI do commit de save final-only (`35510390475`) falhou somente no Clippy por duas sobras diretas da remoção do autosave:

- `Pause Menu` ainda recebia `ResMut<WorldSession>` embora `persist()` agora use apenas `&self`;
- `VoxelWorld::save_revision` / `save_content_revision()` ficaram sem consumidor depois que dirty-detection periódico foi removido.

Correção aplicada:

- `31f9cbe5569092aaca968904a3eff270776065da` — `fix: remove obsolete autosave revision state`.
- `Pause Menu` usa `Res<WorldSession>`.
- `save_revision`, `bump_save_revision()` e `save_content_revision()` foram removidos, sem supressão de warning.
- O bump permanece `VERSION 0.34.9` porque esta é correção de integração do mesmo bloco ainda não fechado/validado.
- Nova CI deste topo ainda precisa ser confirmada antes do próximo bloco funcional.


### Correção de integração adicional do checkpoint 120

A CI subsequente expôs `clippy::too_many_arguments` em `stream_chunks`: adicionar `WorldTickClock` diretamente ao system elevou a assinatura para 8 parâmetros.

Correção aplicada sem suppression:

- `2e9e4e29bd487141481b27a7b33d60efdfd941da` — `fix: keep streaming clock in runtime context`.
- `WorldTickClock` agora compõe `ChunkStreamingWork`, junto do estado/runtime usado pela integração de chunks.
- `stream_chunks` volta a 7 parâmetros e continua lendo um único `current_tick` por frame.
- `VERSION` permanece `0.34.9`; esta é continuação do mesmo bloco de performance ainda em validação.


### CI verde do bloco 0.34.9

- Push CI `35510690984`: **success** no commit funcional `2e9e4e29bd487141481b27a7b33d60efdfd941da`.
- Passou:
  - auditoria de localizações;
  - Clippy `--locked --all-targets --all-features -- -D warnings`;
  - `cargo check --locked`.
- PR CI equivalente ainda estava em execução no instante desta anotação, mas o pipeline de push validou o mesmo commit.
- Não executei `cargo test`, `cargo run` nem QA Windows.


## Checkpoint 121 — 2026-09-20: engineering best-practices canon adopted [DOC/PROCESS BLOCK]

### Pedido

O usuário forneceu um guia technology-agnostic de boas práticas cobrindo responsabilidade, ownership, boundaries, composição, modelagem de estado, async/concurrency, cache, determinismo, performance, testes, refactoring triggers e review checklist, e pediu que ele fosse aplicado ao trabalho em andamento.

### Aplicação

- Novo `ENGINEERING_PRACTICES.md` adapta o guia ao Asteria sem transformar `ARCHITECTURE.md` em um arquivo monolítico.
- `ARCHITECTURE.md` declara os dois documentos conjuntamente normativos:
  - `ARCHITECTURE.md` continua autoritativo para contratos específicos do Asteria;
  - `ENGINEERING_PRACTICES.md` define as práticas gerais reutilizáveis.
- Em conflito/aparente sobreposição, a regra específica do Asteria vence.
- O refactor/performance em andamento passa a revisar explicitamente:
  - single responsibility e owner único;
  - boundaries/dependency direction;
  - contexts estreitos;
  - change-driven work;
  - cache contracts;
  - bounded async/work budgets;
  - stale result protection;
  - deterministic ordering;
  - optimize-the-owner-of-the-cost;
  - regression/invariant tests;
  - refactoring triggers e code-review checklist.

### Versionamento

- `VERSION`: **0.34.9 → 0.34.10**.
- Nenhuma mudança de runtime neste checkpoint; o bump é patch por alteração normativa/documental do projeto.
- O próximo bloco funcional de performance deverá partir deste canon e atualizar novamente o handoff.


## Checkpoint 122 — 2026-09-20: chunk-owned fluid frontier source index [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Mesmo após remover o polling global de fluid work dormente, a entrada de qualquer chunk com fluido ainda percorria os 4096 voxels para reencontrar fontes que talvez tivessem algum target vazio. Em grandes volumes estáticos isso repetia trabalho derivado a cada residency transition.

### Implementação

- `VoxelChunk` agora possui um bitset derivado `fluid_frontier_sources`, COW via `Arc`.
- O bitset não representa a frontier autoritativa; ele só marca conservadoramente fontes que merecem inspeção:
  - fluido com algum target local down/horizontal vazio; ou
  - fluido numa boundary por onde um target cross-chunk pode existir.
- `set_block` e `set_fluid` atualizam apenas a célula afetada e as fontes cujos targets dependem dela.
- `edit_content` (worldgen/load/archive restore) usa a mesma mutation primitive e portanto constrói a metadata junto do conteúdo, sem um scan posterior.
- `enqueue_chunk_fluid_spread_targets()` deixou de varrer 16³ e agora visita apenas essas fontes potenciais.
- A decisão real de spread continua chamando o solver/sampling contra o `VoxelWorld` atual.
- O scan de uma face vizinha no seam permanece por enquanto; é no máximo 16² e será otimizado somente se profiling justificar.
- Foram adicionados testes de invariantes da metadata, mas não executados manualmente.

### Boas práticas aplicadas

- optimize the owner of the cost: metadata pertence ao `VoxelChunk`, dono do conteúdo;
- derived data close to source;
- cache contract explícito: key=voxel local, value=potential-source bit, invalidation=block/fluid mutation local, lifetime=chunk, bounded=4096 bits;
- não existe segunda fonte de verdade do solver;
- capability API `visit_potential_fluid_frontier_sources` não expõe o layout interno do bitset.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta o índice derivado e sua semântica conservadora.
- `VERSION`: **0.34.10 → 0.34.11**.
- CI ainda pendente neste instante; não executei `cargo test`, `cargo run` ou QA Windows.


### CI verde do checkpoint 122

- Push CI `35510994315`: **success**.
- PR CI `35510996944`: **success**.
- O bloco do índice derivado de frontier passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.11`.
- Não executei `cargo test`, `cargo run` nem QA Windows.


## Checkpoint 123 — 2026-09-20: revision-guarded vertical skylight transmission cache [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`seed_chunk_direct_lighting()` recalculava a transmissão de céu de um chunk novo percorrendo novamente todos os voxels de cada seção já residente acima dele. Em uma coluna vertical carregada incrementalmente, esse custo se repetia e crescia cumulativamente.

### Implementação

- `LightingContext` agora mantém `chunk_vertical_dampening`.
- Cada entrada contém:
  - key: `IVec3` do chunk;
  - guard: `chunk_content_revision`;
  - value: 256 valores (`16×16`) de dampening vertical agregado, um por coluna local.
- O primeiro seed que precisa de uma versão de chunk calcula no máximo os 4096 voxels daquela seção.
- Seeds posteriores aplicam apenas os 256 totais já calculados.
- Se bloco/fluido muda, `chunk_content_revision` muda; a entrada é recomposta lazily no próximo uso.
- `enqueue_chunk_unloads()` remove explicitamente as entradas dos chunks descarregados.
- `LightingContext::clear()` foi renomeado para `reset_query_scratch()` porque a operação não apaga o cache persistente; o nome agora descreve corretamente o side effect.
- Streaming deixou de chamar o helper de seed diretamente e usa a capability `PendingLightingUpdates::seed_chunk_direct_lighting()`, mantendo cache e seed sob o mesmo owner.
- O seed do próprio chunk continua reconstruindo luz/emissão normalmente; a otimização só elimina re-scan redundante das seções superiores.
- Adicionado teste de regressão para invalidation por `chunk_content_revision`; não executado manualmente.

### Práticas aplicadas

- optimize the owner of the cost;
- one authoritative owner;
- cache com key/value/validity/invalidation/lifetime/bound explícitos;
- stale derived data protegida por revision;
- API por capability, sem expor o `HashMap` interno;
- nome de método alinhado ao side effect real.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta o cache e seu contrato.
- `VERSION`: **0.34.11 → 0.34.12**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


## Checkpoint 124 — 2026-09-20: scan-miss cache cobre todas as remesh queues [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`ChunkRemeshQueue` já evitava repetir scans sem resultado em `fluid` e `immediate_geometry`, mas `geometry` e `lighting` ainda executavam `DeduplicatedQueue::pop_where()` linear novamente mesmo quando nem a fila nem o membership do render pool haviam mudado.

### Implementação

- Adicionados `geometry_scan_miss` e `lighting_scan_miss`.
- Geometry e lighting agora reutilizam o mesmo `pop_renderable_from()` já usado pelas outras filas.
- A validade continua sendo exatamente:
  - `queue.revision()`;
  - `render_pool.membership_revision()`.
- Qualquer enqueue/remove/promotion ou mudança no pool invalida naturalmente a miss anterior.
- Scheduling, coalescing, prioridade e round-robin entre Fluid/Lighting/Geometry não mudaram.
- Adicionados testes de invariantes equivalentes para geometry e lighting; não executados manualmente.

### Práticas aplicadas

- reuse invariants, not superficial similarity;
- specialized collection mechanics centralizadas;
- change-driven work;
- nenhuma nova abstraction/wrapper;
- nenhuma alteração semântica de rendering.

### Versionamento

- `VERSION`: **0.34.12 → 0.34.13**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI dos checkpoints 123–124

A CI do cache de skylight falhou por uma call site mecânica restante após renomear `LightingContext::clear()` para `reset_query_scratch()`:

- `src/voxel/lighting/propagation.rs` ainda chamava o nome antigo.
- Correção aplicada no topo atual, sem mudança de comportamento.
- `VERSION` permanece `0.34.13`, pois é correção de integração dos blocos 123–124 ainda não validados em conjunto.


### CI verde dos checkpoints 123–124

- Push CI `35511266089`: **success**.
- PR CI `35511268056`: **success**.
- O topo `7dbf4162ce595157737a1c5e1754a111b0132752` validou em conjunto:
  - cache revisionado de transmissão vertical de skylight;
  - scan-miss cache para geometry/fluid/immediate/lighting remesh queues;
  - correção do rename de `LightingContext::reset_query_scratch()`.
- Passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.13`.
- Não executei `cargo test`, `cargo run` nem QA Windows.


## Checkpoint 125 — 2026-09-20: aggregate fluid solver performance diagnostics [CÓDIGO APLICADO; CI PENDENTE]

### Motivo

A auditoria identificou o BFS downhill do solver como possível hotspot algorítmico, mas o novo `ENGINEERING_PRACTICES.md` exige medir antes de introduzir cache/rewrite mais complexo.

### Instrumentação

`FluidSolverScratch` agora acumula, sem allocations extras por chamada:

- desired-state evaluations;
- horizontal candidates considerados;
- downhill BFS searches;
- BFS nodes efetivamente processados.

`process_fluid_updates()` agrega esses counters em `Local<FluidPerformanceDiagnostics>` e emite no máximo um log a cada 10 segundos quando houve trabalho, incluindo:

- searches por desired evaluation;
- nodes por search;
- frames com trabalho;
- backlog topology/wake/scheduled;
- quantidade de dormant chunks;
- estado de catch-up no instante do relatório.

A instrumentação:

- não é Resource/autoritativa;
- não altera queue order, due ticks, budgets ou solver result;
- não faz logging por voxel;
- usa wall-clock somente para frequência observacional do relatório;
- reseta apenas os counters após publicar a amostra.

### Próxima decisão

Usar os números observados em gameplay para escolher entre:

1. cache revisionado de preferred directions, caso haja muita repetição com mundo estável;
2. solver source-centric, caso searches por desired evaluation permaneçam altas sob mutação;
3. nenhuma mudança algorítmica, caso o BFS não seja custo relevante.

### Arquitetura / versionamento

- `ARCHITECTURE.md` registra que fluid performance metrics são observacionais e nunca participam de correctness.
- `VERSION`: **0.34.13 → 0.34.14**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 125

A primeira CI da instrumentação falhou por `private_interfaces`: `process_fluid_updates` é `pub(super)` e expõe `Local<FluidPerformanceDiagnostics>` na assinatura, enquanto o tipo era privado ao módulo.

Correção:
- `FluidPerformanceDiagnostics` passou a `pub(super)`, exatamente a visibilidade necessária ao system;
- nenhuma field/mutation API interna foi exposta;
- nenhuma suppression de lint foi adicionada;
- `VERSION` permanece `0.34.14`.


### CI verde do checkpoint 125

- Push CI `35511440972`: **success** no topo `05f29c69a7a9af2cf6055a451f0a7c19bab2cb08`.
- Passou localization audit, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked`.
- A instrumentação agregada do solver está compilando/validando sem participar do estado autoritativo.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 126 — 2026-09-20: WorldSaveContext decomposto por responsabilidade [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`WorldSaveContext` concentrava 18 dependências de três naturezas distintas:

- estado autoritativo necessário para construir o snapshot;
- queries/runtime state de player e creatures;
- registries usados para validação/serialization.

Isso contrariava a regra de contextos estreitos e escondia as fronteiras reais de save.

### Implementação

`WorldSaveContext` agora compõe três `SystemParam`s privados e coesos:

1. `WorldSnapshotState`
   - seed;
   - current dimension;
   - game rules;
   - new/load metadata;
   - day/night clock;
   - inventory;
   - voxel world;
   - pending fluid updates;
   - world tick clock.

2. `WorldSaveEntities`
   - player query;
   - creature query;
   - pending creature restores.
   - também concentra `saved_player()` e `saved_creatures()`, mantendo entity snapshot logic perto do owner.

3. `WorldSaveRegistries`
   - block/fluid/tool/creature/dimension/day-night registries;
   - expõe somente `for_validation()`, que constrói o `SaveRegistries` exigido pela boundary de persistence.

Outros ajustes:

- player query agora expressa a identidade via filters `With<GameplayCamera> + With<PlayerId>` em vez de carregar `PlayerId` no tuple e ignorar o valor;
- `WorldSession::persist` pede os registries pela capability do contexto, sem conhecer seus fields internos;
- formato de `WorldSnapshot`, validação, publicação atômica e save-only-on-exit não mudaram.

### Práticas aplicadas

- single responsibility;
- narrow abstractions;
- composition over monolithic objects;
- explicit boundaries;
- refactor when parameters/dependencies grow;
- nenhuma generic/everything context.

### Arquitetura / versionamento

- `ARCHITECTURE.md` passa a citar `WorldSaveContext` como exemplo canônico de composição de contexts.
- `VERSION`: **0.34.14 → 0.34.15**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 126

A primeira CI do refactor de save falhou somente em `clippy::type_complexity` na assinatura inline da player query.

Correção:
- as queries de snapshot ganharam aliases semânticos `SavedPlayerQuery` e `SavedCreatureQuery`;
- `WorldSaveEntities` agora lê como um contexto de capabilities, não como um bloco de tipos aninhados;
- nenhum `allow(clippy::type_complexity)` foi adicionado;
- comportamento e filtros permanecem idênticos;
- `VERSION` permanece `0.34.15`.


### CI verde do checkpoint 126

- Push CI `35511577443`: **success**.
- PR CI `35511579777`: **success**.
- O topo `829d9173db4b85b9324c9a3b4d6984bc8c8310ef` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.15`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 127 — 2026-09-20: exact terrain-material interning [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`TerrainMaterials::from_registry()` criava um novo `TerrainMaterial` para cada face/layer de cada bloco, mesmo quando vários faces/blocos possuíam exatamente o mesmo estado de material. Isso aumentava `Assets<TerrainMaterial>` e o número de material/bind-group states que o sync global de iluminação precisa tocar.

### Implementação

A construção de `TerrainMaterials` agora possui um interner local, válido apenas durante `from_registry()`.

Chave exata:

- texture path (ou ausência de textura);
- `dyable` / tint-enabled;
- alpha semantics:
  - Opaque;
  - Mask com cutoff exato em bits;
  - Blend;
- layer index, que também define o depth bias.

`roughness` e `metallic` não entram na chave porque são parâmetros únicos da própria chamada `from_registry()`; portanto todo item do interner já compartilha esses valores por construção.

Quando a chave coincide:

- reutiliza o mesmo `Handle<TerrainMaterial>`;
- não cria novo material asset;
- não recarrega a mesma textura para aquele material;
- não altera a lista face→layers observada pelos meshes.

Fluid materials continuam independentes porque possuem base color/opacity/double-sided semantics diferentes.

### Decisão sobre sky_light_factor

Não movi `sky_light_factor` para uniform global neste bloco. Fazer isso corretamente exige alterar a boundary de render/view bind groups; será tratado somente com profiling/evidência suficiente, não como micro-otimização especulativa.

### Arquitetura / práticas

- `ARCHITECTURE.md` documenta exact material interning.
- reuse only a real invariant: materiais só compartilham quando o estado renderizado é idêntico;
- public/render mapping permanece igual;
- nenhuma cache runtime ou invalidation policy adicional foi criada.

### Versionamento

- `VERSION`: **0.34.15 → 0.34.16**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 127

A primeira CI do material interning falhou em `clippy::too_many_arguments` no helper `create_material`.

Correção aplicada sem suppression:

- criado `TerrainMaterialBuilder`, contexto privado e coeso da operação de construção;
- ele possui somente:
  - `AssetServer`;
  - `Assets<TerrainMaterial>`;
  - interner local;
  - roughness;
  - metallic;
- `layers_for()` e `material_for()` agora operam nesse contexto;
- os helpers de 7/8 parâmetros foram removidos;
- a chave de interning e a semântica dos materiais permanecem idênticas;
- `VERSION` permanece `0.34.16`.


### CI verde do checkpoint 127

- Push CI `35511769135`: **success**.
- PR CI `35511772132`: **success**.
- O topo `b834d64921b0b5524776d8a70bcb6e05a6a342ee` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.16`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 128 — 2026-09-20: save-catalog locking encapsulado [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`save_catalog.rs` era owner simultâneo de schema, catálogo, publicação, recovery/pruning e também dos detalhes de concorrência: mutex de escrita, reader count, condvar, session file lock e prune flag. Os atomics/mutexes eram manipulados diretamente por vários caminhos do arquivo.

### Implementação

Novo `src/world/save_catalog/locking.rs` é o owner único do invariant de concorrência da persistência:

- `WorldDirectoryLock` — lease do lock de sessão no filesystem;
- `WorldGate` — gate in-process por world id;
- `ReadLease` — reader pin liberado/notificado por RAII;
- `PruneLease` — prune-running flag liberado por RAII;
- `lock_write()`;
- `pin_read()`;
- `try_begin_prune()`;
- `lock_after_readers()`;
- `world_lock()`;
- `acquire_world_directory_lock()`.

`save_catalog.rs` agora usa essas capabilities e não toca diretamente em fields/atomics/condvar do gate.

### Partial failure

`PruneLease` substitui o antigo `ResetPruneFlag` local e também cobre falha de `thread::Builder::spawn`: se a closure capturada for descartada porque o spawn falhou, o lease é derrubado e o flag volta a false.

### Semântica preservada

- save/delete/list continuam serializados pelo per-world write gate;
- snapshot candidates continuam pinando readers;
- prune só entra na fase destrutiva quando todos os readers terminaram e mantém o write gate durante remoção;
- o lock de sessão entre processos mantém a mesma política;
- schema, formato, publication e recovery não mudaram.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta `save_catalog::locking` como owner desse invariant.
- `VERSION`: **0.34.16 → 0.34.17**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 128

A primeira CI do módulo de locking falhou porque os dois rollbacks de `create_new_world()` ainda removiam `session.lock` pelo nome literal.

Correção:

- `save_catalog::locking` agora expõe `remove_world_directory_lock_file(directory)`;
- o filename `SESSION_LOCK_FILE` permanece completamente privado ao owner de locking;
- a capability trata `NotFound` como cleanup já concluído e propaga outros erros;
- `save_catalog.rs` deixou de conhecer o nome físico do lock file;
- `VERSION` permanece `0.34.17`.


### CI verde do checkpoint 128

- Push CI `35512218627`: **success**.
- PR CI `35512221277`: **success**.
- O topo `acaf4b63cd8accec11cf26bcd3c2218beed522d5` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- O owner `save_catalog::locking` agora encapsula também o cleanup físico do session lock file; `save_catalog.rs` não conhece mais `SESSION_LOCK_FILE`.
- `VERSION` permanece `0.34.17`.
- Não executei `cargo test`, `cargo run` nem QA Windows.


## Checkpoint 129 — 2026-09-20: streaming predicate scan-miss cache [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Duas filas de streaming ainda repetiam scans lineares em frames onde nenhuma entrada elegível existia e nenhum input relevante havia mudado:

- `pending` quando generation tasks estão saturadas e só chunks críticos podem furar a capacidade;
- `retired` quando todos os candidatos ainda estão dentro do retention radius ou voltaram para `desired/retained`.

`ready` foi revisada e deliberadamente NÃO recebeu cache: um `pop_ready()` com fila não vazia termina consumindo uma entrada e portanto já muda a revision; não há miss linear estável para amortizar.

### Implementação

`ChunkStreamingState` agora mantém:

- `pending_critical_scan_miss: Option<CriticalPendingScanKey>`;
- `retired_scan_miss: Option<RetiredScanKey>`;
- `selection_revision`.

Critical pending key:

- `pending.revision()`;
- current streaming center.

Retired key:

- `retired.revision()`;
- `selection_revision`;
- current x/z center;
- squared retention radius.

`selection_revision` avança somente quando `rebuild_queue()` publica uma nova seleção `desired/retained`, tornando explícita a invalidation de elegibilidade que não pertence à própria retired queue.

### Semântica

- ordering e prioridade das filas não mudaram;
- nenhum coord é removido/reordenado por causa do cache;
- qualquer enqueue/pop/remove/promotion altera a queue revision e força novo scan;
- mover o centro invalida critical pending;
- rebuild de seleção ou mudança de retention radius invalida retired;
- cache só representa "nenhum item satisfazia este predicado com estes inputs".

### Testes adicionados

Foram adicionados testes de invariantes para:

- critical pending miss permanecer estável até queue/center mudar;
- retired miss permanecer estável até selection revision mudar.

Os testes NÃO foram executados manualmente, conforme regra do projeto.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta que predicate scan caching deve incluir todos os inputs de elegibilidade.
- `VERSION`: **0.34.17 → 0.34.18**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 129

- Push CI `35512515832`: **success**.
- PR CI `35512517643`: **success**.
- O topo `91ea8b8e349145caa9b54fcf3d9f43ec686e030e` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.18`.
- Não executei `cargo test`, `cargo run` nem QA Windows.


## Checkpoint 130 — 2026-09-20: save snapshot + validation owners extracted [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Após o locking ser extraído, `save_catalog.rs` ainda misturava:

- serialized world/player/snapshot/manifest shapes;
- snapshot capture validation;
- content-registry-backed playable validation;
- filesystem catalog/publication/load/recovery/prune orchestration.

O arquivo continuava sendo alterado por responsabilidades independentes.

### Implementação

Novo `save_catalog/snapshot.rs` possui:

- `WorldManifest`;
- `SavedPlayer`;
- `WorldSnapshot`;
- `SnapshotSource`;
- `SAVE_FORMAT_VERSION`;
- defaults serde ligados ao snapshot;
- `WorldSnapshot::capture()` e suas invariantes intrínsecas.

Novo `save_catalog/validation.rs` possui:

- `SaveRegistries`;
- `PruneRegistries`;
- validação de clock/biome-size/player/inventory;
- validação de fluid updates;
- validação de creature definitions;
- construção do registry snapshot owned usado pelo background prune.

`save_catalog.rs` agora:

- reexporta somente os tipos que os consumidores externos precisam;
- mantém filesystem catalog;
- atomic publication;
- generation manifests/candidates;
- load/fallback recovery;
- backup pruning orchestration.

### Semântica preservada

- `SAVE_FORMAT_VERSION` continua **1**;
- nomes/campos serde não mudaram;
- defaults legacy não mudaram;
- capture continua serializando o mesmo conjunto de persistent chunks;
- validation rules não mudaram;
- fallback entre gerações não mudou;
- save continua somente no lifecycle de saída.

### Práticas aplicadas

- split by responsibility, not file size;
- explicit persistence boundaries;
- serialization outside orchestration;
- validation near the owner of the invariant;
- small public surface via reexports;
- nenhuma generic utils layer.

### Versionamento

- `VERSION`: **0.34.18 → 0.34.19**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 130

A primeira CI do split falhou porque `WorldSummary`, que é read-model do catálogo, foi removido junto do bloco antigo de schema.

Correção:

- `WorldSummary` foi restaurado em `save_catalog.rs`;
- ele permanece com o owner correto: listagem/catalog read model, não serialized snapshot;
- nenhum campo/formato persistido mudou;
- `VERSION` permanece `0.34.19`.


### CI verde do checkpoint 130

- Push CI `35512759083`: **success**.
- PR CI `35512761340`: **success**.
- O topo `58667508125fe07d7535f29de59c959cfc77ee80` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.19`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 131 — 2026-09-20: persistence naming alinhado ao ownership real [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Duas APIs de `VoxelWorld` usavam “generated” para conceitos diferentes do contrato atual:

- `save_generated_chunks()` serializava exclusivamente `persistent_chunks`;
- `has_generated_chunk()` retornava true para qualquer chunk residente OU archived persistent, inclusive chunks runtime/persistidos e não apenas worldgen.

Os nomes escondiam o owner real e incentivavam raciocínio incorreto sobre derived worldgen vs authoritative persistence.

### Renomes

- `save_generated_chunks()` → `save_persistent_chunks()`.
- `has_generated_chunk()` → `has_resident_or_persisted_chunk()`.

Call sites atualizados em:

- `VoxelWorld::insert_chunk()`;
- streaming generation result integration;
- restore-before-generation path;
- snapshot capture;
- testes internos de archive/persistence.

Assertions/comments também foram ajustados para falar em resident/persisted, não “generated”.

### Semântica preservada

- untouched deterministic worldgen continua descartável ao sair da retenção;
- persistent mutation continua promovendo o chunk a authoritative saved state;
- archived persistent chunks continuam restauráveis;
- snapshot continua serializando apenas `persistent_chunks`;
- nenhum formato de save mudou.

### Arquitetura / versionamento

- `ARCHITECTURE.md` agora fixa a nomenclatura de persistence.
- `VERSION`: **0.34.19 → 0.34.20**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 131

A primeira CI do rename encontrou dois call sites adicionais que a busca de código do connector não havia indexado:

- `world/setup/bootstrap.rs`;
- `world/setup/progress.rs`.

Ambos agora usam `has_resident_or_persisted_chunk()`. Nenhuma semântica mudou e `VERSION` permanece `0.34.20`.


### CI verde do checkpoint 131

- Push CI `35512944231`: **success**.
- PR CI `35512948102`: **success**.
- O topo `bb7eda356efa3d9cde62b43f7e649a49ad1aa103` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.20`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 132 — 2026-09-20: archived persistent chunks serializam sem restore/repack [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`VoxelWorld::save_persistent_chunks()` tratava chunk persistent já arquivado assim:

1. `ArchivedChunk::restore()`;
2. construção de um `VoxelChunk` completo;
3. alocação/rebuild de storage/metadata derivada;
4. `DiskChunk::from_chunk()` fazia novo scan de 4096 voxels;
5. o runtime chunk temporário era descartado.

Isso fazia trabalho e alocação que não participa do formato durável.

### Implementação

`ArchivedChunk` agora expõe capabilities read-only:

- `block_entries()`;
- `fluid_entries()`.

Elas iteram apenas bits ocupados em ordem crescente e reconstroem somente o `VoxelCell`/`FluidCell` necessário ao consumidor. O layout interno de occupancy/palette continua encapsulado.

`DiskChunk` ganhou um `DiskChunkBuilder` privado que centraliza:

- block-state normalization/properties sorting;
- portable block palette + runs;
- runtime fluid ID → authored fluid ID;
- portable fluid palette + runs.

Tanto `from_chunk()` quanto o novo `from_archived_chunk()` usam o mesmo builder, portanto não existem dois encoders de disk semantics.

`save_persistent_chunks()` agora chama `DiskChunk::from_archived_chunk()` para chunks archived.

### Ganho estrutural

Para um archived persistent chunk o save deixa de:

- alocar light array/runtime chunk;
- reconstruir block/fluid counts;
- reconstruir frontier metadata;
- executar setters sobre todo conteúdo;
- fazer um segundo scan 16³.

O archive é lido diretamente e `DiskChunk` continua dono exclusivo da representação portable.

### Regressão adicionada

Foi adicionado teste que compara JSON de `DiskChunk::from_chunk()` e `DiskChunk::from_archived_chunk()` para o mesmo conteúdo. O teste NÃO foi executado manualmente; ele será compilado pela CI `--all-targets`.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta o direct archive → disk path.
- `VERSION`: **0.34.20 → 0.34.21**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 132

- Push CI `35513064739`: **success**.
- PR CI `35513066521`: **success**.
- O topo `93d01004f03974a825549c045f82a1290cd03337` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.21`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 133 — 2026-09-20: serialized chunk catalog boundary [CÓDIGO APLICADO; CI PENDENTE]

### Objetivo

Criar o primeiro boundary real para a roadmap de chunk storage sem violar a decisão atual de produto:

- save continua **somente ao sair**;
- formato version 1 continua compatível;
- nenhum chunk é escrito durante Gameplay;
- nenhuma mudança de unload/load ownership ainda.

### Implementação

Novo `save_catalog/chunks.rs` define `SavedChunkCatalog`.

Responsabilidades:

- capturar authoritative persistent chunks de `VoxelWorld`;
- encapsular a coleção de `DiskChunk`;
- reconstruir um `VoxelWorld` a partir dessa coleção no load.

`SavedChunkCatalog` usa `#[serde(transparent)]`, então o campo de snapshot continua serializando exatamente como array JSON:

`"chunks": [ ... ]`

`WorldSnapshot` agora possui `chunks: SavedChunkCatalog` em vez de `Vec<DiskChunk>` e não chama mais diretamente `VoxelWorld::save_persistent_chunks()`.

No load, `save_catalog.rs` retira o catálogo do snapshot e pede `into_world()`, em vez de conhecer `Vec<DiskChunk>`.

### Identidade de chunk em disco

`DiskChunk.coord` deixou de ser field `pub(crate)`.

Novo `DiskChunk::coord()`:

- é a capability única de leitura da identidade espacial do disk chunk;
- valida Y não negativo;
- é reutilizado por `into_chunk()`;
- é reutilizado pelo restore de `VoxelWorld` para duplicate-coordinate detection.

Isso impede consumidores de reinterpretarem diretamente o array `[i32; 3]`.

### Compatibilidade

- `SAVE_FORMAT_VERSION` continua 1;
- nenhuma key JSON mudou;
- `chunks` continua sendo array;
- legacy per-voxel `DiskChunk` continua legível;
- fallback/recovery continua igual;
- nenhuma escrita adicional foi introduzida.

Foi adicionado teste de shape transparente (`SavedChunkCatalog::default()` serializa como `[]`), mas ele NÃO foi executado manualmente.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta `SavedChunkCatalog` como boundary para evolução futura de storage e reafirma que isso não permite gameplay-time disk writes.
- `VERSION`: **0.34.21 → 0.34.22**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 133

A primeira CI revelou um consumidor pré-existente relevante: `world/chunk_storage.rs` já possui `ChunkDiskIdentity` e publication física por generation directory.

O encapsulamento de `DiskChunk.coord` expôs que `ChunkDiskIdentity::from_disk_chunk()` ainda reinterpretava o array diretamente.

Correção:

- `ChunkDiskIdentity::from_disk_chunk()` agora retorna `io::Result<Self>`;
- usa exclusivamente `DiskChunk::coord()`;
- negative-Y/identity validation fica centralizada no `DiskChunk`;
- generation chunk publication propaga o erro;
- fixtures/testes internos foram atualizados para exigir identidade válida;
- `VERSION` permanece `0.34.22`.

Também fica registrado que `chunk_storage.rs` já existe como primeira peça de per-generation chunk storage; próximos slices devem integrá-lo ao catálogo existente, não criar um segundo mecanismo paralelo.


### CI verde do checkpoint 133

- Push CI `35513357982`: **success**.
- PR CI `35513360503`: **success**.
- O topo `627773621e61d6a8e725f2559ef9e470aaf2baf1` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.22`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 134 — 2026-09-20: world-selection view extraída para layout owner [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`world_selection.rs` misturava no mesmo módulo:

- worker lifecycle de scan/load/cancel;
- ativação do mundo carregado e mutação de resources;
- input/actions de Load/Delete/Back;
- construção completa da UI;
- marker components puramente visuais;
- formatting de timestamp para apresentação.

Isso fazia mudanças de layout tocarem o mesmo arquivo que controla thread/cancelamento/persistence activation.

### Implementação

Novo `screens/world_selection/layout.rs` possui exclusivamente a camada de view:

- `WorldListEntry`;
- `SelectionError`;
- `WorldListStatus`;
- `WorldListContainer`;
- `spawn_world_entry()`;
- `spawn_world_selection()`;
- `format_save_time()`.

O módulo principal `world_selection.rs` continua owner de:

- `WorldSelectionState`;
- `WorldSelectionAction`;
- scan worker;
- load worker;
- cancellation/disposal;
- delete/load orchestration;
- accepted-world activation;
- feedback synchronization.

### Boundaries

- detalhes de `ScrollArea`, scrollbar, cosmic background, screen/surface/theme/typography e button styling saíram do módulo de orchestration;
- o layout recebe somente state/localization/domain read-model necessário para renderizar;
- marker components são `pub(super)` apenas porque o parent precisa consultá-los para atualizar/despawn;
- `WorldSelectionAction` continua no parent porque representa intent/control flow, não apresentação;
- não foi criado helper genérico nem design-system paralelo.

### Semântica preservada

- ordem OnEnter/Update/OnExit não mudou;
- Back continua precedendo worker result;
- scan/load/delete behavior não mudou;
- UI tree, labels, spacing e timestamp format não mudaram;
- nenhum save/load format mudou.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta a boundary view vs async/domain orchestration para screens complexas.
- `VERSION`: **0.34.22 → 0.34.23**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 134

- Push CI `35513586406`: **success**.
- PR CI `35513588753`: **success**.
- O topo `75504f23a66cba225f79ac7de7d4818fd6ab0dd7` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.23`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 135 — 2026-09-20: world-selection worker lifecycle extraído [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Depois da extração de layout, `world_selection.rs` ainda era owner direto de toda mecânica low-level dos workers:

- `Arc<Mutex<...>>`;
- slot de result;
- spawn de scan;
- spawn de load;
- catch de panic;
- owned registry copies;
- polling do mutex;
- abandoned flag;
- disposal off-thread de world load descartado.

Essas invariantes formam um lifecycle coeso e não pertencem aos systems que decidem intents/transitions.

### Implementação

Novo `screens/world_selection/tasks.rs` possui:

- `PendingWorldScan`;
- `PendingWorldLoad`;
- `WorldLoadCompletion`;
- slots/result guards privados;
- captura owned de registries para load;
- spawn/panic containment/logging dos workers;
- polling não bloqueante;
- abandonment + stale-result disposal.

`WorldSelectionScanContent` permanece no parent como `SystemParam` read-only e agora expõe apenas `registries()`. Isso evita esconder Bevy resource dependencies dentro do task owner.

Os systems no parent continuam owner de decisões:

- `refresh_world_list()` decide quando iniciar scan;
- `poll_world_scan()` decide como aplicar o resultado à UI/state;
- `handle_world_selection()` decide quando iniciar load;
- `poll_world_load()` decide se o resultado pode ser aceito e aplica authoritative resources;
- Back/OnExit decidem quando abandonar um load.

### Semântica preservada

- scan/load continuam em detached OS threads;
- copies dos content registries continuam no main thread antes do worker;
- worker panic continua virando `io::Error`;
- polling continua non-blocking via `try_lock`;
- Back continua tendo precedência;
- resultado abandonado continua descartado fora do input frame;
- nenhuma resource de Bevy escapa para thread;
- nenhuma semântica de save/load mudou.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta `tasks` como owner de lifecycle mecânico, mantendo systems como owner das decisões.
- `VERSION`: **0.34.23 → 0.34.24**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção mecânica do checkpoint 135

Na revisão imediata pós-commit, antes de considerar o bloco validado, foi encontrado um call site transformado incorretamente por replace mecânico:

- `let id = id().to_owned();` → `let id = pending.id().to_owned();`.

A correção restaura apenas o acesso ao ID do `PendingWorldLoad`; nenhuma semântica de task/load mudou e `VERSION` permanece `0.34.24`.


### Correção de integração CI do checkpoint 135

A primeira CI do worker split falhou por uma única vírgula ausente em `WorldSelectionState`:

- `scan: Option<PendingWorldScan>` não terminava com vírgula antes de `loading`.

Isso gerou uma cascata de erros de parsing/derive (`Resource`, fields inexistentes etc.), sem indicar falha arquitetural do split.

Correção aplicada:

- vírgula restaurada;
- nenhum comportamento alterado;
- `VERSION` permanece `0.34.24`.


### Segunda correção de integração do checkpoint 135

Após corrigir a sintaxe, Clippy encontrou `large_enum_variant` em `WorldLoadCompletion`: a variante que carregava o world result era muito maior que `Abandoned`.

Em vez de suppression ou heap indirection artificial:

- `WorldLoadCompletion` virou struct com `abandoned: bool` e `result: Option<...>`;
- isso espelha diretamente o estado observado do slot concluído;
- o parent continua decidindo se aceita/ignora o resultado;
- nenhuma allocation adicional foi introduzida;
- `VERSION` permanece `0.34.24`.


### Terceira correção de integração do checkpoint 135

A CI seguinte encontrou apenas um import obsoleto após a troca do completion enum por struct:

- removido `WorldLoadCompletion` do import do parent;
- nenhum comportamento alterado;
- `VERSION` permanece `0.34.24`.


### CI verde do checkpoint 135

- Push CI `35516875608`: **success**.
- PR CI `35516877188`: **success**.
- O topo `a71961697173692f98d0e2dea2f8fb969b0d5a9e` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.24`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 136 — 2026-09-20: ready streaming priority em single scan [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`ChunkStreamingState::pop_ready()` podia percorrer a mesma ready queue até duas vezes antes de consumir um item:

1. `pop_where(critical)`;
2. se não houvesse critical, `pop_where(forward)`;
3. se também não houvesse forward, FIFO `pop()`.

Como cada pop bem-sucedido muda a queue revision, scan-miss cache não amortiza esse caminho. Com backlog grande de mesh-ready chunks, um frame podia pagar dois scans lineares para cada integração.

### Implementação

`DeduplicatedQueue<T>` ganhou `pop_min_by_key()`:

- percorre as entradas ativas uma única vez;
- ignora records stale de priority promotions;
- escolhe a menor key;
- preserva a primeira entrada/FIFO entre keys iguais;
- usa o mesmo path central de remoção/revision da fila.

`pop_where()` e `pop_min_by_key()` compartilham agora `remove_active_index()`, eliminando duplicação da mutation primitive.

`ChunkStreamingState::pop_ready()` usa ranks:

- 0 = critical;
- 1 = forward relativo ao movement direction;
- 2 = fallback/background.

O resultado observável é o mesmo ordering anterior, mas com no máximo um scan da ready queue por item integrado.

### Testes de invariantes adicionados

Sem execução manual:

- `pop_min_by_key()` preserva FIFO dentro do melhor rank;
- records stale de `enqueue_front` não participam do ranking;
- streaming continua consumindo critical → forward → background.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta ranked single-scan como generic queue mechanic.
- `VERSION`: **0.34.24 → 0.34.25**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 136

- Push CI `35517017404`: **success**.
- PR CI `35517019499`: **success**.
- O topo `3645cc71c208e654e8de5a63c64e38f3fb3461a0` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.25`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 137 — 2026-09-20: sky-light factor em shared GPU buffer [CÓDIGO APLICADO; CI/RUNTIME SHADER PENDENTES]

### Problema

`sync_sky_light_factor()` tratava `sky_light_factor` — estado global da cena — como field de cada `TerrainMaterialExtension`.

Sempre que o ciclo dia/noite mudava:

- iterava todos os `Assets<TerrainMaterial>`;
- marcava cada material como mutado;
- reescrevia o mesmo float em todos os assets;
- o custo crescia com a quantidade de materiais/content packs.

Isso permanecia mesmo após exact material interning.

### Implementação

Novo `TerrainLightingBuffer` em `rendering::terrain_material`:

- é um Resource com um único `Handle<ShaderBuffer>`;
- o buffer contém um `vec4<f32>`, usando `.x` para `sky_light_factor`;
- terrain e fluid material extensions clonam o MESMO handle;
- `set_sky_light_factor()` atualiza somente esse asset via API oficial do Bevy.

`TerrainMaterialExtension` agora separa:

- binding 100: shared read-only storage buffer global;
- binding 101: uniform material-local com `fluid_animation_factor` e `tint_enabled`.

O WGSL usa `terrain_global_lighting[0].x` no mesmo ponto onde antes lia `terrain_material_extension.sky_light_factor`.

### Lifecycle

- o buffer é criado uma vez em `begin_world_loading()`;
- o mesmo handle é passado para `TerrainMaterials` e `FluidMaterials`;
- `lighting.rs` mantém o existing change gate e atualiza apenas o shared buffer;
- `release_world_session()` remove o Resource junto dos material registries.

### Semântica pretendida

O cálculo visual de skylight não mudou:

`pow(sky_level, SKY_LIGHT_GAMMA) * sky_light_factor`

Mudou somente o transporte do fator CPU → GPU.

### Performance

Antes: O(número de TerrainMaterial assets) mutations por alteração de iluminação global.

Agora: uma mutation de `ShaderBuffer` por alteração.

### Validação

- `VERSION`: **0.34.25 → 0.34.26**.
- CI Rust pendente.
- O shader WGSL é compilado em runtime pelo renderer; Clippy/cargo check não validam o bind-group WGSL end-to-end.
- Não executei `cargo test`, `cargo run` nem QA Windows; runtime shader validation continuará explicitamente pendente mesmo se CI Rust ficar verde.


### CI verde do checkpoint 137

- Push CI `35517590973`: **success**.
- PR CI `35517593395`: **success**.
- O topo `ec335843b46dd685678585d65965bcd0ec4548b7` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.26`.
- Runtime shader/bind-group validation permanece pendente porque não houve `cargo run`/QA de renderer.

## Checkpoint 138 — 2026-09-20: physical save storage extraído do catálogo [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`save_catalog.rs` ainda misturava policy/orchestration de catálogo com detalhes físicos de arquivo: filename protocol, generation parsing/scanning, JSON IO, size limit, fsync e atomic rename.

### Implementação

Novo `save_catalog/storage.rs` é owner do mecanismo físico:

- canonical manifest/snapshot naming;
- generation parsing e manifest directory scan;
- highest generation lookup;
- snapshot regular-file + 512 MiB validation;
- bounded JSON serialization;
- temp file → flush → fsync → atomic rename → cleanup;
- JSON reads.

O parent continua owner de world identity, locks, generation selection, recovery ordering, playable validation e retention/prune policy.

O parser genérico de prefix permanece privado; o parent recebe apenas a capability estreita `snapshot_generation()`.

### Semântica

- save format, filenames, size limit, fsync, fallback e retention não mudaram;
- save continua final-only;
- `invalid_data()` permanece no parent porque snapshot/validation também usam esse boundary error.

### Versionamento

- `VERSION`: **0.34.26 → 0.34.27**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 138

A primeira CI do split de storage falhou porque `now_unix_ms()` foi removido junto do antigo tail de helpers, embora pertença à policy do catálogo e ainda seja usado em create/save manifest.

Correção:

- `now_unix_ms()` restaurado em `save_catalog.rs`;
- `SystemTime/UNIX_EPOCH` voltam a ter owner correto no parent;
- nenhum detalhe físico de JSON/publication voltou ao parent;
- `VERSION` permanece `0.34.27`.


### CI verde do checkpoint 138

- Push CI `35517955553`: **success**.
- PR CI `35517958828`: **success**.
- O topo `20837617bf2e9b99895b1df23bf96a7f1e1ff438` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.27`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 139 — 2026-09-20: streaming pipeline dividido por lifecycle stage [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`streaming.rs` ainda possuía, no mesmo módulo:

- state/queue policy;
- selection orchestration;
- generation task dispatch + result integration;
- initial mesh task dispatch + result integration;
- render-neighbor reconciliation.

Essas partes mudam por motivos diferentes, embora a ordem do pipeline deva continuar explícita.

### Implementação

Novo `streaming/generation.rs` possui:

- generation dispatch budget;
- mesh-backlog generation cap;
- generation-result integration budget;
- `collect_generated_chunks()`;
- `dispatch_generation_tasks()`.

Novo `streaming/meshing.rs` possui:

- initial mesh dispatch budget;
- mesh-result integration budget;
- `dispatch_initial_mesh_tasks()`;
- empty-chunk render integration;
- `collect_built_chunk_meshes()`;
- loaded-neighbor geometry/fluid reconciliation.

`streaming.rs` continua owner de:

- `ChunkStreamingState`;
- desired/retained/pending/ready/retired policy;
- predicate scan caches;
- player criticality;
- selection rebuild;
- `stream_chunks()` e sua stage order;
- `seed_loaded_chunk_lighting()`, porque é um invariant compartilhado por generation integration e mesh dispatch.

### Semântica preservada

A ordem continua exatamente:

1. rebuild selection quando necessário;
2. sync generation/mesh snapshots;
3. collect generation results;
4. collect mesh results;
5. dispatch initial mesh;
6. dispatch generation.

Todos os budgets, maximum-items, preemption rules e task limits foram mantidos nos mesmos valores.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta ownership por pipeline stage mantendo orchestrator explícito.
- `VERSION`: **0.34.27 → 0.34.28**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 139

- Push CI `35518106747`: **success**.
- PR CI `35518108350`: **success**.
- O topo `46d8bd14c15d6c0c1573d9866e561e0441b79f39` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.28`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 140 — 2026-09-20: remesh queue semantics extraídas [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`chunk_remesh.rs` misturava dois owners:

- semântica das quatro filas de remesh;
- execution/scheduling de rebuilds.

A fila possui invariantes próprias: dedup, priority promotion, geometry→lighting coalescing, fluid independence, round-robin background fairness e renderability scan caching.

### Implementação

Novo `chunk_remesh/queue.rs` possui:

- `ChunkRemeshQueue`;
- geometry/fluid/immediate-geometry/lighting queues;
- enqueue/remove/coalescing rules;
- task-kind requeue;
- background-kind fairness;
- render-pool filtering;
- queue+pool revision scan-miss caches;
- testes de invariantes da fila.

`chunk_remesh.rs` agora possui apenas:

- immediate geometry execution;
- task snapshot sync;
- completed-task integration;
- async remesh dispatch;
- per-frame budgets.

O parent reexporta `ChunkRemeshQueue` com a mesma API crate-level.

### Semântica preservada

- quatro caminhos permanecem separados;
- lighting coalesce terrain geometry sem consumir fluid work;
- immediate geometry não consome fluid;
- fairness continua round-robin entre fluid/lighting/geometry;
- scan-miss invalidation continua por queue revision + render-pool membership revision;
- budgets/task limits não mudaram.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta queue semantics vs execution policy.
- `VERSION`: **0.34.28 → 0.34.29**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 140

- Push CI `35518264802`: **success**.
- PR CI `35518267116`: **success**.
- O topo `93a632567dc099a32453276ed388af7b472e67ae` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.29`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 141 — 2026-09-20: fluid scheduler state encapsulado [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`fluid_updates.rs` misturava o estado autoritativo do scheduler com processing/diagnostics. Diagnostics e topology processing liam diretamente collections internas como `topology_queue`, `wake_queue`, `scheduled_due` e `dormant_scheduled`.

### Implementação

Novo `fluid_updates/state.rs` possui:

- `FluidTickKey`;
- fair wake queue por chunk;
- equal-due scheduled bucket round-robin por chunk;
- `PendingFluidUpdates`;
- dormant unloaded ticks;
- `SavedFluidUpdates` + serialization/restore;
- saved fluid ID/position validation;
- catch-up threshold;
- testes de queue/scheduling invariants.

O parent reexporta `PendingFluidUpdates` e `SavedFluidUpdates`, preservando todos os paths externos.

### Encapsulamento

Processing usa apenas capabilities:

- `pop_topology()`;
- `pop_wake()`;
- `pop_due()`;
- `schedule_at()`;
- `schedule_neighborhood()`;
- `defer_unloaded()`;
- `should_catch_up()`.

Diagnostics usa `PendingFluidBacklog`, read-model com topology/wake/scheduled/dormant counts. Nenhum consumer lê collections internas.

### Semântica preservada

- scheduled ticks continuam keyed por fluid + voxel;
- earliest due wins;
- equal-due fairness continua por chunk;
- frontier wake fairness continua por chunk;
- unloaded due ticks continuam dormant até residency;
- serialization ordering/dedup não mudou;
- solver e fluid transition rules não foram alterados.

### Arquitetura / versionamento

- `ARCHITECTURE.md` documenta scheduler state como owner único e backlog como read-model.
- `VERSION`: **0.34.29 → 0.34.30**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 141

- Push CI `35518456659`: **success**.
- PR CI `35518458949`: **success**.
- O topo `689e82379265f9c4bf5e0079e3561f1657de52a1` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.30`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 142 — 2026-09-20: fluid diagnostics separados da simulação [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Depois do scheduler state split, `fluid_updates.rs` ainda possuía timer, accumulation e logging de performance no mesmo módulo que decide/aplica fluid transitions.

Observabilidade não é estado de negócio e não deve aumentar as dependências da simulação.

### Implementação

Novo `fluid_updates/diagnostics.rs` possui:

- `FluidPerformanceDiagnostics`;
- intervalo de 10s;
- accumulation de `FluidSolverMetrics`;
- derived `searches_per_desired` / `nodes_per_search`;
- active-frame count;
- structured diagnostic log;
- consumo read-only de `PendingFluidBacklog`.

`fluid_updates.rs` apenas entrega:

- frame delta;
- metrics retiradas de `FluidSolverScratch`;
- scheduler read-model;
- catch-up state.

### Semântica

- diagnostics continuam sem participar da simulação;
- cadence/log text e métricas não mudaram;
- fluid budgets, scheduling, solver, lighting e remesh não mudaram.

### Arquitetura / versionamento

- `ARCHITECTURE.md` fixa diagnostics como observabilidade separada do authoritative state.
- `VERSION`: **0.34.30 → 0.34.31**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 142

A primeira CI do diagnostics split falhou por `private_interfaces`: `process_fluid_updates` é visível em `crate::world`, enquanto o tipo usado no `Local<FluidPerformanceDiagnostics>` estava restrito ao parent imediato `fluid_updates`.

Correção:

- `FluidPerformanceDiagnostics` passou a `pub(in crate::world)`;
- fields e mutation API continuam privados ao módulo de diagnostics;
- nenhum lint suppression foi adicionado;
- `VERSION` permanece `0.34.31`.


### CI verde do checkpoint 142

- Push CI `35518688223`: **success**.
- PR CI `35518691317`: **success**.
- O topo `41d8f53fc5e0ab34c38cdd80b0be74a89b675e98` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.31`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 143 — 2026-09-20: streaming tie-break determinístico [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`rebuild_queue()` enumerava `scratch.desired: HashSet<IVec3>` e usava esse `ordinal` como último critério de ordenação.

Para chunks com `PendingPriority` idêntica, a ordem final dependia da iteração do HashSet. Isso tornava a sequência de streaming incidental/não determinística entre execuções.

### Implementação

- removido `PendingEntry.ordinal`;
- critérios semânticos de `PendingPriority` permanecem idênticos;
- empate final agora usa `(coord.y, coord.z, coord.x)`;
- sort continua em uma única `sort_unstable_by_key`;
- nenhuma allocation/cache nova foi adicionada.

### Regressão

O teste de empate agora prova ordenação por coordenada estável em vez de preservar uma ordem de origem proveniente do HashSet.

### Arquitetura / versionamento

- `ARCHITECTURE.md` proíbe HashSet iteration como tie-break de streaming.
- `VERSION`: **0.34.31 → 0.34.32**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 143

- Push CI `35518845006`: **success**.
- PR CI `35518847380`: **success**.
- O topo `a79e9bacfb475119d4d1c2b4021355f2b5302f35` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.32`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 144 — 2026-09-20: validated generation-chunk read path [CÓDIGO APLICADO; CI PENDENTE]

### Objetivo

Preparar a migração real para chunk storage por geração sem fazer manifest/snapshot depender de um writer que ainda não possuía reader simétrico.

### Implementação

`chunk_storage.rs` agora possui `load_generation_chunks(world_directory, generation)`.

A leitura valida a estrutura canônica:

`generation-N/chunks/<x>/<y>/<z>.chunk.json`

Regras:

- generation slot precisa ser diretório real;
- `chunks` ausente significa generation sem persistent chunks;
- níveis x/y precisam ser diretórios reais;
- leaf z precisa ser arquivo real;
- componentes x/y/z precisam ser `i32` em representação decimal canônica;
- Y negativo é rejeitado;
- symlink/arquivo/diretório em nível inesperado é rejeitado;
- `DiskChunk::coord()` precisa coincidir exatamente com o path;
- resultado é ordenado deterministicamente por `(y,z,x)`.

### Testes de invariantes adicionados/estendidos

Sem execução manual:

- publish existente agora também faz generation read round-trip;
- empty generation round-trip;
- payload coord divergente do path é rejeitado.

### Importante

Este checkpoint NÃO muda o save format e NÃO conecta manifests ao chunk directory ainda. Ele completa a storage boundary necessária para que o próximo migration slice possa fazer isso sem introduzir write-only persistence.

### Arquitetura / versionamento

- `ARCHITECTURE.md` exige reader/writer simétricos antes da migração.
- `VERSION`: **0.34.32 → 0.34.33**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 144

- Push CI `35519022642`: **success**.
- PR CI `35519024483`: **success**.
- O topo `134eae71f11d8c3f5b3be4f323209b13e2883deb` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.34.33`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Próximo migration slice de persistence — executar como um protocolo único

A próxima mudança de formato NÃO deve ser parcialmente aplicada. O reader/writer de generation chunks agora existe e está verde; o próximo slice deve migrar a publicação para chunks externos mantendo leitura de saves antigos.

Contrato planejado:

1. **Format v2 para novos saves**
   - bump do save-format interno de 1 para 2;
   - `WorldSnapshot` runtime continua sendo o snapshot autoritativo capturado no main thread;
   - criar disk DTO v2 metadata-only para serialização, em vez de clonar/mutar `WorldSnapshot`;
   - snapshot v2 NÃO serializa a coleção de chunks inline.

2. **Compatibilidade de leitura v1**
   - manifests/snapshots format 1 continuam legíveis;
   - v1 continua usando `chunks: [...]` inline;
   - um mundo v1 carregado normalmente será salvo como v2 na próxima saída;
   - reservation manifest generation 0 antigo não pode impedir essa migração só por ser format 1.

3. **Publication v2, ainda final-only**
   Ordem de commit:
   - publicar `generation-N/` via `publish_generation_chunks()`;
   - publicar snapshot metadata-only `snapshot-N.json`;
   - publicar `manifest-N.json` POR ÚLTIMO como commit marker.
   Nenhuma dessas escritas acontece durante Gameplay.

4. **Rollback**
   - falha antes do manifest deve remover qualquer snapshot/chunk generation não publicado;
   - manifest jamais pode apontar para payload parcial;
   - orphan não referenciado pode ser limpo, mas nunca tratado como authoritative.

5. **Load v2**
   - validar metadata do snapshot primeiro;
   - carregar chunks pela generation indicada pelo próprio manifest generation;
   - `load_generation_chunks()` valida path ↔ payload identity;
   - converter pelo existing `SavedChunkCatalog`/`VoxelWorld::from_saved_chunks`.

6. **Recovery/listing**
   - `valid_manifest()` deve aceitar v1 e v2;
   - generation v2 só é complete quando snapshot + generation chunk directory existem;
   - fallback continua newest → older verified generation.

7. **Prune**
   - remover manifest/snapshot antigos e a generation chunk directory correspondente sob o mesmo write gate;
   - v1 sem chunk directory continua válido/no-op nesse cleanup.

8. **Uma fonte autoritativa**
   - não escrever chunks inline E externos em v2;
   - não manter dois catálogos mutáveis;
   - manifest continua sendo o único commit marker de uma generation.

9. **Validação**
   - CI Rust após cada commit coerente;
   - runtime/save-load QA continua explicitamente pendente se não houver `cargo run`;
   - não rodar `cargo test` sem autorização.

### Outros itens ainda dependentes de runtime evidence

- Fluid solver BFS: diagnostics de downhill searches/nodes já estão instrumentados; não alterar algoritmo/cache sem capturar números reais.
- Shared `TerrainLightingBuffer`: Rust CI verde no checkpoint 137, mas WGSL/bind-group runtime ainda precisa de execução do renderer.
- CSM/shadow budget: manter como profiling/A-B item; não reduzir qualidade por análise estática.


## Checkpoint 145 — 2026-09-20: save format v2 com chunks externos por generation [CÓDIGO APLICADO; CI PENDENTE]

### Objetivo

Executar como um protocolo único a migração preparada nos checkpoints anteriores:

- novos saves deixam de duplicar chunks dentro de `snapshot-N.json`;
- persistent chunks passam a usar o `chunk_storage` já validado;
- saves v1 continuam legíveis;
- save continua exclusivamente no leave/exit lifecycle.

### Formato

`SAVE_FORMAT_VERSION`: **1 → 2**.

Format v1 (legacy read-only):

- `manifest-N.json` com `format_version: 1`;
- `snapshot-N.json` contém `chunks: [...]` inline;
- não requer `generation-N/`.

Format v2 (current writer):

- `generation-N/chunks/<x>/<y>/<z>.chunk.json` contém os persistent `DiskChunk`;
- `snapshot-N.json` contém apenas metadata/runtime state, sem field `chunks`;
- `manifest-N.json` com `format_version: 2` é publicado por último e continua sendo o único commit marker.

### DTOs e uma única fonte autoritativa

`WorldSnapshot` continua sendo o snapshot autoritativo em memória e ainda possui `SavedChunkCatalog`.

Novo boundary em `save_catalog::snapshot`:

- `StoredWorldSnapshot` — reader compatível v1/v2;
- `WorldSnapshotV2<'_>` — writer metadata-only para v2;
- `is_supported_save_format()`;
- `LEGACY_SAVE_FORMAT_VERSION = 1`;
- `SAVE_FORMAT_VERSION = 2`.

Regras:

- v1 exige inline chunks;
- v2 rejeita inline chunks;
- snapshot/manifest precisam declarar a mesma format version;
- após load válido, o runtime `WorldSnapshot` é promovido para format atual 2;
- não existem dois catálogos mutáveis para a mesma generation.

`SavedChunkCatalog` ganhou capabilities estreitas:

- `disk_chunks()` para publication;
- `from_disk_chunks()` para restore externo.

### Publication v2

`save_world_owned()` agora publica na ordem:

1. `publish_generation_chunks(directory, N, ...)`;
2. `snapshot-N.json` via `snapshot.disk_v2()`;
3. `manifest-N.json` por último.

A busca do próximo generation ID também considera published/staging generation-directory slots, além de snapshot/manifest files.

### Rollback

Se chunk publication falhar:

- best-effort cleanup do generation slot.

Se snapshot publication falhar:

- remove snapshot parcial/temp pelo existing storage mechanism;
- remove `generation-N/`.

Se manifest publication falhar:

- remove snapshot ainda não committed;
- remove `generation-N/`.

Nenhum manifest é publicado apontando para payload conhecido como parcial.

### Compatibilidade / load

Reservation manifest generation 0 pode ser v1 ou v2; ele não bloqueia migração desde que world identity/worldgen metadata sejam válidos.

`valid_manifest()` aceita v1 e v2.

Load:

1. abre/limita snapshot file;
2. desserializa `StoredWorldSnapshot`;
3. exige snapshot format == manifest format;
4. valida world/player metadata;
5. converte para runtime snapshot e executa registry/playable validation;
6. v1 usa inline `SavedChunkCatalog`;
7. v2 chama `load_generation_chunks(directory, manifest.generation)`;
8. chunks viram `VoxelWorld` pelo existing owner.

Assim metadata inválida é rejeitada antes de ler a árvore externa de chunks.

### Listing / recovery

- newest→older fallback continua igual;
- v1 generation é complete com snapshot publicado;
- v2 generation só é complete quando snapshot + `generation-N/` existem;
- generation v2 danificada/missing cai para backup anterior;
- path↔payload chunk identity continua validado por `chunk_storage`.

### Prune

Sob o mesmo reader-drain/write gate, generations abaixo do cutoff removem:

- manifest;
- snapshot;
- `generation-N/` se existir.

Para v1 a remoção do diretório é no-op.

### Storage boundary

`save_catalog::storage` ganhou `read_json_file()`, removendo do policy parent o último `serde_json::from_reader` direto para snapshots.

`chunk_storage` ganhou:

- `generation_storage_slot_exists()`;
- `generation_chunks_published()`.

Essas capabilities mantêm directory/path policy no owner correto.

### Validação / versionamento

- aplicação: **0.34.33 → 0.35.0** por ser uma evolução backward-compatible de formato/persistence;
- save format interno: **1 → 2**;
- CI pendente;
- testes de regressão foram adicionados/ajustados, mas NÃO executei `cargo test`;
- NÃO executei `cargo run` nem QA Windows;
- runtime save→exit→load e renderer shader validation continuam explicitamente pendentes sem evidência de execução.


### Correção de integração CI do checkpoint 145

A primeira CI do format v2 falhou porque `world::chunk_storage` ainda estava declarado com `#[cfg(test)]`.

Isso era correto enquanto os checkpoints 133/144 apenas preparavam e validavam reader/writer sem conectar manifests ao storage. Com o format v2, `save_catalog` passa a depender desse owner em runtime.

Correção:

- removido `#[cfg(test)]` de `mod chunk_storage`;
- nenhuma API/public surface adicional foi exposta;
- o módulo continua privado a `world`;
- protocolo v2 e `VERSION 0.35.0` permanecem inalterados.


### CI verde do checkpoint 145

- Push CI `35521746295`: **success**.
- PR CI `35521748186`: **success**.
- O topo `916fe3543059d242508f7824db8d26cece5162bb` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- Save format v2 está integrado em runtime; `chunk_storage` deixou de ser test-only.
- `VERSION` permanece `0.35.0`.
- Não executei `cargo test`, `cargo run` nem QA Windows.
- Runtime save→exit→load de v2 e migração v1→v2 continuam pendentes sem evidência de execução.

## Checkpoint 146 — 2026-09-20: new-world reservation sem side effect antes do clock [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`create_new_world()` criava o world directory e adquiria o session lock antes de chamar `now_unix_ms()`.

Se o system clock falhasse nesse ponto, o operador `?` retornava sem executar o rollback explícito do diretório recém-reservado, deixando uma world directory incompleta.

### Correção

- `created_at = now_unix_ms()?` agora é resolvido antes de `create_dir_all(WORLDS_DIRECTORY)` e antes de qualquer world-directory reservation;
- o reservation manifest usa esse valor já validado;
- todos os fallible paths após a criação do world directory continuam nos rollback branches existentes.

### Semântica

- world timestamp continua sendo capturado no momento de criação;
- naming/identity/locking não mudaram;
- save format v2 não mudou;
- nenhuma escrita durante Gameplay foi introduzida.

### Versionamento

- `VERSION`: **0.35.0 → 0.35.1**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 146

- Push CI `35521850495`: **success**.
- PR CI `35521852808`: **success**.
- O topo `8b24c067c6da7cb27dc37b529dae370c2eca05c9` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.1`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 147 — 2026-09-20: runtime snapshot não é mais serializable [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Após a migração v2, `WorldSnapshot` ainda derivava `Serialize`/`Deserialize`.

Mesmo que o save path correto já usasse `WorldSnapshotV2`, isso deixava uma capability perigosa disponível: um future caller poderia serializar o runtime snapshot diretamente e reintroduzir `chunks: [...]` em format v2.

### Correção

- removidos `Serialize` e `Deserialize` de `WorldSnapshot`;
- runtime snapshot permanece `Clone + Debug`;
- `StoredWorldSnapshot` é o único DTO de leitura;
- `WorldSnapshotV2<'_>` é o único DTO de escrita atual;
- `WorldManifest` e `SavedPlayer` continuam serde porque são formatos de boundary reais.

### Invariant

A regra “v2 nunca serializa chunks inline” deixa de depender apenas do call site correto: o tipo runtime não possui mais a capability de serialização.

### Versionamento

- `VERSION`: **0.35.1 → 0.35.2**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 147

A primeira CI do hardening encontrou atributos `#[serde(...)]` ainda presentes nos fields de `WorldSnapshot` depois que o tipo deixou de derivar serde.

Correção:

- removidos os atributos serde apenas do runtime `WorldSnapshot`;
- defaults/compatibilidade permanecem em `StoredWorldSnapshot`, que continua sendo o único reader de disco;
- nenhum formato persistido mudou;
- `VERSION` permanece `0.35.2`.


### Segunda correção de integração CI do checkpoint 147

A CI seguinte encontrou o último atributo serde remanescente no runtime snapshot:

- removido `#[serde(default = "default_saved_biome_size_multiplier")]` de `WorldSnapshot::biome_size_multiplier`;
- todos os `#[serde(...)]` do runtime snapshot agora foram removidos;
- `StoredWorldSnapshot` mantém os defaults de compatibilidade;
- `VERSION` permanece `0.35.2`.


### CI verde do checkpoint 147

- Push CI `35522171662`: **success**.
- PR CI `35522173732`: **success**.
- O topo `27fb6918560631dff1bdc1af6846ec3289d8bb50` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.2`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 148 — 2026-09-20: JSON publication sincroniza directory commit [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`save_catalog::storage::publish_json()` sincronizava o conteúdo do arquivo temporário e fazia atomic rename, mas não executava `fsync` no diretório pai depois do rename.

Para snapshot/manifest de save — especialmente `manifest-N.json`, que é o commit marker do format v2 — isso deixava uma janela de crash/power-loss em que o conteúdo do arquivo estava durável mas a entrada de diretório recém-renomeada podia não estar.

`chunk_storage` já sincronizava o world directory após publicar uma generation; JSON publication precisava oferecer a mesma garantia.

### Implementação

Após:

1. serialize + flush;
2. `file.sync_all()`;
3. atomic `rename(temp, final)`;

`publish_json()` agora chama `sync_directory(directory)`.

Se esse post-rename fsync falhar:

- remove o arquivo final recém-visível;
- sincroniza o diretório novamente para tornar o rollback durável;
- retorna o erro original de publication;
- se o rollback também falhar, retorna erro combinado com ambos os failures.

O caller continua vendo a mesma capability: `Ok` significa published/durable; `Err` significa não tratar o arquivo como committed.

### Semântica

- JSON shape/save format não mudou;
- publication ordering v2 não mudou;
- nenhuma escrita em Gameplay;
- custo adicional ocorre somente em create/save final, onde durability é o objetivo.

### Versionamento

- `VERSION`: **0.35.2 → 0.35.3**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 148

- Push CI `35522284424`: **success**.
- PR CI `35522286937`: **success**.
- O topo `975f3d81544b185d15ad3465bea67c1e49ee127d` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.3`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 149 — 2026-09-20: chunk storage serializa direto para arquivo [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`write_generation_chunks_to_staging()` fazia, para cada persistent chunk:

1. `serde_json::to_vec(chunk)`;
2. alocava um buffer do tamanho completo do JSON;
3. abria o arquivo;
4. copiava o buffer inteiro para o arquivo;
5. `sync_all()`.

No format v2 isso adicionava uma allocation temporária proporcional ao payload de cada chunk durante o save final.

### Implementação

Cada `DiskChunk` agora é serializado diretamente para `io::BufWriter<&mut File>` via `serde_json::to_writer()`.

Sequência:

1. abre arquivo com `create_new`;
2. serialize diretamente no writer;
3. `flush()`;
4. `file.sync_all()`.

Não há `Vec<u8>` intermediária.

### Semântica

- JSON produzido permanece o mesmo shape;
- canonical path/identity validation não mudou;
- per-file fsync continua igual;
- generation directory fsync/rename publication continua igual;
- save continua final-only.

### Versionamento

- `VERSION`: **0.35.3 → 0.35.4**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 149

- Push CI `35522392558`: **success**.
- PR CI `35522395734`: **success**.
- O topo `7d59e79460f1f5d425758a169e0b78b633387c24` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.4`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 150 — 2026-09-20: generation recovery/retention extraído do save catalog facade [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Após o format v2, `save_catalog.rs` voltou a concentrar duas responsabilidades:

- facade/publication de world catalog;
- lifecycle de generations publicadas: candidate discovery, fallback load, decode v1/v2, completeness e retention/prune.

Esses blocos mudam por motivos diferentes e carregam dependências distintas.

### Implementação

Novo `save_catalog/generations.rs` possui:

- `RETAINED_GENERATIONS`;
- candidate enumeration + read lease;
- newest restorable timestamp;
- `load_world()`;
- snapshot open/decode;
- v1 inline vs v2 external chunk selection;
- manifest validity/completeness;
- latest complete generation;
- retention cutoff e prune de manifest/snapshot/chunk generation.

`save_catalog.rs` continua owner de:

- create world reservation;
- current-format save publication;
- rollback de unpublished generation;
- prune worker scheduling;
- delete;
- list/list_verified facade;
- wall-clock boundary;
- public reexports.

`load_world()` é reexportado pelo facade, portanto os callers externos não conhecem o submódulo novo.

### Semântica preservada

- v1/v2 compatibility não mudou;
- fallback newest→older não mudou;
- reader lease/write gate não mudou;
- retention count continua 4 restorable generations;
- save publication/rollback não mudou;
- save continua final-only.

### Versionamento

- `VERSION`: **0.35.4 → 0.35.5**.
- CI pendente; não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 150

- Push CI `35522626564`: **success**.
- PR CI `35522628973`: **success**.
- O topo `7f01abf8fe814782756a2b9ba12651f78023db81` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.5`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 151 — 2026-09-20: bootstrap lighting relaxation agora é realmente budgeted [CÓDIGO APLICADO; CI PENDENTE]

### Problema

A fase `WorldLoadingPhase::Lighting` aparentava respeitar `INITIAL_LOADING_BUDGET = 12 ms`, mas o budget era externo a `initialize_chunks_lighting()`.

Cada item fazia:

1. limpar/enfileirar 2 chunks;
2. chamar `relax()`;
3. `relax()` processava a fila inteira até convergir;
4. só depois o loading consultava novamente o frame budget.

Logo uma única iteração podia atravessar o budget por uma quantidade arbitrária de propagação. Reduzir 2 chunks para 1 diminuiria o pico, mas não corrigiria o owner do custo.

### Implementação

`PendingLightingUpdates` ganhou a capability estreita `enqueue_initial_chunk_lighting(world, coord)`:

- limpa a luz do chunk;
- enfileira todos os voxels;
- enfileira os neighbors de boundary;
- usa a mesma `LightingQueue`/contexto autoritativo da iluminação dinâmica.

Novo `WorldSetupSimulation` agrupa somente estado de simulação do bootstrap:

- `PendingFluidUpdates`;
- `PendingLightingUpdates`;
- scratch local de changed lighting chunks.

Isso substitui o parâmetro isolado de fluids e evita aumentar o broad system signature.

### Novo fluxo de lighting no loading

Para cada chunk, em ordem:

1. se a fila está vazia, enfileira o próximo chunk;
2. chama `process_pending_lighting()`;
3. o callback registra voxels processados no `FrameWorkBudget`;
4. `relax_budgeted()` pode devolver controle durante a propagação;
5. se a fila ainda contém trabalho, o mesmo chunk continua no frame seguinte;
6. só incrementa `loading_state.lit` quando aquela fila converge;
7. só entra em Meshing quando todos os chunks convergiram e a fila está vazia.

A granularidade de budget agora existe **dentro** da propagação, com checks do próprio solver de lighting, em vez de ao redor de uma operação ilimitada.

### Lifecycle

- `PendingLightingUpdates` é resetado também em `OnEnter(GameState::Loading)`, antes do bootstrap;
- a mesma resource continua sendo usada depois no gameplay;
- changed-chunk scratch do loading é descartado porque ainda não existem meshes a invalidar;
- não foi criada uma segunda fila/cache de iluminação.

### Semântica

- algoritmo de lighting/desired light não mudou;
- ordem de chunks do bootstrap não mudou;
- meshing só começa após convergência da iluminação;
- final lighting continua vindo do mesmo `relax_budgeted()` usado pelo runtime;
- nenhum comportamento de save/worldgen/fluid foi alterado.

### Versionamento

- `VERSION`: **0.35.5 → 0.35.6**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 151

A primeira CI do bootstrap-lighting budget falhou porque três helpers de generation em `setup/progress.rs` continuam recebendo `&mut PendingFluidUpdates` diretamente, enquanto o import desse tipo foi removido ao introduzir `WorldSetupSimulation`.

Correção:

- restaurado somente o import de `PendingFluidUpdates`;
- `WorldSetupSimulation` continua owner do agrupamento de resources no system principal;
- os helpers mantêm dependências estreitas em vez de receber o context inteiro;
- nenhum comportamento de lighting/fluid/loading mudou;
- `VERSION` permanece `0.35.6`.


### Segunda correção de integração CI do checkpoint 151

Depois de restaurar o import de fluids, Clippy encontrou `initialize_chunks_lighting()` sem consumidores runtime.

Isso é consequência esperada da nova path budgeted: o bootstrap não deve mais possuir uma API síncrona que relaxa toda a fila até convergir.

Correção:

- removida `initialize_chunks_lighting()` da API runtime;
- o helper test-only `initialize_chunk_lighting()` monta sua própria fila mínima para preservar os testes existentes;
- runtime passa a ter uma única path de bootstrap lighting: `enqueue_initial_chunk_lighting()` + `process_pending_lighting()`;
- nenhum lint suppression foi adicionado;
- `VERSION` permanece `0.35.6`.


### Terceira correção de integração CI do checkpoint 151

Após remover a runtime API síncrona, `relax` permaneceu importado no build normal embora só seja usado pelo helper `#[cfg(test)]`.

Correção:

- `relax_budgeted` permanece no import runtime;
- `relax` agora é importado apenas sob `#[cfg(test)]`;
- runtime continua sem path de lighting inicial não-budgeted;
- `VERSION` permanece `0.35.6`.


### Quarta correção de integração CI do checkpoint 151

Com a runtime API antiga removida, `propagation::relax()` também ficou exclusivamente test-only.

Correção:

- `relax()` agora é compilado somente sob `#[cfg(test)]`;
- `relax_budgeted()` é a única propagation entrypoint no build runtime;
- testes existentes continuam podendo usar a wrapper síncrona;
- nenhuma lógica de iluminação mudou;
- `VERSION` permanece `0.35.6`.


### CI verde do checkpoint 151

- Push CI `35526614181`: **success**.
- PR CI `35526617306`: **success**.
- O topo `84f80d7e7371e0aa3a8323a15828b8a2f8f17755` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.6`.
- Runtime `relax_budgeted()` é agora a única entrypoint de propagation no build normal.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 152 — 2026-09-20: bootstrap phases separados por lifecycle stage [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`setup/progress.rs` já modelava corretamente `WorldLoadingPhase`, mas também possuía a implementação completa de todas as fases:

- generation dispatch/integration;
- lighting propagation;
- mesh dispatch/integration;
- player spawn/transition.

A state machine e a mecânica de cada stage mudam por motivos diferentes.

### Implementação

`setup/progress.rs` agora possui somente:

- `setup_world()`;
- dispatch explícito por `WorldLoadingPhase`;
- `INITIAL_LOADING_BUDGET`.

Novos owners:

- `setup/progress/generation.rs`: generation task sync/collect/dispatch + frontier seed;
- `setup/progress/lighting.rs`: bootstrap lighting queue + propagation budget;
- `setup/progress/meshing.rs`: mesh task sync/collect/dispatch + render integration;
- `setup/progress/spawning.rs`: spawn-position policy, player restore/new spawn e Gameplay transition.

### Boundaries

- `setup_world()` continua mostrando explicitamente `Generating → Lighting → Meshing → Spawning`;
- nenhuma event bus/callback layer foi criada;
- submódulos recebem apenas dependencies necessárias;
- `WorldSetupSimulation` continua agrupando apenas simulation resources no system Bevy;
- helpers de generation continuam recebendo diretamente `PendingFluidUpdates`, não o context inteiro.

### Semântica preservada

- mesmo budget de 12 ms;
- mesma ordem de fases;
- mesmos task limits;
- mesma stale-task reschedule policy;
- mesma regra de lighting convergence antes de meshing;
- mesma spawn/load behavior;
- nenhum save/worldgen/fluid rule mudou.

### Versionamento

- `VERSION`: **0.35.6 → 0.35.7**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 152

- Push CI `35526772483`: **success**.
- PR CI `35526774395`: **success**.
- O topo `cf68195effcb2d69fe3ff3ca3c43de37c6bbd193` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.7`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 153 — 2026-09-20: inventory layout dividido por view ownership [CÓDIGO APLICADO; CI PENDENTE]

### Problema

`hud/inventory/layout.rs` possuía ~879 linhas e quatro responsabilidades de view distintas:

- composition root do HUD;
- creative catalog/search/categories;
- player backpack/hotbar/trash;
- rendering de item/cursor.

Essas áreas mudam por motivos diferentes e já tinham consumers externos estreitos em `sync.rs`.

### Implementação

`inventory/layout.rs` agora possui somente:

- `InventoryItemView`;
- `InventoryLayoutState`;
- `spawn_inventory_root()`;
- reexports internos das capabilities consumidas por `sync.rs`.

Novos owners:

- `layout/creative.rs`: creative panel, search field, category list/buttons, catalog filtering/sorting/grid/slots;
- `layout/player.rs`: player inventory panel, backpack, hotbar, trash button e inventory slots;
- `layout/item.rs`: cursor icon e item rendering compartilhado.

### API interna preservada

`sync.rs` continua importando exatamente de `layout`:

- `spawn_inventory_root`;
- `spawn_cursor_icon`;
- `spawn_creative_catalog_rows`;
- `spawn_inventory_item`.

Os submódulos novos não vazam para o restante do HUD.

### Semântica preservada

- UI tree, sizing, spacing e markers não mudaram;
- creative filtering/sorting não mudou;
- item tint/icon behavior não mudou;
- hotbar/backpack/trash behavior não mudou;
- nenhuma interaction/sync/style policy foi movida para layout.

### Versionamento

- `VERSION`: **0.35.7 → 0.35.8**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 153

A primeira CI do inventory-layout split falhou porque três helpers reexportados pelo parent `layout` continuaram `pub(super)` dentro dos submódulos aninhados.

Nesse nível, `pub(super)` permite acesso apenas ao próprio `layout`, mas `sync.rs` é sibling de `layout` dentro de `inventory`.

Correção de visibilidade mínima:

- `spawn_creative_catalog_rows`;
- `spawn_cursor_icon`;
- `spawn_inventory_item`;

agora são `pub(in crate::hud::inventory)`.

O parent continua reexportando as mesmas capabilities para `sync.rs`; nada foi aberto em `pub(crate)` nem para fora do owner `inventory`.

Nenhum comportamento/layout mudou e `VERSION` permanece `0.35.8`.


### CI verde do checkpoint 153

- Push CI `35530280801`: **success**.
- PR CI `35530283148`: **success**.
- O topo `68c737ea3736e0b0913afff3289c5fb44c5bb8b9` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.8`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 154 — 2026-09-20: world-selection activation preparada antes do commit [CÓDIGO APLICADO; CI PENDENTE]

### Objetivo

Aplicar explicitamente os critérios de responsabilidade única, limite de dependências e performance ao último bloco grande de `world_selection`.

### Problema

`poll_world_load()` ainda misturava:

- polling/acceptance do worker;
- rebuild de scheduled fluid state;
- validação/restauração de inventory;
- construção do in-memory save;
- montagem de session resources;
- commit de resources Bevy;
- UI error routing e screen transition.

Além disso, `PlayerHotbar::restore_items_and_selection()` alocava um `Vec` temporário de 36 slots antes de copiar para arrays fixos.

### Implementação

Novo `screens/world_selection/activation.rs` possui a responsabilidade única de transformar um world carregado validado em `PreparedWorldActivation`.

`prepare()` recebe somente:

- loaded snapshot/world/session lock;
- `BlockRegistry`;
- `FluidRegistry`;
- `ToolRegistry`.

Ele NÃO recebe o `WorldSelectionScanContent` inteiro.

Todo passo fallible acontece antes de tocar em resources live:

- rebuild de `PendingFluidUpdates`;
- resolução/validação de inventory IDs.

Depois disso, o aggregate prepara:

- restored `PlayerHotbar`;
- novo `InMemoryWorldSave`;
- pending creature restores;
- seed/dimension/rules/world/session/lock.

`commit()` é não-fallible e recebe apenas:

- `Commands`;
- live `PlayerHotbar`;
- live `InMemoryWorldSave`.

O parent continua owner de worker acceptance, localization/error presentation e transition para Loading.

### Limite de params / ownership

- nenhum novo mega-`SystemParam`;
- `WorldSelectionLoadContext` continua agrupando o contexto coerente já existente: seis registries read-only + inventory/save mutáveis (~8 dependencies significativas);
- activation recebe apenas três definition registries realmente usados;
- tasks/layout continuam separados.

### PlayerHotbar

Novo `PlayerHotbar::from_saved_items_and_selection()` constrói o aggregate restaurado diretamente nos arrays fixos.

`restore_items_and_selection()` agora delega para esse constructor e substitui `self` somente após sucesso.

Ganhos:

- restauração fica atômica;
- removida allocation temporária de `Vec<Option<&'static str>>`;
- o owner do invariant continua sendo `PlayerHotbar`, não o screen.

### Semântica preservada

- mensagens de load vs inventory error continuam distintas;
- fluid saved ticks continuam resolvidos por authored IDs;
- session resources e Loading transition continuam iguais;
- worker thread/lifecycle não mudou;
- nenhum save format/worldgen/simulation rule mudou.

### Versionamento

- `VERSION`: **0.35.8 → 0.35.9**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### Correção de integração CI do checkpoint 154

A primeira CI encontrou `PlayerHotbar::restore_items_and_selection()` sem consumidores após a mudança para prepare/commit.

Correção alinhada à regra de public surface mínima:

- removida a API mutating antiga;
- `PlayerHotbar::from_saved_items_and_selection()` passa a ser a única capability de restore persistido;
- nenhum lint suppression foi adicionado;
- restauração continua atômica e sem `Vec` temporário;
- `VERSION` permanece `0.35.9`.


### CI verde do checkpoint 154

- Push CI `35530587637`: **success**.
- PR CI `35530590065`: **success**.
- O topo `51c5b890dc2759e78f1cbf67dadee05d81295491` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.9`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 155 — 2026-09-20: streaming selection remove trabalho loop-invariant [CÓDIGO APLICADO; CI PENDENTE]

### Hot path

`rebuild_desired_chunk_coords()` percorre uma área horizontal O(radius²) quando a seleção de streaming é reconstruída.

Dentro desse loop, `inside_forward_preload()` normalizava o MESMO `movement_direction` para cada coordenada candidata, executando conversão + normalização/sqrt repetidamente.

Também eram recalculados em cada iteração:

- `horizontal_radius * horizontal_radius`;
- `center.xz()`.

### Implementação

Agora, uma vez por rebuild:

- `movement_direction` é convertido/normalizado para `Option<Vec2>`;
- o raio² é calculado;
- a projeção horizontal do center é calculada.

O nested loop recebe/reusa esses valores.

`inside_forward_preload()` agora exige um `Vec2` já normalizado, deixando explícito que a transformação é responsabilidade do caller de rebuild e não da avaliação por coordenada.

### Semântica preservada

- shape do forward preload não mudou;
- movement direction ZERO continua sem forward preload;
- diagonal directions continuam normalizadas;
- desired/retained/pending priority não mudou;
- nenhum cache/invalidation state foi criado.

### Performance / arquitetura

É remoção de trabalho repetido no owner do custo, sem nova allocation, estado duplicado ou cache.

### Versionamento

- `VERSION`: **0.35.9 → 0.35.10**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 155

- Push CI `35530702521`: **success**.
- PR CI `35530704811`: **success**.
- O topo `5a56f2bc86f0520d320433a6490601579c960225` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.10`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 156 — 2026-09-20: authored surface fluids pré-computados por coluna [CÓDIGO APLICADO; CI PENDENTE]

### Hot path

`rasterize_fluid_pass()` percorre 16×16 colunas e até 16 voxels Y por chunk.

Antes, para cada voxel vazio da coluna, `authored_surface_fluid_at()` repetia:

- surface-biome lookup;
- biome registry lookup;
- surface-fluid rule match;
- volcano terrain destructuring;
- authored fluid ID lookup;
- terrain-strength clamp;
- crater level arithmetic;
- world-seed + biome-ID hashing;
- `fractal_noise_2d(..., 3)` para decidir spill channel.

Esses valores não mudam com `world_y`.

### Implementação

Novo `AuthoredSurfaceFluidColumn` é um valor local/derivado da coluna, não estado persistente nem cache global.

Uma vez por coluna potencialmente intersectada:

- resolve biome/rule/fluid;
- calcula strength;
- calcula crater level aplicável;
- calcula o spill/noise uma única vez;
- guarda apenas `fluid_id`, `surface_height`, optional crater level e optional spill level.

O loop Y chama apenas `fluid_at(world_y)`, que preserva a precedência:

1. crater fluid;
2. spill fluid;
3. nenhum authored surface fluid.

### Vertical early-out

Se o topo do chunk está abaixo de `column.surface_height`, authored surface fluid não pode aparecer nesse chunk.

Nesse caso o precompute inteiro é pulado, incluindo biome lookup e noise.

### Performance

No pior caso superficial, trabalho caro authored passa de até 16 vezes por coluna para 1 vez por coluna.

Em chunks inteiramente subterrâneos, passa para zero.

Não foi criado cache, revision tracking ou estado duplicado.

### Semântica preservada

- crater exige strength mínimo;
- crater só existe em/above surface;
- spill continua restrito exatamente ao surface voxel;
- same deterministic spill seed/noise;
- crater continua vencendo spill quando ambos se aplicam;
- hydrology e underground cave water paths não mudaram.

### Versionamento

- `VERSION`: **0.35.10 → 0.35.11**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 156

- Push CI `35530849830`: **success**.
- PR CI `35530853126`: **success**.
- O topo `2b3650a8cc907f12ca440c2a1f90482e45d21060` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.11`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 157 — 2026-09-20: weighted biome fallback reutiliza pesos calculados [CÓDIGO APLICADO; CI PENDENTE]

### Problema

Surface-biome selection calculava `distribution_strength` para cada candidato no draw climático e, quando todos os pesos climáticos zeravam, recalculava as mesmas distributions/noise novamente para o fallback raw-weight.

Surface e volume selection também criavam um `Vec` heap por cache miss/site.

### Implementação

Novo `WeightedCandidate` representa o invariant compartilhado:

- index;
- climate weight;
- fallback weight.

`WeightedDraw::{Climate, Fallback}` torna explícito qual peso participa de cada sorteio.

Surface selection calcula `distribution_strength` uma única vez e deriva ambos os pesos desse resultado.

Volume selection usa a mesma representação com authored weight como fallback.

Os candidates usam `SmallVec<[WeightedCandidate; 16]>`:

- até 16 candidatos ficam inline;
- content packs maiores mantêm comportamento correto via heap fallback.

### Semântica preservada

- candidate order não mudou;
- climate draw continua em `hash.rotate_left(17)`;
- fallback continua em `hash.rotate_left(29)`;
- eligibility/distribution/climate math não mudou;
- nenhuma cache/revision state foi introduzida.

### Performance

- remove segunda avaliação de distribution/noise no fallback de surface biomes;
- remove heap allocation no caso comum de até 16 candidatos;
- otimização permanece no owner de biome selection.

### Versionamento

- `VERSION`: **0.35.11 → 0.35.12**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 157

- Push CI `35531151486`: **success**.
- PR CI `35531154271`: **success**.
- O topo `c7e25cad743d80e1af33ffe1a36a86b40d92ef95` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.12`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 158 — 2026-09-20: tunnel-margin escolhe segmento por distância² [CÓDIGO APLICADO; CI PENDENTE]

### Hot path

`surface_tunnel_margin_density_delta()` procura, para cada túnel candidato, o segmento horizontal mais próximo do ponto da coluna.

Antes, cada segmento chamava `point.distance(closest)`, executando uma raiz quadrada, e só depois `min_by` escolhia o menor.

### Implementação

O helper agora retorna `distance_squared` + `path_y`.

- todos os segmentos são comparados por distância²;
- como sqrt é monotônico, o vencedor é exatamente o mesmo;
- somente depois de escolher o segmento mínimo é executado `.sqrt()` uma única vez;
- o restante da margem usa a mesma distância linear de antes.

### Semântica preservada

- closest-point/projection math não mudou;
- degenerate segment handling não mudou;
- nearest-segment ordering é idêntico;
- tunnel margin/core/exposure math recebe a mesma distância final.

### Performance

Passa de até uma sqrt por segmento do tunnel path para uma sqrt por túnel avaliado.

Nenhum cache, allocation ou estado novo foi criado.

### Versionamento

- `VERSION`: **0.35.12 → 0.35.13**.
- CI pendente.
- Não executei `cargo test`, `cargo run` nem QA Windows.


### CI verde do checkpoint 158

- Push CI `35531267739`: **success**.
- PR CI `35531270942`: **success**.
- O topo `bd03b50c9ad716b593938064da0712d7ac497b10` passou localization audit, Clippy com `-D warnings` e `cargo check --locked`.
- `VERSION` permanece `0.35.13`.
- Não executei `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 159 — 2026-09-20: handoff consolidado do refactor/performance pass [DOCS ONLY]

Este checkpoint consolida o estado real do `develop` após os checkpoints 145–158 para que a próxima sessão não repita trabalho já concluído.

### Topo autoritativo

- branch: `develop`;
- commit funcional atual: `bd03b50c9ad716b593938064da0712d7ac497b10`;
- `VERSION`: **0.36.1**;
- CI do topo: **verde** em push + PR;
- `Cargo.toml` continua com a versão deliberadamente independente do app;
- não houve `cargo test`, `cargo run` nem QA Windows nesta sequência.

### Persistence / save — estado atual

A migração para save format v2 ESTÁ CONCLUÍDA e não deve ser refeita.

Contrato atual:

- gameplay não possui autosave periódico;
- gameplay não possui clock-only checkpoint;
- durable save ocorre somente ao sair do mundo/jogo, inclusive window-close;
- falha no save final impede a saída/fechamento e permite retry;
- `WorldSnapshot` é o owner runtime do estado persistente e deixou de ser serializável diretamente;
- save format 2 é o único formato aceito e escrito;
- chunks persistentes são publicados em `generation-N/`;
- depois é publicado `snapshot-N.json` metadata-only;
- `manifest-N.json` é publicado por último e é o commit marker;
- uma generation só é restorable quando metadata snapshot + external chunk generation existem;
- formatos históricos, chunks inline e encodings antigos são rejeitados; não existe caminho de migração/fallback;
- não existem dois writable chunk catalogs.

Durability/ownership já aplicados:

- chunk JSON é streamado diretamente para storage, sem `Vec<u8>` proporcional ao payload;
- JSON publication faz temp → serialize/flush → file fsync → rename;
- directory fsync cobre durability da entrada renomeada;
- generation lifecycle foi extraído do catálogo;
- `chunk_storage` possui identidade canônica, duplicate rejection, reader validado e cleanup;
- archived persistent chunks serializam direto para `DiskChunk`, sem restore/repack de `VoxelChunk`;
- physical storage mechanics estão separados da save/recovery policy.

### Streaming / remesh — estado atual

Já concluído:

- `DeduplicatedQueue<T>` centraliza dedup/FIFO/priority mechanics;
- predicate scan misses usam revision-aware cache quando a elegibilidade pode ficar estável;
- ready priority usa ranked single scan, preservando critical → forward → background e FIFO dentro do rank;
- tie breaks de streaming são determinísticos;
- streaming pipeline foi separado em generation stage e meshing stage, mantendo `stream_chunks()` como orchestrator explícito;
- remesh queue semantics foram extraídas do scheduler;
- remesh scan misses estão cacheados em todas as filas relevantes;
- unload continua usando boundary metadata em vez de remesh amplo do halo.

Não reintroduzir:

- múltiplos `pop_where()` encadeados sobre a mesma fila para tiers de prioridade;
- queue-specific dedup logic espalhada nos consumidores;
- event bus para esconder a ordem explícita do streaming pipeline.

### Fluid simulation — estado atual

Já concluído:

- scheduler state foi encapsulado;
- diagnostics foram separados do scheduler;
- frontier seed usa metadata/bitset sparse por chunk, não full 16³ scan ingênuo;
- fluid frontier metadata é mantida pelo owner do chunk;
- solver já possui diagnostics de downhill/BFS (`downhill_searches`, `downhill_nodes` etc.);
- sem medição runtime, NÃO introduzir cache/BFS rewrite especulativo;
- água/lava authored semantics permanecem data-driven.

Próxima otimização do solver deve partir de métricas runtime reais, não de static guess.

### Lighting / render — estado atual

Já concluído:

- direct skylight possui transmission/cache por seção/revision;
- initial/bootstrap lighting propagation agora passa pelo mesmo path budgeted de pending lighting;
- o antigo path runtime de relaxation sem budget foi removido e qualquer helper equivalente ficou test-only;
- scene-global `sky_light_factor` não é mais replicado em cada `TerrainMaterial`;
- `TerrainLightingBuffer` usa um shared GPU `ShaderBuffer` referenciado por terrain + fluid materials;
- terrain material construction faz exact-identical material interning.

PENDÊNCIA IMPORTANTE:

- a mudança do shared `TerrainLightingBuffer` passou Rust CI, mas WGSL/bind-group runtime não foi validado com `cargo run`/QA renderer;
- não afirmar validação visual/runtime até existir evidência;
- se aparecer erro de shader/bind group no jogo, investigar esse checkpoint primeiro.

### World selection / load activation / UI

Já concluído:

- `world_selection/layout.rs` possui view/layout/formatting e view-only markers;
- `world_selection/tasks.rs` possui worker lifecycle, owned input capture, polling, abandon e off-thread disposal;
- parent mantém decisão de quando iniciar/aceitar tasks e mutation de resources;
- bootstrap lifecycle foi dividido em stages;
- world activation agora prepara/valida estado antes de commit em resources autoritativos;
- inventory layout ownership foi separado;
- hotbar restore persistido usa prepare/construct → commit, sem mutation parcial;
- API mutating antiga de restore do hotbar foi removida.

### Performance wins recentes 155–158

Checkpoint 155:
- streaming selection hoista movement-direction normalization, radius² e center.xz() para fora do loop O(radius²).

Checkpoint 156:
- authored surface fluid/volcano fluid metadata é pré-computada uma vez por coluna;
- chunks totalmente abaixo da superfície fazem vertical early-out;
- fractal noise deixou de ser recalculado por voxel Y.

Checkpoint 157:
- surface biome selection reutiliza `distribution_strength` para climate + fallback weight;
- candidate storage usa `SmallVec<[WeightedCandidate; 16]>`;
- volume selection usa a mesma representação sem heap no caso comum.

Checkpoint 158:
- tunnel nearest-segment selection compara distância²;
- `.sqrt()` ocorre somente para o segmento vencedor;
- resultado geométrico permanece idêntico.

### Princípios que continuam obrigatórios

Aplicar `ARCHITECTURE.md` + engineering practices passados pelo usuário:

- um owner autoritativo por fato;
- split por invariant/responsabilidade, não por tamanho;
- contexts/SystemParams estreitos e coesos;
- reutilizar invariants reais, não similaridade superficial;
- work change-driven sempre que possível;
- cache somente com key/validity/invalidation/lifetime definidos;
- async bounded + stale-result protection;
- deterministic ordering quando ordem afeta resultado;
- hot paths sem allocations/recomputations evitáveis;
- otimização baseada em evidência quando a mudança altera algoritmo/complexidade;
- não esconder side effects;
- não criar `Utils`/Manager/general abstractions sem invariant concreto.

### Próxima direção recomendada

Continuar o pass de performance/componentização a partir do topo `0.35.13`, reavaliando o código ATUAL antes de usar findings antigos.

Ordem prática:

1. procurar trabalho loop-invariant/recomputation/allocation em hot paths de worldgen/streaming/render que ainda não tenha sido atacado;
2. preferir otimizações locais e semanticamente demonstráveis como checkpoints 155–158;
3. para fluid solver, usar diagnostics existentes antes de alterar BFS/cache;
4. evitar novos refactors de persistence/streaming/remesh apenas por tamanho — os boundaries principais já foram estabelecidos;
5. manter handoff atualizado a cada checkpoint e fechar CI antes de empilhar o próximo bloco quando possível.

### Pendências de validação

- runtime shader/bind-group do `TerrainLightingBuffer`;
- `cargo run`/QA Windows do conjunto de mudanças;
- benchmarks/FPS só podem ser afirmados quando medidos;
- testes existentes podem ser compilados pela CI, mas não executar `cargo test` sem autorização explícita do usuário.

### Regra operacional permanente

A cada novo passo substancial, dar feedback ao usuário sobre o que está sendo investigado/aplicado. Não trabalhar longos blocos em silêncio.


## Checkpoint 160 — 2026-09-20: remoção total de compatibilidade de save legado [BREAKING STORAGE CLEANUP; VERSION 0.36.0]

### Decisão de produto

O usuário definiu que compatibilidade retroativa de save não deve ser preservada. O runtime deve suportar somente o formato produzido pela versão atual; código mantido exclusivamente para abrir/migrar formatos antigos deve ser removido.

### Remoções

- removido suporte a save format v1 e a snapshots com `chunks: [...]` inline;
- `SAVE_FORMAT_VERSION = 2` passa a ser o único formato aceito pelo reader e writer;
- removidos `LEGACY_SAVE_FORMAT_VERSION` e `is_supported_save_format()`;
- `StoredWorldSnapshot` não possui mais campo de chunks inline nem branch de migração;
- `SavedChunkCatalog` deixou de ser serializável: é somente estado runtime/capture;
- removido decoder de `DiskChunk` per-voxel legado (`blocks`/`fluids`);
- disk chunks atuais rejeitam campos desconhecidos para evitar interpretar payload antigo como chunk vazio;
- removidos defaults de serde mantidos para manifests/snapshots anteriores ao schema atual;
- removido fallback de `worldgen_version` ausente e os testes específicos de compatibilidade legada;
- saved fluid state e saved creature payloads passam a usar schema estrito;
- fixture tooling não oferece mais o modo antigo de duplicate inline chunk e exige format 2 + generation directory atual.

### Contrato atual

Persistência aceita somente:

1. manifest com `format_version = 2`;
2. snapshot metadata-only com `format_version = 2`;
3. external chunk generation `generation-N/` usando palette/run encoding atual;
4. schema atual completo para player, fluid work, creatures e worldgen identity.

Qualquer save histórico que dependa de defaults, chunks inline ou encoding per-voxel é deliberadamente incompatível e deve falhar validation/load em vez de ser migrado.

### Versionamento

- `VERSION 0.35.13 → 0.36.0` por mudança incompatível no contrato de persistência.


## Checkpoint 161 — 2026-09-20: durability de save portátil no Windows [FIX; VERSION 0.36.1]

### Sintoma observado

Ao criar um mundo no Windows, a publicação do manifest falhava após o rename com:

`Acesso negado. (os error 5), rollback of published file failed: Acesso negado. (os error 5)`

O arquivo temporário já havia sido flushado/sincronizado e renomeado. A falha vinha de `File::open(directory).sync_all()`, usado como se fosse um directory fsync portátil. Em Windows esse padrão não é suportado pela std e retorna `ERROR_ACCESS_DENIED` para diretórios normais. O rollback repetia o mesmo sync e produzia a segunda mensagem de acesso negado.

### Correção

Novo owner `world::storage_durability` centraliza a semântica:

- arquivos continuam usando `sync_all()` antes de publication/rename;
- Unix mantém fsync do diretório após rename/rollback;
- plataformas sem directory sync portátil pela std, incluindo Windows, tratam esse passo como no-op;
- `chunk_storage` usa o mesmo owner;
- no Windows, `sync_directory_tree` também deixa de percorrer toda a árvore de chunk directories apenas para executar operações que não eram suportadas.

Isso corrige tanto:

- criação/publicação de `manifest-N.json` e `snapshot-N.json`;
- publicação/remoção de `generation-N/`.

### Arquitetura

`ARCHITECTURE.md` agora deixa explícito que file fsync é obrigatório antes do rename, enquanto directory-entry fsync é uma capability de plataforma e não pode ser emulado com `File::open(directory).sync_all()` no Windows.

### Versionamento

- `VERSION 0.36.0 → 0.36.1`.


## Checkpoint 162 — 2026-09-20: fog de streaming restaurada + ajustes de bioma [FIX/TUNING; VERSION 0.36.2; CI VERDE]

### Fog volta a esconder chunks ainda não publicados

Foi revertida deliberadamente a decisão de 19/09 que prendia a fog somente ao envelope configurado de render distance.

O comportamento restaurado usa o frontier real do `ChunkRenderPool`:

- `ChunkRenderPool::active_coords()` expõe somente a membership já publicada;
- o sistema colapsa as sections verticais para colunas x/z, portanto um gap vertical isolado não encurta a fog;
- a posição horizontal do player, render distance, membership revision do pool e replacement da câmera formam os inputs change-driven;
- dentro do círculo de render distance, a coluna publicada faltante mais próxima limita o fim da fog;
- existe um guard de 1 chunk antes da coluna ausente;
- quando todas as colunas estão publicadas, a fog volta ao range normal `0.78 → 0.98` do raio configurado;
- o terminal da fog continua combinando com o sky background para o espaço não publicado não aparecer como um buraco.

Essa é a versão change-driven da fog adaptativa que existia antes da remoção; não foi restaurada uma variante que dependesse de readiness vertical/lighting transitória.

### Volcano cinza

`data/dimensions/overworld/biomes/volcano.json` agora usa saturation 0 em todas as fases de `skyColor` e `fogColor`.

- dawn/day/dusk/night preservam suas intensidades relativas;
- hue é neutralizado;
- resultado authored é escala de cinza, inclusive à noite.

### Witchwood × Enchanted Forest

No owner correto, `data/dimensions/overworld/dimension.json`:

- Witchwood declara `avoidNear: enchanted_forest`;
- Enchanted Forest declara `avoidNear: witchwood`.

Embora o runtime já trate `avoidNear` simetricamente, os dois lados ficam authored explicitamente. As duas regiões não podem compartilhar uma borda Voronoi real.

### Commit / CI

- commit funcional: `8205525f5ab762aadb92645cc4630fbf0f21d30a`;
- `VERSION 0.36.1 → 0.36.2`;
- push CI `35533424186`: **success**;
- PR CI `35533426981`: **success**;
- não houve `cargo test`, `cargo run` nem QA Windows.

## Checkpoint 163 — 2026-09-20: initial fluid settling antes do primeiro mesh [FEATURE; VERSION 0.37.0; CI VERDE]

### Requisito recuperado

O requisito correto é diferente de apenas “acordar” generated fluid frontiers quando Gameplay começa:

**um chunk novo deve ter o seu spread inicial materializado antes da primeira publicação visual.**

O scheduler runtime continua responsável por mudanças posteriores e por propagação que cruza para estado fora do conjunto recém-gerado.

### Solver único

Não foi criada uma segunda física de fluidos.

Novo `fluid_updates::settling` reutiliza diretamente:

- `desired_fluid_with_scratch()`;
- as mesmas regras gravity-first;
- o mesmo horizontal spread state;
- o mesmo nearest-drop/downhill BFS;
- as mesmas regras de waterfall/lower-pool;
- `maxSpread` e identidade de fluido authored.

A única diferença deliberada é temporal:

- initial settling ignora `spreadSpeed`/scheduled delay;
- executa a convergência até a frontier local estabilizar;
- Gameplay continua usando scheduled voxel ticks com cadência authored.

### Frontier compartilhada

`fluid_updates::frontier` agora possui um único traversal que emite:

- `FluidId`;
- target;
- prioridade vertical.

Esse traversal é usado por:

1. runtime `PendingFluidUpdates`;
2. initial settling.

Assim generated frontier eligibility e runtime frontier não podem divergir silenciosamente.

### Bootstrap de mundo novo

Novo loading lifecycle:

`Generating → SettlingFluids → Lighting → Meshing → Spawning`

Somente `WorldLoadMode::New` passa por `SettlingFluids`.

Quando toda a bootstrap region já está residente:

- todas as coords bootstrap formam o conjunto mutável;
- sources e seams são seeded;
- spread converge através dos chunks recém-gerados;
- somente depois começam direct lighting e initial meshing.

Loaded saves pulam essa fase: estado persistido não é reinterpretado como worldgen derivado.

### Streaming em Gameplay

Quando generation tasks terminam durante streaming:

1. resultados novos são integrados;
2. coords novas do batch formam o conjunto de settling;
3. incoming frontier de chunks vizinhos já residentes também pode preencher targets dentro dos chunks novos;
4. settling converge dentro do conjunto novo;
5. só então cada chunk recebe initial lighting seed e entra na ready queue de mesh.

Se uma propagação quiser sair do conjunto novo para um chunk antigo/já publicado, o target não é alterado pelo pre-settle. A frontier runtime continua existindo e assume essa transição com a cadência normal.

### Persistência preservada

Foi adicionado `VoxelWorld::set_derived_fluid_at()`.

Diferença para `set_fluid_at()` runtime:

- atualiza voxel content revision e mesh revision;
- mantém metadata de frontier via `VoxelChunk::set_fluid()`;
- **não insere o chunk em `persistent_chunks`**;
- recusa o path se o target já é persistent.

Isso evita transformar worldgen derivado em save autoritativo apenas porque a água/lava assentou antes do primeiro mesh.

### Commit / CI

- commit funcional: `c85c0831226dea12578dbd6aab97492d8c7f86a4`;
- `VERSION 0.36.2 → 0.37.0`;
- push CI `35533512875`: **success**;
- PR CI `35533515535`: **success**;
- localization audit, Clippy rigoroso e `cargo check --locked` verdes;
- não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Novo mundo em Volcano: crater/spill lava deve já aparecer assentada no primeiro frame de Gameplay dentro da bootstrap region, sem começar como source estática e “acordar” depois.
2. Novo mundo com água exposta: mesma regra.
3. Waterfall dentro da bootstrap region: coluna e routing horizontal devem existir antes do primeiro mesh.
4. Streaming: aproximar-se de Volcano/água nova; o chunk deve ser publicado já com o estado inicial local resolvido.
5. Seam streaming: incoming fluid de chunk antigo pode preencher o chunk novo no pre-settle; outgoing para chunk antigo continua como scheduled runtime work.
6. Afastar/reaproximar de chunk derivado que só sofreu initial settling: ele deve continuar descartável/regenerável, não virar persistente.
7. Fog: durante generation/meshing backlog, a frontier não publicada deve permanecer escondida; depois que o pool preencher as colunas, a fog deve voltar ao range normal.
8. Witchwood e Enchanted Forest não devem compartilhar borda regional.
9. Volcano: sky/fog devem ser neutros/cinza em dawn/day/dusk/night.


## Checkpoint 164 — 2026-09-20: initial fluid settling incremental; loading não pode bloquear [HOTFIX; VERSION 0.37.1; CI VERDE]

### Regressão observada

Após o checkpoint 163, criação de mundo travava visualmente na tela de loading.

A causa era objetiva: `settle_generated_fluid_chunks()` executava a convergência inteira em um único `while queue.pop()` síncrono. Uma bootstrap region pode conter centenas de sections e uma frontier grande de ocean/lava/waterfall; cada voxel ainda pode executar o downhill solver. Mesmo sendo trabalho finito, o sistema não devolvia o frame até esvaziar toda a fila, congelando a loading UI.

Não foi identificado deadlock de lock/thread; o problema era ausência de yield/frame budget.

### Novo owner incremental

`fluid_updates::settling::GeneratedFluidSettling` agora mantém entre frames:

- conjunto de chunks recém-gerados autorizados a receber mutação derivada;
- `DeduplicatedQueue<IVec3>` de voxels pendentes;
- `FluidSolverScratch` reutilizado;
- flag de activity/publication barrier.

API interna:

- `begin(world, coords)`;
- `extend(world, coords)`;
- `process(world, fluids, FrameWorkBudget)`;
- `take_completed_chunks()`.

A convergência só libera lighting/first mesh depois de a queue chegar a zero, mas **nunca tenta esvaziá-la inteira sem respeitar budget**.

### Bootstrap

`WorldLoadingState` passa a possuir o settling state.

`SettlingFluids`:

- inicializa uma vez com todas as bootstrap coords;
- processa no máximo uma fatia por frame;
- budget temporal: 6 ms;
- mínimo antes de checar relógio: 32 updates;
- máximo absoluto: 2048 updates/frame;
- quando converge, consome a publication barrier e avança para `Lighting`.

Portanto loading continua apresentando frames enquanto água/lava convergem.

### Streaming

`ChunkStreamingState` também possui um `GeneratedFluidSettling`.

Generated results:

1. entram no `VoxelWorld`;
2. são adicionados ao conjunto de settling;
3. podem receber novos chunks no mesmo owner enquanto generation continua;
4. settling recebe budget separado de 2 ms / 512 updates por frame;
5. nenhum desses chunks entra em initial lighting/ready mesh até a queue convergir;
6. ao convergir, chunks ainda desejados recebem lighting e ready;
7. chunk que saiu da seleção enquanto aguardava é arquivado/descartado conforme sua persistência.

`stream_chunks()` continua chamando collection/settling mesmo quando generation tasks já chegaram a zero, enquanto o settling owner estiver ativo.

### Liveness e ownership

O pre-settle continua:

- reutilizando o mesmo desired-state/downhill solver do runtime;
- ignorando spread delay somente na fase derivada;
- recusando mutação derivada em persistent chunks;
- limitando targets ao conjunto recém-gerado;
- deixando cross-boundary para runtime quando o target não pertence ao conjunto.

Novo requisito arquitetural: **generated-fluid convergence nunca pode ser implementada novamente como loop síncrono sem frame budget/yield**.

### Commits / CI

- `8c9fa715b7a887ccf0b3b6929f2ae596e2a5e9cd`: implementação incremental + `VERSION 0.37.1`.
- CI push `35533856014` falhou no Clippy por `E0365`: re-export de `GeneratedFluidSettling` ampliava a visibilidade além de `crate::world`.
- `c15c1cc14df1361de06213bffa5e668b2dddc0ce`: mantém o state world-private com re-export na mesma visibility.
- CI push `35533883399`: **success**.
- CI PR `35533885662`: **success**.
- auditoria de idiomas, Clippy rigoroso e `cargo check --locked` verdes.
- não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

Recriar exatamente o caso que congelou:

1. criar mundo novo com ocean/água/lava dentro da bootstrap region;
2. confirmar que loading continua animando/responsivo durante SettlingFluids;
3. confirmar entrada em Gameplay após convergência;
4. confirmar que o primeiro frame do chunk já contém o spread inicial;
5. durante streaming de região com fluido pesado, confirmar ausência de hitch longo e ausência de publicação antes do pre-settle local.


## Checkpoint 165 — 2026-09-20: remover fixed-point de fluidos gerados; prime step finito e source pool não é drop [ROOT CAUSE FIX; VERSION 0.37.2; CI VERDE]

### Sintoma

Mesmo após tornar o initial fluid settling budgetado entre frames, a criação de mundo podia continuar presa na tela de loading.

O primeiro hotfix de budget (`0.37.1`) corrigia apenas monopolização de frame. Ele NÃO corrigia liveness da própria fila: se o solver alternasse estados, a queue podia nunca chegar a zero.

### Investigação profunda

O bootstrap máximo atual é limitado a raio horizontal 4 e vertical 2 (até ~245 chunks), portanto o problema não era simplesmente “render distance inteira”.

Também foi descartada a hipótese de bulk-simulation do interior de Ocean/Lake/etc.:

- worldgen gera esses volumes como sources;
- source não flui em source;
- frontier seed considera apenas targets vazios expostos;
- o bitset `fluid_frontier_sources` evita scan 16³ ingênuo.

A causa real estava em uma regra state-dependent de downhill routing.

#### Regressão histórica identificada

Commit histórico `fa0b7d6dbe9ee6c7e7028fbe8e9738f05c10449d` (“Treat lower same-fluid pools as reachable drops”) alterou `can_fall_from()` para usar:

- célula atual vazia + source do mesmo fluido abaixo => considerar como drop;
- depois que a célula enchia, o mesmo ponto deixava de ser drop porque o fluido abaixo era source.

Isso permitia ciclo:

1. caminho A, ainda vazio acima de source pool, é classificado como drop mais próximo;
2. A recebe flow;
3. A deixa de ser vazio e deixa de ser drop;
4. outro caminho B passa a ser preferido;
5. A perde alimentação e esvazia;
6. A volta a ser vazio e volta a ser drop;
7. repetir.

O pre-settle fixed-point re-enfileirava exatamente centro/baixo/horizontais depois de cada mutação, portanto podia perpetuar esse ciclo indefinidamente.

A regra também contrariava o contrato reafirmado pelo usuário: **source não flui em source**. Source pool é suporte/estado authored, não uma abertura downhill.

### Correção do solver

`can_fall_from()` voltou a uma classificação estável:

- vazio real abaixo => drop;
- coluna dinâmica existente do mesmo fluido com `spread_distance == 0` => drop persistente, preservando waterfall routing;
- source do mesmo fluido abaixo => NÃO é drop.

Foi substituído o teste histórico `empty_ledge_above_same_fluid_pool_is_still_a_drop` por uma regressão que cria:

- source pool em uma direção;
- drop real concorrente em outra;

e exige que o source pool não atraia o downhill routing.

### Correção arquitetural: priming, não settling

A releitura do contrato dos checkpoints 109–112 mostrou que o modelo histórico era:

- volume de hydrology/surfaceFluid gerado é estado inicial autoritativo;
- somente targets vazios expostos entram no solver;
- propagação visual deve continuar step-by-step respeitando `spreadSpeed`.

Portanto o checkpoint 163 interpretou errado o requisito ao introduzir convergência até fixed point.

Novo modelo:

`Generating → PrimingFluids → Lighting → Meshing → Spawning`

`GeneratedFluidPriming`:

- captura somente a frontier exposta no início do batch;
- avalia cada target vazio no máximo uma vez;
- pode materializar a primeira célula `spreading`;
- nunca sobrescreve fluido authored/source;
- nunca re-enfileira vizinhos criados pelo próprio priming;
- não procura fixed point;
- continua frame-budgeted;
- usa `set_derived_fluid_at()`, então prime step não promove chunk gerado a persistent;
- Runtime scheduler assume toda continuação com cadência authored.

Resultado pretendido:

- primeiro mesh já mostra que o fluido começou a fluir;
- waterfall/spread NÃO aparece inteiro instantaneamente;
- Gameplay apresenta os próximos steps no intervalo normal de água/lava.

### Bootstrap

Priming inicial:

- budget 4 ms/frame;
- mínimo 16 targets antes do clock check;
- máximo 1024 targets/frame;
- queue é finita por construção, pois mutations não criam novo priming work;
- após consumir a snapshot da frontier, Loading avança para Lighting.

No OnEnter Gameplay, o reseed normal encontra a nova célula dinâmica criada pelo prime step e agenda a continuação.

### Streaming

Streaming usa a mesma semântica, com budget menor:

- 1 ms/frame;
- mínimo 8 targets;
- máximo 256 targets/frame.

Follow-up `e02a3e239950fa5bd4356c3e31ca685a37e44cf4` fecha outra fonte de starvation:

- enquanto um batch está em priming, novos generation results NÃO são incorporados ao mesmo batch;
- o batch atual precisa terminar e publicar primeiro;
- somente então o próximo batch de generation results é coletado.

Isso dá um limite de publicação por batch e impede um conjunto de priming de crescer indefinidamente durante movimento/streaming contínuo.

### Limpeza

A abstração `settling` foi removida por completo:

- `2d7a806605e421be60522d3cfe2d43516697d4a2` cria `fluid_updates/priming.rs`;
- `2ffff8c70826e5f64be7036b3be5449d7eefb7ec` muda o owner/module para `priming`;
- `911360936b51799f41e53bd51551cc60187028a8` remove `settling.rs`.

Não fica alias/caminho legado para a arquitetura errada.

### Commits / versionamento

- `72fef101829b50d00011ca20be4e1a4af5595585` — source pool deixa de ser drop + fixed-point vira one-step priming + `VERSION 0.37.2`;
- `e02a3e239950fa5bd4356c3e31ca685a37e44cf4` — batches de streaming ficam bounded;
- `911360936b51799f41e53bd51551cc60187028a8` — HEAD funcional final antes deste handoff.

### CI

HEAD funcional `911360936b51799f41e53bd51551cc60187028a8`:

- push CI `35534437410`: **success**;
- PR CI `35534440795`: **success**;
- localization audit: success;
- Clippy `--locked --all-targets --all-features -- -D warnings`: success;
- `cargo check --locked`: success.

Não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Recriar exatamente o mundo que travava em Loading.
   - esperado: PrimingFluids termina; Gameplay é alcançado.
2. Source ao lado/acima de source do mesmo fluido:
   - source existente não pode receber/atrair flow como downhill destination.
3. Volcano:
   - primeiro mesh deve mostrar ao menos o primeiro spill step quando houver target elegível;
   - próximos steps devem aparecer gradualmente com cadência da lava.
4. Água:
   - mesma regra com cadência mais rápida.
5. Waterfall:
   - primeiro segmento pode nascer materializado;
   - coluna inteira não deve ser pre-resolvida instantaneamente.
6. Streaming contínuo:
   - chunks novos devem publicar batch a batch sem primer crescer indefinidamente.


## Checkpoint 166 — 2026-09-20: priming entrega frontier explicitamente ao scheduler de Gameplay [FIX; VERSION 0.37.3; CI VERDE]

### Sintoma

Após o root-cause fix do checkpoint 165:

- Loading destravou;
- o primeiro prime step aparecia;
- porém a continuação não estava garantida como runtime work em Gameplay.

O bootstrap ainda dependia demais de um reseed global posterior no `OnEnter(Gameplay)`.

### Causa arquitetural

Durante geração de mundo novo, antes do priming:

- `generate_initial_chunks()` já enfileirava frontier do estado authored;
- o priming então alterava o mundo;
- a fila existente podia representar o snapshot pré-prime;
- ao entrar em Gameplay, `reseed_loaded_fluid_frontiers()` resetava `PendingFluidUpdates` para New e tentava redescobrir tudo outra vez.

Mesmo que o rescan devesse reconstruir a frontier, isso quebrava o ownership correto: o estágio que produz o primeiro flow visível não estava entregando explicitamente a continuação ao scheduler que deve possuir os próximos ticks.

### Novo contrato

Mundo novo:

1. generation insere chunks sem criar runtime frontier wakes;
2. `PrimingFluids` cria no máximo o primeiro step;
3. ao concluir:
   - qualquer work pré-prime é descartado;
   - a frontier **pós-prime** é semeada em `PendingFluidUpdates`;
4. lighting/meshing continuam;
5. `OnEnter(Gameplay)` preserva a fila recebida;
6. o reseed de Gameplay passa a ser apenas reconciliação idempotente, nunca reset.

Assim o estado do primeiro mesh e a fila runtime descrevem o mesmo snapshot do mundo.

### Cadência

O priming continua sem executar a continuação.

`PendingFluidUpdates` recebe apenas frontier wake. No primeiro `process_fluid_updates()` em Gameplay:

- wake recebe due tick conforme `spreadSpeed`;
- água/lava continuam através de scheduled voxel ticks;
- primeiro step continua pre-mesh;
- steps seguintes continuam visíveis em Gameplay.

### Loaded worlds

`WorldLoadMode::Load` mantém o comportamento de semear/restaurar frontier durante bootstrap, pois não existe prime step de worldgen sendo aplicado sobre o conteúdo persistido.

### Streaming

Streaming já estava no ordering correto:

`generate batch → prime batch → seed_loaded_chunk_lighting()/enqueue_loaded_fluid_frontier → ready/mesh`

Portanto foi mantido e documentado como o mesmo contrato de handoff pós-prime.

### Commit / CI

- commit funcional: `d63132a8def450bb819a38b4972ae0b00049251b`;
- `VERSION 0.37.2 → 0.37.3`;
- push CI `35534782633`: **success**;
- PR CI `35534786204`: **success**;
- localization audit, Clippy `-D warnings` e `cargo check --locked` verdes;
- não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Criar mundo novo com lava/água exposta.
2. Confirmar primeiro step já visível ao entrar em Gameplay.
3. Ficar parado sem quebrar/colocar bloco:
   - próximos steps devem continuar sozinhos;
   - água deve continuar mais rápida;
   - lava deve continuar mais lenta.
4. Streaming em direção a chunk novo com fluido:
   - primeiro step pre-mesh;
   - continuação automática depois da publicação.


## Checkpoint 167 — 2026-09-20: generated fluids fazem full settling até não poderem mais mudar [FEATURE CORRECTION; VERSION 0.38.0; CI VERDE]

### Clarificação final do requisito

O usuário corrigiu a interpretação do checkpoint 165/166:

**não é para materializar apenas uma célula de flow antes do primeiro mesh. O fluido gerado deve continuar simulando durante generation/streaming até alcançar um estado em que não exista mais nenhuma mutação possível dentro do conjunto recém-gerado.**

Portanto os checkpoints de one-step priming em `0.37.x` ficam historicamente preservados, mas sua semântica está supersedida por este checkpoint.

### Contrato atual

Bootstrap de mundo novo:

`Generating → SettlingFluids → Lighting → Meshing → Spawning`

Durante `SettlingFluids`:

1. sources authored permanecem imutáveis;
2. targets expostos entram na queue;
3. cada target é avaliado com `desired_fluid_with_scratch()`, o mesmo desired-state solver do runtime;
4. se `current != desired`, a mutação derivada é aplicada;
5. centro, abaixo e horizontais dependentes são re-enfileirados;
6. esse processo continua, atravessando múltiplos steps de spread/fall, até a queue chegar realmente a zero;
7. somente então lighting e primeiro mesh podem começar.

Isso significa que o estado visual inicial já representa o fluido estabilizado dentro da região gerada, não apenas o primeiro passo.

### O que impede a regressão de loading infinito

O full settling original de `0.37.0` podia não convergir por causa da regra histórica corrigida no checkpoint 165.

A correção permanece:

- source pool abaixo de uma célula vazia **não** é downhill drop;
- source não é destino de flow;
- uma coluna dinâmica de queda do mesmo fluido com `spread_distance == 0` continua sendo um drop estável;
- portanto preencher uma queda não altera sua classificação de forma a alternar rotas source-pool/empty indefinidamente.

Além disso, full settling continua **incremental**:

- bootstrap: budget de 4 ms, mínimo 16 e máximo 1024 avaliações por frame;
- streaming: budget de 1 ms, mínimo 8 e máximo 256 avaliações por frame;
- não existe `while` síncrono sem yield que tente esvaziar toda a região em um frame.

### Streaming

Chunks gerados durante Gameplay usam o mesmo contrato:

`generate batch → settle batch até fixed point → seed runtime frontier/lighting → ready → first mesh`

O batch atual precisa terminar antes de absorver o próximo batch de generation results. Isso evita crescimento indefinido do conjunto enquanto o player se move.

O settling só pode alterar chunks do próprio conjunto recém-gerado. Se uma propagação cruza para chunk antigo/já publicado/persistido, essa transição continua sendo runtime work normal em Gameplay.

### Runtime depois do settling

Ao concluir o settling:

- `PendingFluidUpdates` é reconstruído a partir da frontier **pós-settle**;
- localmente, não deve sobrar trabalho que ainda poderia estabilizar dentro do conjunto já gerado;
- cross-boundary work e futuras topology changes permanecem responsabilidade do scheduler normal;
- quebrar/colocar bloco ou carregar um chunk vizinho pode acordar novamente o fluido e ele volta a espalhar conforme `spreadSpeed`.

### Persistência

`set_derived_fluid_at()` continua sendo usado durante settling:

- altera conteúdo/revisions;
- mantém metadata de fluid frontier;
- não promove o chunk a `persistent_chunks`;
- recusa mutação derivada em chunk já persistente.

### Nome/ownership

A abstração one-step `priming` foi removida novamente, agora por mudança real de semântica:

- `b9f4f45c245ab67c31daf9572fb03b0bd34adb81` cria `fluid_updates/settling.rs`;
- `ad38a13110c467aa3ff79e639e06e4ef1377eb00` move o owner para `settling`;
- `c504e2c5a13098a8332930c49722e09d4882d458` remove `priming.rs`.

Não existe alias legado de priming no código atual.

### Commits / versão

- `2b68876f69ffd7b1fa0a03c76655a4f338258d99` — full settling e `VERSION 0.38.0`;
- `c504e2c5a13098a8332930c49722e09d4882d458` — HEAD funcional final antes deste handoff.

### CI

HEAD funcional `c504e2c5a13098a8332930c49722e09d4882d458`:

- push CI `35535128155`: **success**;
- PR CI `35535130905`: **success**;
- localization audit: success;
- Clippy `--locked --all-targets --all-features -- -D warnings`: success;
- `cargo check --locked`: success.

Não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Novo mundo com Volcano:
   - loading permanece responsivo;
   - lava deve chegar em Gameplay já totalmente assentada dentro da bootstrap region.
2. Água em desnível:
   - queda + spread horizontal devem seguir até não haver novo estado possível antes do primeiro mesh.
3. Source ao lado/acima de source:
   - source não deve invadir source nem ser escolhido como downhill opening.
4. Streaming:
   - chunk novo com fluidos só deve publicar depois de seu batch estabilizar;
   - não deve ocorrer hitch longo porque settling continua budgetado.
5. Topology posterior em Gameplay:
   - quebrar suporte/abrir passagem deve reativar o solver e permitir novo spread.


## Checkpoint 168 — 2026-09-20: settling obrigatório em todo chunk gerado + botão para abrir pasta dos saves [FEATURE/FIX; VERSION 0.39.0; CI VERDE]

### Full settling em todo chunk recém-gerado

O requisito foi reafirmado: fluid settling não é apenas parte do bootstrap inicial. **Toda saída nova de worldgen precisa estabilizar seus fluidos antes da primeira publicação**, inclusive chunks gerados durante streaming em Gameplay.

Auditoria dos owners mostrou dois caminhos normais que integram `ChunkGenerationTasks`:

1. bootstrap em `setup::progress::generation`;
2. streaming em `streaming::generation`.

Bootstrap já passa por:

`Generating → SettlingFluids → Lighting → Meshing → Spawning`.

Streaming já passa por:

`generation result → insert resident chunk → GeneratedFluidSettling → seed lighting/runtime frontier → ready → initial mesh`.

Chunks apenas restaurados/persistidos não são worldgen novo e portanto não rerodam settling.

### Bypass encontrado em selection rebuild

A auditoria encontrou um edge case real que podia quebrar a garantia durante Gameplay:

- um generation result era integrado ao `VoxelWorld` e ficava residente enquanto `GeneratedFluidSettling` ainda o possuía;
- antes da publicação, o player podia mover/alterar a seleção;
- `rebuild_queue()` incluía todo coord sem render entity novamente em `pending`;
- o dispatcher genérico via o chunk como resident/persisted;
- esse caminho podia chamar lighting + `mark_ready()` antes do settling terminar.

Correção:

- `GeneratedFluidSettling::contains(coord)` expõe ownership do gate;
- selection rebuild exclui coords ainda owned pelo settling;
- dispatcher também ignora defensivamente uma entrada stale de pending que ainda pertença ao settling;
- `ChunkStreamingState::mark_ready()` contém assert que proíbe publicar um chunk enquanto settling ainda o possui.

Assim o invariant agora existe no código, não apenas na ordem feliz do pipeline.

### Botão Abrir Pasta dos Saves

Na tela World Selection / Load Worlds:

- footer agora tem `Return` e `Open Saves Folder` lado a lado, usando o button padrão do design system;
- novas traduções EN/PT-BR/ES:
  - `worldSelection.openSavesFolder`;
  - `worldSelection.openSavesFolderError`;
- ação `WorldSelectionAction::OpenSavesFolder` funciona independentemente de uma load task estar em andamento;
- falha de abertura aparece no feedback/error existente da tela.

Owner de filesystem:

- `save_catalog::open_worlds_directory()` reutiliza `WORLDS_DIRECTORY`;
- cria a pasta se ela ainda não existir;
- resolve caminho absoluto normal por `current_dir().join(WORLDS_DIRECTORY)`;
- Windows usa `explorer.exe`;
- macOS usa `open`;
- Unix usa `xdg-open`;
- foi evitado `fs::canonicalize()` no Windows porque pode produzir path com prefixo `\\?\`, que não é um formato confiável para a linha de comando do Explorer.

### Commits / versão

- `0858690254db7fe00a28f0e53479ecca6fec941c` — botão da pasta de saves, traduções e invariant arquitetural; `VERSION 0.39.0`;
- `298150b6625e3dc23d41158a54445d731cdb1da0` — path compatível com Explorer;
- `45e273970231e64e9b9044d4c0ca98a288a87214` — fecha bypass de settling durante selection rebuild; HEAD funcional final antes deste handoff.

### CI

HEAD funcional `45e273970231e64e9b9044d4c0ca98a288a87214`:

- push CI `35535600959`: **success**;
- PR CI `35535603233`: **success**;
- localization audit: success;
- Clippy rigoroso: success;
- `cargo check --locked`: success.

Não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Entrar num mundo e caminhar continuamente para área nunca gerada:
   - chunks novos devem fazer full settling antes de aparecer;
   - mover durante o settling não pode publicar o chunk cedo.
2. Aproximar-se de Volcano/Ocean/Lake novos:
   - primeiro mesh deve conter estado já estabilizado dentro do batch recém-gerado.
3. World Selection:
   - botão deve aparecer ao lado de Return;
   - no Windows deve abrir a pasta real `worlds` no Explorer;
   - se a pasta não existir, deve ser criada antes de abrir.


## Checkpoint 169 — 2026-09-20: fluid settling como camada pós-geração por generation region [ARCHITECTURE; VERSION 0.40.0; CI VERDE]

### Problema identificado pelo usuário

O usuário observou a causa provável do spread residual em Gameplay:

1. chunk A termina generation;
2. A faz settling enquanto B ainda está em generation task;
3. o spread de A para na seam porque B ainda não existe no `VoxelWorld`;
4. B termina generation depois;
5. B foi produzido sem participar do mesmo estado settled de A;
6. a continuidade acaba aparecendo apenas mais tarde pelo scheduler runtime.

Mesmo com frontier seam revisit, o resultado ainda ficava dependente da ordem temporal em que outputs async eram integrados.

### Novo pipeline

Streaming passa a tratar fluid spread como uma **camada posterior à geração coerente de chunks**, igual ao princípio já usado no bootstrap.

A unidade finita escolhida é a `generation_region` existente de 8×8×8 chunks, mas somente para os chunks atualmente desejados dentro dela; NÃO são gerados 512 chunks cegamente.

Fluxo atual:

`select generation region cohort`
→ `dispatch async generation apenas dessa region`
→ `integrate outputs em qualquer ordem`
→ `manter novos chunks staged/resident/unpublished`
→ `aguardar zero pending desejado + zero generation task da region`
→ `GeneratedFluidSettling sobre toda a coorte nova`
→ `fixed point`
→ `seed runtime frontier + lighting`
→ `ready`
→ `first mesh`
→ `próxima generation region`

### Estado novo do streaming

`ChunkStreamingState` agora possui:

- `active_generation_region: Option<IVec3>`;
- `staged_generated_chunks: HashSet<IVec3>`;
- `GeneratedFluidSettling` continua como owner da fase seguinte.

Um chunk novo pode estar em dois estados unpublished:

1. staged — generation terminou, mas a coorte ainda não terminou de gerar;
2. settling — a coorte terminou generation e está convergindo fluidos.

Ambos são proibidos de entrar em ready/mesh.

### Coerência da coorte

Enquanto uma generation region está ativa:

- dispatcher só retira `pending` pertencente à mesma region;
- chunks de outras regions ficam aguardando;
- generation tasks podem terminar em qualquer ordem;
- todos os outputs novos são integrados e staged;
- settling NÃO começa enquanto existir:
  - generation task da region; ou
  - pending desejado da mesma region.

Selection rebuild pode adicionar novos chunks desejados à coorte ativa; eles entram antes do settling.

### Restored/persisted chunks

Chunks já existentes no save/resident/archive não passam por worldgen settling novamente.

Eles podem ser restaurados e usados como halo/read-only context da coorte nova.

Somente outputs realmente novos de `generate_chunk()` entram em `staged_generated_chunks`.

### Publicação e safety guards

- selection rebuild exclui staged e settling chunks do pending genérico;
- `mark_ready()` rejeita qualquer coord ainda unpublished;
- generic resident/restored fast path não pode reclamar um chunk staged;
- `finish_generation_region()` só aceita encerrar a coorte quando:
  - staging está vazio;
  - settling está inativo.

### Scheduler / prioridade

A preempção antiga de generation tasks por distância foi removida do generation owner.

Motivo: cancelar uma task da coorte ativa para introduzir trabalho de outra prioridade/region reintroduziria mistura de regiões e faria a semântica depender novamente da ordem temporal de dispatch.

A próxima generation region ainda é escolhida pela ordem/prioridade da queue de streaming; a coerência só é exigida depois que uma region começa.

Mesh task preemption continua independente e não foi alterada.

### Commits / versão

- `d01002506ab17e94a5b74f6300d5e7604fb40f8d` — coorte por generation region, staging pós-generation e settling global; `VERSION 0.40.0`;
- `7311587a3157cd6c5d6cbde0643df3a551128276` — remove generation preemption obsoleta; HEAD funcional final antes deste handoff.

### CI

HEAD funcional `7311587a3157cd6c5d6cbde0643df3a551128276`:

- push CI `35536185243`: **success**;
- PR CI `35536186909`: **success**;
- localization audit: success;
- Clippy rigoroso: success;
- `cargo check --locked`: success.

Não houve `cargo test`, `cargo run` nem QA Windows.

### QA prioritária

1. Caminhar até área nunca gerada com água/lava atravessando seams.
2. Observar chunks de uma mesma generation region:
   - nenhum deve aparecer enquanto parte da coorte ainda está gerando;
   - ao aparecer, o spread entre eles já deve estar estabilizado.
3. Testar waterfall cruzando múltiplos chunks da mesma region.
4. Testar movimento contínuo atravessando boundary de generation region.
   - próxima region deve aguardar a anterior publicar;
   - não deve haver fluid correction visível causada apenas pela ordem de conclusão das generation tasks.
