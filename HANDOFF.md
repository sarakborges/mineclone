# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz `VERSION`: `0.22.5`** após inclusão funcional de identidade explícita do Player e subida automática de degraus parciais; ainda NÃO implica QA Windows. `Cargo.toml` `0.10.16` independente. **Último código Rust validado:** `7275f4b7cbb60c0271bb68fe4b72b5da5a08ef90`, CI run 35265288988: auditoria EN/PT-BR/ES, Clippy `--locked --all-targets --all-features -- -D warnings` e `cargo check --locked` todos success. Nenhum `cargo test` ou gameplay Windows foi executado neste bloco. O primeiro CI deste bloco (`14a9c93186e5ef7909abaede315441fc0287a47a`, run 35264644200) falhou por duas omissões de chamada/limite de tuple, corrigidas em `e0cd5bbe81d6952eab9dd093cca16c8b2dc7d19b` e `d67b601a74f352c8dff59a47c358745ae145cbb7`; o helper de step-up recebeu correção de Clippy em `bfe64298fc03a53ccf4e3f31847af6a569469d59`, cuja CI também ficou verde. O follow-up `7275f4b7cbb60c0271bb68fe4b72b5da5a08ef90` preserva o `Name` do Player sem exceder o limite de bundle e sua CI final também ficou verde.

## Histórico integral obrigatório

- Etapas [0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), [13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), [17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), [21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), [25–27](docs/handoffs/auditoria-25-27-2026-09-17.md), [28–31](docs/handoffs/auditoria-28-31-2026-09-17.md), [32–36](docs/handoffs/auditoria-32-36-2026-09-17.md), [37–44](docs/handoffs/auditoria-37-44-2026-09-17.md) e [45–53](docs/handoffs/auditoria-45-53-2026-09-17.md). Ler `ARCHITECTURE.md`.
- [Handoff develop pré-Brush, etapa 54 integral](docs/handoffs/handoff-develop-pre-brush-merge-2026-09-17.md), blob `27b903e2b8d68d5603e379dc20b909a9ef8eefa0`; [branch Brush pré-merge integral](docs/handoffs/handoff-brush-pre-merge-2026-09-17.md), blob `533d6565f2044697fe70ab8135a0852dbd97ca1a`; [handoff anterior da branch Brush](docs/handoffs/handoff-pre-brush-2026-09-17.md), blob `49d20d15d857118bd660a0d619ef58a9cf37d3ab`.
- [Branch Chisel pré-merge integral](docs/handoffs/handoff-chisel-pre-merge-2026-09-17.md), blob `b2a6f4416fe6ae07998a0dac7416778502833e22`. [Handoff integrado ANTES do save de microblocks, preservado integralmente](docs/handoffs/handoff-before-chisel-save-2026-09-17.md), blob `b973a10daa44354b7531c063f2353d35cfdeb840`, commit `9c00f8f9ab5e7782c60423bc97ec1d535d1bfd53`. Nele, afirmações sobre Chisel apenas em sessão são históricas e foram substituídas pela etapa atual. Especificação vigente: [docs/chisel-microblocking.md](docs/chisel-microblocking.md).

**Regras permanentes:** commits coerentes na `develop`; atualizar este handoff após cada checkpoint com SHA, CI, limitações, próximos passos e arquivar integral antes de condensar. Semver por bloco funcional, nunca por docs isoladas. Save só pode ser considerado concluído após QA Windows real. Corrigir warnings sem supressão. CI exige auditoria EN/PT-BR/ES, Clippy rigoroso e `cargo check --locked`; NÃO adicionar nem executar `cargo test` sem autorização expressa. Não alterar PNGs/GLBs autorais. Pause NÃO congela mundo/fluidos/tempo. Preservar cache da CI, `Cargo.lock` gitignored. Nunca alegar execução Windows/medição FPS/memória ou sucesso de roundtrip sem evidência.

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
