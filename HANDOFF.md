# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz `0.20.1`**, independente do `Cargo.toml` `0.10.16`. Consultar HEAD/VERSION antes de alterar. Snapshot completo imediatamente anterior deste handoff: [v0.20.0](https://github.com/sarakborges/mineclone/blob/7311927eaed015c72d076de0a43b2560e6c08431/HANDOFF.md) (se hash não resolver, [snapshot anterior confirmado](https://github.com/sarakborges/mineclone/blob/fa643508f517f530e75cc69bf2a7967dd2c7a904/HANDOFF.md)); histórico mais antigo: [v0.19.1](https://github.com/sarakborges/mineclone/blob/c3fc2646b58971df62b08ae163315cf1320a1273/HANDOFF.md), [v0.19.0](https://github.com/sarakborges/mineclone/blob/ae0aa17a7117caa1cf08f82e51e2cdf4f2491ea0/HANDOFF.md), [direção de arte](https://github.com/sarakborges/mineclone/blob/bbc6505e7deeff2b41125baf9ec32815e4786d69/HANDOFF.md), [v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md), [v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Não fechar bugs ou apagar histórico ao resumir.

## Regras permanentes

- `continua`/`go`: investigar HEAD, VERSION, CI, código e handoff e fazer o trabalho fundamentado. Pode atualizar `develop` para mudanças ordinárias; regra de branches isolados se limitou às features antigas de criaturas/chat/editor.
- Incrementar VERSION conforme SemVer a cada bloco de código/asset jogável/testes relevantes (minor feature, patch fix); documentação isolada não altera versão. Registrar sempre mudanças reais, commits, validação e pendências no handoff, sem confundir commit e CI com gameplay. Preservar PNG/GLB fornecidos pelo usuário; consultar `ARCHITECTURE.md`.
- **CI solicitada pelo usuário:** somente `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`, jamais reintroduzir execução de `cargo test` sem pedido. Testes podem permanecer no código e Clippy os compila. Corrigir warnings sem `#[allow]`. CI verde não confirma runtime, Windows, visuais ou FPS. Não fazer polling sem benefício.
- **Direção de arte:** modelos e todos os detalhes exclusivamente quadrados/cúbicos, planos retos/cantos vivos, sem curvas/bevel/smoothing. Slime: corpo, núcleo, olhos, boca e detalhes retangulares, skin 64×64 com nearest. Não impor 64×64 a outras espécies sem pedido.

## Bloco 16/09/2026 — v0.20.1: corrigir estilização dos campos de input

**Feedback do usuário recebido duas vezes:** inputs estão com estilização quebrada. Inspeção de `numeric_input.rs`, `spawn_biome_section/{layout,systems}.rs` e `hud/chat/visual.rs` confirmou estilos locais divergentes, placeholder de bioma inserido como filho no fluxo do editor em vez de overlay, e campo chat sem moldura/clip horizontal apropriados. Solução gravada na `develop`, mas **qualidade visual real ainda precisa de QA runtime**.

- [`a989e6c`](https://github.com/sarakborges/mineclone/commit/a989e6c5e2088000c324518e74ae4a69a3971779), `src/ui/text_input.rs`: tokens compartilhados `INPUT_FILL`, `INPUT_BORDER`, `INPUT_RADIUS`, `INPUT_PADDING_X`, `input_border(focused)`; helper de leitura do editor mantido. Teste unitário no código, fora da execução CI.
- [`5d343c5`](https://github.com/sarakborges/mineclone/commit/5d343c5f8ddd973af1d516e4ec9659b4bf6927d0), `src/ui/numeric_input.rs`: aplica tokens ao campo numérico de seed/ticks, borda conforme foco e `Overflow::clip()` para evitar vazamento do texto; preserva comportamento de edição/validação.
- [`25ffc67`](https://github.com/sarakborges/mineclone/commit/25ffc67b025ec56e637c97a153ee1b2a0ad0e9a4), `src/screens/settings_screen/spawn_biome_section/layout.rs`: padroniza input de busca, coloca texto de dica em overlay `position_type: Absolute`, fora de layout flex, ajusta fonte, clip e padding.
- [`8ac2a72`](https://github.com/sarakborges/mineclone/commit/8ac2a7268cdf3c54dd09c1008171389bd4b9259e), `src/hud/chat/visual.rs`: estiliza a caixa do chat conforme tokens, borda real e clipping, limita editor horizontalmente, sem mudar histórico ou painel de autocomplete.
- [`d725adb`](https://github.com/sarakborges/mineclone/commit/d725adb354800d1a03b241738e408bb17aa9ab70), `src/screens/settings_screen/spawn_biome_section/systems.rs`: sincroniza cor de foco usando tokens compartilhados.
- [`f89bee2`](https://github.com/sarakborges/mineclone/commit/f89bee26313e4cf77862769669758e30da3029cf), `src/hud/chat/autocomplete.rs`: novo lint da segunda CI do autocomplete exigia `rfind` em vez de `filter().next_back()`; corrigido sem `allow` e sem alterar algoritmo.
- [`21c84ce`](https://github.com/sarakborges/mineclone/commit/21c84ce9fbb9affea6db6f40efc57623aaafcf47): `VERSION` `0.20.0 → 0.20.1` (patch de estilo/compilação).

**CI:** primeiro run de autocomplete [`35163973182`](https://github.com/sarakborges/mineclone/actions/runs/35163973182) falhou no lint `.last()`. Segundo [`35164333148`](https://github.com/sarakborges/mineclone/actions/runs/35164333148) falhou no lint `filter().next_back()`, log recomendou `rfind`; `cargo check` pulado em ambos. A correção `rfind` está incluída na v0.20.1. **Run da v0.20.1 [`35164919545`](https://github.com/sarakborges/mineclone/actions/runs/35164919545): in_progress na consulta de 17/09 UTC, Clippy in_progress, cargo check pending. Não declarar sucesso até obter estado final.** Nenhum teste unitário foi reintroduzido no workflow.

**QA visual/manual aberto:** captura de campos numéricos em seed/ticks, campo de busca de biomas fechado/aberto/filtrando, chat aberto com e sem sugestões, foco, placeholder, seleção, caret em começo/fim, texto longo, scaling/resize e legibilidade. Não afirmar estilização aprovada apenas por inspeção do código ou CI verde.

## Bloco anterior — v0.20.0: autocomplete de comandos/parâmetros no chat

**Contrato:** digitando `/` com chat aberto aparece menu. ↑/↓ mudam seleção (sem mover caret no editor enquanto menu ativo); Tab completa token sob cursor sem enviar mensagem; com menu aberto, primeiro Esc só dispensa sugestões sem limpar texto, desfocar ou fechar chat; segundo Esc fecha chat pelo comportamento existente. Cada posição de parâmetro tem provedor de sugestões por tipo. Não mostrar durante composição IME. Limite anterior de 256 caracteres e histórico de 15 linhas preservados.

- `src/hud/chat/autocomplete.rs`: [feature `298df864`](https://github.com/sarakborges/mineclone/commit/298df864d8d1f1254d9209116a1d23c96109c57e) centraliza registro/assinaturas, parser e suggestion providers; atualmente único comando existente é `/spawn_creature <id>`, com sugestões de IDs reais de `CreatureRegistry` dos JSONs (nome localizado e filtro por ID completo ou curto). Registro tipado extensível para outros parâmetros e comandos; novos handlers ainda precisam ser registrados. Substitui apenas token no caret e preserva sufixos/argumentos; completa comando com espaço quando preciso. Testes de parser/token/caret/completar foram adicionados ao código, não à execução de CI.
- `src/hud/chat.rs`: [feature `b1d1bb0`](https://github.com/sarakborges/mineclone/commit/b1d1bb0c15b5f61aee160eed88a8a57bf8a8b38d) integra cadeia de sistemas, estado de autocomplete e primeiro Esc sem fechar chat. Limpa estado ao pausar/reabrir. Preserva texto normal e comando spawn.
- `src/hud/chat/visual.rs`: [feature `85f7727`](https://github.com/sarakborges/mineclone/commit/85f772728f6181e260efa9b30dbc9774c17790ae) cria painel acima do campo com seleção destacada, instruções e janela até 7 sugestões, atualiza por revisão, não a cada frame.
- Versão `0.19.1 → 0.20.0` em [`ad9156c`](https://github.com/sarakborges/mineclone/commit/ad9156cbee5c9e080b238c67e8f9f762840d7755). O lint `.last()` foi trocado por `.next_back()` em [`bb7fd215`](https://github.com/sarakborges/mineclone/commit/bb7fd215117f8ab0148d3ec850244a6516ed1107), e depois por `.rfind()` na v0.20.1 acima, em resposta aos lints reais.

**QA manual aberto:** abrir chat com T e digitar `/`, completar comando e depois id, testar setas/Tab/Esc 1x/2x, foco/caret/IME, digitação em meio da linha, prefixos curtos e completos, nomes longos, texto normal e comando executado. Código/CI não substituem gameplay.

## Bloco v0.19.1: remoção de PNG duplicado e correção de Clippy

PNG legado `assets/models/creatures/slime/slime_skin_64.png` excluído em [`3062fecd`](https://github.com/sarakborges/mineclone/commit/3062fecd9413e3ff8cb4173e21cb7005ad7bc58d), deixando GLB, gerador e collider JSON na pasta do modelo; texturas reais exclusivamente externas nos PNGs por espécie. Falha antiga Clippy de visibilidade `BrushTintIcon` e complexidade de cache em run `35162431472`; correções sem suppressions [`57ec510`](https://github.com/sarakborges/mineclone/commit/57ec510390ddc9aa8de919d64c101f9e89e92434), [`ac7538a`](https://github.com/sarakborges/mineclone/commit/ac7538a9a3e87da4408aef98d661ab0d79ab60bd), bump patch [`92c4940`](https://github.com/sarakborges/mineclone/commit/92c494028a71a61c6858cc0892313d07ca79c89d). Não fechar QA só pela CI.

## Bloco v0.19.0: slime quadrado e PNGs por JSON

[`0ae0f0ba`](https://github.com/sarakborges/mineclone/commit/0ae0f0ba4723411ccec256d4d00f64afa282ce1e): `slime.glb` com seis meshes cúbicas, sem `rounded_box`, `ellipsoid`, `mouth_curve`, imagens glTF embutidas ou referência a texturas; seis animações, materiais, `SlimeRoot`, collider AABB e hierarquia preservados. As skins distintas 64×64 de Meadow/Ember ficam em `assets/textures/creatures/{name}.png`, caminho do mapa `textures` nos JSONs `data/creatures/slimes/{meadow,ember}.json`. `src/content/creature.rs` valida, `src/creatures/visual.rs` aplica por nome de material com tint e cache por cor/imagem; nearest global. [Documentação](https://github.com/sarakborges/mineclone/blob/develop/docs/creature-textures.md). Validação estrutural gerador sucesso run `35162342535`; QA runtime de aparência, alpha, UV, animação/collider/FPS **pendente**.

## Bloco v0.18.0: Grass/Bassalt/Brush

Preservar PNGs do usuário para bassalt, brush e brush-tint. `asteria:grass` exibe `Grass`; `asteria:bassalt` está em `stone_blocks`; brush tem base sem tinta em Clear, overlay tingido quando cor selecionada (hotbar, inventário, catálogo e arrasto), Clear remove dye e seleção aplica dye compatível. Código [`9cbee3e`](https://github.com/sarakborges/mineclone/commit/9cbee3edf5a3d665f2fe95f2809dbcddf7f2b7ce), QA ainda aberto. Features antigas criaturas/chat/editor mescladas [`b421c5e`](https://github.com/sarakborges/mineclone/commit/b421c5e248738f9f5f42cc122ea5973890f0a397), PRs #9/#10, hidrologia `.42–.51`, iluminação `.41`; features experimentais de iluminação não integradas. Ver snapshots para detalhes integrais.

## Nove bugs de mundo — TODOS ABERTOS até confirmação em gameplay

| # | Prio | Sintoma |
|---|---|---|
| 1 | P0 | Chunks pretos e costuras de iluminação/AO, medir transiente/persistência e FPS. |
| 2 | P0 | Rios terminam no nada; conectividade nascente→oceano por seeds. |
| 3 | P0 | Após 2000 blocos, só Plains/Mountains, comparar IDs de bioma e render. |
| 4 | P0 | Margens de rio afundam antes da água; evitar água suspensa. |
| 5 | P1 | Confluências rio–lago abruptas. |
| 6 | P1 | Túneis com paredes retas e paredes extras cruzando rios. |
| 7 | P1 | Lua parece acompanhar câmera. |
| 8 | P1 | Lua visível durante o dia. |
| 9 | P1 | Target highlight não cobre todas as camadas da textura. |

Outras pendências: fogColor artístico, benchmark FPS/streaming, held/R, HUD/avatar/status do jogador e target HUD. Único fechamento visual confirmado anteriormente pelo usuário: céu por bioma `.24`. Não fechar bugs por CI verde.

## Próxima execução

1. Conferir resultado de Clippy + Check da v0.20.1 SHA `21c84ce` [run `35164919545`](https://github.com/sarakborges/mineclone/actions/runs/35164919545); se falhar, ler o log e corrigir warning/erro concreto sem executar testes na CI.
2. QA manual visual dos três inputs e teclado do autocomplete; não confundir lint com jogo rodando.
3. QA criaturas/slime/brush; retomar P0 mundo rios/biomas/luz/margens, depois P1, registrando VERSION/handoff por bloco.
