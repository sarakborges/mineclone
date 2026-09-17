# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **VERSION raiz confirmado: `0.20.3` em 16/09/2026 (horário de São Paulo); `Cargo.toml` tem versão independente `0.10.16`.** Último HEAD examinado antes desta atualização: [`e3d4533`](https://github.com/sarakborges/mineclone/commit/e3d4533e6d0ed8a3a789573ef61a3c8f1f9d5cd0). **ATENÇÃO: há código novo publicado após o último bump de VERSION, mas a feature ainda está incompleta. Não afirmar que `/spawn`/`/place` funcionam, nem que a CI passou.**

**Preservação integral do histórico:** [HANDOFF imediatamente anterior, antes da presente consolidação](https://github.com/sarakborges/mineclone/blob/e3d4533e6d0ed8a3a789573ef61a3c8f1f9d5cd0/HANDOFF.md). Snapshots: [v0.20.0](https://github.com/sarakborges/mineclone/blob/bb7fd215117f8ab0148d3ec850244a6516ed1107/HANDOFF.md), [v0.19.1](https://github.com/sarakborges/mineclone/blob/c3fc2646b58971df62b08ae163315cf1320a1273/HANDOFF.md), [v0.19.0](https://github.com/sarakborges/mineclone/blob/ae0aa17a7117caa1cf08f82e51e2cdf4f2491ea0/HANDOFF.md), [direção de arte](https://github.com/sarakborges/mineclone/blob/bbc6505e7deeff2b41125baf9ec32815e4786d69/HANDOFF.md), [v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md), [v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Consultar os snapshots para os detalhes de cada commit e não apagar nem dar baixa nas pendências históricas ao consolidar.

## Regras permanentes

- Quando usuário disser `continua`/`go`: consultar HEAD, VERSION, CI, código e handoff e executar o trabalho fundamentado. `develop` é a branch canônica para mudanças ordinárias. Não alegar publicação, compilação, visual ou gameplay sem confirmação.
- A cada **bloco funcional** de código/asset/testes relevantes, incrementar VERSION conforme SemVer (minor para feature, patch para correção) e atualizar HANDOFF com alterações reais, commits, validações e pendências. Mudança apenas documental não incrementa VERSION. Nesta retomada, fechar a feature de comandos e incrementar versão antes de considerá-la entregue; não fazer bump por este commit apenas de handoff.
- CI exigida: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`; **NÃO executar nem reintroduzir `cargo test` no workflow** sem solicitação. Corrigir todos os warnings reais sem `#[allow]`. CI não substitui QA visual, Windows, FPS ou gameplay; não repetir aviso de teste em toda resposta.
- Direção de arte: modelos/detalhes em cubos/quadrados, superfícies planas, cantos vivos; sem curvas, bevels, rounded boxes ou smoothing. Slime: casca e núcleo cúbicos, face somente no PNG externo por espécie de 64×64, nearest. Não impor 64×64 a outras criaturas; preservar assets entregues pelo usuário, especialmente Bassalt e Brush. Consultar `ARCHITECTURE.md`.
- **Feedback ao usuário:** comunicar resultados concretos e bloqueios com agilidade; não ficar só investigando nem chamar partes publicadas de feature concluída.

## Trabalho mais recente — 16/09/2026: comandos, posicionamento, slimes e auditoria global dos inputs (EM ANDAMENTO)

### Contrato definitivo pedido pelo usuário

1. Substituir `/spawn_creature <id>` por **`/spawn <id>`** para criatura; criar **`/place <id>`** para estrutura do `StructureRegistry`. Autocomplete lista ambos e, no argumento, IDs reais das criaturas/estruturas carregadas por JSON; descrição e comando em **duas linhas**, sem contador `Suggestions 1/1`. Preservar ↑/↓, Tab e primeiro Esc apenas dispensando sugestões, chat aberto.
2. Criatura/estrutura nascem **exatamente onde estava o jogador** (não em oito pontos ao redor). O **jogador** é deslocado para uma posição livre fora do volume ocupado. Preflight antes de mutar: destino carregado, seco, com suporte e sem colisões com blocos, criaturas e estrutura. Se não houver espaço, cancelar inteiramente e escrever no chat **`not enough space to spawn <id>`** ou **`not enough space to place <id>`** (capitalização solicitada pelo usuário para prefixo: `not enough space to [...]`). Não deixar colocações parciais ou jogador preso. A regra antiga de spawn do bicho em um dos oito vizinhos foi expressamente REVOGADA.
3. Remover spawn automático de slimes na criação do mundo; slime deve escolher rumo aleatório ao pular e girar corpo na mesma direção em cada pulo. Verificar movimento e rotação real in-game.
4. Corrigir recuo **de TODOS os campos editáveis**, não só seed e filtro de biomas: seed, ticks por segundo, filtro de biomas, busca do inventário, chat e qualquer outro input descoberto. Texto, cursor e placeholder precisam de distância real da borda, foco/clip consistentes. QA com todos os campos e texto longo.

### O que foi realmente publicado

- [Commit `e8097c5`](https://github.com/sarakborges/mineclone/commit/e8097c5e67903be77fc562c99e00b09b300aa061): aliases de tipos em `src/hud/inventory/search_style.rs` para o warning `clippy::type_complexity` das duas `Query`s de input/placeholder. **Não comprova CI verde.**
- [Commit `9ccf80f`](https://github.com/sarakborges/mineclone/commit/9ccf80f0c3135aea1db1760adff124566830b545): **criado `src/hud/chat/placement.rs`** com `ChatPlacementContext`, preflight dos volumes, busca de posição para deslocar o jogador, rejeição por colisão ou chunks não carregados, mensagem de espaço insuficiente, operação de colocar voxels via `VoxelTopologyRuntime` (iluminação, fluido, remesh), e reservas de posições de criaturas geradas no mesmo frame. Código publicado, **ainda não conectado ao módulo `chat.rs`**.
- [Commit `e3d4533`](https://github.com/sarakborges/mineclone/commit/e3d4533e6d0ed8a3a789573ef61a3c8f1f9d5cd0): em `src/creatures/mod.rs`, função antiga que procurava oito vizinhos foi substituída por **`spawn_creature_at(..., feet)`**, que cria a criatura exatamente na posição passada, deixando o preflight/deslocamento para o contexto do chat. Isto alterou a assinatura pública interna, **mas o chamador em `src/hud/chat.rs` ainda usa a assinatura antiga**.
- Conferência do código atual: `src/hud/chat.rs` começa com `mod autocomplete; mod visual;`, **falta `mod placement;`**; `interpret_chat_submissions` ainda aceita `ParsedLine::SpawnCreature(id)` e passa `VoxelWorld`, `eye` etc. para `spawn_creature_at` usando parâmetros obsoletos. **Incompatibilidade de compilação concreta até corrigir.** A nova lógica de `/place` também está desconectada; `autocomplete.rs`/parser ainda precisam ser alinhados com nomes/IDs dos dois comandos.
- `src/hud/chat/visual.rs` **já possui** menu sem contador `Suggestions x/y` e desenha `suggestion.value` numa linha e `suggestion.description` na linha seguinte, com destaque de seleção. Essa mudança de layout está visível no código atual; ainda falta QA real do layout e verificar se o cabeçalho somente com instruções é apropriado. Não recriar o contador.
- Na inspeção anterior, spawn automático havia sido retirado e direção aleatória + rotação do slime por salto estavam presentes no código. **Validar os respectivos arquivos/commits e comportamento em jogo** antes de declarar finalizado; esta atualização do handoff não alterou essas rotinas.
- Auditoria dos campos: `src/ui/numeric_input.rs` (seed/ticks), `src/screens/settings_screen/spawn_biome_section/layout.rs` e `src/hud/inventory/layout.rs` ainda põem `EditableText` e borda/padding no **mesmo nó**; o estilo compartilhado sozinho não garantiu o afastamento visível reclamado pelo usuário. O chat tem um layout diferente, mas deve ser verificado também. A solução de moldura externa + editor interno foi planejada, **NÃO publicada**. Atenção à compatibilidade dos markers, `InputFocus`, queries, `sync_numeric_input_view`, placeholders e interação de mouse ao refatorar.
- Possível caso adicional a validar em `placement.rs`: `/place` faz preflight de chunks e criaturas, mas não rejeita blocos sólidos existentes (pode sobrescrever terreno intencionalmente ou por acidente), e o loop de `set_block` ignora `None`; esclarecer sem inventar comportamento, e garantir pré-validação/atomicidade segundo o contrato antes de finalizar.

### CI / validação factual

- [Run `35165956285`](https://github.com/sarakborges/mineclone/actions/runs/35165956285) da v0.20.3: consulta posterior confirmou **`completed/failure`**, etapa Clippy falhou e `cargo check` foi **skipped**. O handoff antigo dizia que ainda executava; esse estado ficou obsoleto. Warning `clippy::type_complexity` do inventário identificado em execução anterior e aliases aplicados em `e8097c5`, mas não assumir que esse era o único problema.
- Para os commits `9ccf80f` e `e3d4533`: **nenhuma execução de CI aprovada foi confirmada nesta atualização**, e o descompasso de chamada acima exige correção. Nem teste unitário nem QA em jogo foram executados nesta atualização documental.
- `VERSION` continua `0.20.3` apesar dos dois commits de código. Ao fechar a implementação funcional, incrementar corretamente (feature minor) e registrar commit/version/CI; não atualizar VERSION apenas porque o handoff mudou.

### Próxima execução obrigatória, nesta ordem

1. Integrar `placement.rs` via `mod placement` em `chat.rs`, trocar o contexto antigo por `ChatPlacementContext`, usar preflight/transação e mensagens de erro adequadas; ajustar parser/autocomplete de `SpawnCreature` para `/spawn` e `/place` e provedores do `CreatureRegistry`/`StructureRegistry`, inclusive uso, ID desconhecido, seleção e Tab. Garantir sem criação parcial e controle da posição do jogador após colocar estrutura; revisar limites, suporte e chunks.
2. Resolver integralmente padding em TODOS os campos editáveis (numéricos, busca de biomas, inventário, chat); garantir `InputFocus`, caret, clique, placeholder, borda, texto longo, layout responsivo. Não marcar como resolvido só por tokens de estilo.
3. Verificar alterações de remoção de spawn automático e movimento/rotação dos slimes; não confundir código presente com QA em jogo. Incrementar VERSION por feature concluída e atualizar este HANDOFF com os commits exatos.
4. Rodar CI **Clippy -D warnings + cargo check** no código integrado; corrigir todos os erros/warnings; não rodar testes unitários. Depois QA em gameplay de `/spawn`, `/place` (espaço livre/lotado, id válido/inválido, estrutura grande, jogador encurralado, colisões) e painel de autocomplete/inputs. Relatar apenas o que for observado.

## Histórico recente — implementação anterior (preservado integralmente no snapshot acima)

### v0.20.3 — núcleo maior e face extraída da geometria do slime

[Implementação `ab0bc6c`](https://github.com/sarakborges/mineclone/commit/ab0bc6c35200078c2c6c46c8fdd9b24ce5482531), [migração estrutural validada `35165913764`](https://github.com/sarakborges/mineclone/actions/runs/35165913764), [documentação `1097ce1`](https://github.com/sarakborges/mineclone/commit/1097ce1b67c854b92acc8481293588ce2514a1a9). `slime.glb` passou a duas meshes somente (casca 0.96×0.90×0.96, núcleo ampliado 0.36×0.40×0.36 → 0.58×0.62×0.58), materiais `SlimeShell` e `SlimeCore`. Olhos, boca, bochechas, brilhos deixaram de ser geometria e estão no tile frontal (2,0) dos PNGs externos Meadow/Ember 64×64; tiles 0,0 corpo e 1,0 núcleo. Sem imagens embutidas; seis clips e collider preservados. **QA visual, UV, alpha, animações e colisão em jogo ainda aberto.**

### v0.20.2 — busca inventário e CI antiga

[Estilo criado `18c342e`](https://github.com/sarakborges/mineclone/commit/18c342e0f67e1f6c3c4a8fb7a580e1fd9d154da4), [integrado `77b84a8`](https://github.com/sarakborges/mineclone/commit/77b84a8803c7c98e09a38bfd48e082768f7ea36f), [ajuste `bc48e95`](https://github.com/sarakborges/mineclone/commit/bc48e958126e05689a9f852f31909c52f3f7fe35), [bump `15749ea`](https://github.com/sarakborges/mineclone/commit/15749ea923b968fc4ea50746cbb7d91f4f8f9b83). CI antiga [run `35165096445`](https://github.com/sarakborges/mineclone/actions/runs/35165096445) passou Clippy/Check; **não certifica alterações seguintes nem aparência visual**.

### v0.20.1 — estilo compartilhado e lints do autocomplete

Tokens `src/ui/text_input.rs` [`a989e6c`](https://github.com/sarakborges/mineclone/commit/a989e6c5e2088000c324518e74ae4a69a3971779); numéricos [`5d343c5`](https://github.com/sarakborges/mineclone/commit/5d343c5f8ddd973af1d516e4ec9659b4bf6927d0); filtro de biomas [`25ffc67`](https://github.com/sarakborges/mineclone/commit/25ffc67b025ec56e637c97a153ee1b2a0ad0e9a4) e foco [`d725adb`](https://github.com/sarakborges/mineclone/commit/d725adb354800d1a03b241738e408bb17aa9ab70); chat [`8ac2a72`](https://github.com/sarakborges/mineclone/commit/8ac2a7268cdf3c54dd09c1008171389bd4b9259e); lint `rfind` [`f89bee2`](https://github.com/sarakborges/mineclone/commit/f89bee26313e4cf77862769669758e30da3029cf); [bump `21c84ce`](https://github.com/sarakborges/mineclone/commit/21c84ce9fbb9affea6db6f40efc57623aaafcf47). Runs falhos de autocomplete [`35163973182`](https://github.com/sarakborges/mineclone/actions/runs/35163973182) e [`35164333148`](https://github.com/sarakborges/mineclone/actions/runs/35164333148). QA visual aberto.

### v0.20.0 — autocomplete original

[Parser/registro `298df864`](https://github.com/sarakborges/mineclone/commit/298df864d8d1f1254d9209116a1d23c96109c57e), [integração `b1d1bb0`](https://github.com/sarakborges/mineclone/commit/b1d1bb0c15b5f61aee160eed88a8a57bf8a8b38d), [painel `85f7727`](https://github.com/sarakborges/mineclone/commit/85f772728f6181e260efa9b30dbc9774c17790ae), [versão `ad9156c`](https://github.com/sarakborges/mineclone/commit/ad9156cbee5c9e080b238c67e8f9f762840d7755). `/` mostra comandos; ↑↓ selecionam, Tab completa token no caret e preserva sufixo, primeiro Esc cancela menu sem fechar chat e segundo Esc fecha; sugestões por posição de parâmetro, não durante IME. Inicialmente só `/spawn_creature`; **substituir por `/spawn` e `/place` conforme novo pedido**. QA do teclado aberto.

### v0.19.1 / v0.19.0 / v0.18.0

PNG duplicado `assets/models/creatures/slime/slime_skin_64.png` removido em [`3062fec`](https://github.com/sarakborges/mineclone/commit/3062fecd9413e3ff8cb4173e21cb7005ad7bc58d). Lints antigos da textura corrigidos em [`57ec510`](https://github.com/sarakborges/mineclone/commit/57ec510390ddc9aa8de919d64c101f9e89e92434) e [`ac7538a`](https://github.com/sarakborges/mineclone/commit/ac7538a9a3e87da4408aef98d661ab0d79ab60bd). Slime cúbico e externalização PNG/JSON inicialmente em [`0ae0f0ba`](https://github.com/sarakborges/mineclone/commit/0ae0f0ba4723411ccec256d4d00f64afa282ce1e), antes da extração posterior da face. Grass (`Grass`), Bassalt (`asteria:bassalt`) e Brush/tinta overlay no [commit `9cbee3e`](https://github.com/sarakborges/mineclone/commit/9cbee3edf5a3d665f2fe95f2809dbcddf7f2b7ce). Preservar texturas do usuário; QA de gameplay desses assets ainda aberto. Históricos completos nos snapshots do início.

## Nove bugs do mundo — TODOS ABERTOS até confirmação em gameplay

| # | Prioridade | Sintoma |
|---|---|---|
| 1 | P0 | Chunks pretos e costuras de iluminação/AO; investigar duração e FPS. |
| 2 | P0 | Rios terminam no nada; conectividade nascente→oceano por seeds. |
| 3 | P0 | Depois de 2000 blocos aparecem apenas Plains/Mountains; comparar biomas lógicos/render. |
| 4 | P0 | Margens de rios afundam antes da água; água aparentemente suspensa. |
| 5 | P1 | Confluências rio–lago abruptas. |
| 6 | P1 | Túneis com paredes retas e paredes extras nos cruzamentos com rios. |
| 7 | P1 | Lua parece seguir a câmera. |
| 8 | P1 | Lua visível durante o dia. |
| 9 | P1 | Target highlight não cobre todas as camadas de textura. |

Outros abertos: fogColor artístico, benchmark FPS/streaming, held item/R, HUD/avatar/status do jogador, target HUD. Único fechamento visual antigo confirmado pelo usuário: céu por bioma `.24`. **Não dar baixa por CI verde.**
