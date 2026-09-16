# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch de trabalho: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras de trabalho

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica e persistente do projeto. Exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`; feature branch só sob pedido explícito.
- Antes de escrever, buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Commits pequenos e coerentes; não misturar mudanças sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível; minor para feature compatível; major para breaking.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- Commit exclusivamente documental de `HANDOFF.md` não sobe `VERSION`.
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Corrigir warnings de Rust nos blocos tocados.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando aplicável.
- Não gerar imagens sem pedido explícito.
- Quando o usuário disser `go`/`continua`, executar o próximo bloco aplicável sem confirmação desnecessária.
- Não repetir que falta `cargo check`/`cargo run`; CI canônico valida Clippy + check.
- `cargo test` só é rodado manualmente sob pedido explícito.
- `cargo fmt`/`rustfmt` não é gate do projeto.

## Versionamento e validação

- Fonte operacional: arquivo raiz `VERSION`.
- `src/app/version.rs` mantém `VERSION` separado de `Cargo.toml` para evitar invalidar fingerprints do Cargo em bumps frequentes.
- `[package].version = 0.10.16` permanece intencionalmente divergente enquanto essa política existir; não sincronizar silenciosamente.
- CI `.github/workflows/ci.yml` executa:
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo check`
- Push/PR exclusivamente de `HANDOFF.md` é ignorado via `paths-ignore`; código, `VERSION` e config continuam validando normalmente.

## Canon arquitetural

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair semelhança superficial.
3. Preferir `SystemParam`s estreitos/coerentes e availability/run conditions canônicas.
4. UI compartilhada pertence a `src/ui`.
5. Targeting tem um único target autoritativo e consumers change-driven.
6. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são os primitives canônicos de fila deduplicada.
7. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
8. Cores internas são HSI-first; visuals ponderados usam `CurrentBiomeVisuals`.
9. Evitar scans globais, allocations temporárias e dirty writes quando o owner já tem sinal/metadata.
10. Pipeline inicial: generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> spawn.
11. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
12. Terrain/fluid/lighting background remesh é async; immediate player-edit geometry permanece síncrono.
13. Não trocar corretude por performance aparente e não criar abstração genérica sobre generation/mesh task queues.
14. Sistemas visuais não devem reescrever Components/Assets com o mesmo valor.
15. Movimento com delta zero permanece ocioso.
16. Scratch recorrente/estruturalmente limitado prefere stack/reuse a heap repetida.
17. `NewWorldConfig` é owner das escolhas de criação de mundo; forced biome valida coluna inicial e safe-spawn final no mesmo biome.
18. Search/text-input compartilhado pertence a `src/ui`.
19. Preferências de HUD pertencem a `HudSettings`; targeting não conhece layout.
20. Queries mutáveis múltiplas sobre o mesmo Component precisam ser disjuntas via `Without<T>` ou `ParamSet`.
21. Child UI que obedece ao hide do parent usa `Visibility::Inherited`.
22. Revisions derivadas são separadas por domínio quando consumers têm dependências diferentes.
23. Prioridade de streaming deve sobreviver às fronteiras async; quando generation/mesh saturarem, o raio imediato do jogador pode preemptar apenas trabalho não imediato, que precisa voltar para a fila em vez de ser perdido.

---

# Estado atual

Último HEAD de código publicado: `37959acf5ff9f0997b3817dc09d8970348a47861`  
Bloco: `Prioritize critical chunks through async streaming`  
`VERSION = 0.14.56`

## Histórico recente relevante

- 0.14.31–0.14.50: ciclo de streaming/worldgen/lighting/perf com COW, queues, scratch reuse, Bevy hash collections, async mesh halo e eliminação de lookups repetidos.
- `cbce686` / 0.14.51: CI ignora commits exclusivamente de `HANDOFF.md`.
- `573898d` / 0.14.52: terrain/modifiers/seed derivados do `BiomeField` por índice em vez de lookup textual repetido.
- `b28b3c2` / 0.14.53: corrige call site esquecido da nova assinatura no bootstrap.
- `7a0283d` / 0.14.54: `BiomeFieldSample.primary_surface_index` + hydrology metadata por índice.
- `ee806d0` / 0.14.55: corrige initializer de teste; CI verde.
- `37959ac` / 0.14.56: preserva prioridade do raio imediato do jogador depois do dispatch async; generation/mesh distante podem ser preemptados quando saturam os 8 slots, trabalho preemptado é refileirado e a fila `ready` deixa de bloquear chunks imediatos pela ordem FIFO de conclusão.

## CI recente

- 0.14.51 / run `35041238324`: success.
- 0.14.52 / run `35042143508`: failure; bootstrap usava assinatura antiga.
- 0.14.53 / run `35042627256`: success.
- 0.14.54 / run `35042991888`: failure; teste sem `primary_surface_index`.
- 0.14.55 / run `35043364577`: Clippy + `cargo check` success.
- 0.14.56 / run `35045326563`: em andamento no momento deste handoff.
- Commits handoff-only não abrem Rust CI.

---

# Refactor/performance já consolidado

## Streaming/render

- unload/rebuild reutilizam scratch;
- generation/mesh/remesh pesados ficam em tasks; integração main-thread é budgetada;
- `stream_chunks` e `ChunkRemeshQueue` evitam polling/dispatch vazio;
- empty chunks compartilham buffers por `Arc`/COW;
- pending sort reutiliza scratch e preserva empate via ordinal;
- halo de mesh materializa async a partir de clones COW + revisions;
- mesh output mantém sort explícito;
- hot maps/sets sem semântica de ordem usam Bevy collections;
- quando generation/mesh estão saturados, chunks no raio imediato do jogador podem preemptar apenas tarefas não imediatas; vítimas retornam às filas e `ready` procura primeiro trabalho imediato.

## Worldgen/biome

- feature cache retention reutiliza scratch;
- `SurfaceCarverColumn`/`SurfaceMaterialColumn` reutilizam Vecs;
- `BiomeField::sample_surface` usa `ArrayVec`; generation columns usam `SmallVec`;
- `BiomeInfluence.surface_index` evita resolver o mesmo biome por string nos hot paths;
- terrain/hydrology metadata recorrente resolve pelo snapshot do `BiomeField`.

## Lighting

- direct seed pula upper chunks vazios;
- propagation usa leitura local de vizinhos quando possível;
- changed chunks consolidam mesh revision por batch;
- dynamic lighting só roda com trabalho;
- fluid frontier usa boundary metadata + scan limitado à face.

O antigo roadmap de **micro-refactor genérico** terminou em 0.14.55. Nova evidência runtime reabriu o trabalho com foco em throughput, continuidade e regressões visuais reais.

---

# Ciclo ativo — prioridades runtime

## P0.1 — Chunk streaming: tempo até chunk visível

Sintoma confirmado:

- o jogo pode manter ~60 FPS, mas rendering/generation de chunks fica muito atrás do movimento;
- o jogador consegue atravessar **diversos chunks vazios** antes de terrain/mesh aparecer;
- o problema é portanto throughput/latência do pipeline, não apenas FPS.

Pipeline auditado:

`stream selection -> generation dispatch/task -> generation integration -> initial lighting -> halo snapshot -> mesh dispatch/task -> mesh integration -> spawn/visibility`

### Achado concreto em 0.14.56

A seleção inicial já ordenava chunks por proximidade, mas essa prioridade não sobrevivia integralmente às etapas seguintes:

- `ChunkGenerationTasks` e `ChunkMeshTasks` mantêm até 8 tarefas in-flight; com todos os slots ocupados, um chunk que se tornou imediato ao jogador não tinha mecanismo para tomar o lugar de trabalho distante;
- `ready` era consumida em FIFO pela ordem em que generation results eram integrados, então um chunk distante na frente da fila podia causar head-of-line blocking antes do mesh dispatch;
- isso permite starvation perceptível mesmo com a seleção `pending` corretamente ordenada.

Mudança de 0.14.56:

- raio crítico = 1 chunk em X/Y/Z ao redor do player chunk, alinhado com a prioridade imediata já usada pela seleção;
- se generation estiver saturada e existir trabalho crítico ainda não despachado, a tarefa in-flight não crítica mais distante pode ser cancelada e refileirada no fim de `pending`;
- se mesh estiver saturada e um chunk crítico estiver `ready`, uma mesh task não crítica distante pode ser cancelada e o chunk vítima volta para `ready`;
- `ready` agora procura chunks críticos antes de cair no FIFO normal;
- nenhum limite, budget ou número máximo de tasks foi aumentado.

Estado:

- o gargalo de **prioridade perdida após o dispatch** recebeu correção estrutural;
- ainda não declarar P0.1 resolvido sem runtime em movimento normal;
- a seleção ainda não possui viés explícito por vetor de movimento; ela é proximity-first, e 0.14.56 garante prioridade forte somente quando o chunk entra no raio crítico;
- `seed_chunk_direct_lighting` continua síncrono entre generation pronta e mesh dispatch e permanece candidato caso ainda exista backlog visual após 0.14.56.

Próximo trabalho, se o runtime ainda deixar o jogador alcançar vazio:

1. medir/logar `pending`, `ready`, generation in-flight e mesh in-flight durante movimento;
2. identificar se o backlog restante está antes de generation completion, no direct-light seed, mesh task ou mesh integration;
3. adicionar viés de direção somente se chunks à frente continuarem perdendo para chunks de distância equivalente;
4. não aumentar limits/budgets sem evidência do estágio saturado;
5. objetivo runtime continua: em velocidade normal, o jogador não deve alcançar área sem terrain visível.

## P0.2 — Lighting/shadows: lamp latency + seam entre chunks

Sintomas confirmados:

- lighting/shadow demora perceptivelmente para estabilizar;
- posicionar `lamp` evidencia bastante a latência;
- durante convergência/carregamento aparece sombra falsa/linha escura na fronteira entre chunks.

Estado atual relevante:

- dynamic lighting: budget de 2 ms / até 4096 voxels por frame;
- voxel edits entram em `PendingLightingUpdates::enqueue_voxel_edit` com voxel + vizinhos prioritários;
- changed chunks entram em remesh conforme a propagação altera o campo;
- initial/direct lighting seed continua síncrono e separado da convergência dinâmica.

Próximo trabalho:

1. seguir `lamp placement -> lighting queue -> changed_chunks -> remesh -> mesh visível/shadow`;
2. localizar se a latência está na propagação, remesh queue, mesh task ou integration;
3. não resolver apenas aumentando budget;
4. impedir que lighting incompleto de chunk se manifeste como shadow seam;
5. considerar readiness/revision explícita por chunk se geometry e lighting ficam observáveis em estados incoerentes;
6. se necessário, introduzir content/light revision por chunk antes de qualquer async lighting mais agressivo.

Não marcar resolvido sem runtime com lamp junto e longe de fronteiras.

## P0.3 — Hydrology: continuidade de rivers/lakes/tunnels

Sintomas confirmados:

- rivers e lakes ainda geram paredes/cortes retos em vez de carvar margens/topo naturalmente;
- rivers ainda podem nascer no meio do nada;
- rivers ainda podem morrer no meio do nada;
- intersection river/tunnel ainda faz o tunnel terminar abruptamente.

### Causa concreta já identificada para river/tunnel

Em `src/world/generation/density.rs`:

- `surface_carver_allowed = pass.region.hydrology.water_at(horizontal).is_none()`;
- isso desliga o **surface tunnel carver inteiro por coluna** ao entrar em coluna com água;
- o gate binário produz uma descontinuidade exatamente na borda de river/lake;
- `surface_carver_density_delta` já compõe sobre a densidade atual, então o próximo fix deve substituir/remover esse veto por composição/blend correto, sem reintroduzir shaft/água quebrada.

### River graph/selection

Investigar:

- `keep_only_complete_downstream_paths` garante destino lake/ocean dentro do trace, mas não garante origem visível semanticamente válida;
- cells podem entrar em `channels` pelo flow threshold sem serem spring/lake source;
- continuidade entre hydrology regions e margins;
- primeiro edge materializado não pode parecer uma nascente só porque predecessor ficou fora da região;
- confluences que reposicionam tributários precisam preservar conexão visual/geométrica;
- origem válida: spring/lake/upstream/confluence; destino válido: downstream contínuo até lake/ocean/confluence/trunk.

### Margens/topo

- river/lake bed já varia por horizontal `strength`;
- carve usa `VerticalDensityDelta` em uma faixa Y fixa por coluna;
- `enforce_hydrology_water_volume` também força air no volume molhado e headroom acima;
- revisar composição para slope/blend vertical + horizontal contínuo em vez de parede/patamar;
- river carve pode ser mais largo que water boundary para formar bank, mas a transição precisa ser suave;
- lake shore reinforcement precisa acompanhar o relevo realmente gerado.

Não marcar resolvido sem runtime de margem lateral, topo, nascente, foz, lago, confluence, waterfall e tunnel cruzando/chegando em água.

## P1.1 — Mountains extremamente dominantes + outros biomas raros

Feedback reforçado pelo usuário:

- mountains estão **MUITO** presentes, não apenas levemente acima do desejado;
- Witchwood, Enchanted Forest e Wasteland aparecem extremamente raramente;
- aumentar um pouco a quantidade de oak trees em Plains.

Valores atuais:

- Plains/Wasteland regional weight: 1.0;
- Witchwood/Enchanted Forest regional weight: 1.0;
- Mountains dimension weight: 0.85;
- Plains/Wasteland size: 120–420;
- Witchwood/Enchanted Forest size: 140–460;
- Mountains metadata size: 90–260;
- Plains oak: spacing 80, chance 0.48, jitter 24.

Importante: Mountains **não é um regional biome simples**. O JSON usa:

- `mountain_belt`: scale `0.0017`, threshold `0.86`, width `0.22`, warpStrength `85`;
- `mountain_peak`: spacing `760`, chance `0.48`, radius `120–230`, warpStrength `42`.

Logo, a sobrepresença provavelmente é controlada principalmente por **belt/peak distribution geometry** (threshold/width/chance/spacing/radius/warp), e não apenas por `weight = 0.85` ou tamanho mínimo.

Próximo trabalho:

1. medir/entender cobertura efetiva de mountain belts + peaks no selector;
2. reduzir cobertura de mountains pelas distributions especiais;
3. auditar por que regionais de peso igual aparecem tão pouco: `select_surface_biome_index`, dominant-neighbor suppression, site spacing e fallback;
4. preservar regiões grandes — não resolver transformando o mapa em mosaico pequeno;
5. aumentar levemente árvores em Plains, mantendo Plains claramente menos arborizado que Witchwood/Enchanted Forest.

Tratar a presença excessiva de mountains como bug de balanceamento severo, não tuning cosmético.

## P1.2 — Coast aparecendo isolada no interior

Sintoma confirmado:

- `coast` observado no meio de `plains`, sem ocean visível por perto.

Estado atual:

- `coast.json` e `ocean.json` são `kind: hydrology`;
- `BiomeField::from_dimension` os exclui corretamente de `surface_biomes` regionais;
- portanto coast isolada não vem do weighted regional selector normal.

Próximo trabalho:

- auditar hydrology overlay/identity e consumers de nome/visual do biome;
- coast deve existir apenas na faixa real de transição oceânica/hydrology;
- conferir `ocean_strength`, coast blend thresholds e residual de continentalness que possa promover coast em terra sem ocean próximo.

## P1.3 — Clouds não renderizam

Sintoma confirmado:

- nuvens não estão visíveis/renderizando em gameplay.

Causa provável forte já identificada em `src/rendering/sky_layers/clouds.rs`:

- `altitude = 34.0 + ... * 14.0`, portanto clouds ficam em Y absoluto ~34–48;
- Overworld usa `seaLevel = 90`;
- `update_cloud_positions` acompanha a câmera em X/Z, mas mantém Y em `cloud.altitude` absoluto;
- em gameplay normal, clouds ficam abaixo do terreno/jogador e parecem não renderizar.

Próximo fix esperado:

- tornar altitude coerente com o mundo/câmera/dimensão em vez de hardcode absoluto abaixo do sea level;
- preservar data-driven density/color já vindos dos biome visuals;
- validar visibilidade em plains, mountains e biomas com cloud density baixa/alta;
- não transformar isso em reestruturação grande de sky layers sem necessidade.

## P2 — Ghost/held block

Código atual está alinhado ao invariant pedido:

- held block observa `PlayerHotbar.selected_slot()` + `item_at(selected_slot)`;
- slot vazio -> root/faces hidden;
- placement preview limpa id/transform/visibility sem item.

Como houve bug histórico, manter como verificação de regressão. Transparência do ghost deve ser percebida no bloco inteiro.

## P2 — Outros itens pendentes

- dye anteriormente fraco;
- Player HUD / target HUD e demais produto fora deste ciclo até repriorização explícita.

---

# Refactor estrutural ainda relevante

O próximo refactor não deve ser uma sequência de micro-otimizações sem repro. Ele deve apoiar os P0 atuais.

## Candidate: per-chunk content/light readiness + revisions

Problemas que podem se beneficiar da mesma fundação:

- mover direct-light seed para trabalho async stale-safe;
- impedir shadow seam com lighting parcial;
- saber quando chunk está pronto para mesh/spawn;
- reduzir reprocessamento de mesh/lighting após edits.

As revisions atuais não são ideais:

- `chunk_mesh_revision` é ampla demais porque lighting também a altera;
- `block_content_revision` é global e não cobre fluid changes como revision por chunk.

Se os P0 confirmarem essa necessidade, considerar owner explícito para revision/readiness de **conteúdo por chunk (blocks + fluids)** e estado de lighting, separado de mesh revision.

Lighting lazy expansion só é aceitável se preservar exatamente FIFO + dedup global + priority promotion da fila atual.

---

# Ordem de execução no próximo `go`

1. **P0.1 streaming/time-to-visible** — validar 0.14.56; se ainda houver vazio à frente, instrumentar backlog por estágio e atacar o estágio comprovadamente saturado, incluindo direction tie-break/direct-light seed se a evidência apontar para eles.
2. **P0.2 lighting/shadows** — lamp latency e seam entre chunks; coordenar com initial lighting se for o mesmo gargalo.
3. **P0.3 hydrology continuity** — primeiro river/tunnel gate binário; depois endpoints e margens/topo.
4. **P1.1 biome distribution** — reduzir mountains de forma material, tornar regionais perceptíveis e aumentar levemente árvores de Plains.
5. **P1.2 coast isolada** — corrigir overlay/identity hidrológico.
6. **P1.3 clouds** — corrigir altitude absoluta incompatível com sea level/mundo.
7. Retomar refactor estrutural/perf conforme a evidência dos P0, evitando micro-churn sem efeito runtime.

---

# Performance direction

Meta não é apenas ~60 FPS; também é streaming responsivo e mundo visualmente pronto à frente do jogador.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados sem criar backlog visível;
- reduzir custo indivisível dentro de budgets;
- preservar prioridade de chunks próximos ao longo de todo o pipeline;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- stack/reuse para scratch limitado;
- evitar scans globais, allocations temporárias e writes idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative.
