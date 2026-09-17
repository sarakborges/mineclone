# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`; Rust + Bevy 0.19.1. **Versão raiz `VERSION`: `0.20.3`** (bloco de saves não encerrado); `Cargo.toml`: `0.10.16`, independente. **Último código com CI verde:** `ff3918de1c3bff5d7acb842ebdaee8f3dfe71bd0`, https://github.com/sarakborges/mineclone/actions/runs/35227081508 — Clippy `--all-targets --all-features -- -D warnings` e `cargo check --locked` success. Não confundir CI com QA Windows ou roundtrip real.

## Histórico integral — consultar obrigatoriamente antes de alterar decisões

- [Etapas 0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 e requisitos expressos de save](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`.
- [Etapas 28–31 integralmente](docs/handoffs/auditoria-28-31-2026-09-17.md), blob `9b46ae054ed205590d67a353fb5117c471e419a8`.
- [Etapas 32–36 integralmente e requisitos/cache CI](docs/handoffs/auditoria-32-36-2026-09-17.md), blob `33ea041165e699a5a95480f60f9b84a26d33d005`: cópia integral do handoff anterior, sem excluir história ou pendências. Ler também `ARCHITECTURE.md`.

**Regras permanentes:** efetuar commits coerentes na `develop`; incrementar `VERSION` major/minor/patch ao concluir cada bloco funcional, nunca apenas por documentos e não encerrar bloco de saves sem QA. Atualizar este handoff após checkpoints com SHAs, CI, limites e próximos passos, arquivando íntegra antes de condensar. Corrigir erros e warnings sem supressão, Clippy rigoroso e cargo check, não adicionar/executar `cargo test` sem autorização expressa. Não alterar PNG/GLB autorais. Pause NÃO congela mundo/fluidos/horário. Cache efetivo de pacotes Linux, `Cargo.lock` restaurado antes de rust-cache e artefatos compilados entre commits; evitar reinstalar/recompilar Bevy sem necessidade. Não inventar QA ou execução do jogo; informar avanços concretos sem prometer trabalho posterior.

## Base de saves — etapas 28–36

UI de nomes exclusivos, reserva de diretório, manifesto e snapshots imutáveis (`worlds/{id}`, ignorado no git); chunks editados residentes/arquivados com IDs estáveis; inventário, posição, regras, dimensão, relógio e autosave condicional a cada 60s; Leave/Exit salvam antes de sair e erro mantém mundo carregado. Fallback tenta cada geração até validar estado jogável completo; retenção conserva quatro backups recuperáveis, poda valida backups em uma única thread e só trava durante exclusão; `load_world` trava enumeração+reconstrução para impedir corrida com a poda. `save_edit_revision` ignora worldgen/streaming/iluminação e aumenta após mutações efetivas de blocos/fluidos. Referências completas, ressalvas e SHAs no handoff 32–36 arquivado.

**Etapa 35, cache CI validado:** `7a556186943ef777a4d79fd5872a64a999ec697b` criou cache de `Cargo.lock` antes de `Swatinem/rust-cache`; `d5772dcc90b48eacd850a7b5a110c7ba6cae75db`, https://github.com/sarakborges/mineclone/actions/runs/35224192069, confirmou HIT exato no lock e artefatos (~412 MB), geração de lock pulada, Clippy 15,56s vs ~3m35s em run anterior. `Cargo.lock` permanece gitignored por decisão existente; índice crates.io ainda pode atualizar. Sem `cargo test`.

## Etapa 37 — data recuperável na lista de mundos e limite de snapshot [CÓDIGO + CI VERDE; QA PENDENTE]

- `ec6eb38542605c7dcc44d03bb121b564e250cc57`: `src/world/save_catalog.rs` expõe `PruneRegistries` próprio e `list_verified_worlds`, que enumera candidatos e, **somente quando chamado em worker**, tenta gerações em ordem decrescente usando `load_snapshot` + `validate_playable`; usa `SAVE_LOCK` enquanto abre/reconstrói cada mundo para impedir a poda de remover o candidato. Só exibe mundos com ao menos uma geração recuperável; timestamp é o da geração efetivamente validada, não de manifesto corrompido. Também rejeita na escrita JSON de snapshot acima de `MAX_SNAPSHOT_BYTES = 512 MiB` antes da publicação; um save não pode confirmar arquivo que seu próprio loader rejeitaria por tamanho.
- `ff3918de1c3bff5d7acb842ebdaee8f3dfe71bd0`, CI https://github.com/sarakborges/mineclone/actions/runs/35227081508 success Clippy rigoroso + cargo check: `src/screens/world_selection.rs` inicializa uma única tarefa `asteria-world-scan` reutilizável se a tela for reaberta durante varredura; copia registries do conteúdo, verifica snapshots fora do frame e entrega resultado por `Arc<Mutex<Option<Result<...>>>>`, consultado via `try_lock`. Enquanto analisa mostra 'Verifying saved worlds...'; após retorno cria botões e mostra timestamps verificados. Erros de iniciar/pânico capturado são exibidos; Load não avança antes de acabar a análise. O carregamento continua revalidando no clique, pois arquivos podem mudar após a varredura.

**Limites explícitos:** o worker ainda reconstitui um mundo completo por candidato e pode usar muita CPU/RAM em mapas grandes; a verificação ocupa `SAVE_LOCK` de cada mundo até concluir, podendo atrasar uma gravação simultânea. Não existe medição de tempo nem prova em Windows. Strings novas na UI ainda estão hardcoded em inglês e exigem integração com recursos de localização. Se todos os backups de um mundo falharem, ele fica fora da lista e há warning em log; não há UI de reparo. O snapshot novo ainda é capturado, serializado em memória e gravado via `fsync` na thread principal; o limite impede publicação impossível, mas não elimina custo de memória da serialização. CI não executou jogo nem testes de corrupção. `VERSION` continua `0.20.3`.

## Próximos passos — prioridade

1. Medir e reduzir custo síncrono do novo snapshot (`save_modified_chunks`, JSON, fsync) sem alterar contrato: Leave World/Exit só terminam após gravação confirmada e falha mantém mundo carregado. Investigar streaming/worker e sincronização com captura consistente; benchmarks reais no Windows sem estimativas inventadas.
2. Rever a varredura assíncrona da etapa 37 para orçamento de CPU/RAM, cancelamento e bloqueio do `SAVE_LOCK` em mundos grandes; localizar textos. O índice é snapshot do momento da varredura, revalidar no Load permanece obrigatório.
3. QA real Windows/restart: nome duplicado e mundos isolados, blocos/fluidos/brush em chunks arquivados, inventário, posição, regras, relógio, autosave/Leave/Exit e reabertura do processo; corromper geração mais recente e confirmar data de fallback; interrupção entre snapshot/manifesto, permissões, recuperação da poda, carga de mundos grandes. Sem alegar sucesso sem executar.
4. Preservar cache CI de lock e artefatos, `--locked` e Clippy `-D warnings`; não introduzir `cargo test` sem pedido.
5. Após QA e conclusão funcional, subir semanticamente `VERSION`; não sincronizar `Cargo.toml` sem decisão expressa.
