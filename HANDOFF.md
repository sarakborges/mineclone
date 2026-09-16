# HANDOFF — Asteria / Mineclone

Repo: `sarakborges/mineclone`  
Branch de trabalho: `develop`  
Stack: Rust + Bevy 0.19.1

## Fonte canônica e regras de trabalho

Este `HANDOFF.md` na raiz de `develop` é a fonte canônica e persistente do projeto. Exports/anexos são snapshots derivados.

- Trabalhar diretamente em `develop`; feature branch só sob pedido explícito.
- Antes de escrever, buscar HEAD/VERSION atuais e abrir os arquivos reais envolvidos.
- Commits pequenos/coerentes; não misturar mudanças arquiteturais sem relação.
- Todo bloco coerente sobe `VERSION`: patch para fix/refactor/tooling compatível; minor para feature compatível; major para breaking.
- Depois de mudança material em código, versão, arquitetura, roadmap ou processo, atualizar este handoff.
- A atualização do handoff faz parte do próprio bloco de trabalho. Não encerrar um bloco material deixando `HANDOFF.md` desatualizado.
- Se o último bloco ainda estiver aguardando CI, registrar isso como pending; quando o CI fechar ou houver fix subsequente, atualizar novamente.
- Runtime error/warning enviado pelo usuário tem prioridade sobre roadmap/refactor.
- Corrigir todos os warnings de Rust encontrados nos blocos tocados.
- Não declarar bug visual/gameplay resolvido sem evidência runtime quando o comportamento depender disso.
- Não gerar imagens sem pedido explícito.
- Quando o usuário disser `go`/`continua`, executar o próximo bloco aplicável sem confirmação desnecessária.
- Não ficar repetindo que `cargo check` não foi rodado ou que está esperando `cargo run`; CI canônico valida Clippy + check.
- `cargo test` só é rodado manualmente sob pedido explícito do usuário.
- `cargo fmt`/`rustfmt` não é gate do projeto.
- Commit exclusivamente documental de `HANDOFF.md` não sobe `VERSION`.

## Versionamento

- Fonte operacional acordada: arquivo raiz `VERSION`.
- Estado atual: `VERSION = 0.14.49`.
- `src/app/version.rs` deixa explícito que `VERSION` fica fora de `Cargo.toml` de propósito para que bumps frequentes não invalidem fingerprints do Cargo. A divergência de `[package].version = 0.10.16` é intencional enquanto essa política existir; não sincronizar silenciosamente.

## Validação

CI automático em `.github/workflows/ci.yml`:

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check`

Roda em push para `develop`/`main` e em pull requests.

## Canon arquitetural

`ARCHITECTURE.md` continua sendo o canon principal. Regras operacionais relevantes:

1. Cada fato de gameplay tem um owner autoritativo.
2. Reutilizar invariants/state machines reais; não abstrair semelhança superficial.
3. Preferir `SystemParam`s estreitos/coerentes e availability/run conditions canônicas.
4. UI compartilhada pertence a `src/ui`.
5. Targeting tem um único target autoritativo e consumidores change-driven.
6. `DeduplicatedQueue<T>` / `VoxelUpdateQueue` são os primitives canônicos de fila deduplicada.
7. `FrameWorkBudget` é o primitive canônico de budget por tempo/quantidade.
8. Cores internas são HSI-first; biome visuals ponderados usam `CurrentBiomeVisuals`.
9. Evitar scans globais, allocations temporárias e dirty writes quando existe sinal/metadata no owner certo.
10. Pipeline inicial: generation task -> integrate -> initial lighting -> halo snapshot -> mesh task -> spawn.
11. Resultados async são revisionados; stale results são descartados/rescheduled; integração main-thread é budgetada.
12. Terrain/fluid/lighting background remesh é async; immediate geometry de edit do player permanece síncrono.
13. Não trocar corretude por performance aparente e não criar abstração genérica acima das generation/mesh task queues.
14. Sistemas visuais não devem reescrever Components/Assets com o mesmo valor.
15. Movimento com delta zero deve permanecer ocioso.
16. Scratch recorrente/estruturalmente limitado deve preferir stack/reuse a heap repetida.
17. `NewWorldConfig` é owner das escolhas de criação de mundo; forced biome valida coluna inicial e safe-spawn final no mesmo biome.
18. Search/text-input compartilhado pertence a `src/ui`.
19. Preferências de HUD pertencem a `HudSettings`; targeting não conhece layout.
20. Queries mutáveis múltiplas sobre o mesmo Component precisam ser provadamente disjuntas via `Without<T>` ou `ParamSet`.
21. Child UI que deve obedecer ao hide do parent usa `Visibility::Inherited`.
22. Revisions derivadas devem ser separadas por domínio quando consumers têm dependências diferentes.

---

# Estado atual

Último HEAD de código publicado:

`84dfbe2564da82045fa5a1a59f9b36a9e7018966`

Bloco: `Use Bevy hash collections in worldgen caches`

`VERSION`: `0.14.49`

Commits recentes relevantes:

- `e22c888` — `0.14.31`, `VoxelChunk::empty()` compartilha buffers vazios de blocks/fluids via `Arc`/COW.
- `ba23847` — `0.14.32`, `DeduplicatedQueue` evita lookups duplicados.
- `0aaefe8` — `0.14.33`, `DeduplicatedQueue` usa hash map do Bevy.
- `7b64f5b` — `0.14.34`, `VoxelWorld` usa hash collections do Bevy onde não há ordenação semântica.
- `383ae33` — `0.14.35`, `LightingContext` usa hash map do Bevy.
- `e4ebe14` — `0.14.36`, propagation usa o chunk central para vizinhos cardinais locais.
- `ed45170` — `0.14.37`, shell de mesh é montado sem prefill.
- `d5915fa` — `0.14.38`, mesh revision de lighting consolidada por chunk.
- `e54afd6` — `0.14.39`, remove API de light órfã.
- `da309b3` — `0.14.40`, hash collections quentes restantes de lighting usam Bevy.
- `30c54ce` — `0.14.41`, streaming selection usa hash collections do Bevy; feature-cache retention aceita iterador.
- `a6fc5c3` — `0.14.42`, `ChunkRenderPool.active` usa hash map do Bevy.
- `8a96146` — `0.14.43`, halo de `ChunkMeshSnapshot` é materializado dentro da task async a partir de clones COW.
- `686d3cb` — `0.14.44`, pending sort do streaming reutiliza scratch preservando empates via ordinal.
- `a1a36c7` — `0.14.45`, corrige warning do teste do sort.
- `86d5f4b` — `0.14.46`, `BiomeInfluence` carrega `surface_index`.
- `181ccb8` — `0.14.47`, corrige consumers do novo índice; structure support usa o índice carregado.
- `dd12f2e` — `0.14.48`, content registries usam `bevy::platform::collections::HashMap`.
- `84dfbe2` — `0.14.49`, cache de sites de `BiomeField`, `FeatureCaches`, structure-origin maps e retention scratch usam hash collections do Bevy.

## CI recente

- `0.14.40` / run `35036777258`: Clippy **success**, `cargo check` **success**.
- `0.14.41` / run `35037142837`: Clippy **success**, `cargo check` **success**.
- `0.14.42` / run `35037660941`: Clippy **success**, `cargo check` **success**.
- `0.14.43` / run `35038065308`: Clippy **success**, `cargo check` **success**.
- `0.14.44` / run `35038433002`: **failure** de Clippy no teste novo por `clippy::useless_vec`.
- `0.14.45` / run `35038881264`: Clippy **success**, `cargo check` **success**.
- `0.14.46` / run `35039241647`: **failure** por consumers esquecidos do novo `surface_index`.
- `0.14.47` / run `35039797881`: Clippy **success**, `cargo check` **success**.
- `0.14.48` / run `35040141257`: Clippy **success**, `cargo check` **success**.
- `0.14.49` / run `35040506838`: **pending/in progress** nesta atualização.

---

# Refactor/performance consolidado

## Streaming / render

- Unload e rebuild reutilizam scratch.
- Generation/mesh/remesh pesados ficam em tasks; integração main-thread é budgetada.
- `stream_chunks` evita polling/dispatch vazio.
- `ChunkRemeshQueue` evita background polling sem trabalho; immediate geometry continua síncrono.
- `VoxelChunk::empty()` compartilha blocks/fluids vazios com COW.
- `DeduplicatedQueue`, `VoxelWorld`, streaming selection e `ChunkRenderPool.active` usam hash collections do Bevy quando não há requisito de ordem.
- Streaming pending sort não depende mais do scratch temporário interno de `sort_by_cached_key`.
- `ChunkMeshSnapshot` não expande mais o halo de 1736 samples no main thread; a task async materializa o shell congelado a partir de clones COW dos vizinhos.
- O light storage de `VoxelChunk` também é `Arc<[VoxelLight]>`; clones de snapshot permanecem baratos e mutação usa COW via `Arc::make_mut`.

## Worldgen / biome

- Feature cache retention reutiliza quatro HashSets internos.
- `SurfaceCarverColumn` e `SurfaceMaterialColumn` reutilizam Vecs por coluna.
- `BiomeField::sample_surface` usa `ArrayVec` para o máximo estrutural de 26 influências.
- `GenerationColumnSample` usa `SmallVec` inline no caso comum.
- Structure support reutiliza a própria amostra de biome.
- `BiomeInfluence` conserva o surface biome index calculado pelo sampler; geração de colunas e structure support não reconstruem mais esse índice por busca textual.
- Content registries genéricos e secondary-property map usam o hasher de `bevy::platform`.
- `BiomeField.surface_site_biomes`, caches concorrentes de worldgen, structure-origin maps e retention scratch usam as hash collections do Bevy; locking/retention semantics permanecem iguais.

## Lighting

- Direct seed pula upper chunks vazios.
- Propagation usa leitura local de vizinhos quando possível.
- Changed chunks de lighting consolidam mesh revision uma vez por batch.
- Dynamic lighting só roda quando há trabalho.
- `enqueue_loaded_fluid_frontier` usa metadata de boundary e scan limitado à face necessária.

---

# Alvos descartados/bloqueados sem nova evidência

- Movimento/câmera/viewmodel já têm guards de idle/change.
- Visual/HUD paths revisados evitam writes idempotentes relevantes.
- `track_current_biome` precisa amostrar continuamente por causa de blends.
- Fluid/dynamic lighting já possuem scratch/budget/run conditions.
- Reduzir limites de task arbitrariamente tende a reduzir throughput e não prova ganho de FPS.
- `ChunkTaskQueue` é pequeno por design; hasher não é prioridade.
- Clouds só justificam reestruturação com profiling devido risco visual/lifecycle.
- `surface_cache` já calcula apenas misses, usa cinco probes conservadores e só é podado em mudança de generation-region/radius.
- `GenerationSnapshot` só é reconstruído quando inputs autoritativos mudam; `WorldFeatureFields` compartilha caches por `Arc`.
- A expansão eager de 4096 lighting seeds é custo real, mas uma fila lazy precisa preservar FIFO, dedup global e priority promotion do `DeduplicatedQueue`; não criar segunda semântica de fila sem invariant explícito.

---

# Hot paths / roadmap restante

1. **Próximo (`0.14.50`) — mesh buffers**: `build_chunk_mesh` agrupa faces expostas em `std::HashMap`; roda async, mas afeta throughput. O output já é ordenado explicitamente, então o hasher não é semântico.
2. **Depois (`0.14.51`) — CI docs-only**: `.github/workflows/ci.yml` roda Clippy/check também quando o push altera apenas `HANDOFF.md`. Adicionar `paths-ignore` para não gastar validação Rust em commit exclusivamente documental, preservando CI quando há qualquer arquivo de código/config junto.
3. **Candidato estrutural validado para implementação depois (`0.14.52` se os gates anteriores forem verdes)**: `surface_height_from_sample` faz lookup textual no `BiomeRegistry` e recalcula `string_hash` + `mix_seed` para cada influência. `BiomeFieldEntry` já é snapshot derivado imutável; `BiomeTerrain`/modifiers são clonáveis/Copy, e o `density_seed` atual usa exatamente o mesmo FNV + mix do terrain. Carregar terrain/modifiers no entry e reutilizar esse seed permite resolver altura por `surface_index`, eliminando lookup textual e hashing por amostra sem alterar ruído/weights/IDs.
4. Depois do terrain metadata, revisar o lookup adicional de `surface.primary_id` usado pelo hydrology region; só mexer se puder reutilizar índice/metadata já derivado sem ampliar estado duplicado.
5. Reavaliar a expansão de 4096 lighting seeds apenas se houver desenho que preserve FIFO + dedup + priority promotion exatamente; atualmente bloqueado por corretude.
6. A divergência `VERSION` vs `Cargo.toml` é intencional conforme `src/app/version.rs`; não há patch sem mudança explícita dessa política.
7. Fazer auditoria global final; sem novo alvo sustentado por evidência/invariant, encerrar o roadmap.

---

# Bugs/produto fora do refactor atual

Só retomar quando o usuário priorizar ou runtime indicar regressão relacionada:

- ghost/held block deve refletir hotbar; transparência é do bloco inteiro;
- hydrology river/lake margin e água escapando da margem;
- dye anteriormente fraco;
- regressões de iluminação/sombreamento pós-09/09 foram revertidas; evitar alterações visuais casuais;
- terrain generation ainda foi relatada como lenta durante movimento, motivando este roadmap.

---

# Próximos passos

1. Aguardar CI de `0.14.49` (`35040506838`).
2. Se verde, publicar `0.14.50` migrando o mapa temporário de buffers do mesher para `bevy::platform`.
3. Ajustar workflow docs-only em `0.14.51`.
4. Implementar terrain metadata/seed precomputado em `0.14.52`, validando que output determinístico permanece idêntico por construção.
5. Revisar o lookup primário de hydrology e publicar só se houver ganho sem duplicação de autoridade.
6. Manter lighting seed expansion bloqueada sem semântica de fila equivalente.
7. Fazer auditoria global final e encerrar roadmap quando não restar alvo comprovado.

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
