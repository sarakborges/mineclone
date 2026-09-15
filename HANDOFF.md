# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`
Branch de trabalho: `develop`
Stack: Rust + Bevy 0.19.0-dev

## Fonte canônica

Este arquivo, `HANDOFF.md` na raiz de `develop`, é a fonte canônica e persistente do projeto. Cópias `.txt`, exports e anexos são derivados e não substituem este arquivo.

## Regras de trabalho

- Trabalhar diretamente em `develop`.
- Não criar feature branch sem pedido explícito.
- Antes de alterar código, buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Fazer commits pequenos e coerentes; não misturar mudanças arquiteturais sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível, minor para feature compatível, major para breaking change.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- Se o usuário enviar erro/warning/runtime report, isso tem prioridade sobre roadmap/refactor.
- Não declarar bug visual/gameplay resolvido sem evidência de runtime quando a correção depender desse comportamento.
- Não gerar imagens sem pedido explícito.
- Comunicação direta: menos narração, mais mudança concreta.

## Validação

CI automático em `.github/workflows/ci.yml`:

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check`

O workflow roda em push para `develop`/`main` e em pull requests.

`cargo test` não faz parte do CI e só deve ser rodado manualmente quando o usuário solicitar explicitamente. Não reintroduzir tests no workflow nem rodá-los por rotina sem pedido.

`cargo fmt`/`rustfmt` não é gate.

## Canon arquitetural

`ARCHITECTURE.md` continua sendo o canon principal. Regras relevantes:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair semelhanças superficiais.
3. Preferir SystemParams estreitos/coerentes.
4. Usar availability/run conditions canônicas.
5. UI compartilhada pertence a `src/ui`.
6. Targeting tem um único target autoritativo e consumidores change-driven.
7. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são os primitives de fila deduplicada.
8. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
9. Cores internas são HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
10. Evitar scans globais, allocations temporárias e dirty writes quando existe sinal de mudança/metadata.
11. Pipeline inicial: generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> spawn.
12. Resultados async são revisionados; stale results são descartados/rescheduled.
13. Terrain/fluid/lighting background remesh é async; immediate geometry de edit do player permanece síncrono para feedback.
14. Não trocar corretude por performance aparente.
15. Não criar abstração genérica acima de generation/mesh task queues.
16. Sistemas visuais não devem reescrever Components/Assets com o mesmo valor.
17. Separar refresh estrutural de material/textura de refresh leve de tint/orientação/layout quando os inputs diferirem.
18. Movimento com delta zero deve permanecer ocioso.
19. Scratch de cardinalidade fixa ou recorrente deve preferir stack/reuse a heap repetida quando isso não impuser limite artificial ao conteúdo.
20. `NewWorldConfig` é owner de escolhas de criação de mundo.
21. Spawn forçado por biome deve validar tanto a coluna inicial quanto o safe-spawn final no mesmo biome.
22. Search/text-input compartilhado pertence a `src/ui`; telas continuam owners de suas opções/filtros.
23. Preferências de HUD pertencem a `HudSettings`; targeting não conhece layout.
24. Queries mutáveis múltiplas sobre o mesmo Component em um system precisam ser provadamente disjuntas via `Without<T>` ou `ParamSet`.
25. Child UI que deve obedecer o hide do parent deve usar `Visibility::Inherited`; usar `Visible` só quando realmente quiser sobrescrever a herança.

---

# Estado atual

Último HEAD de código/version:

`b8159597991911bf5bc2587deb4597ece217fd0b`

Commit: `Reuse chunk streaming integration buffers`

`VERSION`: `0.14.4`

Commits recentes relevantes:

- `bb3b1347c69162739a1e58dc0b01780531d14d86` — último HEAD consolidado antes do bump 0.14.0; Clippy/check/test verdes sob a política antiga.
- `845f9b544de6a77771a8e5ae19a6b2e4f7c2600a` — bump `0.14.0`.
- `88abc44f20695c2fd1889f83b17a3e657a425dd4` — remove `cargo test` do CI.
- `50bb5cd2fdd9353cf9f8c9e88ecce786e8892d79` — bump `0.14.1`.
- `d020a14c8db9c287c1a54f19caa013d890a27102` — corrige Bevy B0001 no dropdown de Target Block Position com queries de `Text` explicitamente disjuntas.
- `24501774a428a1d5f7640f61f8d0a7cb8c679d6d` — bump `0.14.2`; CI Clippy/check verde.
- `e6c082e8c475e1e46ff00c29ea71d125593c8493` — faz o inventory hint herdar a visibilidade do Player HUD.
- `fc983fbefe3077bfec77c5223cb39322e5f70fb1` — bump `0.14.3`; CI Clippy/check verde.
- `b8159597991911bf5bc2587deb4597ece217fd0b` — `0.14.4`, reutiliza scratch do unload e pré-aloca buffers de integração/spawn de mesh; CI Clippy/check verde.

## CI atual

- `0.14.3` / HEAD `fc983fbe...` / run `35010612188`: Clippy **success**, `cargo check` **success**.
- `0.14.4` / HEAD `b8159597...` / run `35011640625`: Clippy **success**, `cargo check` **success**.
- `cargo test` só quando o usuário pedir.

---

# Estado consolidado das features recentes

## Player HUD / hints

- Crosshair target hint:
  - sem bloco selecionado: `Left click to break block.`
  - com bloco selecionado: `Left click to break block, or right click to place.`
- Crosshair hint não aparece com inventory ou pause abertos e respeita `Display Tooltips`.
- Player HUD permanece visível com Inventory aberto e deve desaparecer inteiro no pause.
- Inventory hint:
  - fechado: `Press E to open inventory, or ESC to pause game.`
  - aberto: `Press E or ESC to close inventory.`
- `E` abre/fecha Inventory; `ESC` preserva a semântica de fechar Inventory quando ele está aberto.
- O inventory hint é filho do `PlayerHudRoot`.
- Em `0.14.3`, quando tooltips estão habilitados ele usa `Visibility::Inherited` em vez de `Visible`, para herdar corretamente o hide do parent no pause; `Hidden` continua sendo usado quando `Display Tooltips` está desligado.

## Settings → HUD

- `Miscellaneous` foi substituído por seção `HUD`.
- `Display Tooltips` continua em `HudSettings`.
- `Target Block Position` em `HudSettings` com `Center`, `Top-right`, `Hidden`.
- Target HUD altera apenas layout/visibilidade segundo essa preferência; `TargetedBlock` continua owner do target.
- Layout e conteúdo/material do Target HUD são sincronizados separadamente.
- Snapshot textual e visual do Target HUD são separados para evitar dirty asset writes.

## Shared UI / Settings

- State machine de text input/search foi extraído para `src/ui` e reutilizado pelo Creative Inventory e Spawn Biome.
- Dropdowns usam chevron desenhado por UI, não glyph dependente de fonte.
- Settings/World Settings usam scrollbar compartilhado em sidebar e conteúdo.
- Spawn Biome dropdown é overlay absoluto, não empurra os controles seguintes, tem search visualmente distinta e viewport de até 5 opções com scroll.

## New World / Spawn Biome

- `NewWorldConfig` é owner único; `None` = Random.
- Opções são data-driven dos `BiomeKind::Surface` da dimensão e armazenam biome ID.
- Labels/search reagem a idioma.
- Foco entre Seed/Ticks/search/outros controles é coordenado.
- Forced spawn usa busca coarse-first por coluna seca do biome escolhido.
- Safe spawn final recebe predicate do mesmo biome; árvore/estrutura pode deslocar o player apenas dentro dele.
- Não existe teleporte corretivo pós-bootstrap.
- Caches pesados usados só na procura são podados antes da geração inicial.

## Worldgen balance

Ajustes leves e data-driven:

- Plains tree chance `0.45 -> 0.48`.
- Witchwood tree chance `0.62 -> 0.66`.
- Enchanted Forest tree chance `0.62 -> 0.66`.
- Mountains weight `0.90 -> 0.85`.

---

# Runtime fixes recentes

## 0.14.2 — Bevy B0001

Runtime panic em `QueryState`: `sync_target_block_position_dropdown` tinha duas queries mutáveis de `Text` sem prova de disjunção. A query das option labels agora inclui `Without<TargetBlockPositionDropdownLabel>`.

## 0.14.3 — inventory hint vazando no pause

Runtime report: `Press E to open inventory...` permanecia visível no pause enquanto o resto do Player HUD sumia.

Causa: o hint já era filho do `PlayerHudRoot`, mas usava `Visibility::Visible`, sobrescrevendo a herança visual do parent.

Fix: usar `Visibility::Inherited` quando tooltips estão habilitados e `Visibility::Hidden` quando desabilitados. Isso mantém o owner local do toggle sem furar o lifecycle/visibilidade do Player HUD.

A confirmação definitiva desse comportamento depende do próximo runtime do usuário.

---

# Refactor/performance recente

## 0.14.4 — buffers de streaming/render integration

- `unload_chunk_meshes` reutiliza `Local<Vec<IVec3>>` para o batch de chunks descarregados, preservando capacidade entre frames em vez de recriar scratch variável no hot path.
- `spawn_built_chunk_meshes` pré-aloca `entities`, `mesh_handles` e `mesh_keys` pelo número conhecido de meshes produzidas para o chunk.
- `spawn_terrain_meshes_into_existing_allocation` pré-aloca entities pelo número de replacements, reduzindo reallocs durante remesh/integration.
- Nenhum lifecycle, prioridade, budget ou regra de seleção foi alterado.

---

# Próximos passos

Se nenhum runtime error/warning tiver prioridade:

1. Tornar `TargetedBlock` change-driven: hoje o raycast roda todo frame. Introduzir metadata autoritativa no `VoxelWorld` para mudanças de conteúdo de blocos e recalcular somente quando câmera, disponibilidade de interação ou conteúdo relevante mudar; não reagir a lighting-only/fluid-only mutations.
2. Depois revisar consumers de targeting (`highlight`/placement preview) para aproveitar sinais reais de mudança sem duplicar owner/cache.
3. Revisar estrelas: posições dependem da câmera, mas hoje `Transform` das 96 estrelas é reescrito em todo tick do day/night; material também pode ser marcado mutável com cor/alpha idênticos durante fases estáveis.
4. Continuar auditoria objetiva de `Update`/`PostUpdate` por scans globais, builds síncronos, allocations temporárias e dirty writes.
5. Revisar integração/spawn de mesh apenas se houver ganho estrutural real sem lifecycle parcial por submesh.
6. Revisar rebuild de seleção somente com ganho claro; não duplicar geração de volume para eliminar sort pequeno.
7. Manter `notify_loaded_chunk_neighbors` conservador até existir metadata suficiente para provar otimização segura.
8. Quando o usuário solicitar, rodar `cargo test` manualmente.

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh background fora da main thread;
- integração/restore/unload/lighting/fluid budgetados;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar scans globais, allocations temporárias e mutações idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative.
