# HANDOFF — Asteria / Mineclone

**Canônico:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. `VERSION` raiz 0.20.3 (bloco de feature save ABERTO), versão Cargo 0.10.16 deliberadamente independente. Último código comprovadamente verde: `2ab6b17c6c88811cb1e63952fd6cba65b9850dc1`, CI Clippy `-D warnings` + Check https://github.com/sarakborges/mineclone/actions/runs/35183637652. Código validado por CI não equivale a QA do jogo/Windows/save.

## Histórico INTEGRAL obrigatório, inclusive próximos passos originais

- [Etapas 0–12 integralmente](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16 integralmente](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20 integralmente](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24 integralmente](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 integralmente e decisões expressas do usuário](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`, inclui requisitos de saves, Name, Load Worlds, Leave World e correção da preferência HUD.
- [Handoff pré-auditoria](https://github.com/sarakborges/mineclone/blob/ea4a33e134e7ae55af614f34f5d194953b1a2292/HANDOFF.md); `ARCHITECTURE.md` continua obrigatório. Não descartar nenhuma decisão, evidência ou próximo passo do histórico arquivado.

**Regras:** commits coerentes em `develop`; registrar evidência, SHA, QA e próximo passo de cada etapa, atualizar este handoff sempre; corrigir todos erros/warnings SEM supressão; Clippy rigoroso + Check na CI; não executar/adicionar `cargo test` sem pedido expresso; preservar assets autorais PNG/GLB, estruturas JSON apenas em `data/structures/`, dono único/change-driven/frame budget. Não repetir ritualisticamente que não executou cargo run nem alegar background. Versão sobe somente em bloco funcional fechado e QA. O usuário quer feedback contínuo e máximo avanço nesta resposta.

## Etapa 27 — correção do HUD [CÓDIGO + CI VERDE; QA EM JOGO PENDENTE]

`a15991ec8132780f20d87bd43d1eb9ccb6aa7562`: `TargetBlockPosition` serializável `snake_case` no `src/hud/mod.rs`; `GameConfig.miscellaneous.target_block_position` lê/grava a seleção no `config.json`, default Center compatível com configs antigas. CI https://github.com/sarakborges/mineclone/actions/runs/35183075575 `completed/success`. QA Windows: escolher Center/TopRight/Hidden, reiniciar e conferir visual e fallback antigo. Sem alteração de save.

## Etapa 28 — Name e ID único [CÓDIGO + CI VERDE; IDENTIDADE AINDA NÃO RESERVADA NO DISCO]

**Commits sequenciais:** `fa9aa021` cria `src/world/world_names.rs` com `available_world_name` e `validate_world_name`, `152c49c9` registra módulo, `79ce1189` inclui `name:String`/getter/setter/default `New World` em `NewWorldConfig`, `5a35ea1c` cria UI nativa, `a2cdd689` injeta campo antes de Seed e lê texto committed (não IME preedit), valida antes de Create, mostra erro e guarda nome resolvido no draft; `c5318436` conecta sistemas/UI, `15509eb4` adiciona locale inglês, `cccfd29d` conserta editor centralizado e elimina TextColor duplicada, `9b20d336` reduz complexidade de query, `2ab6b17c6c88811cb1e63952fd6cba65b9850dc1` encapsula parâmetros do footer em `SystemParam` para Clippy. Primeira CI de `15509eb4` FALHOU por `too_many_arguments` e `type_complexity`, log https://github.com/sarakborges/mineclone/actions/runs/35183514983; ambas corrigidas sem `allow` e CI final VERDE https://github.com/sarakborges/mineclone/actions/runs/35183637652 (Clippy `-D warnings`, Check).

**Semântica existente:** Name validado contra vazio, chars de caminho/controle, dispositivos reservados Windows, trailing dot/space, 200 unidades UTF16, prefixa `Copy of ` quantas vezes necessário após leitura case-insensitive de nomes em `worlds/`. Falha de IO retorna erro visível e impede Create. A rotina apenas RESOLVE um nome e NÃO reserva diretório; risco de race está documentado no próprio código: escritor futuro deve usar `fs::create_dir` exclusivo, nunca sobrescrever e revalidar colisão. Nome só está no `NewWorldConfig`; NÃO existe manifesto/save em disco, tampouco lista Load Worlds, autosave ou unload. Os dois testes unitários foram escritos mas NÃO executados (regra do projeto).

**QA pendente:** digitação Unicode e IME, Enter/Escape/foco, tela com campo extra sem overflow, nome repetido/case-insensitive/limite, permissões Windows, colisão por criação paralela. `available_world_name` usa Unicode lowercase como aproximação, só `create_dir` definitivo garante exclusividade. Nome de erro está hardcoded em inglês; ideal centralizar mensagem em locale ao finalizar UI.

**Próximo passo da etapa 28:** criar arquivo de save/manifests com versão, timestamp da gravação CONCLUÍDA e ID reservado atomicamente; garantir que Name usado é o da pasta criada antes de iniciar o mundo, sem perder preferências. Projetar serialização portátil de chunks editados carregados E arquivados com string IDs de block/fluid, propriedades, rotação, orientação, estados fluidos; luz derivada é reconstruída. Integrar leitura/escrita sem sobrescrever snapshots antigos.

## Etapa 29 — segurança de conteúdo pessoal [COMMIT DE INFRA, NÃO FEATURE SAVE]

`1a0c30c6c8e6e60e79ead32e2edf2719bbbea8a0` adiciona `/worlds` ao `.gitignore` raiz, preservando outras regras (`/Cargo.lock` continua intencionalmente ignorado). Isso impede versionamento acidental da pasta local e NÃO cria, grava nem carrega nenhum save. `develop` SHA verificado nesse commit. Arquivo `.gitignore` não justifica incremento VERSION. Verificar se build.cmd copia `worlds` apenas se explicitamente requerido; dados pessoais NÃO devem integrar distribuição.

**Próximo passo após etapa 29:** implementar domínio de persistência funcional, em blocos verdes e cumulativos: reserva de Name e manifesto versionado, armazenamento/restauração de edições loaded+archived por ID estável, publicação segura/rollback de falha, autosave 60s dirty-driven sem tick writes, flush Leave World/Exit, descarte completo da sessão em memória, tela Load Worlds apontando SEMPRE para disco, QA Windows/restart. Atualizar este documento a cada checkpoint com commits/CI/status reais. PENDENTE tudo isso; não declarar recurso disponível antes de integração e QA.
