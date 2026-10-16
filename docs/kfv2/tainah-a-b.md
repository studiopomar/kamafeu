# Avaliação cega de Tainah e straycat-rs

O utilitário `tainah-ab` gera comparações do renderizador de projeto em APS
usando os mesmos projetos, voicebanks, taxa de saída e wavtool. O corpus aponta
para frases revisadas e voicebanks autorizados; caminhos relativos são
resolvidos a partir do arquivo JSON. O exemplo está em
[`tainah-ab-corpus.example.json`](tainah-ab-corpus.example.json).

```sh
cargo run --bin tainah-ab -- corpus-autorizado.json /caminho/straycat-rs resultados-tainah
```

O executável do `straycat-rs` é obrigatório. Um wavtool externo pode ser
informado como quarto argumento; sem ele, ambos os renders usam Galapagos.
Prepare ao menos 20 casos com cobertura de vogais sustentadas, ataques e
caudas, transições e mudanças de pitch. Cada caso deve usar os mesmos planos
musicais ao comparar os dois motores.

Para cada caso, o utilitário faz cinco renders sem cache por motor e interrompe
se o PCM de um motor variar entre as repetições. `A.wav` e `B.wav` são
aleatorizados por caso e recebem ganho RMS aproximado, limitado a ±12 dB e pico
de 0,95. `raw-tainah.wav` e `raw-straycat.wav` preservam os renders sem ajuste
para análise. `answer-key.csv` revela o mapeamento dos pares; não o entregue aos
ouvintes. `ratings-template.csv` registra preferência, intensidade do caráter
stretch, inteligibilidade e continuidade. `report.json` registra PCM SHA-256,
duração, pico, RMS, clipping, amostras inválidas, silêncio por janelas de 10 ms,
mediana de latência e hashes SHA-256 do projeto APS e do `oto.ini`.

O gate planejado usa três ouvintes, ao menos 20 frases, pelo menos 65% de
preferência por Tainah (contando “sem preferência” como meia preferência) e
nenhuma queda média superior a 0,5 ponto em inteligibilidade ou continuidade
numa escala de 1 a 5. Métricas técnicas e fixtures sintéticas não substituem a
escuta, e nenhuma alegação de superioridade deve ser feita antes de completar
essa avaliação.
