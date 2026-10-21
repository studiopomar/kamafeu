# Caio · banco piloto

Banco nativo pt-BR criado a partir da gravação e da arte fornecidas pelo autor.

- A gravação `src/kak.mp3` e o retrato `assets/caio.png` são cópias byte a byte dos originais em `Documents/caio_ilc`.
- `assets/avatar.png` é uma miniatura quadrada derivada do retrato, mantendo transparência.
- O banco contém uma única tomada (`k a k`) na camada MIDI 60. Ela gera as unidades CV `ka` e VC `ak`.
- A configuração atual usa as marcações `[32,6; 133,0; 800,0; 1057,9] ms` para o início e os limites de `k a k`. Elas são referências de edição e ainda devem ser revisadas ouvindo a gravação no KAMUSE antes de usar o banco como voz final.
- O ganho inicial é `+5,5 dB` e a transição entre unidades é `20 ms`; ambos podem ser ajustados durante a revisão no KAMUSE.
- Este piloto foi definido para trabalhar com uma única tomada e uma camada MIDI 60. A segunda camada não faz parte do escopo deste banco.

O pacote `../caio.kfv` é gerado a partir desta pasta e mantém as mesmas marcações e ajustes do `cantor.toml`. Revise a gravação no KAMUSE antes de usar a tomada em uma música.
