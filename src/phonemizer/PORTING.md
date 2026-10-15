# Catálogo de phonemizers

Este arquivo é a referência do port Rust e também explica a etiqueta de
origem exibida na janela de seleção.

Referências upstream:

- [OpenUtau](https://github.com/stakira/OpenUtau)
- [Plugin Árabe de dorayakito](https://github.com/dorayakito/OpenUtau.Plugin.Arabic)
- [Phonemizer Islandês de dorayakito](https://github.com/dorayakito/Icelandic-Diffsinger-Phonemizer)
- [CC-Canto Cantonese Readings](https://github.com/amadeusine/cc-canto-data) (CC BY-SA 3.0)

## Contrato de compatibilidade

Os nomes OpenUtau serializados pelo Kamafeu são acompanhados contra a revisão
`13184085f2cee51c174de9ee99163a76ee970e8c`. O mapeamento USTX já preserva as
classes nativas das famílias abaixo, mas isso não significa que o algoritmo
fonético de cada família esteja validado.

| Estado | Famílias |
| --- | --- |
| Portado e coberto pelo corpus atual | Japonês CV, VCV, CVVC e Presamp; Inglês Arpasing/VCCV; Chinês CVV/CVV+; Francês CMUSphinx/CVVC/VCCV |
| Portado com corpus sintético por família | Coreano CV/CVC/VCV/CVVC/CVCCV/CBNN; Russo CVC/VCCV; Espanhol Syllable-Based/Makkusan/VCCV; Chinês VCV/CVVC; Cantonês CVVC/Syo; PT-BR CVC e BRAPA |
| Portado por dados/regras existentes | Alemão, Turco, Árabe e Islandês; os dois últimos preservam a origem dos plugins externos |

`OpenUtau (portado)` indica que há um caminho de execução específico para a
família e que seus aliases foram derivados do upstream. A etiqueta não é uma
prova de igualdade de áudio; essa validação ainda exige executar os mesmos
USTX e voicebank no OpenUtau.

| Idioma | Modo | Origem / implementação de referência |
| --- | --- | --- |
| Japonês | CV, VCV, CVVC | OpenUtau: `JapaneseCVVCPhonemizer`, `JapaneseVCVPhonemizer` e `JapaneseBasicPhonemizer` |
| Inglês | Arpasing, VCCV, G2P | OpenUtau: `ArpasingPhonemizer`, `EnglishVCCVPhonemizer`, `EnG2pPhonemizer` |
| Português | BRAPA VCCV/CVC | Kamafeu nativo (legado; não é o PT-BR CVC do OpenUtau) |
| Português | PT-BR CVC | OpenUtau: `BrazilianPortugueseCVCPhonemizer` (núcleo em desenvolvimento) |
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
