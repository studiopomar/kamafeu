# Avisos de terceiros

Este arquivo complementa a [Licença MIT do Kamafeu](LICENSE). A licença MIT
cobre o código original deste repositório, não os arquivos, modelos, bancos de
voz, executáveis ou dados de terceiros distribuídos com ele ou usados pelo
usuário.

## Projeto e referências de formato

- **OpenUtau** — editor de síntese vocal e fonte de regras/dados de
  phonemizers portados para o Kamafeu. O projeto upstream é distribuído sob a
  Licença MIT: [OpenUtau](https://github.com/openutau/OpenUtau).
- **UTAU** — o Kamafeu implementa compatibilidade com o fluxo clássico de
  `oto.ini`, voicebanks e protocolos de resampler/wavtool. Compatibilidade não
  implica afiliação ou endosso.
- **UtaFormatix Data** — usado como formato de intercâmbio `.ufdata`. O
  esquema é mantido em [utaformatix-data](https://github.com/sdercolin/utaformatix-data),
  sob Apache-2.0.
- **MIDI, VSQX e SVP** — formatos de terceiros; o Kamafeu implementa
  conversores e não redistribui as marcas ou aplicações que os definem.

## Phonemizers e dados linguísticos

As atribuições detalhadas dos recursos incluídos estão em:

- [`src/phonemizer/data/openutau/README.md`](src/phonemizer/data/openutau/README.md)
- [`src/phonemizer/data/PLUGIN-ATTRIBUTION.md`](src/phonemizer/data/PLUGIN-ATTRIBUTION.md)
- [`src/phonemizer/data/CC-CANTO-LICENSE.md`](src/phonemizer/data/CC-CANTO-LICENSE.md)
- [`src/phonemizer/PORTING.md`](src/phonemizer/PORTING.md)

Esses arquivos preservam os créditos e as licenças dos dados derivados de
OpenUtau, OpenUtau.Plugin.Arabic, Icelandic-Diffsinger-Phonemizer e CC-Canto.

## Resamplers, wavtools e modelos

Os executáveis externos não são relicenciados pelo Kamafeu. Cada pacote deve
ser acompanhado pelos avisos de `resamplers/README.md` e pelos arquivos de
licença em `resamplers/licenses/`:

| Componente | Origem | Licença/aviso |
| --- | --- | --- |
| `straycat-rs` | [UtaUtaUtau/straycat-rs](https://github.com/UtaUtaUtau/straycat-rs) | MIT; `resamplers/licenses/straycat-rs-MIT.txt` |
| Organum | [KakouLabs/Organum](https://github.com/KakouLabs/Organum) | MIT e WORLD BSD-3-Clause; avisos em `resamplers/licenses/` |
| `macres` | [titinko/macres](https://github.com/titinko/macres) | GPL-3.0; o texto correspondente deve acompanhar qualquer artefato redistribuído |
| OpenMoresampler | [dorayakito/OpenMoresampler](https://github.com/dorayakito/OpenMoresampler) | GPL-3.0; `resamplers/OPENMORESAMPLER-GPL-3.0.txt` |
| Catalina / NSF HiFi-GAN | [OpenVPI vocoders](https://openvpi.github.io/vocoders/) | modelos sob CC BY-NC-SA 4.0; avisos em `resamplers/catalina/model/NOTICE.txt` |

`Hifisampler`, pesos NSF HiFi-GAN, voicebanks, WAVs, plugins, Wine e outros
componentes instalados pelo usuário não são automaticamente licenciados pelo
Kamafeu. Antes de redistribuí-los, preserve a licença, a origem, a versão, o
checksum e os avisos de seus autores.

## Dependências Rust

O executável é construído sobre crates publicados no crates.io. As
dependências diretas usadas nesta versão incluem `clap`, `encoding_rs`,
`eframe`, `egui`, `flacenc`, `hound`, `image`, `midly`, `ndarray`, `rand`,
`rayon`, `rodio`, `rustfft`, `serde`, `serde_json`, `ureq`, `wasmi`, `winit`,
`world-rs`, `yaml_serde`, `zip` e as crates específicas de Android, macOS e
Web declaradas em `Cargo.toml`. As dependências transitivas são fixadas por
`Cargo.lock` e mantêm os avisos de licença publicados por cada crate.

Para uma distribuição jurídica completa, gere o inventário de licenças a
partir do `Cargo.lock` com uma ferramenta como `cargo-about` e inclua o
resultado junto do artefato. Não substitua os avisos upstream por uma lista
manual incompleta.

## Assets e identidade visual

- A identidade e o banner do Kamafeu estão nos arquivos locais de `assets/`.
- O logo exibido no topo do README é servido pelo projeto
  [Studio Pomar / pomar-lts](https://github.com/studiopomar/pomar-lts).
- Os botões gráficos de download usados no README vêm de
  [ungive/discord-music-presence](https://github.com/ungive/discord-music-presence).
  Eles são apenas elementos visuais externos e não fazem parte do software
  Kamafeu.
- As imagens, fontes, ícones e temas locais devem manter seus créditos de
  autoria quando forem substituídos ou redistribuídos.

## Voicebanks e conteúdo do usuário

Voicebanks, gravações, `oto.ini`, `character.txt`, `prefix.map`, projetos e
áudios carregados pelo usuário permanecem sujeitos às licenças de seus autores
e intérpretes. O Kamafeu não concede permissão para redistribuir esse conteúdo
e não instala voicebanks automaticamente.

## Direitos autorais do Kamafeu

O código original do Kamafeu Studio é distribuído sob a [Licença MIT](LICENSE),
com copyright dos contribuidores do projeto. O nome Kamafeu, a identidade
Studio Pomar e marcas de projetos externos não são transferidos pela Licença
MIT.
