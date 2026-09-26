# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, `develop`, Rust + Bevy 0.19.1. `VERSION` raiz 0.20.3 confirmado; Cargo.toml 0.10.16 independente. Último código com Clippy `-D warnings` e cargo check comprovados: `40b36d721d3b7546ae50a1aef087b30cd4ba789e`, https://github.com/sarakborges/mineclone/actions/runs/35175467885. CI não prova gameplay/Windows/FPS nem save persistente. Não executar cargo test sem pedido explícito. Preservar PNG e GLB autorais.

## Histórico integral — leitura obrigatória

- [Etapas 0–12 integralmente](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16 integralmente](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20 integralmente](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24 integralmente, snapshot LITERAL do HANDOFF do commit `4fbf8ad`](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Handoff anterior à auditoria](https://github.com/sarakborges/mineclone/blob/ea4a33e134e7ae55af614f34f5d194953b1a2292/HANDOFF.md) e `ARCHITECTURE.md` continuam vigentes. Cada arquivo acima contém evidências e próximo passo de CADA etapa, não são resumos opcionais.

**Escopo da auditoria:** 710 commits do baseline `05859d1`/0.12.33 até HEAD inicial `2d86724`/0.20.3, segmentos 512+57+31+11+99, não revisão linha a linha de todos os patches. Auditoria etapas 0–24 registrada. Versão 0.21.0 apenas após bloco completo validado por CI e QA in-game, não por documentação. Em cada passo registrar estado, evidência, SHA, QA pendente e próximo passo; commits pequenos na develop. Warnings corrigidos sem `#[allow]`.

## Etapa 25 — decisões explícitas do usuário [REQUISITOS CONFIRMADOS, SAVES AINDA NÃO IMPLEMENTADOS]

1. Pause NÃO congela nada: tempo, fluidos e simulação continuam; retirar clock-durante-pausa da lista de defeitos. Bloquear controles quando menus exigirem continua correto.
2. Persistência de mundo ENTRE EXECUÇÕES é obrigatória, em `worlds/{id}/`; `Load Worlds` atual de memória é funcionalidade incompleta.
3. Posição do target HUD escolhida pelo usuário deve persistir em `config.json`.
4. Desativação de `.github/workflows/commit-slime-asset.yml` explicitamente autorizada; NÃO executar gerador, NÃO alterar arquivos GLB/PNG. Retirada do YAML na develop preserva histórico Git; se a antiga branch ainda tiver cópia, NÃO declarar desabilitação global sem verificar os workflows ativos no repositório.
5. Novo campo **Name** na tela de geração. Name é ID único do mundo e pasta. Se colidir, prefixar `Copy of ` sucessivamente até obter nome livre. Tratar comparação case-insensitive em Windows, nomes/dispositivos reservados, separadores, traversal e trailing espaço/ponto sem renomear ou sobrescrever saves pré-existentes.
6. Todo save registra data e hora do ÚLTIMO SALVAMENTO CONCLUÍDO (UTC em disco, UI local). Salvar a cada tick está PROIBIDO: desenho inicial acertado de autosave a cada 60 segundos APENAS quando existir estado alterado, mais gravação nas saídas normais, sem serializar mundos inteiros a cada frame. Arquivos temporários e publicação atômica, revision/ordenação impedindo snapshots antigos sobrescreverem recentes; falha mantém versão anterior e permite retry.
7. **Decisão adicional explícita:** Leave World DEVE descarregar também o mundo da memória. Ordem obrigatória: coletar estado atual do jogador/regras/tempo + chunks carregados editados E arquivados → salvar/flush e confirmar publicação com sucesso → liberar `VoxelWorld`, `InMemoryWorldSave`/estado de sessão, campos/cache/tarefas de mundo e voltar ao menu. Se falha de save, informar e permanecer no mundo, sem apagar alterações. `Load Worlds` SEMPRE deve carregar dados de disco e nunca reutilizar instância anterior. Exit Game normal deve persistir e aguardar flush; saída por crash pode perder no máximo desde último autosave concluído.

**Evidências de implementação atual:** `src/world/save.rs` mantém seed, dimensão, regras e jogador em `Resource` sem arquivos. `src/world/setup/bootstrap.rs` requer `existing_world` para Load. `src/screens/pause_menu.rs` Leave World só guarda estado em memória e faz transição, ExitGame emite `AppExit` sem save; ambos precisam redesenho para evitar perda de dados. `src/voxel/world.rs` mantém edições em chunks carregados OU `archived_chunks`; salvar apenas loaded perde edições distantes. `src/voxel/chunk_archive.rs` é estrutura de RAM com `&'static str`/FluidId u16, NÃO formato portátil: serializar IDs estáveis, orientação, rotação, propriedades, estado do fluido; luz é derivada e reconstruível. `src/app/runtime_paths.rs` altera cwd para pasta do exe empacotado; escolher localização gravável `worlds` na raiz pretendida, conferir permissões/erro. Player, regras, horário e geração procedural devem ser preservados; entidades runtime exigem contrato próprio, NÃO afirmar persistência antes de implementar. Deletar `VoxelWorld` apenas depois da gravação bem-sucedida.

**Próximo passo da etapa 25:** remover workflow legado autorizado preservando histórico; implementação em blocos pequenos compiláveis: validação Name/ID e manifesto com timestamp; snapshot editado carregado+archived e conversão de IDs; escrita atômica ordenada, flush leave/exit, descarte do ECS; leitura e restauração do save. Depois config HUD, CI rigorosa e QA Windows para concluir versão. Não anunciar implementação por especificar comportamento.

## Etapa 26 — tela Load Worlds [REQUISITO CONFIRMADO, NÃO IMPLEMENTADA]

Clicar Load Worlds no menu SEMPRE abre nova tela `WorldSelection`/equivalente mesmo sem saves. Listar `worlds/{id}` válidos, com Name e data/hora do save concluído, seleção explícita, Load e Back. Estado vazio com mensagem. Atualizar inventário na entrada, não buscar arquivos a cada frame; save inválido deve ser apresentado de forma isolada sem bloquear demais. Seleção usa ID durável, nunca único slot em RAM; carregar prepara seed/dimensão/regras/jogador/chunks e encaminha à Loading. Não adicionar deleção/renomeação de mundos sem requisito. Harmonizar estilos e localização existentes. `src/app/game_state.rs` hoje não tem WorldSelection; `src/screens/mod.rs` sem plugin de lista; `src/screens/starting_screen.rs` liga LoadWorlds diretamente à memória: alterar todos coerentemente.

**Próximo passo da etapa 26:** implementar domínio de persistência e Name, depois menu WorldSelection end-to-end, autosave e segurança no Leave World; não criar tela fake que não carrega mundo. Registrar cada bloco com SHA, critérios de aceite e pendências. Retomar investigação Cargo.lock/inventário/ferramentas da etapa 24 quando bloco funcional estiver estável.

## Etapa 27 — persistência de preferência visual de target HUD [PATCH DE CÓDIGO; CI/QA PENDENTES]

**Mudanças:** `src/hud/mod.rs` adiciona `Serialize`/`Deserialize` ao enum `TargetBlockPosition`, codificação estável `snake_case`; `src/app/game_config.rs` inclui `miscellaneous.target_block_position` com `#[serde(default)]` já existente, restaura preferência no startup e salva a seleção junto às demais configurações. Configs anteriores sem esse campo permanecem compatíveis com o valor Center. Nenhuma alteração de mundo ou save de voxel nesta etapa. Este checkpoint usa o mesmo commit das duas mudanças de código; consultar histórico Git do arquivo para SHA exato e status de CI.

**Aceite pendente:** CI `cargo clippy --all-targets --all-features -- -D warnings` + `cargo check` no SHA deste commit; em jogo, alterar Center/TopRight/Hidden, fechar e reabrir aplicativo, conferir recuperação e compatibilidade com `config.json` antigo. Não executar `cargo test` sem pedido. Não subir VERSION enquanto bloco completo e QA estiverem abertos.

**Próximo passo após etapa 27:** implementar o domínio de save durável começando por ID único seguro e manifesto versionado com horário do save concluído; garantir estratégia de escrita/recuperação e integração com chunks carregados+arquivados antes de ligar a UI Load Worlds. Registrar cada bloco e só anunciar funcionalidade que realmente foi entregue.
