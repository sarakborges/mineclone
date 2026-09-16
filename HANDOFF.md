# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz `0.20.0`**, independente do `Cargo.toml` `0.10.16`. Consultar HEAD/VERSION antes de alterar. Snapshot completo imediatamente anterior deste handoff com histórico de implementações: [fa643508](https://github.com/sarakborges/mineclone/blob/fa643508f517f530e75cc69bf2a7967dd2c7a904/HANDOFF.md). Histórico mais antigo: [v0.19.1](https://github.com/sarakborges/mineclone/blob/c3fc2646b58971df62b08ae163315cf1320a1273/HANDOFF.md), [v0.19.0](https://github.com/sarakborges/mineclone/blob/ae0aa17a7117caa1cf08f82e51e2cdf4f2491ea0/HANDOFF.md), [direção de arte](https://github.com/sarakborges/mineclone/blob/bbc6505e7deeff2b41125baf9ec32815e4786d69/HANDOFF.md), [v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md), [v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Não fechar bugs ou apagar histórico ao resumir.

## Regras permanentes

- `continua`/`go`: investigar HEAD, VERSION, CI, código e handoff e fazer o trabalho fundamentado. Pode atualizar `develop` para mudanças ordinárias; regra de branches isolados se limitou às features antigas de criaturas/chat/editor.
- Incrementar VERSION conforme SemVer a cada bloco de código/asset jogável/testes relevantes (minor feature, patch fix); documentação isolada não altera versão. Registrar sempre mudanças reais, commits, validação e pendências no handoff, sem confundir commit e CI com gameplay. Preservar PNG/GLB fornecidos pelo usuário; consultar `ARCHITECTURE.md`.
- **CI solicitada pelo usuário:** somente `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`, jamais reintroduzir execução de `cargo test` sem pedido. Testes podem permanecer no código e Clippy os compila. Corrigir warnings sem `#[allow]`. CI verde não confirma runtime, Windows, visuais ou FPS. Não fazer polling sem benefício.
- **Direção de arte:** modelos e todos os detalhes exclusivamente quadrados/cúbicos, planos retos/cantos vivos, sem curvas/bevel/smoothing. Slime: corpo, núcleo, olhos, boca e detalhes retangulares, skin 64×64 com nearest. Não impor 64×64 a outras espécies sem pedido.

## Bloco 16/09/2026 — v0.20.0: autocomplete de comandos/parâmetros no chat

**Contrato:** digitando `/` com chat aberto aparece menu. ↑/↓ mudam seleção (sem mover caret no editor enquanto menu ativo); Tab completa token sob cursor sem enviar mensagem; com menu aberto, primeiro Esc só dispensa sugestões sem limpar texto, desfocar ou fechar chat; segundo Esc fecha chat pelo comportamento existente. Cada posição de parâmetro tem provedor de sugestões por tipo. Não mostrar durante composição IME. Limite anterior de 256 caracteres e histórico de 15 linhas preservados.

- `src/hud/chat/autocomplete.rs`: [feature `298df864`](https://github.com/sarakborges/mineclone/commit/298df864d8d1f1254d9209116a1d23c96109c57e) centraliza registro/assinaturas, parser e suggestion providers; atualmente único comando existente é `/spawn_creature <id>`, com sugestões de IDs reais de `CreatureRegistry` dos JSONs (nome localizado e filtro por ID completo ou curto). Registro tipado extensível para outros parâmetros e comandos; novos handlers ainda precisam ser registrados. Substitui apenas token no caret e preserva sufixos/argumentos; completa comando com espaço quando preciso. Testes de parser/token/caret/completar foram adicionados ao código, não à execução de CI.
- `src/hud/chat.rs`: [feature `b1d1bb0`](https://github.com/sarakborges/mineclone/commit/b1d1bb0c15b5f61aee160eed88a8a57bf8a8b38d) integra cadeia de sistemas, estado de autocomplete e primeiro Esc sem fechar chat. Limpa estado ao pausar/reabrir. Preserva texto normal e comando spawn.
- `src/hud/chat/visual.rs`: [feature `85f7727`](https://github.com/sarakborges/mineclone/commit/85f772728f6181e260efa9b30dbc9774c17790ae) cria painel acima do campo com seleção destacada, instruções e janela até 7 sugestões, atualiza por revisão, não a cada frame.
- Versão `0.19.1 → 0.20.0` em [`ad9156c`](https://github.com/sarakborges/mineclone/commit/ad9156cbee5c9e080b238c67e8f9f762840d7755).

**Validação e correção:** primeira [CI `35163973182`](https://github.com/sarakborges/mineclone/actions/runs/35163973182) **COMPLETED FAILURE**: Clippy apontou exclusivamente `clippy::double_ended_iterator_last` em `src/hud/chat/autocomplete.rs:147` (`.last()` em `DoubleEndedIterator`); cargo check skipped. Corrigido por substituição de `.last()` por `.next_back()` em [commit `bb7fd215`](https://github.com/sarakborges/mineclone/commit/bb7fd215117f8ab0148d3ec850244a6516ed1107), sem suppression, no mesmo bloco de feature `0.20.0`. Novo run automático [CI `35164333148`](https://github.com/sarakborges/mineclone/actions/runs/35164333148) para SHA `bb7fd215`: **in_progress na leitura inicial, resultado de Clippy e Check ainda NÃO validado**. Consultar resultado quando estiver disponível; se falhar, ler log e corrigir motivo concreto, nunca afirmar compilação aprovada antecipadamente. Esta documentação usa `paths-ignore: HANDOFF.md`, então SHA final de código é `bb7fd215`.

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

1. Conferir CI do SHA de código `bb7fd215` [run `35164333148`](https://github.com/sarakborges/mineclone/actions/runs/35164333148), verificar Clippy + Check, corrigir novos erros reais se surgirem sem acrescentar cargo test.
2. QA manual autocomplete e criaturas/slime/brush; não confundir lint com jogo rodando.
3. Retomar P0 mundo rios/biomas/luz/margens, depois P1, registrando nova versão e handoff por bloco.
