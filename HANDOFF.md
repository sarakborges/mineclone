# HANDOFF — Asteria / Mineclone

**Fonte canônica:** `sarakborges/mineclone`, branch `develop`, Rust + Bevy 0.19.1. **Versão raiz atual: `0.19.1`**. `Cargo.toml` mantém `0.10.16` independente. Consultar HEAD e VERSION antes de modificar. Histórico canônico anterior integral continua acessível: [handoff v0.19.0 antes da correção da CI](https://github.com/sarakborges/mineclone/blob/ae0aa17a7117caa1cf08f82e51e2cdf4f2491ea0/HANDOFF.md); [checklist original de direção de arte](https://github.com/sarakborges/mineclone/blob/bbc6505e7deeff2b41125baf9ec32815e4786d69/HANDOFF.md); [antes da alteração da CI](https://github.com/sarakborges/mineclone/blob/07eb75dc99f9e57eb83e19a4143857c4d1dcad22/HANDOFF.md); [criaturas/chat/editor v0.17.3](https://github.com/sarakborges/mineclone/blob/274b5f7aec5ff410736e4a2a2297b752273b87a7/HANDOFF.md); [mundo/branches v0.15.51](https://github.com/sarakborges/mineclone/blob/0af21e7c87ba75828acd65841f2902ddcb9cfc7a/HANDOFF.md). Não apagar história, defeitos ou pendências ao compactar.

## Regras permanentes

- `continua`/`go`: inspecionar HEAD, VERSION, CI, código e handoff; executar trabalho fundamentado. Pode gravar trabalho ordinário solicitado na `develop`; exigência de branches isolados era exclusiva das features anteriores de criaturas/chat/editor.
- Cada bloco de código/asset jogável/testes relevantes incrementa VERSION conforme SemVer; documentação isolada não altera VERSION. Atualizar handoff na mesma sessão com versão, commit, validação real e pendências. Preservar assets do usuário; não alterar PNG/GLB fornecido apenas para conectá-lo. `ARCHITECTURE.md` é referência arquitetural.
- **CI conforme pedido expresso do usuário:** `.github/workflows/ci.yml` mantém somente Clippy `cargo clippy --all-targets --all-features -- -D warnings` e `cargo check`, **sem executar `cargo test`** (remoção em [`a08d890`](https://github.com/sarakborges/mineclone/commit/a08d890c2772fb5198bb7b7b672362252afb7af6)). Clippy analisa targets de testes sem executá-los. Não reintroduzir a etapa de testes. Não suprimir warnings para tornar CI verde. CI verde não equivale a QA visual, Windows, gameplay ou FPS. Evitar polling repetitivo sem descoberta útil e informar resultados concretos.

## Direção de arte obrigatória para TODOS os modelos

Todo modelo e cada detalhe devem ser **quadrados/cúbicos e pixelados, com planos, arestas retas e cantos nítidos**. Proibidos bevel/quinas arredondadas, subdivisão suavizante, esferas, elipsoides, tubos curvos, olhos ovais ou boca suavizada. Slime deve ter corpo, núcleo, face, olhos, reflexos, bochechas e detalhes estritamente quadrados. Animações não podem arredondar a geometria. A skin do slime deve ser 64×64, com UVs e filtro nearest; não impor essa resolução a todas as outras criaturas sem novo pedido.

## Bloco 16/09/2026 — v0.19.1: limpeza de textura e correção dos erros reais da CI

- Pedido do usuário: nenhum PNG dentro da pasta do modelo, texturas data-driven nos JSONs das criaturas em `assets/textures/creatures/{name}.png`; conferir e rodar CI até resolver erro. O arquivo `assets/models/creatures/slime/slime_skin_64.png` de 217 bytes era um resíduo **não referenciado**, anterior à migração. Foi removido em [`3062fecd`](https://github.com/sarakborges/mineclone/commit/3062fecd9413e3ff8cb4173e21cb7005ad7bc58d). Não apagar `slime.glb`, `generate_slime.py` nem `slime.collider.json` (não são texturas). O gerador atual não embute imagens nem referencia o PNG excluído; usa os PNGs por espécie na pasta de texturas. Não regenerar esse arquivo obsoleto.
- Diagnóstico do [run falho `35162431472`](https://github.com/sarakborges/mineclone/actions/runs/35162431472), no evento PR do SHA `cc94f04`: Clippy falha em `src/hud/tool_icon.rs` por `BrushTintIcon` ser privado na assinatura pública do sistema `sync_brush_tint_icons` e no `src/hud/mod.rs` chamador; há também `clippy::type_complexity` para a chave `HashMap` de materiais de criaturas em `src/creatures/visual.rs`. `cargo check` foi skipped porque Clippy falhou. Não interpretar avisos de Node.js 20 ou instalação de dependências como o erro principal; não adicionar `#[allow(...)]`.
- Correção da visibilidade: `BrushTintIcon` passa de privado a `pub(super)` em [`57ec510`](https://github.com/sarakborges/mineclone/commit/57ec510390ddc9aa8de919d64c101f9e89e92434); o sistema HUD continua referenciando o mesmo marcador, sem mudar comportamento do brush.
- Correção do cache: nova chave nomeada `CreatureMaterialCacheKey` derivando `Clone, Copy, PartialEq, Eq, Hash` contém `material`, `tint_bits`, `texture`, substituindo a tupla aninhada na estrutura e na busca; mantém cache separado por material original, cor e PNG, sem mudar a funcionalidade de tingimento. Commit [`ac7538a`](https://github.com/sarakborges/mineclone/commit/ac7538a9a3e87da4408aef98d661ab0d79ab60bd). Bump patch `0.19.0 → 0.19.1` em [`92c4940`](https://github.com/sarakborges/mineclone/commit/92c494028a71a61c6858cc0892313d07ca79c89d).
- A execução da CI foi disparada automaticamente pelo push do patch de código e versão; [run `35162905516`](https://github.com/sarakborges/mineclone/actions/runs/35162905516) para SHA `92c4940` estava **in_progress** na consulta inicial. Conferir Clippy e Check do run final desta documentação por SHA; se falhar, ler job log e corrigir erro concreto. Não declarar build aprovado até ambas etapas concluírem com sucesso. Não executar testes unitários na CI. Não confundir falhas dos commits antigos com estado do HEAD atual.

## Bloco anterior — v0.19.0: slime cúbico e texturas externas data-driven

Implementação publicada em [`0ae0f0ba`](https://github.com/sarakborges/mineclone/commit/0ae0f0ba4723411ccec256d4d00f64afa282ce1e); geração/validação estrutural [run `35162342535`](https://github.com/sarakborges/mineclone/actions/runs/35162342535) **success**, não equivalente a teste do jogo. `assets/models/creatures/slime/generate_slime.py` usa stdlib para criar `slime.glb` com seis meshes de cubos, normals planas, UV `TEXCOORD_0`, e não inclui arrays glTF `images`/`textures`/`samplers`, nem PNG binário embutido. Casca, núcleo, olhos, brilhos, bochechas e sorriso em degraus são retangulares. Preservados nomes `SlimeRoot`, `Visual`, `BodyPivot`, `InnerCore`, frente `-Z`, origem nos pés, `Hitbox_AABB` não animado, materiais `SlimeShell`, `SlimeCore`, `SlimeEyes`, `SlimeHighlights`, `SlimeCheeks`, seis clips `Idle`, `Anticipate`, `Airborne`, `Land`, `Hurt`, `Death`; collider `[.78,.84,.78]`, centro `[0,.42,0]`. A geometria foi alterada intencionalmente a pedido do usuário.

As imagens **externas** `assets/textures/creatures/meadow_slime.png` e `ember_slime.png` são 64×64 RGBA e padrões iniciais grayscale para manter HSI. O gerador cria padrão apenas quando ausente, nunca sobrescreve edições do artista. `data/creatures/slimes/meadow.json` e `ember.json` usam mapa `textures` de nome do material para caminho PNG da respectiva espécie relativo a `assets/`. `src/content/creature.rs` valida caminhos PNG relativos seguros em `textures/creatures/` e aceita mapeamento opcional; `src/creatures/visual.rs` faz `AssetServer::load` por JSON e associa por `GltfMaterialName`, clonando materiais por espécie, alpha preservado e cache por material/cor/textura. `ImagePlugin::default_nearest()` é global. Detalhes da especificação em [`docs/creature-textures.md`](https://github.com/sarakborges/mineclone/blob/develop/docs/creature-textures.md). Não mudar para texture embutida ou fixa no GLB/gerador.

**Aceite de runtime aberto:** verificar simultaneamente Meadow e Ember, aparência e texturas de cada espécie, face quadrada sem blur, casca translúcida/núcleo, UVs de todas as faces, nearest, collider estável, animações de salto/dano/morte, frente/lado/cima, FPS. Nunca declarar QA visual realizado com base apenas em validação estrutural ou CI.

## Bloco anterior — v0.18.0: Grass, Bassalt e Brush

Os PNGs fornecidos pelo usuário `assets/textures/blocks/bassalt.png`, `assets/textures/tools/brush.png`, `assets/textures/tools/brush-tint.png` devem permanecer originais. `asteria:grass` exibe `Grass` sem mudar ID; `asteria:bassalt` pertence a `stone_blocks` e não substitui stone. `brush.json` tem `icon` base + `tintIcon`; `src/hud/tool_icon.rs` renderiza somente base em Clear e segunda camada colorida quando há cor, em inventário, hotbar, catálogo e arrasto. Clear remove dye e seleção aplica dye em blocos compatíveis. Código [`9cbee3e`](https://github.com/sarakborges/mineclone/commit/9cbee3edf5a3d665f2fe95f2809dbcddf7f2b7ce). QA real continua aberto.

**Integração histórica:** Meadow/Ember compartilham GLB; chat T/Enter/Esc e `/spawn_creature`, 64 mensagens/15 linhas, editor nativo EditableText + clipboard e scrollbars; merge [`b421c5e`](https://github.com/sarakborges/mineclone/commit/b421c5e248738f9f5f42cc122ea5973890f0a397), PRs #9/#10; hidrologia `.42–.51` e iluminação `.41` preservadas. Features experimentais de luz óptica/direcional e diagnóstico não integradas. Histórico completo nos snapshots do topo.

## Nove defeitos de mundo — TODOS ABERTOS até confirmação em gameplay

| # | Prio | Sintoma |
|---|---|---|
| 1 | P0 | Chunks pretos e costuras iluminação/AO; medir transiente, persistência e FPS. |
| 2 | P0 | Rios terminam no nada; verificar conectividade nascente→oceano por seeds. |
| 3 | P0 | Após 2000 blocos só Plains/Mountains; comparar IDs de bioma vs render. |
| 4 | P0 | Margens de rio afundam antes da água; evitar água suspensa. |
| 5 | P1 | Confluências rio–lago abruptas. |
| 6 | P1 | Túneis de paredes retas e paredes extras em cruzamento com rios. |
| 7 | P1 | Lua parece acompanhar câmera. |
| 8 | P1 | Lua visível durante o dia. |
| 9 | P1 | Target highlight não cobre todas as camadas da textura. |

Outras pendências: fogColor artístico, benchmark FPS/streaming, held/R, HUD/avatar/vida do player e target HUD. Única correção visual confirmada pelo usuário anteriormente: céu por bioma `.24`. Não fechar bugs por CI verde.

## Próxima execução

1. Conferir run da CI no **HEAD final** (Clippy + Check), ler logs e corrigir qualquer erro concreto, mantendo ausência de cargo test. Atualizar handoff caso status evolua; não afirmar aprovação de jobs pendentes.
2. QA visual no jogo dos slimes e texturas externas sem PNG em pasta de modelos, import GLB, HSI, transparência, nearest, física, animações e FPS.
3. QA Bassalt, Grass e Brush; retomar P0 rios, biomas, luz e margens, depois P1, com VERSION e handoff por bloco.
