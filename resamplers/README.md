# Resamplers externos incluídos

Os binários desta pasta são executados pelo Kamafeu por meio da interface de linha de comando clássica do UTAU. O código-fonte de cada projeto permanece no repositório original.

| Arquivo | Projeto | Versão/plataforma | Licença | SHA-256 |
| --- | --- | --- | --- | --- |
| `macres` | https://github.com/titinko/macres | macOS x86_64 | GPL-3.0 | `196a9037b197c7f342d03a6e975f89d7ef9f6042e30f7004bf91a7269629ac93` |
| `organum-resampler` | https://github.com/KakouLabs/Organum | v0.0.8 macOS arm64 CPU | MIT | `56194eeea9e7c64cdbaa332060862282f49427fb3b36b6a51d898abb1c2a7ede` |
| `straycat-rs` | https://github.com/UtaUtaUtau/straycat-rs | v1.1.0 macOS arm64 | MIT | `11a8ecec5d57e09636d7b6bc4bdfe082ae102ff7abaf07ce0e29f6a658c5b1d5` |

O Organum também incorpora componentes WORLD sob licença BSD de 3 cláusulas; consulte `licenses/organum-WORLD-BSD-3-Clause.txt`. Os avisos de licença acompanham os binários em `licenses/`.

## Instalação local do Hifisampler

## Catalina / NSF HiFi-GAN

O executável `catalina` é construído pelo próprio projeto e mantém o protocolo
clássico de resamplers UTAU. Ele procura um backend NSF HiFi-GAN ao lado dele,
em `resamplers/` ou no `PATH`; também aceita `CATALINA_BACKEND=/caminho/do/backend`.

O pacote de weights recomendado é `pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.oudep`,
da release oficial do projeto vocoders. Extraia o arquivo e coloque o ONNX em
`resamplers/model/`; o Kamafeu cria a configuração padrão automaticamente.

Os executáveis `hifisampler`, `hifisampler-rs`, `hifiserver-rust`, o arquivo
`hificonfig.ini` e a pasta `model/` são instalações locais opcionais e estão
ignorados pelo Git. Não são incluídos automaticamente na distribuição.
Antes de distribuí-los, registre origem, versão, arquitetura, checksum e
licenças dos executáveis e de cada modelo. Os arquivos locais são preservados.
