# HANDOFF — Asteria / Mineclone

**Canônico:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. `VERSION` raiz `0.20.3` enquanto o bloco de save estiver aberto; `Cargo.toml` `0.10.16` independente. **Último código comprovadamente verde:** `99dcc63ec579c9302c67f3e31c307c9885c89f54`, CI https://github.com/sarakborges/mineclone/actions/runs/35221981757 — Clippy `-D warnings` e `cargo check` ambos success. CI não equivale a QA no Windows ou teste de salvar/reiniciar.

## Histórico integral obrigatório (não perder requisitos nem próximos passos)

- [Etapas 0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 e requisitos expressos de save](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`.
- [Etapas 28–31 integralmente, handoff arquivado sem cortes](docs/handoffs/auditoria-28-31-2026-09-17.md), blob `9b46ae054ed205590d67a353fb5117c471e419a8`.
- [Handoff pré-auditoria](https://github.com/sarakborges/mineclone/blob/ea4a33e134e7ae55af614f34f5d194953b1a2292/HANDOFF.md); ler também `ARCHITECTURE.md`.

**Regras:** commits coerentes na `develop`; ao terminar cada bloco funcional incrementar VERSION major/minor/patch conforme impacto, nunca por documento isolado e não fechar sem QA; atualizar este handoff a cada checkpoint com SHA, resultado CI, limitação e próximo passo. Clippy `--all-targets --all-features -- -D warnings` + cargo check, corrigir warnings sem supressão; não executar/adicionar `cargo test` sem pedido explícito; preservar PNG/GLB autorais; pause NÃO congela mundo, fluidos ou horário; comunicar avanços concretos sem alegar trabalho em segundo plano nem repetir perguntas respondidas. O usuário pediu feedback frequente e avanço até limite desta execução.

## Estado das etapas 27–31

HUD `config.json` persistindo TargetBlockPosition, CI verde `a15991e`; UI Name, nome único/Copy of, reserva de diretório, manifesto versionado, serialização portátil de blocos/fluidos e chunks residentes/arquivados; world selection `src/screens/world_selection.rs` lê saves em disco; autosave intervalado 60 s com assinatura da posição, modo, inventário, regras e revisão de mundo; Leave World/Exit salva primeiro e não sai em falha; ao voltar ao menu descarrega recursos do mundo. Detalhes, limites e SHAs completos no arquivo de etapas 28–31. `d6a5bb0` CI verde: https://github.com/sarakborges/mineclone/actions/runs/35188043009. **Não afirmar roundtrip confiável até QA real.** O nome `src/screens/load_worlds.rs` no handoff histórico era impreciso: arquivo real `src/screens/world_selection.rs`.

## Etapa 32 — fallback e retenção de saves [CÓDIGO + CI VERDE; QA PENDENTE]

`18c7715f7814f2f1b30a11203a5422fbb451272d` (CI verde https://github.com/sarakborges/mineclone/actions/runs/35188546197): `src/world/save_catalog.rs` procura gerações mais novas primeiro e tenta restaurar todas as células/chunks antes de aceitar uma; snapshot mais novo corrompido causa tentativa dos anteriores. Publica snapshot antes do manifesto; limpeza posterior best-effort e warning em falha. Retenção inicial contava presença de arquivo erroneamente.

`525961383717a77151dde1e4d8502b5702a173a4` (CI verde https://github.com/sarakborges/mineclone/actions/runs/35217580685): limpeza só estabelece cutoff após verificar quatro gerações decodificáveis via `load_snapshot`; arquivos inválidos não contam, gerações zero/antigas recuperáveis preservadas. Otimização com até quatro manifestos completos mais o zero: retorna sem revalidar. **Limitação:** a validação e reconstrução de até quatro mundos é síncrona no frame da gravação, ainda exigindo transferência para caminho orçamentado/assíncrono seguro e medição. Com corrupção contínua, quantidade de arquivos pode superar quatro para não perder backups bons.

## Etapa 33 — fallback para estado jogável completo [CÓDIGO + CI VERDE; QA PENDENTE]

`99dcc63ec579c9302c67f3e31c307c9885c89f54`, CI https://github.com/sarakborges/mineclone/actions/runs/35221981757 (Clippy rigoroso + Check success): `SaveRegistries` encapsula registries de blocos, fluidos, ferramentas, dimensões e ciclos; `validate_playable` confirma existência da dimensão/ciclo, dia com duração positiva, tick dentro do dia, comprimento e IDs de inventário (via `PlayerHotbar::default().restore_items`). `load_snapshot` usa essa validação ANTES de aceitar geração; `load_world` tenta a anterior em qualquer falha. A poda reutiliza `load_snapshot`, portanto inventário ou relógio inválidos também NÃO contam como backup recuperável. `WorldSaveContext` passa os registries ao escritor; este valida o próprio snapshot antes da publicação. A tela só altera inventário e recursos ativos após `load_world` terminar com sucesso; checagem redundante de restauração do inventário foi preservada como proteção. Commit toca apenas `src/world/save_catalog.rs`, `src/world/save_session.rs`, `src/screens/world_selection.rs`.

**QA não feito:** não houve execução real Windows nem teste de corrupção/roundtrip, e não foi executado `cargo test` (regra do projeto). Timestamp do catálogo é do manifesto mais recente publicado e pode não corresponder à geração recuperada por fallback; alinhar UI depois. O fallback não é promessa de recuperação caso TODAS as gerações estejam ilegíveis ou incompatíveis.

## Próximos passos, ordem de segurança

1. Remover a reconstrução síncrona de até quatro mundos antigos do frame de save; preservar lock contra corrida entre gravação, carregamento e limpeza; manter publicação síncrona, cleanup best-effort e logs, com medição de memória/performance. Não declarar isso resolvido só por compilar.
2. Revisar `list_worlds` para timestamp realmente recuperável sem bloquear a UI; distinguir revisão de edição real de geração/streaming (`save_content_revision()` inclui geração/restauração), sem escrita por tick.
3. QA real Windows/restart: Name duplicado, dois mundos isolados, bloco/fluido/brush, chunks arquivados, inventário, posição, regras, relógio, autosave, Leave World, fechamento e reabertura, corrupção da geração mais recente e interrupção entre snapshot/manifesto; testar permissões/erro sem descarregamento. Não inventar resultados.
4. Somente após ciclo funcional e QA, subir `VERSION` semanticamente e encerrar bloco; manter `Cargo.toml` independente até decisão explícita.
