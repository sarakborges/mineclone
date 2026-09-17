# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **VERSION raiz: `0.20.3` (ainda não incrementada; bloco de feature/QA aberto); `Cargo.toml` usa `0.10.16` independentemente.** Último commit de código confirmado nesta atualização: [`8db1614`](https://github.com/sarakborges/mineclone/commit/8db1614a1556733885924ec087cc492f7d4b2a1d). Não anunciar CI verde, QA de Windows, física ou UI sem evidência.

## Histórico integral, sem descarte

- [Handoff integral anterior (`fafba08e`)](https://github.com/sarakborges/mineclone/blob/fafba08e92c57a883a689cb694ead6b6e42116d5/HANDOFF.md) registra migração de estruturas, versionamento, alterações de slimes, `/spawn`/`/place`, UI e pendências anteriores; **é fonte histórica vinculante para tudo que não foi explicitamente encerrado abaixo**.
- [Handoff detalhado prévio (`500663e`)](https://github.com/sarakborges/mineclone/blob/500663ea569e0a46361692623a5a3fa68d8042ad/HANDOFF.md), [pré-migração (`dfa28c5`)](https://github.com/sarakborges/mineclone/blob/dfa28c50aabc49bafb152a386a20741d74dae9e8/HANDOFF.md). Preservar todos os snapshots históricos referenciados nesses documentos.

## Regras permanentes

- Quando usuário disser `go`/`continua`, examinar HEAD, VERSION, CI, código e handoff; executar alterações fundamentadas em `develop`; comunicar entregas reais rapidamente. Não inventar compilação, CI, gameplay ou QA visual.
- A cada bloco funcional relevante **concluído**, incrementar `VERSION` conforme SemVer (minor feature, patch correção); atualizar HANDOFF com commits, validações e pendências. Documentação e ajuste de infraestrutura isolados não incrementam versão. O bloco de `/spawn`, `/place`, slimes, colisões e inputs ainda requer CI e QA; não fazer bump prematuro. Ao terminar o conjunto de features, próximo minor a partir de 0.20.3 será 0.21.0.
- CI: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`; corrigir todos os erros e warnings reais, sem `#[allow]`. Não executar nem reintroduzir `cargo test` sem solicitação explícita. CI não substitui QA Windows/render/FPS/gameplay. Não repetir avisos sobre execução local em cada resposta.
- Arte: cubos, arestas retas sem bevel/smoothing. Rostos dos slimes apenas em PNG externo 64×64 de cada espécie, nearest; não impor 64×64 a outras criaturas. Preservar Bassalt, Brush e todos os assets do usuário. Consultar `ARCHITECTURE.md`.
- Estruturas ficam exclusivamente em `data/structures/`; biomas/dimensões referenciam por ID; loader rejeita estrutura fora do diretório global.

## Trabalho recente: colisão, inputs e novos diagnósticos de CI

- Usuário explicitou que **player e entity devem empurrar um ao outro, não só barrar movimento**. Primeiro código do contato bidirecional em `src/player/movement/entity_collision.rs`, integrado ao pipeline, usa AABBs dos roots, deslocamento horizontal compartilhado, passos conservadores contra voxels sólidos ou chunks descarregados; commits do conjunto anterior incluem [`97f9ca4`](https://github.com/sarakborges/mineclone/commit/97f9ca4bf1c9d8cf34a331013ae93d7174617e5d), [`2c4d1aa`](https://github.com/sarakborges/mineclone/commit/2c4d1aa6e8546870a9f6562d996800999b1d15fe) e [`cf9b603`](https://github.com/sarakborges/mineclone/commit/cf9b603ce17d2fa6c308692dbb915cdebbf4e089); conferir diffs exatos antes de retomar. A física precisa de QA ingame, em terreno irregular, múltiplas criaturas e junto a paredes; não declarar terminada.
- Revisões de inputs do filtro de biomas e busca do inventário publicadas no mesmo conjunto, com criação de frame externo separado de `EditableText`, preservação de foco/caret e placeholder; commits incluem [`88f7f39`](https://github.com/sarakborges/mineclone/commit/88f7f392cfad56a7a5e8e7c69cb14ea0197b3c50), [`e29d3c1`](https://github.com/sarakborges/mineclone/commit/e29d3c12147c7f013cfd66215074d5c851e426f2), [`baafe93`](https://github.com/sarakborges/mineclone/commit/baafe9341dab061deda9f357b941ac96fb4e2819), [`8eecca8`](https://github.com/sarakborges/mineclone/commit/8eecca84ad7b78233233a75ab1331307f3558b92) e [`ed4af42`](https://github.com/sarakborges/mineclone/commit/ed4af4229eeef8b3bdef38dd1119ce6fa3f57605). Validar UI visualmente, caret, IME, texto comprido e responsividade; seed, ticks, chat e demais EditableText também devem ser auditados.
- [CI run `35172165397`, commit `ed4af42`](https://github.com/sarakborges/mineclone/actions/runs/35172165397) **falhou** no Clippy, Check skipped, com exatamente cinco erros: duas `private_interfaces` de `CreativeSearchFrame` em `focus_inventory_search_frame` e `style_inventory_search_field`, duas exposições do tipo privado ao registrar os sistemas em `hud/inventory.rs`, e `clippy::type_complexity` na Query de `resolve_player_creature_contacts`. Diagnóstico confirmado nos logs, não usar `#[allow]`.
- [`c612070`](https://github.com/sarakborges/mineclone/commit/c612070c171633004e9725a7b4c72d8446c959ec): altera visibilidade de `CreativeSearchFrame` para `pub(super)` e resolve os quatro erros de privacidade; [`8db1614`](https://github.com/sarakborges/mineclone/commit/8db1614a1556733885924ec087cc492f7d4b2a1d): extrai alias `CreatureContacts<'w, 's>` e resolve o `type_complexity` sem silenciar lints.
- CI para o código `8db1614`: [push `35172553467`](https://github.com/sarakborges/mineclone/actions/runs/35172553467) e [PR `35172555995`](https://github.com/sarakborges/mineclone/actions/runs/35172555995). Na última consulta: ambos rodando Clippy; cache Linux e Cargo aprovados, Check ainda pendente. **Reconsultar resultado final e corrigir qualquer diagnóstico restante**.

## Otimização da CI publicada

- [`8503bac`](https://github.com/sarakborges/mineclone/commit/8503bac97aa3c88d1204eb56f79a891a7ad2da7f): `.github/workflows/ci.yml` fixa `ubuntu-24.04`, substitui `apt-get update && apt-get install` em cada job por `awalsh128/cache-apt-pkgs-action` fixada em SHA `553a35b` (v1.6.3), com nove bibliotecas Linux necessárias ao Bevy; adiciona `Swatinem/rust-cache@v2` com `cache-on-failure: true`; mantém clippy rigoroso e check. Na nova execução, as duas etapas de cache passaram, mas ainda não se mediu o ganho em uma execução com cache aquecido. Runner continua efêmero; primeira execução precisa popular o cache.
- `.gitignore` ignora `/Cargo.lock`; foi observada resolução nova de 559 crates no run antigo. Avaliar lockfile para reprodutibilidade em um bloco separado e deliberado, sem mudança gratuita de resolução de versões no meio da correção de CI.

## Funcionalidades históricas ainda abertas (preservadas)

- Migração dos cinco JSONs de boulders/oak tree para `data/structures` e manutenção de stone_marker, commits `69bf4ec` e `2ab75a0`, verificada por árvore Git; geração no jogo não observada. Handoff anterior contém detalhes.
- `/spawn` e `/place`: spawn/placement exatos na origem do jogador com realocação segura; pré-verificação de espaço seco, carregado, sustentado, sem sobreposição, e abortos atômicos. Validar posicionamento de árvores/boulders reais, chunk boundary, múltiplos comandos e autocomplete (ID, descrição, ↑↓, Tab, Esc); não encerrar sem QA.
- Slimes: sem spawn automático, movimento e facing visual por salto configurados; comportamento ingame pendente.
- Inputs: alinhamento visual reportado incorreto; patches no inventário/filtro precisam QA, verificar também todos `EditableText` com foco/IME/placeholder.
- Física: empurrão mútuo implementado inicialmente, ainda requer validar resposta efetiva, colisões entre criaturas se aplicável, limites e contato contra paredes e cenário.

## Próximos passos

1. Conferir CI final em `8db1614`, corrigir todos os diagnósticos novos até Clippy e Check passarem no mesmo commit. Se houver falha no cache, ajustar ação/configuração e confirmar nova execução.
2. Realizar QA de gameplay/UI e corrigir colisões e alinhamento ainda abertos; preservar semântica de `/spawn` e `/place`.
3. Incrementar `VERSION` quando bloco funcional estiver de fato concluído, documentar CI e QA verificáveis e manter o histórico intacto neste HANDOFF.
