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

---

# Estado atual

Último HEAD de código publicado: `7a0283d3095be6f374a72c549e9ee9d5f7130feb`  
Bloco: `Reuse biome field hydrology metadata`  
`VERSION = 0.14.54`

## Commits recentes relevantes

- `e22c888` — 0.14.31, storage vazio compartilhado em `VoxelChunk`.
- `ba23847` / `0aaefe8` — 0.14.32–33, dedup queue evita lookup duplicado e usa Bevy hash map.
- `7b64f5b` — 0.14.34, `VoxelWorld` usa Bevy collections.
- `383ae33`–`da309b3` — 0.14.35–40, lighting hot paths/collections/local-neighbor/revision batching.
- `30c54ce` — 0.14.41, streaming selection usa Bevy collections; feature retention aceita iterador.
- `a6fc5c3` — 0.14.42, `ChunkRenderPool.active` usa Bevy hash map.
- `8a96146` — 0.14.43, halo de `ChunkMeshSnapshot` materializa dentro da task async a partir de clones COW.
- `686d3cb` / `a1a36c7` — 0.14.44–45, pending sort reutiliza scratch e fix de Clippy do teste.
- `86d5f4b` / `181ccb8` — 0.14.46–47, `BiomeInfluence.surface_index` e correção de consumers.
- `dd12f2e` — 0.14.48, content registries usam Bevy hash maps.
- `84dfbe2` — 0.14.49, worldgen caches usam Bevy hash collections.
- `b2afe45` — 0.14.50, mesh buffer map usa Bevy hash map com output explicitamente ordenado.
- `cbce686` — 0.14.51, Rust CI ignora commits exclusivamente de `HANDOFF.md`.
- `573898d` — 0.14.52, `BiomeFieldEntry` carrega terrain/modifiers e reutiliza seed derivado; `surface_height_from_sample` resolve por `surface_index`, removendo lookup textual e hash/mix por influência sem alterar a matemática do noise.
- `b28b3c2` — 0.14.53, corrige o único consumer esquecido da nova assinatura de `surface_height_from_sample` no bootstrap.
- `7a0283d` — 0.14.54, `BiomeFieldSample` carrega `primary_surface_index` e `BiomeFieldEntry` carrega `BiomeHydrology`; macro hydrology recorrente resolve metadata por índice sem `BiomeRegistry::get(surface.primary_id)`.

## CI recente

- 0.14.40–0.14.43: green.
- 0.14.44: falhou por `clippy::useless_vec`; corrigido em 0.14.45.
- 0.14.45: green.
- 0.14.46: falhou por consumers esquecidos de `surface_index`; corrigido em 0.14.47.
- 0.14.47–0.14.51: Clippy + `cargo check` green.
- 0.14.51 / run `35041238324`: **success**.
- 0.14.52 / run `35042143508`: **failure**; `setup/bootstrap.rs` ainda chamava `surface_height_from_sample` com assinatura antiga.
- 0.14.53 / run `35042627256`: Clippy **success**, `cargo check` **success**.
- 0.14.54 / run `35042991888`: **pending/in progress** neste update.
- Confirmado: commits handoff-only após 0.14.51 não abrem Rust CI.

---

# Refactor/performance consolidado

## Streaming/render

- Unload/rebuild reutilizam scratch.
- Generation/mesh/remesh pesados ficam em tasks; integração main-thread é budgetada.
- `stream_chunks` e `ChunkRemeshQueue` evitam polling/dispatch vazio.
- Empty chunks compartilham buffers por `Arc`/COW.
- Dedup/world/streaming/render-pool e mesh buffers usam Bevy hash collections quando não há semântica de ordem.
- Pending sort do streaming reutiliza scratch e preserva empate via ordinal.
- Halo de mesh é materializado async; main thread captura clones COW + revisions.
- Mesh output mantém sort explícito.

## Worldgen/biome

- Feature cache retention reutiliza scratch interno.
- `SurfaceCarverColumn`/`SurfaceMaterialColumn` reutilizam Vecs.
- `BiomeField::sample_surface` usa `ArrayVec`; generation columns usam `SmallVec` inline.
- `BiomeInfluence` conserva `surface_index`; geração/structure support reutilizam esse índice.
- Registries e caches quentes de worldgen usam Bevy hash collections.
- 0.14.52 elimina lookup textual + FNV/mix repetido em `surface_height_from_sample`; seed/terrain/modifiers vêm do snapshot derivado imutável do `BiomeField`.
- 0.14.54 elimina o lookup textual de hydrology no macro-terrain recorrente mantendo `primary_id` compatível e usando `primary_surface_index` como referência autoritativa ao snapshot.

## Lighting

- Direct seed pula upper chunks vazios.
- Propagation usa leitura local de vizinhos quando possível.
- Lighting changed chunks consolidam mesh revision por batch.
- Dynamic lighting só roda com trabalho.
- Fluid frontier usa boundary metadata + scan limitado à face.

---

# Alvos descartados/bloqueados sem nova evidência

- Movimento/câmera/viewmodel já têm guards de idle/change.
- Visual/HUD paths revisados evitam writes idempotentes relevantes.
- `track_current_biome` precisa amostrar continuamente por causa de blends.
- Fluid/dynamic lighting já possuem scratch/budget/run conditions.
- Reduzir limites de task arbitrariamente não prova ganho de FPS e pode reduzir throughput.
- `ChunkTaskQueue` é pequeno por design; hasher não é prioridade.
- Clouds só justificam reestruturação com profiling devido risco visual/lifecycle.
- `surface_cache` já calcula só misses com cinco probes conservadores e pruning raro.
- `GenerationSnapshot` só reconstrói quando inputs autoritativos mudam; `WorldFeatureFields` compartilha caches por `Arc`.
- Expansão eager de 4096 lighting seeds continua custo real, mas qualquer alternativa precisa preservar FIFO + dedup global + priority promotion exatamente; sem esse invariant, permanece bloqueada.
- `seed_chunk_direct_lighting` ainda varre chunks superiores não vazios e o chunk atual para reconstruir direct sky na main thread. Não há metadata autoritativa de atenuação vertical por coluna que sobreviva corretamente a edits; cachear ou mover isso async exige owner/invalidation explícitos.

---

# Auditoria final / encerramento do roadmap atual

A auditoria final dos hot paths de movimento/streaming não encontrou outro patch pequeno seguro depois de 0.14.54:

- dynamic lighting já é budgetado (2 ms / até 4096 voxels por frame) e só roda com trabalho;
- unload é budgetado e seu scan de bootstrap ocorre uma única vez;
- streaming rebuild/sort, task dispatch/integration, mesh halo e worldgen caches já foram tratados neste ciclo;
- o custo síncrono material restante é initial/direct lighting seed. A fila atual expande 4096 voxels e boundary neighbors preservando FIFO, dedup global e priority promotion; uma fila lazy separada mudaria semântica;
- mover direct-light seed para task async também não é seguro com as revisions atuais: `chunk_mesh_revision` é ampla demais porque lighting a altera, enquanto `block_content_revision` é global e não cobre fluid changes. Um próximo ciclo estrutural deve primeiro considerar uma revision de **conteúdo por chunk (blocks + fluids)**, separada de mesh/light, para snapshot/dependency validation.

Portanto este roadmap de micro/refactor de performance encerra quando o CI de 0.14.54 fechar verde. O próximo trabalho de performance deve ser guiado por runtime/profiling e pode abrir um ciclo arquitetural para direct-light seed revisionado/async se os spikes continuarem.

# Próximos passos

1. Aguardar CI de 0.14.54 (`35042991888`).
2. Se verde, marcar este roadmap como concluído sem novo bump de versão.
3. Rodar o jogo/perf real e priorizar qualquer hotspot restante com evidência runtime.
4. Se initial lighting continuar sendo fonte relevante de frame spike, desenhar per-chunk content revision (blocks + fluids) antes de mover direct-light seed para async.
5. Lighting lazy expansion só volta se preservar exatamente FIFO + dedup global + priority promotion da fila atual.
6. Divergência `VERSION` vs `Cargo.toml` continua intencional até mudança explícita de política.

# Bugs/produto fora do refactor atual

Retomar apenas quando o usuário priorizar ou runtime indicar regressão relacionada:

- ghost/held block deve refletir hotbar; transparência do ghost é do bloco inteiro;
- hydrology river/lake margin e água escapando da margem;
- dye anteriormente fraco;
- regressões de iluminação/sombreamento pós-09/09 foram revertidas; evitar alterações visuais casuais;
- terrain generation lenta durante movimento foi a motivação principal deste roadmap.

# Performance direction

Meta: ~60 FPS estáveis.

- heavy generation/mesh/remesh fora da main thread;
- integração/restore/unload/lighting/fluid budgetados;
- reduzir custo indivisível dentro de um budget;
- revision tracking para stale async work;
- caches/metadata no owner correto;
- stack/reuse para scratch estruturalmente limitado;
- evitar scans globais, allocations temporárias e mutações idempotentes;
- não trocar corretude por performance aparente;
- natural hydrology permanece generation-authoritative.
