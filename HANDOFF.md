# HANDOFF — Asteria / Mineclone

**Canônico:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. `VERSION` raiz `0.20.3` enquanto o bloco de save estiver aberto; `Cargo.toml` `0.10.16` independente. O checkpoint `18c7715f7814f2f1b30a11203a5422fbb451272d` passou em Clippy rigoroso e Check: https://github.com/sarakborges/mineclone/actions/runs/35188546197. **Código posterior:** `525961383717a77151dde1e4d8502b5702a173a4`, CI https://github.com/sarakborges/mineclone/actions/runs/35217580685 inicialmente em andamento; consultar resultado antes de chamar verde. CI nunca equivale a QA em execução no Windows.

## Histórico integral obrigatório (não perder requisitos nem próximos passos)

- [Etapas 0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 e requisitos expressos de save](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`.
- [Etapas 28–31 integralmente, handoff anterior arquivado sem cortes](docs/handoffs/auditoria-28-31-2026-09-17.md), blob `9b46ae054ed205590d67a353fb5117c471e419a8`.
- [Handoff pré-auditoria](https://github.com/sarakborges/mineclone/blob/ea4a33e134e7ae55af614f34f5d194953b1a2292/HANDOFF.md); ler também `ARCHITECTURE.md`.

**Regras:** commits coerentes na `develop`; ao terminar cada bloco funcional incrementar VERSION major/minor/patch conforme impacto, nunca por documento isolado e não fechar sem QA; atualizar este handoff a cada checkpoint com SHA, resultado CI, limitação e próximo passo. Clippy `--all-targets --all-features -- -D warnings` + cargo check, corrigir warnings sem supressão; não executar/adicionar `cargo test` sem pedido explícito; preservar PNG/GLB autorais; pause NÃO congela mundo, fluidos ou horário; comunicar avanços concretos sem alegar trabalho em segundo plano ou repetir perguntas respondidas. O usuário pediu feedback frequente e avanço até limite desta execução.

## Estado das etapas 27–31

HUD `config.json` persistindo TargetBlockPosition, CI verde `a15991e`; UI Name, nome único/Copy of, reserva de diretório, manifesto versionado, serialização portátil de blocos/fluidos e chunks residentes/arquivados; world selection `src/screens/world_selection.rs` lê saves em disco; autosave intervalado 60 s com assinatura da posição, modo, inventário, regras e revisão de mundo; Leave World/Exit salva primeiro e não sai em falha; ao voltar ao menu descarrega recursos do mundo. Detalhes, limites e SHAs completos no arquivo de etapas 28–31. `d6a5bb0` CI verde: https://github.com/sarakborges/mineclone/actions/runs/35188043009. **Não afirmar roundtrip confiável até QA real.** O nome `src/screens/load_worlds.rs` no handoff histórico era impreciso: arquivo real `src/screens/world_selection.rs`.

## Etapa 32 — fallback e retenção de saves [CÓDIGO PUBLICADO; CI de 5259613 A CONFIRMAR]

`18c7715f7814f2f1b30a11203a5422fbb451272d` (CI verde https://github.com/sarakborges/mineclone/actions/runs/35188546197): `src/world/save_catalog.rs` procura gerações mais novas primeiro e tenta restaurar todas as células/chunks antes de aceitar uma; caso snapshot mais novo esteja corrompido tenta os anteriores. Publicação ocorre snapshot antes de manifesto, seguida de limpeza best-effort e aviso em falha. Retenção inicial preservava quatro manifestos por presença de arquivos, MAS podia contar cópias corrompidas como backups e excluir saves anteriores válidos.

`525961383717a77151dde1e4d8502b5702a173a4`: corrige risco anterior. `WorldSaveContext` fornece registries de block/fluid para `save_world`; a limpeza só estabelece cutoff após verificar quatro gerações decodificáveis via `load_snapshot`, inclusive os chunks; arquivos inválidos não contam e backups mais antigos são preservados. Otimização barata: com até quatro manifests completos mais manifesto zero, retorna sem revalidar. Não modifica assets. CI: https://github.com/sarakborges/mineclone/actions/runs/35217580685 — VERIFICAR status e logs, consertar warnings se houver. **Limitação crítica:** validação e reconstrução de até quatro mundos antigos é síncrona no frame de save; aferir travamento e transferir retenção para fluxo orçamentado/assíncrono seguro antes de fechar a feature. Se saves estiverem corrompidos continuamente, armazenamento pode superar quatro versões para proteger o último recuperável.

**Outras lacunas identificadas na revisão:** `load_snapshot` valida metadados e chunks, mas inventário (ToolRegistry) e duração real de relógio (DayNightCycleRegistry) ainda são validados apenas na tela `world_selection.rs` DEPOIS da escolha; um snapshot com esses problemas ainda pode bloquear o carregamento sem fallback. Mover validação completa de inventário, relógio e dimensão para a seleção de gerações e reutilizar na retenção. O catálogo `list_worlds` pode exibir horário de manifesto mais recente cujo payload é ilegível; harmonizar com candidato realmente recuperável. O contador `save_content_revision()` inclui mera geração/restauração de chunk; refatorar para distinguir edição real de streaming, sem gravar ao gerar terreno intocado. A gravação `publish_json` faz sync do arquivo antes do rename, mas durabilidade de diretório em queda de energia não foi comprovada.

## Próximos passos, ordem de segurança

1. Confirmar CI do SHA exato `5259613`; se falhar, inspecionar log e corrigir todos warnings; registrar último SHA verde neste cabeçalho.
2. Unificar validação completa de cada geração (metadados, blocos/fluidos, inventário e relógio) ANTES de selecionar ou podar; documentar fallback e timestamp efetivo.
3. Retenção sem reconstruir quatro mundos inteiros no frame: medir custo, arquitetar validação/limpeza orçamentada, sem escrever chunks a cada tick.
4. QA real Windows/restart: Name duplicado, dois mundos isolados, bloco/fluido/brush, chunks arquivados, inventário, posição, regras, relógio, autosave, Leave World, fechamento e reabertura, corrupção do último snapshot e interrupção entre snapshot e manifesto; testar permissões e erro sem unload. Não inventar resultados.
5. Somente quando ciclo funcional e QA estiverem concluídos subir `VERSION` semanticamente e encerrar bloco; manter `Cargo.toml` independente até decisão explícita.
