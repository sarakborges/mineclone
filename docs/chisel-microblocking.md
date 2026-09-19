# Chisel — microgeometria persistente e colocação por restauração

Atualizado em 2026-09-17. A implementação inicial entrou em `develop` pela [PR #12](https://github.com/sarakborges/mineclone/pull/12), merge `cf7f3bd866b9e142e6cb8a73f5d1f61b080c2424`. O texto histórico anterior à integração permanece em [`docs/handoffs/handoff-chisel-pre-merge-2026-09-17.md`](handoffs/handoff-chisel-pre-merge-2026-09-17.md). Esta especificação descreve o código atual. `VERSION` raiz `0.21.0` representa a nova funcionalidade, **não** uma certificação de QA Windows; o formato de snapshot continua na versão 1 e `Cargo.toml` tem versão independente.

## Geometria, controles e elegibilidade

O Chisel esculpe um bloco existente: clique esquerdo remove um fragmento, clique direito **somente recoloca material previamente removido dentro do mesmo bloco-alvo**. Não é uma ferramenta para colocar um bloco novo no ar, expandir a geometria para um voxel vizinho nem criar materiais adicionais. O bloco original define ID, textura, orientação, rotação e propriedades visuais. Pressionar `R` com Chisel selecionado alterna Thick → Thin → Extra Thin → Thick; não há modo Full.

| Precisão | Divisões por eixo | Células por macrobloco | Espessura mínima |
| --- | ---: | ---: | ---: |
| Thick | 2 | 8 | 1/2 bloco |
| Thin | 4 | 64 | 1/4 bloco |
| Extra Thin | 8 | 512 | 1/8 bloco |

Somente definições com `tags: ["fragmentable"]` podem ser esculpidas. Atualmente `bassalt`, `dirt`, `grass`, `oak_log`, `sand` e `stone` têm opt-in; `oak_leaf`, `lamp` e `glass` não têm. O handler e o preview verificam o mesmo alvo elegível. O preview branco indica a região removível; o verde só aparece quando o microfragmento adjacente está vazio, o alvo já possui uma máscara parcial e a restauração efetivamente modificaria o mesmo macrobloco. Quando a máscara volta a ficar completamente cheia, a propriedade privada é retirada e o bloco retorna à representação normal.

O tooltip permanece no `ActionHint` ao lado da mira, compartilhado com o Brush, localizado em EN/PT-BR/ES. O sprite do Chisel usa `data/tools/chisel.json` e `textures/tools/chisel.png` e agora é filho do mesmo braço animado do Brush, com mesh retangular, material `AlphaMode::Mask(0.5)`, camada de renderização 1, pegada `(-0.08, 0.46, 0.21)`, tamanho `0.52`, pivô `(-0.36, -0.36)` e rotação local Z `0.30`. Só fica visível quando selecionado na hotbar. A posição final da arte ainda precisa de inspeção visual no Windows.

## Persistência e compatibilidade

`src/voxel/microblock.rs` guarda uma máscara 8×8×8 em oito palavras `u64`. Cada linha é codificada em 16 caracteres hexadecimais, totalizando 128 caracteres. O identificador privado é `asteria:chisel_mask`. O prefixo legado `t` (129 caracteres) ainda é entendido para pais criados no ar pela implementação antiga, mas **novas operações não criam mais esses pais** e não permitem restaurar peças dentro deles. O código preserva o marcador em memória e no save caso encontre uma sessão legada; ao remover seu último fragmento, o pai legado desaparece.

`SecondaryProperties::iter()` continua escondendo máscaras da UI e das propriedades públicas; `iter_for_save()` inclui a máscara somente no snapshot. `DiskChunk::from_chunk` serializa todos os blocos modificados, inclusive pais legados, com todas as propriedades em `chunks[].blocks[].properties`, sem serializar IDs numéricos de runtime ou iluminação derivada. A leitura exige blocos existentes, elegíveis para Chisel, máscaras com comprimento e caracteres válidos e até oito propriedades únicas. Máscara malformada invalida **toda a geração candidata** e aciona o fallback de snapshots existente; não deve ser convertida silenciosamente em um cubo inteiro. Saves antigos sem `asteria:chisel_mask` continuam aceitos como blocos inteiros. A implementação antiga excluía máscaras do disco; formas que já foram perdidas em saves anteriores não podem ser reconstruídas retroativamente.

A revisão de edição do mundo já é incrementada por modificações reais de bloco, por isso mudanças com Chisel entram no fluxo existente de autosave após 60 segundos, Leave World e Exit. Tanto chunks residentes quanto chunks modificados arquivados usam o mesmo `DiskChunk` ao capturar o snapshot. O fluxo de manifesto, publicação atômica, limite de 512 MiB, fallback e quatro gerações recuperáveis permanece inalterado.

## QA manual obrigatório — ainda NÃO EXECUTADO

Use somente mundos descartáveis, siga [`docs/save-roundtrip-qa.md`](save-roundtrip-qa.md) e registre commit, Windows/FS, log, PASS/FAIL/NOT RUN e tempos reais. Em cada uma das três precisões, esculpa um bloco elegível, deixe uma forma assimétrica identificável e confira remoção e reposição. Tente colocar em bloco intacto, ar e macrobloco adjacente: não deve haver edição nem preview verde. Confira que o preview some quando a microcélula selecionada já está ocupada e que o Chisel acompanha animações de mão e alternância da hotbar como o Brush.

Crie um tronco oco, uma placa de espessura 1/8, formas nos limites entre chunks e em coordenadas X/Z negativas. Afaste-se até arquivar os chunks, volte, force autosave ou use Leave, feche o processo, reinicie e carregue: forma, textura, rotação e propriedades devem continuar idênticas. Gere pelo menos duas gerações recuperáveis em cópia isolada, corrompa o texto `asteria:chisel_mask` na geração mais recente e confirme fallback para a anterior, sem cubo cheio inesperado. Verifique também saves pré-Chisel sem máscara. Não execute `cargo test` sem autorização explícita.

**Limitações abertas:** ainda não existe medição real de FPS/RAM, nem QA de screenshot ou roundtrip no Windows. Iluminação, fluidos e certos predicados `VoxelWorld::is_solid` ainda podem tratar cavidades como blocos macro; o interner das máscaras pode crescer durante edições sucessivas. Um save antigo sem máscara não contém informação suficiente para recuperar a forma perdida. Não declarar estes casos resolvidos apenas porque Clippy e `cargo check` passaram.
