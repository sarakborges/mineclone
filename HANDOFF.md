# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`; integração regular em `develop`. **Trabalho de UI solicitado expressamente em branch separada:** `feat/ui-redesign-sidebar-pause`, PR #11 em draft: https://github.com/sarakborges/mineclone/pull/11. Versão raiz `VERSION` nesta branch: **0.21.2**; `Cargo.toml` segue `0.10.16` por versão independente. Bevy 0.19.1.

## Histórico completo preservado

O handoff integral anterior (etapas 0–43, histórico de saves, commits, decisões, CI, pendências e regras) está **imutavelmente preservado no commit** `15832f9d1b777d1411f236363518b5853e76447c`: https://github.com/sarakborges/mineclone/blob/15832f9d1b777d1411f236363518b5853e76447c/HANDOFF.md . Esta versão condensada não invalida nem substitui aquele registro; consultar seu texto completo e as referências arquivadas antes de mudar decisões. Handoffs anteriores: `docs/handoffs/auditoria-0-12-2026-09-17.md`, `auditoria-13-16`, `auditoria-17-20`, `auditoria-21-24`, `auditoria-25-27`, `auditoria-28-31`, `auditoria-32-36` (nomes completos no checkpoint imutável). Ler também `ARCHITECTURE.md`.

**Regras permanentes:** trabalhar na branch expressamente solicitada até revisão, sem merge automático; cada bloco funcional altera `VERSION` semanticamente; atualizar este handoff com commits, CI e limites. Não substituir PNG/GLB autorais; não inventar QA, execução Windows, UI verificada visualmente, save roundtrip ou correção de problemas de render. Não adicionar/executar `cargo test` sem autorização expressa. Corrigir warnings sem supressões; preservar cache efetivo CI e workflow existente, sem criar workflow novo. Pausa NÃO congela mundo/fluidos/horário. Preserve semântica de Leave/Exit: só sair após salvar, falha mantém mundo carregado. UI com atualizações por change detection, sem reconstruir todos os frames. **Cards sem bordas**; contornos permitidos em botões secundários e controles interativos.

## Base histórica e pendências independentes

Etapas 28–41: sistema de saves e backups, manifesto/snapshots, fallback verificado, scanner de mundos em worker, instrumentação `capture`/`publication`, escrita e leitura JSON streaming/buffered, localização do feedback de world selection. Ver handoff integral anterior para limites específicos, commits e logs CI. QA Windows de saves (corrupção, backups, fsync, autossalvamento, inventário/posição, chunks residentes e arquivados) continua pendente. Não confundir CI com QA runtime.

## Etapa 42 — organização de settings [FEATURE BRANCH; VISUAL INCOMPLETO]

Branch criada de `develop` em `cfa934acf0cf34627e995c99b977ae6a0323737c`; commits `502cbe5`, `20fc212`, `b833173`, `f0b92c1`, `84e8df0`, `938ef08`, `11831d7` introduziram `SettingsScope`, seções de pausa WORLD/GAME, categorias mundo separadas de configuração geral, sidebar x=0 (somente no corpo entre cabeçalho e rodapé), conteúdo flexível, textos e containers sem bordas. Versão 0.21.0. Apesar dos commits e CI verde, era essencialmente **refatoração de navegação e layout**; o antigo modal de pausa e o menu inicial centralizado foram mantidos visualmente, portanto foi incorreto apresentá-la como redesign visual concluído. Não repetir essa alegação.

## Etapa 43 — crash de localização [CORRIGIDO NOS DADOS; CI VERDE; WINDOWS PENDENTE]

Usuário executou Windows e `UiLocalization::load` abortou: inglês 81 chaves, PT-BR/espanhol 76. Commits `0f306aee5951affdd292eb5fbbf5e04c113faa25` e `0a7b99c784c6c7535fee47c54df65a9d15d5e118` completaram 5 strings nos dois idiomas; `4b538adece7652dcc4890ed0d1fe12c9ae091bc8` e `f4f26485071da587fe5018558b562ab518e1dd1f` adotaram script de auditoria `tools/check_localizations.py` e passo no workflow CI **já existente**; versão patch 0.21.1 em `2fde2368becd35dd9144f9399c11a793001ea1c8`. CI https://github.com/sarakborges/mineclone/actions/runs/35240261014 success (auditoria, Clippy -D warnings, cargo check). Erros Vulkan de loader de arquivos EOS Overlay Epic são anteriores e distintos do panic. Confirmar execução Windows depois do pull, não alegar execução confirmada.

## Etapa 44 — tornar a alteração visual perceptível [VERSÃO 0.21.2; CI VERDE; QA VISUAL PENDENTE]

Usuário informou que a UI não havia mudado visualmente. Inspeção do diff do PR confirmou que a etapa 42 manteve o visual de menu inicial centralizado e o modal de pausa compacto, alterando apenas categorias/estrutura/posição de sidebar no corpo. Também **não sabemos qual branch está checked out no computador Windows**: `cargo run` mostra a versão de `Cargo.toml` (`0.10.16`), que é igual nas branches; conferir `git branch --show-current`, `git rev-parse HEAD` e `type VERSION`. `develop` permaneceu em `VERSION 0.20.3` na inspeção; não atribuir com certeza a ausência de UI à branch sem checar localmente.

- `aeea179a338c98815468aba0c8ade55c23092b97`: redesign VISÍVEL de `src/screens/pause_menu.rs`: substitui modal 560px e lista vertical pela superfície ampla até 1100px com cabeçalho, dois grupos lado a lado WORLD/GAME em cards sem bordas, acentos discretos e botões contextuais. Sem alteração na lógica de save ou de transição.
- `8f023a21f8aa49adc8a37d5301fc627566e02030`: `src/ui/button.rs` torna `menu_button` fluido (100% do contêiner, máximo 470px, mínimo zero e shrink) para funcionar nos grupos e reduzir overflow do rodapé em larguras menores; precisa de QA de texto/720p.
- `de94ce359209c7244557d0bf10351462cb2a49f8`: `src/screens/starting_screen.rs` deixa de centralizar logo e quatro botões soltos; divide brand hero à esquerda e superfície de navegação escura, sem borda, à direita, mantendo ações e textos. Não alterou PNG de logo.
- `5b03cdf1f4d916ea69d3a91aa0c2fa4c68e5e065`: `VERSION` 0.21.1 → **0.21.2** (patch de conclusão visual da primeira etapa); Cargo.toml intacto.
- CI de `de94ce3` https://github.com/sarakborges/mineclone/actions/runs/35241158879 **success** (auditoria localização, Clippy -D warnings, cargo check --locked). CI do bump de VERSION https://github.com/sarakborges/mineclone/actions/runs/35241221835 **success** também nas três etapas. O workflow existente não foi duplicado. Handoff de checkpoint atualizado em `891ecbe9034a279faebf54a5b1e2fb3971e09c8c` e neste commit de estado.

**Falta:** confirmar branch e versão no Windows, `cargo run`, tela principal e pause realmente visíveis, Sidebar x=0 e settings contextuais; revisar 720p/1080p/1440p, resizes, legibilidade, botões em rodapé, modal da lista de mundos, navegação e save Leave/Exit. Main menu e pause foram redesenhados no código mas não fotografados/executados pela assistente. Settings ainda tem header/rodapé separados e aparência de conteúdo bastante próxima da anterior: não prometer redesign completo de settings/HUD; prosseguir iterativamente com evidências visuais. `develop` avançou independentemente e divergiu da feature branch: sincronizar com cuidado depois de QA, nunca fazer merge sem aprovação. PR segue draft.

## Próximo passo imediato

1. Confirmar branch e versão no Windows (`git branch --show-current`, `type VERSION`); usar branch `feat/ui-redesign-sidebar-pause` atualizada e executar `cargo run`. Se já estiver nela, tratar relato como insuficiência do redesign, não erro do usuário.
2. Com prints novos ou validação de execução, polir layout de settings (sidebar de altura integral, título/rodapé harmonizados), demais menus e HUD, respeitando cards sem bordas.
3. Manter QA de saves anterior pendente e preservar fluxo de save existente. Não criar workflow novo nem `cargo test` não autorizado.
