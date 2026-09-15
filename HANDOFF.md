# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch de trabalho: `develop`  
Stack: Rust + Bevy 0.19.1 (conforme `Cargo.toml`)

## Fonte canônica e regras de trabalho

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica e persistente do projeto. Exports/anexos são derivados.

- Trabalhar diretamente em `develop`; feature branch só sob pedido explícito.
- Antes de escrever, buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Commits pequenos/coerentes; não misturar mudanças arquiteturais sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível; minor para feature compatível; major para breaking.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando o comportamento depender disso.
- Não gerar imagens sem pedido explícito.
- Comunicação direta: menos narração, mais mudança concreta.
- Pedido ativo: continuar os blocos do roadmap até esgotar os itens aplicáveis ou os usos disponíveis do Work, atualizando este handoff em cada bloco.

## Validação

CI automático em `.github/workflows/ci.yml`:

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check`

Roda em push para `develop`/`main` e em pull requests.

`cargo test` **não faz parte do CI** e só deve ser rodado manualmente quando o usuário solicitar explicitamente. Não reintroduzir tests no workflow nem rodá-los por rotina sem pedido. `cargo fmt`/`rustfmt` não é gate.

## Canon arquitetural

`ARCHITECTURE.md` continua sendo o canon principal. Regras operacionais relevantes:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair semelhança superficial.
3. Preferir `SystemParam`s estreitos/coerentes e availability/run conditions canônicas.
4. UI compartilhada pertence a `src/ui`.
5. Targeting tem um único target autoritativo e consumidores change-driven.
6. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são os primitives de fila deduplicada.
7. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
8. Cores internas são HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
9. Evitar scans globais, allocations temporárias e dirty writes quando existe sinal/metadata no owner certo.
10. Pipeline inicial: generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> spawn.
11. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
12. Terrain/fluid/lighting background remesh é async; immediate geometry de edit do player permanece síncrono para feedback.
13. Não trocar corretude por performance aparente e não criar abstração genérica acima das generation/mesh task queues.
14. Sistemas visuais não devem reescrever Components/Assets com o mesmo valor; separar refresh estrutural de refresh leve quando os inputs diferirem.
15. Movimento com delta zero deve permanecer ocioso.
16. Scratch recorrente/estruturalmente limitado deve preferir stack/reuse a heap repetida sem impor limite artificial ao conteúdo.
17. `NewWorldConfig` é owner das escolhas de criação de mundo; forced biome valida coluna inicial e safe-spawn final no mesmo biome.
18. Search/text-input compartilhado pertence a `src/ui`; telas seguem owners de suas opções/filtros.
19. Preferências de HUD pertencem a `HudSettings`; targeting não conhece layout.
20. Queries mutáveis múltiplas sobre o mesmo Component precisam ser provadamente disjuntas via `Without<T>` ou `ParamSet`.
21. Child UI que deve obedecer ao hide do parent usa `Visibility::Inherited`; `Visible` só quando deve sobrescrever herança.
22. Revisions derivadas devem ser separadas por domínio quando consumers têm dependências diferentes; não usar revisão de mesh para invalidar lógica que depende só de blocos.

---

# Estado atual

Base do bloco atual:

`547172973b3e12b6143489ea64f9418821fe145d`

Bloco: `Keep idle settings controls unchanged`

`VERSION`: `0.14.15`

Na retomada, `develop` já estava em `183bcab1be268bcc9511e7e06d9406ade8b63bf0` / `0.14.12`, embora este handoff ainda descrevesse `0.14.5`. Os blocos abaixo foram conferidos no código e no histórico antes de continuar.

Commits recentes relevantes:

- `fc983fbefe3077bfec77c5223cb39322e5f70fb1` — `0.14.3`, inventory hint passa a herdar visibilidade do Player HUD; CI verde.
- `b8159597991911bf5bc2587deb4597ece217fd0b` — `0.14.4`, reutiliza scratch do unload e pré-aloca buffers de integração/spawn de mesh; CI verde.
- `8791150de82518bc32a3b66efb06c78a97d0c90b` — `0.14.5`, targeting de bloco passa a ser change-driven; CI verde.
- `fefe47d` + `cab6ae5` — `0.14.6`, consumers de targeting usam snapshot derivado; ajuste de visibilidade interna do tipo.
- `74893d1` + `7336ff9` — `0.14.7`, evita escritas redundantes no ambiente/estrelas; ajuste de visibilidade interna do snapshot.
- `c74fdf4` — `0.14.8`, reutiliza scratch do solver de iluminação dinâmica.
- `5d61e01` — `0.14.9`, corrige a validação do solver de iluminação.
- `a33f409` — `0.14.10`, scheduling do ambiente passa a usar run conditions específicas.
- `695de53` — `0.14.11`, sincroniza apresentação de nuvens também quando novas entidades entram em Gameplay.
- `183bcab` — `0.14.12`, iluminação ociosa e diagnósticos ficam atrás de run conditions; CI verde.
- `d231116` — `0.14.13`, evita dirty writes de materiais e bordas/fundos da hotbar.

## CI atual

- `0.14.3` / run `35010612188`: Clippy **success**, `cargo check` **success**.
- `0.14.4` / run `35011640625`: Clippy **success**, `cargo check` **success**.
- `0.14.5` / run `35012542969`: Clippy **success**, `cargo check` **success**.
- `0.14.12` / run `35018588468`: workflow **success** no HEAD conferido na retomada.
- `0.14.13` / [run `35019877410`](https://github.com/sarakborges/mineclone/actions/runs/35019877410), commit `f692534`: Clippy **success**, `cargo check` **success**. Revisão estática e `git diff --check` também concluídos. O commit seguinte apenas registra este resultado no handoff.
- `cargo test` somente sob pedido explícito.
- `0.14.14` / [run `35021097252`](https://github.com/sarakborges/mineclone/actions/runs/35021097252), commit `5471729`: Clippy **success**, `cargo check` **success**.
- `0.14.15`: revisão estática e `git diff --check` concluídos; CI pendente para o bloco de Settings.

---

# Features recentes consolidadas

## Player HUD / hints

- Crosshair target hint contextual: break-only sem bloco selecionado; break/place com bloco selecionado.
- Crosshair hint não aparece com inventory ou pause e respeita `Display Tooltips`.
- Player HUD permanece com Inventory aberto e desaparece no pause.
- Inventory hint:
  - fechado: `Press E to open inventory, or ESC to pause game.`
  - aberto: `Press E or ESC to close inventory.`
- `E` abre/fecha Inventory; `ESC` mantém a semântica existente de fechar Inventory quando aberto.
- O inventory hint é filho do `PlayerHudRoot` e usa `Visibility::Inherited` quando habilitado, `Hidden` quando tooltips estão desligados.

## Settings → HUD / Shared UI

- `Miscellaneous` foi substituído por seção `HUD`.
- `Target Block Position` em `HudSettings`: `Center`, `Top-right`, `Hidden`.
- Target HUD separa layout de conteúdo/material; `TargetedBlock` continua owner do target.
- Snapshot textual e visual do Target HUD são separados para evitar dirty asset writes.
- Text-input/search compartilhado foi extraído para `src/ui` e reutilizado pelo Creative Inventory e Spawn Biome.
- Dropdowns usam chevron desenhado por UI.
- Settings/World Settings usam scrollbar compartilhado em sidebar e conteúdo.
- Spawn Biome é overlay absoluto, search visualmente distinta e viewport de até 5 opções com scroll.

## New World / Spawn Biome

- `NewWorldConfig` é owner único; `None` = Random.
- Opções data-driven dos `BiomeKind::Surface`; valor salvo é biome ID.
- Forced spawn usa busca coarse-first por coluna seca e safe spawn final com predicate do mesmo biome; não existe teleporte corretivo pós-bootstrap.
- Caches pesados usados só na procura são podados antes da geração inicial.

## Worldgen balance

- Plains tree chance `0.45 -> 0.48`.
- Witchwood tree chance `0.62 -> 0.66`.
- Enchanted Forest tree chance `0.62 -> 0.66`.
- Mountains weight `0.90 -> 0.85`.

---

# Runtime fixes recentes

## 0.14.2 — Bevy B0001

`sync_target_block_position_dropdown` tinha duas queries mutáveis de `Text` sem prova de disjunção. A query das option labels inclui `Without<TargetBlockPositionDropdownLabel>`.

## 0.14.3 — inventory hint vazando no pause

O hint já era filho do Player HUD, mas `Visibility::Visible` sobrescrevia a herança do parent. Agora usa `Inherited` quando habilitado e `Hidden` quando desabilitado. A confirmação visual definitiva depende do runtime do usuário.

---

# Refactor/performance recente

## 0.14.4 — buffers de streaming/render integration

- `unload_chunk_meshes` reutiliza `Local<Vec<IVec3>>` para chunks descarregados.
- `spawn_built_chunk_meshes` pré-aloca vectors pelo número conhecido de meshes.
- `spawn_terrain_meshes_into_existing_allocation` pré-aloca entities pelo número de replacements.
- Nenhum lifecycle, prioridade, budget ou regra de seleção foi alterado.

## 0.14.5 — targeting change-driven

- `VoxelWorld` possui `block_content_revision`, separada de `chunk_mesh_revision`.
- A revisão de blocos avança quando a disponibilidade/conteúdo de blocos muda: insert/archive/restore de chunk e mudança real de bloco.
- Fluid-only e lighting-only mutations continuam alterando revisão de mesh, mas **não** invalidam targeting.
- `TargetingRaycastCache` armazena os últimos inputs derivados: origem/direção da câmera, revisão de blocos e availability de interação.
- `update_targeted_block` só executa `raycast_voxels` quando algum desses inputs muda.
- O cache é resetado em cada `OnEnter(GameState::Gameplay)`; não atravessa mundos semanticamente.
- `TargetedBlock` continua sendo o único owner autoritativo do hit.

## 0.14.6 — consumers de targeting

- Highlight e placement preview usam `BlockTargetingVisualSnapshot`, derivado de hit, item/slot, posição do player e revisão de blocos.
- Mudanças de definições e de orientação continuam invalidando os consumers apropriados.
- `TargetedBlock` continua como único owner do hit; o snapshot não é estado autoritativo paralelo.

## 0.14.7 / 0.14.10 / 0.14.11 — ambiente e sky layers

- Estrelas guardam snapshot visual e identidade/posição da câmera; posição só é reescrita quando muda, e a orientação é definida no spawn.
- Cor/alpha de materiais, iluminação, céu, fog, cascades e celestial bodies evitam escritas idempotentes nos caminhos revisados.
- Run conditions específicas evitam executar sistemas de ambiente sem inputs relevantes alterados.
- Nuvens separam apresentação (cor/visibilidade) de posição animada; `Added<CloudPart>` garante sincronização da apresentação ao entrar em Gameplay.

## 0.14.8 / 0.14.9 / 0.14.12 — iluminação e diagnósticos

- `LightingContext` recicla colunas e buffers entre batches; cada batch limpa os dados derivados antes de reutilizar a capacidade.
- `LightingRegistries` agrupa as definições do solver sem ampliar seu domínio.
- `process_dynamic_lighting` só roda quando `PendingLightingUpdates` tem trabalho.
- Diagnósticos de assets e de mesh allocator só executam seus scans nos intervalos já definidos.

## 0.14.13 — hotbar sem dirty writes redundantes

- `BlockIconMaterial::has_tint` compara a cor já aplicada no formato linear usado pelo material.
- A hotbar compara tint e orientação antes de pedir acesso mutável ao asset; andar ou trocar seleção com resultado visual igual não emite atualização de material nesse caminho.
- Mudanças de orientação ou definição de bloco continuam atualizando texturas; uma tint realmente diferente continua sendo aplicada.
- Ícones recém-criados continuam passando pelo refresh via `is_added()`; nenhum novo cache persistente foi criado.
- Fundos/bordas de slots só são escritos quando a cor resultante muda, e o snapshot de orientação só muda com a orientação.
- Sem medição de FPS ou validação visual/runtime nesta sessão; não interpretar a revisão estática ou o CI como confirmação de ganho de FPS.

---

## 0.14.14 — inventário e estilos compartilhados

- `src/ui/surface.rs` possui `apply_control_colors`, que recebe os wrappers `Mut` do Bevy e só marca fundo/borda alterados após comparar o resultado.
- Inventário, hotbar, dropdown de posição do target, toggle de tooltips e opções de Spawn Biome reutilizam esse invariant.
- A hotbar usa as cores estáticas canônicas de `surface`; a cópia local idêntica foi removida.
- Cursor do inventário, borda de search e posição de scroll não reescrevem Components quando os valores continuam iguais.
- Criação de entidades, hover, seleção e arraste mantêm seus gatilhos; sem cache de cursor que possa perder um ícone recém-criado.
- Validação visual/FPS continua dependente de runtime.

---

## 0.14.15 — controles de Settings ociosos

- Os helpers de foco/teclado de `NumericInputState` mantêm `ResMut` até detectar e aplicar input; frames sem clique/tecla não invalidam o editor nem disparam formatação do label.
- Seed e ticks preservam seleção de texto, limites numéricos, Enter/Escape e Backspace; valores já aplicados não reescrevem `NewWorldConfig`/`GameRules`/save.
- `sync_selectable_button` recebe `Mut<BackgroundColor>` e compara antes de escrever, preservando o lifecycle de `InteractionDisabled`.
- Labels de idioma/game mode, bordas numéricas e thumb do slider só mudam com o valor apresentado.

---

# Próximos passos

Se nenhum runtime error/warning tiver prioridade:

1. Concluir a auditoria de UI/HUD em execução contínua: overlay de transição reescreve cor/visibilidade enquanto idle, relógio formata texto a cada tick mesmo com minuto igual e FPS reescreve o mesmo label.
2. Continuar auditoria objetiva de `Update`/`PostUpdate` por scans globais, builds síncronos, allocations temporárias e dirty writes. Targeting consumers, estrelas e o bloco da hotbar acima já foram tratados; não repetir esses refactors sem evidência nova.
3. Integração de meshes: eliminar arrays intermediários de keys/meshes antes do caminho de substituição em assets existentes. Manter preflight completo e lifecycle atômico por chunk.
4. Seleção: reaproveitar buffers de desired/pending/retired e da fila sem repetir geração de volume nem mudar prioridades; `dispatch_remesh_tasks` também tem scratch temporário reaproveitável.
5. Manter `notify_loaded_chunk_neighbors` conservador até existir metadata suficiente para provar otimização segura.
6. Quando o usuário solicitar, rodar `cargo test` manualmente.

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh background fora da main thread;
- integração/restore/unload/lighting/fluid budgetados;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar scans globais, allocations temporárias e mutações idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative.
