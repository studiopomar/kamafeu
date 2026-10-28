# Resamplers externos incluídos

Os binários desta pasta são executados pelo Kamafeu por meio da interface de linha de comando clássica do UTAU. O código-fonte de cada projeto permanece no repositório original.

| Arquivo | Projeto | Versão/plataforma | Licença | SHA-256 |
| --- | --- | --- | --- | --- |
| `macres` | https://github.com/titinko/macres | macOS x86_64 | GPL-3.0 | `196a9037b197c7f342d03a6e975f89d7ef9f6042e30f7004bf91a7269629ac93` |
| `organum-resampler` | https://github.com/KakouLabs/Organum | v0.0.8 macOS arm64 CPU | MIT | `56194eeea9e7c64cdbaa332060862282f49427fb3b36b6a51d898abb1c2a7ede` |
| `straycat-rs` | https://github.com/UtaUtaUtau/straycat-rs | v1.1.0 macOS arm64 | MIT | `11a8ecec5d57e09636d7b6bc4bdfe082ae102ff7abaf07ce0e29f6a658c5b1d5` |
| `openmoresampler.exe` | https://github.com/dorayakito/OpenMoresampler | 64-bit Windows via Wine | GPL-3.0 | `396d2b0e87004b00101a26469d46565cc1145b2dfd341be85b7117a743742e3e` |

O Organum também incorpora componentes WORLD sob licença BSD de 3 cláusulas; consulte `licenses/organum-WORLD-BSD-3-Clause.txt`. Os avisos de licença acompanham os binários em `licenses/`.

O `macres` é identificado pelo upstream como GPL-3.0. O texto integral dessa
licença deve acompanhar qualquer pacote que redistribua o binário; antes de
publicar um artefato, confira a licença e o checksum diretamente no projeto
[titinko/macres](https://github.com/titinko/macres). O repositório Kamafeu não
usa a MIT do aplicativo para relicenciar esse executável.

## Instalação local do Hifisampler

Quando um resampler externo não estiver no pacote, o painel **Motor** mostra o
link do projeto e abre a pasta de destino. A pasta
sugerida é `~/Documents/kamafeu/resamplers/<engine>/`; coloque ali o executável
com o nome indicado pelo upstream. Resamplers `.exe` no macOS/Linux também
precisam do Wine. O Kamafeu continua procurando instalações antigas em
`./resamplers/` e no `PATH`.

## Catalina / NSF HiFi-GAN

O executável `catalina` é construído pelo próprio projeto e mantém o protocolo
clássico de resamplers UTAU. Coloque-o em
`~/Documents/kamafeu/resamplers/catalina/catalina`. O modelo deve ficar na pasta
`model/` ao lado dele:

```text
Documents/kamafeu/resamplers/catalina/
├── catalina
├── hificonfig.ini       # criado pelo Kamafeu se estiver ausente
└── model/
    └── pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx
```

Baixe o pacote PC-NSF-HiFiGAN na
[release oficial do OpenVPI Vocoders](https://github.com/openvpi/vocoders/releases/tag/pc-nsf-hifigan-44.1k-hop512-128bin-2025.02).
Renomeie o sufixo `.oudep` para `.zip`, extraia e copie o ONNX para a pasta
`model/` acima. O arquivo `.oudep` é um pacote de dependência OpenUTAU; a release
informa que renomeá-lo para `.zip` permite extrair o ONNX. O modelo usa licença
CC BY-NC-SA 4.0; leia o aviso incluído antes de redistribuir os pesos.

Para o Hifisampler, baixe o pacote completo na
[página de releases do projeto](https://github.com/openhachimi/hifisampler/releases)
e preserve sua estrutura ao extraí-lo em
`~/Documents/kamafeu/resamplers/hifisampler/`. O cliente `hifisampler` é o
resampler chamado pelo Kamafeu; inicie o servidor do pacote conforme as
instruções upstream antes de renderizar. O arquivo `hnsep_model.onnx` também
deve estar em `model/` ao lado do HiFi-GAN. A release v1.1.0 do
[Hifisampler-RS](https://github.com/Slidingwall/hifisampler-rs/releases/tag/v1.1.0)
é somente um backend servidor, não substitui o executável cliente.

Os executáveis `hifisampler`, `hifisampler-rs`, `hifiserver-rust`, o arquivo
`hificonfig.ini` e a pasta `model/` são instalações locais opcionais e estão
ignorados pelo Git. Não são incluídos automaticamente na distribuição.
Antes de distribuí-los, registre origem, versão, arquitetura, checksum e
licenças dos executáveis e de cada modelo. Os arquivos locais são preservados.

## Outros resamplers externos

Instale cada engine em sua pasta sugerida, que o Kamafeu informa ao lado da
seleção do resampler:

| Engine | Executável esperado | Pasta sugerida | Download/instruções |
| --- | --- | --- | --- |
| MoReSampler | `moresampler.exe` | `~/Documents/kamafeu/resamplers/moresampler/` | [Lista de resamplers do OpenUTAU](https://github.com/openutau/OpenUtau/wiki/Resamplers-and-Wavtools) |
| OpenMoresampler | `openmoresampler.exe` | `~/Documents/kamafeu/resamplers/openmoresampler/` | [Releases do OpenMoresampler](https://github.com/dorayakito/OpenMoresampler/releases) |
| World4UTAU | `world4utau` ou `.exe` | `~/Documents/kamafeu/resamplers/world4utau/` | [Projeto World4UTAU](https://github.com/xrdavies/world4utau) |
| TIPS | `TIPS.exe` | `~/Documents/kamafeu/resamplers/tips/` | [Lista de resamplers do OpenUTAU](https://github.com/openutau/OpenUtau/wiki/Resamplers-and-Wavtools) |

O Kamafeu procura também executáveis já instalados diretamente em
`resamplers/`, nos caminhos da aplicação e no `PATH`, então não é necessário
mover instalações que já estejam funcionando.

## OpenMoresampler

O Kamafeu reconhece o [OpenMoresampler](https://github.com/dorayakito/OpenMoresampler)
como um perfil UTAU CLI separado. Coloque `openmoresampler.exe` ou o
`moresampler.exe` gerado pelo projeto em
`~/Documents/kamafeu/resamplers/openmoresampler/` (ou mantenha-o diretamente em
`resamplers/`) e selecione **OpenMoresampler** no painel Motor. O upstream informa que a versão atual é
compatível com o protocolo Moresampler 0.8.4, está em validação e funciona
somente no Windows; em macOS/Linux será necessário executá-lo via Wine.

O binário Windows validado está incluído como `resamplers/openmoresampler.exe`.
Ele é executado nativamente no Windows e via Wine no macOS/Linux. A licença
GPL-3.0 acompanha o arquivo em `OPENMORESAMPLER-GPL-3.0.txt`; o código-fonte
correspondente está disponível no repositório upstream. Confirme a origem e o
checksum antes de substituir ou redistribuir o executável.

No macOS, o build pode ser feito com:

```sh
brew install cmake libomp
./scripts/build-openmoresampler-macos.sh
```

O script gera `resamplers/openmoresampler`, preserva a licença GPL e mantém o
código-fonte correspondente em um diretório temporário.

Para gerar o executável Windows 64-bit a partir do macOS com MinGW:

```sh
brew install mingw-w64
./scripts/build-openmoresampler-windows-cross.sh
```

Binário atualmente gerado: `openmoresampler.exe` — SHA-256
`396d2b0e87004b00101a26469d46565cc1145b2dfd341be85b7117a743742e3e`.
