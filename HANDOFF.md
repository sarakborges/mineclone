# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`
Branch de trabalho: `develop`
Stack: Rust + Bevy 0.19.0-dev

## Fonte canônica

Este arquivo, `HANDOFF.md` na raiz de `develop`, é a fonte canônica e persistente do projeto. Cópias `.txt`, exports e anexos são derivados e não substituem este arquivo.

## Regras de trabalho

- Trabalhar diretamente em `develop`.
- Não criar feature branch sem pedido explícito.
- Antes de alterar código, buscar o HEAD atual de `develop` e abrir os arquivos reais envolvidos.
- Fazer commits pequenos e coerentes; não misturar mudanças arquiteturais sem relação com o bloco atual.
- Não declarar bug visual/gameplay resolvido sem evidência de runtime quando a correção depender desse comportamento.
- Não gerar imagens a menos que o usuário peça explicitamente.
- Para assets binários enviados pelo usuário, usar exatamente os arquivos fornecidos.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- Comunicação direta: menos narração, mais mudança concreta.

## Versionamento — obrigatório

O arquivo raiz `VERSION` é a fonte autoritativa.

Todo bloco coerente deve subir versão:

- `patch`: fixes/refactors/otimizações internas/tooling compatíveis;
- `minor`: nova feature compatível;
- `major`: mudança incompatível/breaking.

O bump faz parte do bloco.

## Validação — obrigatória

O CI de Rust valida automaticamente apenas:

- `cargo clippy --all-targets --all-features -- -D warnings`;
- `cargo check`.

`.github/workflows/ci.yml` roda em `push` para `develop` e `main`, além de `pull_request`.

`cargo test` **não faz mais parte do CI** e **não é gate automático de fechamento**. Testes devem ser executados manualmente somente quando o usuário solicitar explicitamente. Não reintroduzir testes no workflow nem rodá-los por rotina sem pedido.

Se o usuário enviar output de compilação/runtime, corrigir todos os errors e warnings relacionados antes de continuar refactors maiores.

`cargo fmt`/`rustfmt` não é gate de CI.

Não ficar em polling repetitivo de CI; consultar quando houver resultado concreto para agir.

## Handoff — obrigatório

Atualizar `HANDOFF.md` sempre que houver mudança material em estado, arquitetura, roadmap, versão, HEAD relevante, regras ou próximos passos.

Não acumular backlog histórico obsoleto. Bugs antigos só permanecem se ainda estiverem ativos ou se houver regressão reportada.

O HEAD registrado aqui deve apontar para o último commit de **código/version**, não para o commit do próprio handoff.

---

# Canon arquitetural

`ARCHITECTURE.md` é o canon arquitetural atual.

Princípios principais:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair por semelhança superficial.
3. Preferir `SystemParam`s coerentes a bags gigantes; manter contextos mutáveis estreitos.
4. Usar availability/run conditions canônicas.
5. UI compartilhada pertence a `src/ui`.
6. Targeting tem um target autoritativo e consumidores change-driven.
7. Filas deduplicadas usam `DeduplicatedQueue<T>`; regras de voxel ficam em `VoxelUpdateQueue`.
8. `FrameWorkBudget` é o primitive canônico para orçamento por tempo/quantidade.
9. Cores internas são HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
10. Remover helpers/módulos que só encaminham chamadas e não possuem invariant.
11. Evitar scans/rebuilds globais por frame quando existe sinal de mudança ou metadata no owner correto.
12. Pipeline async inicial canônico: generation task -> integrate chunk -> initial lighting seed -> halo snapshot -> mesh task -> spawn render entities.
13. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
14. Não trocar corretude do mundo por performance aparente; mover/stagear custo.
15. Não criar abstração genérica acima de generation/mesh tasks quando o lifecycle comum já está em `ChunkTaskQueue`.
16. Terrain/fluid/lighting remesh de background usa pipeline async; apenas remesh de geometry imediato de edição do jogador permanece síncrono.
17. Solvers dinâmicos caros devem ter teto temporal e de quantidade quando o trabalho puder variar muito por frame.
18. Sistemas visuais e gameplay devem evitar reescrever Components/Assets com o mesmo valor; acesso mutável pode propagar change detection ou upload desnecessário.
19. Quando invariants permitirem, separar refresh estrutural de material/textura de refresh leve de tint/orientação/posição.
20. Movimento com delta zero não deve adquirir mutação de `Transform` nem executar collision stepping; estado ocioso deve permanecer realmente ocioso.
21. Scratch de cardinalidade estruturalmente limitada deve preferir stack/reuse a heap allocation por amostra, sem impor limites artificiais a conteúdo data-driven.
22. Escolhas de criação de mundo pertencem a `NewWorldConfig`; bootstrap consome essa configuração, não cria owner paralelo.
23. Spawn forçado por biome deve escolher coluna e posição segura final pelo biome autoritativo do surface field; não corrigir por teleporte pós-bootstrap.
24. Search/text-input state machine compartilhado pertence a `src/ui`; telas continuam owners de suas opções/regras específicas.
25. Preferências de HUD pertencem a `HudSettings`; `TargetedBlock` continua sendo o único owner do target.
26. Layout/visibilidade e conteúdo/material de HUD devem ser atualizados separadamente quando seus inputs autoritativos diferirem.
27. Queries mutáveis múltiplas sobre o mesmo Component em um system devem ser provadamente disjuntas via `Without<T>` ou agrupadas em `ParamSet`; não depender da composição atual dos bundles para evitar B0001.

---

# Estado atual do branch

Último HEAD de código/version confirmado antes desta gravação do handoff:

`24501774a428a1d5f7640f61f8d0a7cb8c679d6d`

Commit: `Bump version to 0.14.2`

`VERSION`: `0.14.2`

Commits imediatamente relevantes:

- `bb3b1347c69162739a1e58dc0b01780531d14d86` — último HEAD consolidado do pacote funcional `0.14.0`; Clippy/check/test ficaram verdes antes da mudança de política de CI.
- `845f9b544de6a77771a8e5ae19a6b2e4f7c2600a` — bump para `0.14.0`.
- `88abc44f20695c2fd1889f83b17a3e657a425dd4` — remove `cargo test` do CI; testes passam a ser manuais sob pedido.
- `50bb5cd2fdd9353cf9f8c9e88ecce786e8892d79` — bump para `0.14.1`.
- `d020a14c8db9c287c1a54f19caa013d890a27102` — corrige B0001 no sync do dropdown de Target Block Position tornando as duas queries mutáveis de `Text` explicitamente disjuntas.
- `24501774a428a1d5f7640f61f8d0a7cb8c679d6d` — bump para `0.14.2`.

Sempre buscar HEAD/VERSION novamente antes de escrever código.

## CI atual

- O antigo HEAD `bb3b1347...` passou Clippy, `cargo check` e `cargo test`.
- A partir de `0.14.1`, o CI autoritativo contém somente Clippy + `cargo check`.
- `cargo test` só deve ser rodado quando o usuário pedir.
- `0.14.2` / HEAD `24501774...` / run `35009719739`: em execução na última consulta; Clippy estava em andamento e `cargo check` pendente.

---

# Estado consolidado do refactor/performance

## Base até 0.12.103

- Filas deduplicadas, snapshots clonáveis e lifecycle async de generation/mesh.
- Integração main-thread budgetada; remesh background async com revisions/halo validation.
- Metadata de occupancy/boundaries no `VoxelChunk`; índice vertical no `VoxelWorld`.
- Halo de mesh reduzido à shell real; skylight vertical lazy.
- Chunk buffers COW com `Arc<[...]>`; chunks vazios pulam mesh task.
- Dynamic lighting, fluid, archive restore e unload possuem budgets de quantidade/tempo.
- Geometry/Lighting do mesmo coord coalescem quando produzem o mesmo terrain mesh; Fluid permanece independente salvo supersedence por geometry completa.
- UI/HUD/targeting/environment foram progressivamente convertidos para change-driven/idempotent writes.
- Held block separa rebuild estrutural/material de tint/orientation.
- Player idle evita reescritas de transform/state e collision stepping com delta zero.

## 0.12.104–0.12.106 — biome sampling/identity

- Surface neighborhood fixo de 25 sites usa scratch em stack em vez de múltiplos `Vec` temporários.
- Pesos regionais usam scratch compacto preservando ordem/tie-breaking.
- `track_current_biome` reutiliza buffers de identidade/influences em vez de reconstruir `String`/`Vec` descartáveis.
- Reuse de buffers não cria cache autoritativo stale; change detection continua comparando o estado final.

---

# 0.13.x–0.14.0 — pacote de gameplay/UI/world creation

## Gameplay hints / Player HUD

- Crosshair mostra hint contextual quando existe target:
  - sem bloco selecionado: `Left click to break block.`
  - com bloco selecionado: `Left click to break block, or right click to place.`
- Tooltip abaixo da crosshair não aparece com inventory ou pause abertos e respeita `Display Tooltips`.
- Player HUD permanece visível com Inventory aberto e some no pause.
- Hint do Player HUD respeita `Display Tooltips`:
  - inventário fechado: `Press E to open inventory, or ESC to pause game.`
  - inventário aberto: `Press E or ESC to close inventory.`
- `E` abre e fecha Inventory; `ESC` preserva a semântica existente de fechar Inventory quando ele está aberto.

## Settings → HUD

- A antiga seção `Miscellaneous` foi renomeada para `HUD`.
- `Display Tooltips` permanece owner de hints de gameplay no HUD; descrição foi generalizada para não falar apenas de crosshair.
- Nova preferência `Target Block Position` em `HudSettings`, com:
  - `Center`;
  - `Top-right`;
  - `Hidden`.
- Target HUD altera somente layout/visibilidade segundo `HudSettings`; `TargetedBlock` continua owner do target.
- Layout e conteúdo do Target HUD são sincronizados separadamente.
- Snapshot textual e snapshot visual do Target HUD são separados; mudança apenas de luz/idioma/face não marca `BlockIconMaterial` como modificado.

## Shared UI

- State machine de text search/input foi extraído para `src/ui` e reutilizado pelo Creative Inventory e Spawn Biome.
- Dropdowns usam chevron desenhado por UI em vez de glyph dependente de fonte.
- Settings e World Settings usam primitive compartilhado de scrollbar persistente em sidebar e conteúdo/section.

## New World / Spawn Biome

- `NewWorldConfig` é o único owner da seleção; `None` representa `Random`.
- New World → General possui `Spawn Biome` pesquisável.
- Opções são data-driven a partir dos `BiomeKind::Surface` da dimensão; labels vêm das definições/localização e o valor salvo é biome ID.
- Options/search labels reagem a troca de idioma enquanto a tela está viva.
- Search bar é visualmente distinta das opções.
- Dropdown é overlay absoluto e não empurra Seed/Game Mode/outros controles.
- Viewport exibe até 5 opções e usa scroll quando necessário; filtro reseta posição de scroll.
- Foco entre Seed/Ticks/search e outros controles foi coordenado para evitar input oculto ainda capturando teclado.

## Spawn forçado

- Busca inicial é coarse-first e procura coluna seca cujo `BiomeFieldSample.primary_id` seja o biome selecionado.
- `Random` preserva o fluxo anterior.
- A posição física final usa a mesma lógica de safe spawn com predicate de biome; estruturas/árvores podem deslocar a posição apenas para outra coluna ainda no biome escolhido.
- Não existe segundo teleporte corretivo pós-bootstrap.
- Caches pesados usados durante a procura são podados para a região de bootstrap antes da geração inicial.

## Worldgen balance

Ajustes deliberadamente pequenos e data-driven:

- Plains tree chance: `0.45 -> 0.48`.
- Witchwood tree chance: `0.62 -> 0.66`.
- Enchanted Forest tree chance: `0.62 -> 0.66`.
- Mountains weight: `0.90 -> 0.85`.

Nenhuma regra especial em código foi criada para esse balanceamento.

---

# 0.14.2 — runtime ECS query fix

- Runtime report: Bevy `B0001` em `QueryState`, causado por acesso conflitante ao mesmo Component dentro de um system.
- Causa identificada em `sync_target_block_position_dropdown`: havia uma `Query<&mut Text, With<TargetBlockPositionDropdownLabel>>` e outra `Query<(&TargetBlockPositionOptionLabel, &mut Text)>` sem filtro que provasse disjunção ao ECS.
- A query das option labels agora inclui `Without<TargetBlockPositionDropdownLabel>`, tornando os conjuntos explicitamente disjuntos sem duplicar system/state e sem recorrer a `allow`.
- A revisão dirigida dos systems novos de Player HUD, Spawn Biome, Target HUD layout e scrollbar não encontrou outro par equivalente de queries mutáveis sobrepostas.
- Clippy/check não provam esse tipo de conflito de runtime; a confirmação definitiva é o próximo runtime do usuário.

---

# Decisões explícitas da auditoria

Não desfazer sem evidência nova:

- Não criar abstraction genérica acima de `ChunkGenerationTasks` / `ChunkMeshTasks`; lifecycle comum já pertence a `ChunkTaskQueue`.
- Não separar lighting remesh em atributos com índices fixos: block light participa de `should_flip_diagonal` e pode mudar topologia indexada.
- Não expor `VoxelWorld::chunk_mut` genericamente; mutações devem permanecer estreitas e ownership-aware.
- `DeduplicatedQueue` mantém FIFO/prioridade via generations/tombstones.
- Natural hydrology continua source/static; não reenfileirar água natural no solver dinâmico.
- Batch content edit pertence a `VoxelChunk`; não criar builder externo com invariants duplicados.
- Qualquer mesh/remesh async deve validar presença/revisão de todo o halo antes de aplicar resultado.
- Lighting remesh pertence ao background async; immediate geometry permanece síncrono enquanto feedback do edit justificar.
- Scratch containers podem preservar capacidade entre frames, mas caches derivados do conteúdo do mundo não devem sobreviver sem invalidation autoritativa.
- Componentes/Assets não devem ser mutavelmente acessados só para regravar o mesmo valor.
- Block model material/topology refresh deve ficar separado de tint/orientation refresh quando inputs autoritativos permitirem.
- `Spawn Biome` pertence a `NewWorldConfig`; não criar outro resource autoritativo para a mesma escolha.
- Safe spawn forçado deve continuar no mesmo owner de segurança física, usando predicate de biome, sem duplicar collision/surface logic.
- Search compartilhado deve abstrair apenas o invariant de edição de texto; inventory/dropdown continuam owners de suas regras de filtro/opções.
- Preferência de posição do Target HUD pertence a `HudSettings`; targeting não deve conhecer layout.
- Queries mutáveis múltiplas do mesmo Component devem ter disjunção explícita (`Without`) ou usar `ParamSet` quando a sobreposição for intencional.
- Formatação de Rust não é requisito de CI.
- `cargo test` não é requisito de CI; executar manualmente somente sob pedido explícito do usuário.
- Não reabrir bugs antigos automaticamente; só se ativos/regredidos.

---

# Próximos passos

Se nenhum error/warning/runtime report tiver prioridade:

1. Confirmar Clippy + `cargo check` do HEAD `24501774...`; corrigir qualquer failure antes de novo código.
2. No próximo runtime do usuário, confirmar que o panic Bevy `B0001` não reaparece. Se reaparecer, usar o novo stack/output como prioridade absoluta e localizar qualquer segundo system conflitante.
3. Fazer validação runtime das mudanças de UI quando houver output/relato do usuário: Player HUD no inventory/pause, Target Block Position, scrollbars e Spawn Biome overlay/search.
4. Quando o usuário solicitar testes manuais, rodar `cargo test` e corrigir todos os failures antes de continuar.
5. Retomar inspeção objetiva de `Update`/`PostUpdate` por scans globais, builds síncronos, allocations temporárias e dirty writes.
6. Revisar integração/spawn de mesh apenas se houver ganho estrutural sem lifecycle parcial por submesh.
7. Revisar rebuild de seleção somente com ganho estrutural claro; não duplicar geração de volume só para remover pequeno sort local.
8. Manter `notify_loaded_chunk_neighbors` conservador enquanto metadata atual não provar overlap voxel-a-voxel.

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh background fora da main thread;
- integração, restore, unload, lighting e fluid work budgetados;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- evitar scans globais por frame, allocations temporárias e mutações idempotentes em hot paths;
- não trocar corretude por performance aparente;
- natural hydrology continua generation-authoritative.

# Comunicação

- direta e focada em ação;
- não repetir caveats de validação de rotina;
- não dizer “achamos a causa” sem evidência;
- durante sequências longas, atualizar apenas findings/blocos concluídos;
- atualizar `HANDOFF.md` depois de mudanças materiais.
