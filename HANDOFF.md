# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz `VERSION`: `0.20.3` (16/09/2026, horário de São Paulo); `Cargo.toml` usa versão independente `0.10.16`.** Último HEAD de código inspecionado: [`a71a881`](https://github.com/sarakborges/mineclone/commit/a71a8819bb96dddbfaeb2eebd163941edb42538b). Esta retomada publicou código, mas **a feature não deve ser considerada entregue até Clippy/Check, fechamento do padding global e QA**. Não afirmar que a CI passou.

## Preservação integral e fonte do histórico

O [HANDOFF anterior, integral e imutável, no commit `558d0b2`](https://github.com/sarakborges/mineclone/blob/558d0b2d1ec86b1bc3e30d95a7ba26c45f1f2ff0/HANDOFF.md) preserva a narrativa original de 16/09, decisões e pendências históricas. Os snapshots anteriores continuam em [v0.20.0](https://github.com/sarakborges/mineclone/blob/bb7fd215117f8ab0148d3ec850244a6516ed1107/HANDOFF.md), [v0.19.1](https://github.com/sarakborges/mineclone/blob/c3fc2646b58971df62b08ae163315cf1320a1273/HANDOFF.md), [v0.19.0](https://github.com/sarakborges/mineclone/blob/ae0aa17a7117caa1cf08f82e51e2cdf4f2491ea0/HANDOFF.md), [arte](https://github.com/sarakborges/mineclone/blob/bbc6505e7deeff2b41125baf9ec32815e4786d69/HANDOFF.md), [v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md) e [v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). **Todas as pendências antigas NÃO verificadas permanecem abertas; consultar snapshots antes de dar baixa.** O presente documento acrescenta o estado consolidado desta retomada e não reclassifica os resultados passados.

## Regras permanentes

- Quando usuário disser `continua`/`go`: consultar HEAD, VERSION, CI, código e este handoff; executar mudanças fundamentadas em `develop`. Informar commits e resultados concretos com agilidade. Nunca alegar compilação, CI, desempenho, gameplay ou QA visual não observados.
- A cada bloco funcional relevante, incrementar `VERSION` com SemVer: **minor para feature, patch para correção**, e atualizar este HANDOFF no mesmo ciclo com mudanças reais, commits, validações e pendências. Alteração só documental não incrementa VERSION. Nesta retomada, **não subir `0.20.3` enquanto implementação/CI estiverem incompletas; após fechar a nova feature, o próximo minor é `0.21.0`** (confirmar o valor de VERSION no momento do bump).
- Validação exigida: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`, zero erros e warnings reais, sem `#[allow]`. **Não executar `cargo test` nem adicioná-lo ao workflow sem solicitação.** CI não substitui execução Windows, FPS ou QA de gameplay. Evitar repetir aviso de teste em toda resposta.
- Direção de arte: modelos e detalhes exclusivamente cúbicos/quadrados; superfícies planas, cantos vivos, sem curvas, bevels, rounded boxes ou smoothing. Slime com casca e núcleo cúbicos, face no PNG externo 64×64 por espécie, nearest. Não impor 64×64 a outras criaturas; preservar assets do usuário, especialmente Bassalt e Brush. Seguir `ARCHITECTURE.md`.
- Ao atualizar handoff, preservar histórico via snapshots e registrar estado observado, não metas como entregas; não omitir falhas da CI.

## Contrato do bloco atual (pedido do usuário)

1. Substituir `/spawn_creature <id>` por `/spawn <id>` para criaturas; adicionar `/place <id>` para estruturas. Sugestões de ambos os comandos e IDs efetivamente carregados dos registros JSON. Comando/ID numa linha, descrição na seguinte, sem `Suggestions 1/1`. Manter ↑/↓, Tab, primeiro Esc dispensando apenas sugestões sem fechar o chat.
2. Criatura/estrutura ocupam a posição exata original do jogador; mover **o jogador** para posição livre, fora do volume ocupado. Antes de editar, preflight de chunks carregados, espaço seco, suporte e colisões de blocos/criaturas/estrutura. Se impossível, abortar sem operação parcial e usar `not enough space to spawn <id>` / `not enough space to place <id>`. Regra antiga dos oito vizinhos foi revogada.
3. Nada de spawn automático de slimes na criação do mundo; cada salto escolhe direção pseudoaleatória e o visual gira para essa direção, sem rotacionar o collider. Validar no jogo.
4. Padding **real** para todos os editores: seed, ticks, filtro de biomas, busca do inventário, chat e demais inputs. Preservar foco, edição, caret, placeholder, clipping e responsividade; QA com texto longo.

## Retomada publicada em develop — 16/09/2026 (NO CÓDIGO; NÃO É CONCLUSÃO DE FEATURE)

### Comandos e ocupação

- [`682d71a`](https://github.com/sarakborges/mineclone/commit/682d71ad47b55e00dc94dfc46d8461368525c080): `src/hud/chat.rs` registra `mod placement`, conecta `ChatPlacementContext`, executa `/spawn` e `/place`, contabiliza criaturas reservadas no mesmo frame, atualiza testes do parser. Resolve assinatura antiga de `spawn_creature_at` usada pelo chat.
- [`da074b0`](https://github.com/sarakborges/mineclone/commit/da074b042e07fee0ed01b09ee2e2d53a330af49b): `src/hud/chat/autocomplete.rs` passa a definir `/spawn` e `/place` numa tabela, consulta `CreatureRegistry`/`StructureRegistry` e expõe IDs/nomes reais, mantém interação ↑/↓/Tab e troca os testes da sintaxe antiga.
- [`2c711e9`](https://github.com/sarakborges/mineclone/commit/2c711e9077adc14960eed27602a9855f21de7fac): `src/hud/chat/placement.rs` faz varredura do volume real do collider em voxels, exige carregamento/ausência de blocos e fluidos, checa suporte, preflight da estrutura inteira sem sobrescrever blocos sólidos, fundamento mínimo da estrutura e deslocamento do jogador para fora. `set_block` passou a exigir sucesso depois do preflight, em vez de ignorar `None`. **O código foi publicado; ainda faltam CI + QA dos casos extremos, especialmente terreno irregular, estrutura grande, jogador preso, colocação repetida no mesmo frame e chunks nas bordas.**

### Slimes

- [`a479466`](https://github.com/sarakborges/mineclone/commit/a47946618732df8121701ec6d8cd9a7175a03109): adiciona `move_speed` à definição de criatura (serde `moveSpeed`, default zero) com validação numérica; resolve uso do campo em `motion.rs`.
- [`2e46491`](https://github.com/sarakborges/mineclone/commit/2e464916ad4b5f088ad66a05d1d25534fb3bc9b2) configura `moveSpeed: 1.5` no Meadow. Commit seguinte configura `moveSpeed: 1.7` no Ember; consultar histórico do branch para SHA exato caso necessário.
- [`a71a881`](https://github.com/sarakborges/mineclone/commit/a71a8819bb96dddbfaeb2eebd163941edb42538b): implementa `sync_creature_facing` em `src/creatures/visual.rs` e adiciona `Transform` ao wrapper visual GLTF. O sistema de movimento já calcula rumo aleatório por salto; visual gira sem girar root/collider. `src/creatures/mod.rs` não registra spawn automático. **Nada disto foi inspecionado rodando em jogo.**

### Campos editáveis

- [`3e6ba97`](https://github.com/sarakborges/mineclone/commit/3e6ba97a89b4d9819fe44656d49e2b8ac4b4ed5e): `src/ui/numeric_input.rs` separa frame com borda/padding do `Button`/`EditableText` filho e introduz `NumericInputFrame<I>` para sincronizar a borda do wrapper.
- [`25409fe`](https://github.com/sarakborges/mineclone/commit/25409fe612096c07e411bc7277d01a38f4332b1b) e [`3316f7c`](https://github.com/sarakborges/mineclone/commit/3316f7c39f4d29cc74ce5497696e5f12c9e03e95): consultas da borda do seed e ticks passam a mirar `NumericInputFrame`, preservando marcadores do editor interno para foco/interação. **Sem QA visual.**
- Auditoria: `src/screens/settings_screen/spawn_biome_section/layout.rs` ainda põe border/padding e `EditableText` no mesmo nó; `systems.rs` usa marcador `SpawnBiomeSearchBar` tanto para foco/edição quanto para a query da borda. A busca do inventário em `src/hud/inventory/layout.rs` também usa campo e borda no mesmo nó, com placeholder como filho em fluxo normal. **Ambos ainda sem correção**; separar frame/editor preservando os seletores e fixar placeholder absoluto com recuo. `src/hud/chat/visual.rs` já apresenta `ChatInputRoot` como moldura e `ChatDraft` como filho separado; verificar caret/recuo real e texto longo in-game antes de dar baixa.

## CI — evidência, não suposição

- [Run `35170340865`](https://github.com/sarakborges/mineclone/actions/runs/35170340865) para `da074b0`: **completed/failure**; Clippy falhou e Check foi skipped. O log mostra exatamente 3 erros de compilação nesse commit: `visual::sync_creature_facing` ausente e duas leituras de `definition.move_speed` com campo inexistente. Correções específicas estão nos commits `a479466` e `a71a881` mencionados acima; **isso não demonstra que agora esteja verde**.
- Para o HEAD `a71a881`, [run `35170719982`](https://github.com/sarakborges/mineclone/actions/runs/35170719982) estava **in_progress**, Clippy em execução e Check pendente na última consulta desta atualização. Consultar novamente e corrigir qualquer erro/warning real do HEAD; se falhar, ler o log da etapa `validate` para diagnósticos concretos.
- `VERSION` permanece `0.20.3`. Não fazer bump apenas por este documento nem chamar feature de entregue por commits presentes. `cargo test` não foi executado nesta retomada.

## Próximo trabalho, na ordem

1. Reconsultar CI no último HEAD; capturar diagnóstico completo, corrigir todos os erros e warnings reais até `Clippy -D warnings` + `cargo check` passarem no mesmo commit. Lembrar que CI de commit anterior não certifica HEAD.
2. Concluir frame/editor de filtro de biomas e inventário sem quebrar marcadores, foco, borda/placeholder; verificar também chat e qualquer outro `EditableText` no repositório. Revisar casos de perda de foco, espaços longos, caret, IME e clipping. Corrigir bugs observados.
3. Revisar atomicidade/colisões de `/place` e `/spawn`, incluindo voxels fluidos, solo, colisão com entidade, estrutura grande, limite de chunk, deslocamento em terreno irregular, múltiplos comandos no mesmo frame. Inspecionar também remoção de spawn automático e direção/rotação dos slimes. QA gameplay/Windows somente quando executável disponível; nunca inventar resultados.
4. Só após fechamento integral, incrementar `VERSION` corretamente (`0.21.0` se partir de `0.20.3` e realmente concluir feature nova), registrar commit do bump, atualizar este HANDOFF com HEAD/CI e pendências observadas. Preservar links de snapshots históricos.
