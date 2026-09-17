# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz `VERSION`: `0.20.3`**, bloco de saves aberto até QA Windows. `Cargo.toml` `0.10.16` tem versionamento independente. **Último código aprovado:** `1d58862a4c04983a98041189ed8fb272a59b3245`, CI https://github.com/sarakborges/mineclone/actions/runs/35258727224 (auditoria de idiomas, Clippy rigoroso e `cargo check --locked` concluídos). Este run NÃO executa o jogo nem testa o script Python; verificação sintética separada descrita abaixo.

## Histórico canônico — obrigatório preservar

- [Etapas 0–12](docs/handoffs/auditoria-0-12-2026-09-17.md), blob `34161a1ec793041a738ec704e41dc345cc87fff6`.
- [Etapas 13–16](docs/handoffs/auditoria-13-16-2026-09-17.md), blob `cbbb6710e8be6b132467c943f03ed8df326509e3`.
- [Etapas 17–20](docs/handoffs/auditoria-17-20-2026-09-17.md), blob `bc4a5e7f5ff723cc6f52ef0c50edce73b37b5da2`.
- [Etapas 21–24](docs/handoffs/auditoria-21-24-2026-09-17.md), blob `8b35a7c3680f7df1d3aaf4e142fcf39ed605036d`.
- [Etapas 25–27 e requisitos de save](docs/handoffs/auditoria-25-27-2026-09-17.md), blob `681c00fbb1674687affd4cf87d06c9317213c707`.
- [Etapas 28–31 integrais](docs/handoffs/auditoria-28-31-2026-09-17.md), blob `9b46ae054ed205590d67a353fb5117c471e419a8`.
- [Etapas 32–36 integrais](docs/handoffs/auditoria-32-36-2026-09-17.md), blob `33ea041165e699a5a95480f60f9b84a26d33d005`.
- [Etapas 37–44 integrais](docs/handoffs/auditoria-37-44-2026-09-17.md), blob `7a097aac02ebbcad2b5cadf0a2a79dc3cf191b67`.
- [Etapas 45–53 integrais + handoff anterior](docs/handoffs/auditoria-45-53-2026-09-17.md), blob `7ba6c7e95dccd0c8a5cf8be6dccff4f3d3ac5173`; arquivo criado por referência ao MESMO blob em `e6c721cfb781db13dfacefac7f44bc76dd5a100c`, sem reescrita nem perda de linhas. Inclui todos os commits, runs, limitações e ressalvas. Ler também `ARCHITECTURE.md`.

**Regras permanentes:** commits coerentes na `develop`, atualizar este handoff em cada checkpoint com SHA/CI/limitações/seguintes e arquivar o texto integral antes de condensar. Incrementar `VERSION` por semver após um bloco funcional concluído, nunca por docs isoladamente e nunca fechar saves antes de QA real. Corrigir warnings SEM supressão. CI: auditoria de PT-BR/espanhol/inglês, Clippy `--all-targets --all-features -- -D warnings` e `cargo check --locked`. Não executar/adicionar `cargo test` sem autorização expressa. Não alterar PNG/GLB autorais. Pause NÃO congela tempo/fluidos/mundo. Preservar cache de CI; `Cargo.lock` é gitignored. Não alegar runtime, performance ou Windows não verificados. `docs/save-roundtrip-qa.md` é protocolo, não prova.

## Estado do código ao fim da etapa 53

Mundos com ID isolado, save imutável snapshot + manifesto commit, persistência de chunks alterados residentes/arquivados, IDs portáveis, posição/inventário/regras/relógio e brush. Autosave 60s apenas com alterações persistidas; Leave/Exit confirmam publicação ou permanecem no jogo. Fallback e retenção verificam snapshots/chunks/inventário/relógio, preservam quatro gerações recuperáveis; leitura/scan e poda em workers com trava por mundo, leases durante fallback e poda aguardando leitores sem travar writer. Carga cancelada descarta VoxelWorld no worker ou thread de descarte. Logs separam captura/publicação e custo de copiar registries, scan e load. Dup de chunks rejeitada antes de decodificar; cada chunk reconstruído é arquivado diretamente num mundo privado. Nomes de dispositivos Win32 COM¹/²/³, LPT¹/²/³, CONIN$/CONOUT$ rejeitados; cópias de nomes no limite de 200 UTF-16 recebem sufixos numéricos seguros. **A corrida no `create_new_world` após `create_dir AlreadyExists` para nomes longos AINDA NÃO foi resolvida.** Histórico integral e detalhes técnicos no arquivo de etapas 45–53.

## Etapa 54 — fixtures de corrupção isoladas [SCRIPT + VERIFICAÇÃO SINTÉTICA; QA DO JOGO PENDENTE]

- `1d58862a4c04983a98041189ed8fb272a59b3245` adicionou [`tools/make_save_fixture.py`](tools/make_save_fixture.py), blob `f23609d4ca0ef43bfae757ca373b80b97bbceb95`. Cria `output_root/worlds/<id>` sem tocar a árvore original; recusa destino existente ou dentro da fonte, fonte com symlinks, snapshots-base acima de 64 MiB (limite desta ferramenta, **não** do jogo), menos de duas gerações completas e metadados incompatíveis. Modifica apenas a geração mais recente da cópia: `snapshot-json`, `manifest-json`, `inventory-id`, `clock`, `duplicate-chunk` ou `player-null`. Mantém backups anteriores intactos e exige chunk já editado para caso de duplicação. A interpretação de `player-null` é exploratória: o código atual aceita `None`, sendo preciso decidir compatibilidade antes de mudar fallback.
- Verificação local separada: `python3 -m py_compile` passou; fixture sintética de duas gerações exercitou os seis modos, conferiu outputs distintos/original byte-a-byte inalterado e rejeitou reuso de output. Hash git da cópia local verificada = blob do GitHub `f23609d...`. **Nenhum jogo Rust, nenhuma leitura real do Windows e nenhum `cargo test` foram executados.** CI do commit https://github.com/sarakborges/mineclone/actions/runs/35258727224 completed/success (idiomas, Clippy, check), mas pipeline não executa este Python.
- `d63dc342ae52c3f5b32274acfa2d801c8a8866c1` atualiza [QA Windows](docs/save-roundtrip-qa.md) com comando PowerShell, procedimento de troca de cópia, seis modos, limitações, caso exploratório de jogador ausente, duplicatas de chunk, nomes no limite e corrida de prune. Não houve execução no Windows.
- `e6c721cfb781db13dfacefac7f44bc76dd5a100c` arquiva etapas 45–53, por referência ao mesmo blob, antes de compactar a fonte viva neste documento.

## Próximos passos, em ordem

1. Executar QA real Windows do [roteiro](docs/save-roundtrip-qa.md): rodar jogo/restart com backups descartáveis, coletar logs de duração de captura/publicação/cópia de registries/scan/load, medir stalls, RAM/CPU e registrar PASS/FAIL/NOT RUN. O script não demonstra restauração, só prepara cópias.
2. Validar roundtrip de chunks arquivados e brush/fluidos/inventário/posição/modo/regras/relógio; autosave, Leave/Exit, falha de escrita, corrupção+fallback+timestamp, intervenção entre snapshot e manifesto e poda concorrente. Investigar `player-null` e compatibilidade do formato 1 antes de exigir `Some(player)`.
3. Corrigir a corrida de nome longo em `save_catalog.rs::create_new_world`: o ramo `AlreadyExists` ainda chama `available_world_name(&format!("Copy of {candidate}"))`, que pode exceder 200 UTF-16. Reusar nome originalmente solicitado ou criar API explícita de nova tentativa, mantendo `create_dir` exclusivo.
4. Inspecionar custo/concorrência CPU/RAM de scan, load e prune; um save durante `prune_running` pode deixar backups extras até o próximo. Avaliar timeout de leitor sem sacrificar backups; usar medições reais antes de otimizar capture/publication que ainda bloqueia o frame.
5. Manter `VERSION` em `0.20.3` até QA de bloco e incremento semver adequado. CI/cache e restrição de `cargo test` invariantes.
