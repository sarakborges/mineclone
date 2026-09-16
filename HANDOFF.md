# HANDOFF — Asteria / Mineclone

**Fonte canônica da branch de trabalho atual:** `feature/game-chat-creature-command`, baseada em `feature/slime-creature-integration` (PR #9, baseada em `develop`). Repositório `sarakborges/mineclone`, Rust + Bevy 0.19.1. **VERSION = 0.17.0 nesta branch; branch de criaturas = 0.16.0; develop = 0.15.41 ao iniciar o trabalho.** `Cargo.toml` usa deliberadamente versão distinta: não harmonizar com `VERSION`.

**Histórico completo, NÃO descartar:** [HANDOFF imediatamente anterior à branch do chat, cobrindo .40 e todos os nove problemas](https://github.com/sarakborges/mineclone/blob/ba80f79bc263e64c4f5a34181f1d7b32521c67af/HANDOFF.md); [HANDOFF .41 na develop (fix de publicação de luz interativa)](https://github.com/sarakborges/mineclone/blob/479d9d09605e58584a528222969f575d145f4c4e/HANDOFF.md); links às versões anteriores também existem em ambos. Compactação documental não fecha bugs nem prova gameplay. Este handoff dá continuidade aos históricos vinculados.

## Regras obrigatórias

`continua`/`go` = executar, não só planejar. **Alterações de código exclusivamente em branch nova ou na branch de feature pedida pelo usuário, nunca diretamente em `develop` sem nova autorização; regra mais recente prevalece sobre handoffs antigos.** Conferir HEAD, `VERSION`, arquivos e status CI antes de gravar. Cada bloco de código/dados jogáveis incrementa `VERSION` conforme SemVer; documentação isolada não. Atualizar HANDOFF na mesma sessão de cada alteração material com branch, versão, commits/HEAD, CI, pendências e próximo passo. Corrigir erros e warnings reais, não silenciá-los. CI canônica `.github/workflows/ci.yml`: `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`, não executa testes unitários nem valida gameplay/renderização. Não afirmar testes que não foram verificados, não repetir desculpas sobre cargo. Preservar arquivos binários originais, sem placeholders; sem gerar imagens não solicitadas.

## Arquitetura e invariantes

`ARCHITECTURE.md` autoritativo: owner único por fato, hot paths change-driven, evitar full scans e dirty writes. UI compartilhada em `src/ui`, HUD em `src/hud`, conteúdo em registros data-driven. Pipeline async chunks, meshes terrain/fluids/light e revisão/queues separados; halo 3×3×3 e seed por residência .38; unload remesh dos 26 vizinhos relevantes .40; publicação de luz interativa ao final de cada batch .41 (esta última correção EXISTE em `develop`, mas NÃO está incorporada automaticamente à branch de criaturas nem à do chat: reconciliar/rebase antes de merge). Céu por bioma, órbitas e horários sol/lua data-driven; hidrologia com endpoints X/Z/Y e margens gerais .12; hotbar observada pelo held block, tecla R apenas posiciona.

## Status — 16/09/2026

**Chat `0.17.0`**: branch `feature/game-chat-creature-command`, PR [#10](https://github.com/sarakborges/mineclone/pull/10) DRAFT direcionado à branch `feature/slime-creature-integration`, não diretamente à `develop`. Commits principais: `4b2e620` spawn genérico reutilizável; `8911e001` implementação inicial do chat; `93c7c9dc` registro na HUD; `07e3c2c`/`1d780c3` interação de cursor/câmera; `8a7e561` bloqueio do mundo; `2f58d88` ESC sem pausar; `eb7d919` inventário; `a7f27aa` bump 0.17.0; `f68a586` história cronológica e separação UI; `259eced` layout/chat com scroll; `03d1634` scrollbars automáticas. SHA HEAD de código ao escrever: `03d1634e9ff9930d41aa23389cdf74a5f342900f`; HANDOFF commit posterior.

**Especificação do chat:** HUD translúcida flutuante no lado esquerdo acima do player HUD (`left=18`, `bottom=138`). `T` abre e foca campo de texto, sem inserir T inicial; `Enter` envia e fecha; `Esc` fecha sem acionar pause; texto de até 256 caracteres. Entrada normal vira `<Yogg'Sara>: mensagem` (nome provisório igual à HUD do player; integrar identidade de jogador quando existir). Entrada iniciada por `/` vai para interpretador; `/spawn_creature <id>` resolve somente `CreatureRegistry` carregado dos JSONs, tenta nascer nas MESMAS coordenadas do jogador (pé = câmera - eye height, X/Z exatos), valida chunk carregado e AABB livre, cria entidade com modelo/collider/animações/tints escolhidos pela definição; sucesso, uso incorreto, ID desconhecido ou obstáculo retornam mensagens no chat. Não é sistema final de spawning por bioma, combate, rede ou persistência.

**Ordem e scroll, correções exigidas pelo usuário:** mensagens **antigas em cima e novas ABAIXO**, `VecDeque` cronológico `push_back`, retenção das 64 últimas. Janela/corpo crescem naturalmente PARA CIMA, ancorados no rodapé acima da HUD; histórico tem **altura MÁXIMA equivalente a 15 linhas VISUAIS de texto**, não altura fixa e não limite de 15 mensagens; textos com wrap entram no limite. `ChatHistory` usa `max_height = 15 * 22 px`, `Overflow::scroll_y`, `ScrollPosition`, scrollbar automática e autoscroll para o final medido na fase `Last`; roda mouse wheel com chat aberto. Lista some após 10 segundos desde última mensagem se chat fechado; com chat aberto permanece visível. Ajustar/validar ergonomia visual e medidas do texto no jogo.

**Auditoria de TODAS as HUDs com scroll:** no `src/hud`, scrollbars existentes do inventário (`src/hud/inventory/layout.rs`): `CreativeCatalogScrollArea` e `CreativeCategoryScrollArea` já usam `scrollbar::vertical_scrollbar` automático, áreas limitadas por `CREATIVE_GRID_HEIGHT`. Novo chat também usa o helper. `src/ui/scrollbar.rs` agora inicializa scrollbars automáticas em `Display::None`, verifica `ComputedNode.content_size().y > ComputedNode.size().y + 0.5`, mostra somente se houver overflow visual efetivo, preserva versão `persistent_vertical_scrollbar` para consumidores que optarem explicitamente por persistência. Testes de parser/ordem/capacidade, limite de 15 linhas e limiar de overflow escritos; testes unitários não rodam na CI canônica.

**CI da PR #10:** workflow `Rust validation` run `35148336319`, job `104969982586` estava **in_progress / Clippy in_progress** na última consulta; Check pendente. Consultar novamente, corrigir compilação/Clippy na branch de chat e atualizar esta seção com status verificável. CI da branch de criaturas PR #9 não estava confirmada quando este trabalho começou; não confundir com sucesso do workflow binário do GLB. Não se pode afirmar carregamento gráfico, scroll nem gameplay válidos sem execução/feedback.

**Branch `0.16.0` / criatura pré-requisito:** PR [#9](https://github.com/sarakborges/mineclone/pull/9) DRAFT. Registro genérico de `data/creatures/**/*.json` com `id`, `name` obrigatório `LocalizedText`, `model` relativo a `assets/models/creatures/`, collider AABB na raiz não animada, animações nomeadas, tints HSI por instância, parâmetros de salto. Meadow/Ember JSONs compartilham `assets/models/creatures/slime/slime.glb`; binário original foi efetivamente commitado e listado no repositório depois de verificação SHA-256, junto do gerador e arquivo collider. Spawn preview opt-in, física simples, sem IA final. Rebase da correção .41 de develop necessário antes de merge.

## Defeitos de mundo anteriores — NÃO FECHADOS por esta feature

1. P0 chunks inteiros pretos / costuras iluminação/AO; .33/.38/.39/.40 e .41 são correções de código delimitadas, sem confirmação de gameplay/FPS.
2. P0 rios terminando abruptamente, validar grafo/leito/água física entre chunks/regiões.
3. P0 diversidade de biomas: em 2 mil blocos somente Plains/Mountains, coletar amostras por seed/coords.
4. P0 em descidas margem abaixa antes da água; validar suporte de fluido sem gerar água suspensa.
5. P1 confluência rio–lago abrupta (perfil XY, largura, profundidade, margem e altura).
6. P1 túneis com paredes retas ao tocar superfície e paredes extras em interseção com rio.
7. P1 lua parece acompanhar câmera.
8. P1 lua visível de dia; .32 não confirmado visualmente.
9. P1 target highlight não engloba todas as camadas da textura.

Outras pendências antigas: fogColor artístico, medição FPS ao caminhar, HUD status do jogador, held/ghost regressões somente se reproduzidas. Único fechamento confirmado pelo usuário naquele ciclo: céu por bioma em .24; demais não fechar sem feedback. Não perder requisitos de player HUD inferior esquerdo `Yogg'Sara`/`50 / 100`, target HUD acima crosshair com bloco à esquerda e texto à direita, sombra tooltip.

## Próxima execução

1. Conferir PR #10 run `35148336319` e Clippy/Check; corrigir bugs reais e warnings na branch do chat; se correção de código depois do bloco publicado, subir VERSION conforme SemVer e atualizar HANDOFF.
2. Revisar layout ao vivo: scrollbar oculta com <=15 linhas visuais; aparece com >15 e em ambas áreas do inventário somente sob overflow; wrap, mouse wheel, autoscroll último, resize, posição acima da player HUD, 10 s, T/Enter/Esc, texto ao digitar sem ação no mundo.
3. Validar `/spawn_creature asteria:meadow_slime`, `asteria:ember_slime`, ID inexistente, ausência de argumento, chunks descarregados e AABB ocupado; inspecionar material/GLB/anim.
4. Reconciliar `develop` .41 à branch de criaturas e à do chat antes de merge em cadeia; não mesclar PRs rascunho sem checks e avaliação de gameplay. Depois retomar P0 mundo do handoff histórico.
