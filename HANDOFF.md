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
- Estado atual: `VERSION = 0.14.46`.
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

`86d5f4b60ab785004ae3f16fe56bc8559f789484`

Bloco: `Carry surface biome index in influences`

`VERSION`: `0.14.46`

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
- `8a96146` — `0.14.43`, `ChunkMeshSnapshot` captura clones COW + revisions no main thread e materializa os 1736 samples do halo dentro das tasks async.
- `686d3cb` — `0.14.44`, pending sort do streaming reutiliza scratch e elimina allocation de `sort_by_cached_key` preservando empates via ordinal.
- `a1a36c7` — `0.14.45`, corrige o `clippy::useless_vec` no teste do bloco anterior trocando o Vec fixo por array; nenhuma lógica muda.
- `86d5f4b` — `0.14.46`, `BiomeInfluence` carrega `surface_index`; `sample_generation_columns` reutiliza o índice já calculado e deixa de fazer scan linear ID -> índice para cada influência de cada coluna.

## CI recente

- `0.14.40` / run `35036777258`: Clippy **success**, `cargo check` **success**.
- `0.14.41` / run `35037142837`: Clippy **success**, `cargo check` **success**.
- `0.14.42` / run `35037660941`: Clippy **success**, `cargo check` **success**.
- `0.14.43` / run `35038065308`: Clippy **success**, `cargo check` **success**.
- `0.14.44` / run `35038433002`: **failure** de Clippy apenas no teste novo por `clippy::useless_vec`.
- `0.14.45` / run `35038881264`: Clippy **success**, `cargo check` **success**.
- `0.14.46` / run `35039241647`: **queued/pending** nesta atualização.

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

## Worldgen / biome

- Feature cache retention reutiliza quatro HashSets internos.
- `SurfaceCarverColumn` e `SurfaceMaterialColumn` reutilizam Vecs por coluna.
- `BiomeField::sample_surface` usa `ArrayVec` para o máximo estrutural de 26 influências.
- `GenerationColumnSample` usa `SmallVec` inline no caso comum.
- Structure support reutiliza a própria amostra de biome.
- `BiomeInfluence` conserva o surface biome index calculado pelo sampler; geração de colunas não reconstrói mais esse índice por busca textual.

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
- A expansão eager de 4096 lighting seeds é custo real, mas uma fila lazy precisa preservar FIFO, dedup global e priority promotion do `DeduplicatedQueue`; não criar segunda semântica de fila sem invariant explícito.

---

# Hot paths / roadmap restante

1. **Próximo candidato (`0.14.47`) — content registries**: `DefinitionMap<T>` ainda usa `std::HashMap`; ele sustenta Block/Biome/Fluid/etc. registries e `BlockRegistry::get` ocorre por voxel não vazio durante mesh build. `SecondaryPropertyRegistry` também tem mapa externo `std::HashMap`. Não há ordem contratual hoje; troca de hasher para `bevy::platform` é compatível.
2. **Depois (`0.14.48`) — worldgen caches**: `BiomeField.surface_site_biomes` faz até 25 lookups de cache por `sample_surface`; `WorldFeatureFields`/`FeatureCaches` têm maps/sets quentes sob locks e scratch. Nenhum depende de ordenação; trocar para Bevy collections é candidato coerente.
3. **Depois (`0.14.49`) — mesh buffers**: `build_chunk_mesh` agrupa faces expostas em `std::HashMap`; roda async, mas afeta throughput. O output já é ordenado explicitamente, então o hasher não é semântico.
4. Reavaliar a expansão de 4096 lighting seeds apenas se houver desenho que preserve FIFO + dedup + priority promotion exatamente; atualmente bloqueado por corretude.
5. A divergência `VERSION` vs `Cargo.toml` é intencional conforme `src/app/version.rs`; não há patch sem mudança explícita dessa política.
6. Depois desses candidatos, fazer auditoria global final. Sem alvo sustentado por evidência/invariant, encerrar o roadmap.

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

1. Aguardar CI de `0.14.46` (`35039241647`).
2. Se verde, publicar `0.14.47` migrando content registries para `bevy::platform`.
3. Depois migrar caches quentes de worldgen (`0.14.48`).
4. Depois migrar o mapa temporário de buffers do mesher (`0.14.49`).
5. Registrar lighting seed expansion como bloqueada se nenhuma representação preservar a semântica da fila.
6. Fazer auditoria global final. Sem novo alvo sustentado por evidência, encerrar roadmap.

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
