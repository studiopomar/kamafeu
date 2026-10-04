# Catálogo de phonemizers

Este arquivo é a referência do port Rust e também explica a etiqueta de
origem exibida na janela de seleção.

Referências upstream:

- [OpenUtau](https://github.com/stakira/OpenUtau)
- [Plugin Árabe de dorayakito](https://github.com/dorayakito/OpenUtau.Plugin.Arabic)
- [Phonemizer Islandês de dorayakito](https://github.com/dorayakito/Icelandic-Diffsinger-Phonemizer)
- [CC-Canto Cantonese Readings](https://github.com/amadeusine/cc-canto-data) (CC BY-SA 3.0)

| Idioma | Modo | Origem / implementação de referência |
| --- | --- | --- |
| Japonês | CV, VCV, CVVC | OpenUtau: `JapaneseCVVCPhonemizer`, `JapaneseVCVPhonemizer` e `JapaneseBasicPhonemizer` |
| Inglês | Arpasing, VCCV, G2P | OpenUtau: `ArpasingPhonemizer`, `EnglishVCCVPhonemizer`, `EnG2pPhonemizer` |
| Português | BRAPA VCCV/CVC | Kamafeu nativo |
| Português | CVVC, VCV | OpenUtau: famílias de phonemizer português |
| Português | G2P | Dados `g2p-pt` do OpenUtau |
| Francês | G2P, CVVC, VCCV | Dados `g2p-fr` e famílias CV/VCCV do OpenUtau |
| Alemão | G2P, VCCV, Diphone | Dados `g2p-de` e phonemizers do OpenUtau |
| Russo | G2P, CVC, VCCV | Dados `g2p-ru` e phonemizers do OpenUtau |
| Espanhol | G2P, VCCV | Dados `g2p-es` e phonemizers do OpenUtau |
| Turco | G2P, CVVC | OpenUtau: `TurkishCVVCPhonemizer` |
| Chinês | G2P, CVVC | OpenUtau: `ChineseCVVCPhonemizer` / romanização pinyin Rust |
| Cantonês | G2P, CVVC | OpenUtau: `CantoneseCVVCPhonemizer` / Jyutping Rust |
| Árabe | DiffSinger G2P | `dorayakito/OpenUtau.Plugin.Arabic` |
| Islandês | DiffSinger G2P | `dorayakito/Icelandic-Diffsinger-Phonemizer` |

`OpenUtau` na UI significa que o modo foi portado ou que seus dados/regras
foram derivados do projeto upstream. `Plugin dorayakito/OpenUtau` identifica
os dois plugins externos fornecidos para Árabe e Islandês. `Kamafeu nativo`
identifica código desenvolvido neste projeto.
