# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz: `0.20.0`**; versão independente de `Cargo.toml`: `0.10.16`. Consultar `develop` HEAD e `VERSION` antes de modificar. Histórico documental integral preservado no [handoff v0.19.1 antes do autocomplete](https://github.com/sarakborges/mineclone/blob/c3fc2646b58971df62b08ae163315cf1320a1273/HANDOFF.md), [v0.19.0](https://github.com/sarakborges/mineclone/blob/ae0aa17a7117caa1cf08f82e51e2cdf4f2491ea0/HANDOFF.md), [direção de arte](https://github.com/sarakborges/mineclone/blob/bbc6505e7deeff2b41125baf9ec32815e4786d69/HANDOFF.md), [v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md) e [v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Não confundir condensação de texto com fechamento de bugs ou validação.

## Regras permanentes

- `continua`/`go`: inspecionar HEAD, VERSION, CI, código e handoff e executar mudanças justificáveis; alterações ordinárias podem ser gravadas diretamente em `develop`. A regra histórica de branches isolados dizia respeito apenas às features de criaturas/chat/editor anteriores.
- A cada bloco de mudanças jogáveis/código/assets/testes significativos, incrementar `VERSION` segundo SemVer (feature compatível = minor, correção = patch); documentação isolada não eleva versão. Atualizar este handoff na mesma sessão com commits, evidências de validação e pendências. Preservar arquivos de assets fornecidos pelo usuário; não alterar PNG/GLB originais apenas para conectar. `ARCHITECTURE.md` é referência de ownership/change-driven.
- **CI solicitada pelo usuário:** `.github/workflows/ci.yml` executa apenas `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`, **SEM `cargo test`**. Os testes no código continuam; Clippy compila targets de testes sem executá-los. Não adicionar `cargo test`, nem suprimir warnings para CI passar. CI verde não equivale a teste visual, gameplay, Windows ou FPS. Diagnosticar erros concretos pelo log, sem polling repetitivo inútil.
- **Arte:** todos os modelos e detalhes devem ter formas quadradas/cúbicas, arestas e cantos nítidos, sem bevels, esferas, elipsoides, bocas/olhos arredondados ou suavização. Slime: corpo, núcleo, olhos, boca, bochechas e reflexos quadrados; skin **64×64** pixelada com nearest; não impor 64×64 a outras criaturas sem pedido.

## Bloco 16/09/2026 — v0.20.0: autocomplete de comandos e parâmetros no chat

**Pedido do usuário:** quando digitar `/` no chat, mostrar sugestões de comandos; ↑/↓ escolhem; Tab completa; o primeiro Esc com sugestões abertas fecha **apenas o autocomplete**, preservando texto, foco, cursor e chat. Sugestões também para cada parâmetro, conforme seu tipo e posição. O segundo Esc, com autocomplete fechado, mantém o comportamento anterior de fechar o chat.

- `src/hud/chat/autocomplete.rs` [commit `298df864`](https://github.com/sarakborges/mineclone/commit/298df864d8d1f1254d9209116a1d23c96109c57e): nova fonte única de definição de comandos com nome, usage, descrição, esquema de parâmetros e identificador de execução. O parser e o mecanismo de sugestões consultam essa mesma definição para evitar listas paralelas. No registro atual existe **somente** `/spawn_creature <id>`; o parâmetro `CreatureId` sugere IDs reais de `CreatureRegistry` carregados dos JSONs, exibindo nomes localizados, inclusive futuros registros. Filtra prefixos do ID completo ou de sufixo sem `asteria:`; apresenta sugestões em ordem alfabética. O esquema é extensível por parâmetros, mas novos comandos continuam exigindo acrescentar definições e handlers apropriados.
- `src/hud/chat.rs` [commit `b1d1bb0`](https://github.com/sarakborges/mineclone/commit/b1d1bb0c15b5f61aee160eed88a8a57bf8a8b38d): conecta recurso de estado e sistema de autocomplete à cadeia de Update; Escape cancela sugestões antes de encerrar chat. Fecha/limpa sugestões ao enviar mensagem, fechar chat, pausar ou reabrir. Usage do parser usa a definição de comando. Texto normal, mensagens e spawn existentes preservados.
- `src/hud/chat/visual.rs` [commit `85f7727`](https://github.com/sarakborges/mineclone/commit/85f772728f6181e260efa9b30dbc9774c17790ae): painel dedicado acima do campo de texto, seleção destacada, instruções de teclado, janela visível de até sete candidatos, atualizada por revisão de estado (não recriada constantemente por frame). Mantém limite anterior de quinze linhas do histórico.
- Completa somente o token sob o caret, preservando outros argumentos; ao completar nome de comando acrescenta espaço caso necessário para ativar sugestões do primeiro argumento. Suprime sugestões durante composição IME, respeita limite de 256 caracteres e não envia comando apenas por Tab. `EditableText` permanece nativo. Testes unitários no código cobrem parsing, identificação de parâmetro/caret e preservação de sufixo; não executá-los no workflow de CI.
- `VERSION` `0.19.1 → 0.20.0` em [commit `ad9156c`](https://github.com/sarakborges/mineclone/commit/ad9156cbee5c9e080b238c67e8f9f762840d7755).

**Validação:** [CI v0.20.0, run `35163973182`](https://github.com/sarakborges/mineclone/actions/runs/35163973182) iniciada automaticamente no push da versão. Na leitura inicial `Clippy` em execução e `Check` pendente; nenhum resultado final confirmado ainda. Consultar run final e corrigir erros efetivos sem retirar warnings. QA no jogo ainda aberto para teclado físico/IME, foco, caret com texto no meio, nomes longos e ambos os slimes/cores. Não afirmar compile ou gameplay concluídos antes da prova.

## Bloco anterior — v0.19.1: textura duplicada e correções de Clippy

O PNG obsoleto `assets/models/creatures/slime/slime_skin_64.png`, não referenciado, foi excluído em [`3062fecd`](https://github.com/sarakborges/mineclone/commit/3062fecd9413e3ff8cb4173e21cb7005ad7bc58d): a pasta de modelo contém GLB, script gerador e collider JSON, mas **nenhuma textura**. Texturas da espécie permanecem externas em `assets/textures/creatures/`, selecionadas nos JSONs. Run antigo `35162431472` falhou em Clippy por `BrushTintIcon` privado e `type_complexity` do cache de materiais. Correções sem suppressions: [`57ec510`](https://github.com/sarakborges/mineclone/commit/57ec510390ddc9aa8de919d64c101f9e89e92434), [`ac7538a`](https://github.com/sarakborges/mineclone/commit/ac7538a9a3e87da4408aef98d661ab0d79ab60bd), bump `0.19.1` [`92c4940`](https://github.com/sarakborges/mineclone/commit/92c494028a71a61c6858cc0892313d07ca79c89d). Handoff integral desse bloco no snapshot vinculado no topo. Não fechar QA de brush/slime por CI verde.

## Bloco anterior — v0.19.0: slime cúbico e texturas por JSON

[`0ae0f0ba`](https://github.com/sarakborges/mineclone/commit/0ae0f0ba4723411ccec256d4d00f64afa282ce1e) substituiu `rounded_box`/`ellipsoid`/`mouth_curve` por seis meshes de cubos de cantos vivos em `slime.glb` gerado por `generate_slime.py`, preservando seis animações, nomes de materiais, `SlimeRoot`, collider AABB e hierarquia. O GLB **não contém imagens ou referências glTF a texturas**. `assets/textures/creatures/meadow_slime.png` e `ember_slime.png` são PNGs externos 64×64, independentes por espécie, associados pelo mapa `textures` de `data/creatures/slimes/meadow.json` e `ember.json`. `src/content/creature.rs` valida caminhos; `src/creatures/visual.rs` usa AssetServer por JSON e materiais clonados/cacheados por cor+PNG; nearest global em `main.rs`. [Documentação de texturas](https://github.com/sarakborges/mineclone/blob/develop/docs/creature-textures.md). Geração estrutural [run `35162342535`](https://github.com/sarakborges/mineclone/actions/runs/35162342535) sucesso; aparência, alpha, UV, animação/collider e FPS ainda exigem QA runtime. Não regenerar o PNG legado na pasta do modelo.

## Bloco anterior — v0.18.0: Grass, Bassalt e Brush

Preservar PNGs originais fornecidos pelo usuário para `bassalt`, `brush` e `brush-tint`. ID `asteria:grass` exibe `Grass` sem trocar ID; `asteria:bassalt` pertence a `stone_blocks`. Brush usa `icon` base e `tintIcon` opcional; base só em Clear e segunda camada colorida na seleção, em hotbar, inventário, catálogo e arrasto. Clear remove dye e cor aplica dye em blocos compatíveis. Código [`9cbee3e`](https://github.com/sarakborges/mineclone/commit/9cbee3edf5a3d665f2fe95f2809dbcddf7f2b7ce). QA visual/gameplay aberto.

**Histórico anterior:** criaturas Meadow/Ember, chat T/Enter/Esc e `/spawn_creature`, editor nativo EditableText, chat 64 mensagens/15 linhas, integração [`b421c5e`](https://github.com/sarakborges/mineclone/commit/b421c5e248738f9f5f42cc122ea5973890f0a397), PRs #9/#10, hidrologia `.42–.51`, iluminação `.41`. Features experimentais de luz óptica/direcional e diagnósticos não integradas. Snapshots históricos completos no topo.

## Nove defeitos de mundo — TODOS ABERTOS até confirmação runtime

| # | Prio | Sintoma |
|---|---|---|
| 1 | P0 | Chunks pretos e costuras de iluminação/AO; medir transiente/persistência e FPS. |
| 2 | P0 | Rios terminam no nada; verificar conectividade nascente→oceano por seeds. |
| 3 | P0 | Após 2000 blocos, só Plains/Mountains; comparar IDs de bioma e render. |
| 4 | P0 | Margens de rio afundam antes da água; evitar água suspensa. |
| 5 | P1 | Confluências rio–lago abruptas. |
| 6 | P1 | Túneis de paredes retas e paredes extras em cruzamentos com rios. |
| 7 | P1 | Lua parece acompanhar câmera. |
| 8 | P1 | Lua visível durante o dia. |
| 9 | P1 | Target highlight não cobre todas as camadas da textura. |

Outras pendências: fogColor artístico, benchmark FPS/streaming, held/R, HUD/avatar/status do jogador e target HUD. Único fechamento visual confirmado anteriormente pelo usuário: céu por bioma `.24`. Não fechar bugs por CI verde.

## Próxima execução

1. Conferir resultados de Clippy + Check do run `35163973182` para `ad9156c` e PR da mesma versão; ler log e corrigir erros/warnings concretos sem reintroduzir testes unitários na CI. Se correção alterar código fora do bloco, bump patch SemVer e atualizar handoff.
2. QA do autocomplete no jogo: `/`, prefixos, setas, Tab no comando e parâmetro, Esc uma e duas vezes, IME, textos em meio de linha, retorno ao histórico e foco; conferir slimes e materiais externos em jogo.
3. QA Bassalt/Grass/Brush, depois P0 rios/biomas/luz/margens e P1. VERSION e handoff sempre atualizados por bloco.
