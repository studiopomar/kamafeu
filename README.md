<div align="center">

<img src="https://raw.githubusercontent.com/studiopomar/pomar-lts/main/public/studio-pomar-icon-4096.png" alt="Logo do Studio Pomar" width="120" height="120" />

# Kamafeu Studio

**Editor e sintetizador vocal baseado em amostragem e splicing (UTAU) em Rust.**

[![Versão](https://img.shields.io/badge/vers%C3%A3o-1.0.0--A-d7ff3f?style=flat-square)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange?style=flat-square&logo=rust)](Cargo.toml)
[![Licença: MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-yellow.svg?style=flat-square)](LICENSE)
[![Studio Pomar](https://img.shields.io/badge/Studio-Pomar-brightgreen?style=flat-square)](https://studiopomar.github.io/pomar-lts/)

**Programador:** xiao | **Direção de arte:** mori-p | **Bug reporter e QA (Linux):** makki | **Testador e QA (Windows e Linux):** zoneryth | **Testador e QA (macOS):** xiao

[Downloads](https://github.com/studiopomar/kamafeu/releases) | [Primeiros passos](#primeiros-passos) | [Compilação](#instruções-de-compilação) | [Histórico de alterações](CHANGELOG.md)

<img src="assets/kamafeu_banner.png" alt="Kamafeu Studio" width="1200" />

</div>

O **Kamafeu Studio** é um ambiente completo de composição, edição e síntese de canto voltado para o ecossistema UTAU e OpenUtau. Ele combina piano roll de alta precisão, modelagem contínua de afinação, ajustes fonéticos detalhados e pipeline de áudio multithread, com suporte a motores nativos em Rust e executáveis externos da comunidade.

O nome **Kamafeu** faz referência à tradicional joia em camafeu, esculpida manualmente em relevo. O software não utiliza redes neurais nem difusão (como DiffSinger ou modelos estatísticos de aprendizado profundo). O propósito central do projeto é fornecer controle técnico direto e transparente sobre o processo clássico de amostragem, calibração (*oto.ini*), afinação e transição acústica de bancos de voz gravados.

## Sumário

- [Recursos do sistema](#recursos-do-sistema)
  - [Piano roll e composição melódica](#piano-roll-e-composição-melódica)
  - [Bancos de voz e suporte fonético](#bancos-de-voz-e-suporte-fonético)
  - [Processamento e efeitos (DSP)](#processamento-e-efeitos-dsp)
- [Arquitetura e fluxo de síntese](#arquitetura-e-fluxo-de-síntese)
- [Formatos de arquivo suportados](#formatos-de-arquivo-suportados)
- [Motores de áudio](#motores-de-áudio-resamplers-e-wavtools)
  - [Motores de afinação (Resamplers)](#motores-de-afinação-resamplers)
  - [Motores de junção e emenda de fonemas (Wavtools)](#motores-de-junção-e-emenda-de-fonemas-wavtools)
- [Copaiba Voicebank Toolkit (Experimental)](#copaiba-voicebank-toolkit-experimental)
- [Primeiros passos](#primeiros-passos)
- [Versão mobile (Android e iOS)](#versão-mobile-android-e-ios)
- [Instruções de compilação](#instruções-de-compilação)
  - [Linux](#linux)
  - [FreeBSD](#freebsd)
  - [macOS](#macos)
  - [Windows](#windows)
- [Interface de linha de comando](#interface-de-linha-de-comando)
- [Tabela de atalhos de teclado](#tabela-de-atalhos-de-teclado)
- [Estrutura do código-fonte](#estrutura-do-código-fonte)
- [Licença](#licença)

## Recursos do sistema

### Piano roll e composição melódica
- **Ferramentas de edição:** Ponteiro de seleção e movimentação (`V`), lápis de desenho contínuo (`N`), divisão de notas (`C`) e borracha (`E`).
- **Desenho e modelagem de afinação:** Pincel livre de pitch (`P`), glissando em linha reta, pincel gerador de vibrato e suavizador de curvas. Suporte completo a pontos de portamento e envelopes com curvas S, lineares e Bézier.
- **Suporte a escalas musicais e guia tonal:** Assistente de escalas (Maior, Menor Natural, Harmônica, Melódica, Pentatônicas, Blues, Dórico e Mixolídio) com destaque visual das notas dentro do tom e marcação da tônica.
- **Expressões por nota:** Painel inferior retrátil para edição de dinâmica, modulação, velocidade de consoante, sopro (*breathiness*), formante de gênero (*gender*) e envelopes UTAU de amplitude em 5 pontos.
- **Ferramenta de repetição de loop:** Delimitação de região de repetição `[A ... B]` diretamente na régua com `Shift + Clique / Arraste`, além de controles numéricos com recomeço contínuo e sem engasgos.
- **Metrônomo sincronizado:** Síntese de cliques no andamento do projeto com acentuação tonal no primeiro tempo de cada compasso.
- **Exportação rápida de seleção:** Opção de menu e atalho de clique direito para exportar exclusivamente as notas selecionadas no formato `[Projeto] - [Voicebank] - wip.wav`.
- **Área de trabalho otimizada:** Painéis recolhíveis de arranjo multifaixa (`Alt + A`), expressões (`Tab`), fonemas (`Alt + O`), inspetor lateral (`Cmd/Ctrl + B`) e modo de tela cheia (`F11`).

### Bancos de voz e suporte fonético
- **Compatibilidade UTAU e OpenUtau:** Leitura e gravação de arquivos `oto.ini` em codificações UTF-8 e Shift-JIS com suporte a múltiplos tons via `prefix.map`.
- **Fonemizadores integrados:**
  - Japonês: CV (Hiragana), VCV e CVVC com conversão automática Romaji para Kana.
  - Português: BRAPA VCCV, CVC, CVVC e VCV, além de conversão ortográfica G2P.
  - Inglês: VCCV com suporte completo ao inventário fonético de encontros consonantais.
  - Modo manual: Inserção direta de aliases e subfonemas separados por ponto ou ponto e vírgula.
- **Régua de fonemas:** Visualização gráfica dos limites de corte, preutterance, overlap e consoante fixa diretamente abaixo do piano roll.
- **Pacotes compactados `.kfv`:** Formato do Kamafeu para distribuição de cantores virtuais com metadados e áudio empacotados.

### Processamento e efeitos (DSP)
- **Rack de efeitos integrado:** Equalizador paramétrico e gráfico de 31 bandas, compressor de dinâmica, chorus, delay de sincronismo e reverb estéreo aplicáveis por faixa.
- **Alinhamento de fase e equal-power crossfade:** Junção suave entre notas adjacentes na mixagem de saída, eliminando estalos de fase e picos de distorção no somatório do buffer.
- **Auto-Pitch:** Sistema de afinação orgânica para aplicação de portamentos de entrada, quedas de final de frase e vibratos proporcionais ao andamento musical.

## Arquitetura e fluxo de síntese

A arquitetura do Kamafeu Studio foi projetada de ponta a ponta em Rust para garantir baixa latência na interface do usuário, processamento DSP determinístico e síntese paralela de áudio.

```text
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           CAMADA DE PROJETO & MODELAGEM                         │
│  Projeto (.aps, .ustx, .ust, .mid, .ufdata, .svp, .vsqx)                        │
│  ├── Faixas Vocais e Partes de Áudio (Volume, Pan, Mute, Solo)                  │
│  ├── Sequência de Notas (Posição ms, Duração ms, Notação MIDI, Sílaba / Letra)   │
│  ├── Curva Contínua de Afinação (Pontos de Portamento, Vibrato, Glissando)       │
│  └── Parâmetros de Expressão (Dinâmica, Velocity, Modulação, Sopro, Gênero)     │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      CAMADA FONÉTICA & TEMPORIZAÇÃO (OTO)                       │
│  ├── Resolução de Fonemas: Motor Fonemizador (Japonês, Português BRAPA, Inglês)  │
│  ├── Mapeamento Multitom: prefix.map (Seleção automática da amostra por tom)    │
│  └── Calibragem Acústica: oto.ini                                               │
│      (Offset, Consonant Fixo, Cutoff, Preutterance de Ataque, Overlap de Fusão) │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                   PIPELINE DE RENDERIZAÇÃO & SÍNTESE DSP                        │
│  ├── Fila Concorrente (Thread Pool Rayon) & Cache de Amostras em Memória        │
│  ├── Resampler (Modulação Tonal & Estiramento Temporal):                        │
│  │   ├── VENUS (Nativo): Análise YIN, Síntese Formântica & Fase Mínima          │
│  │   └── straycat-rs / Motores Externos via UTAU CLI Protocol                    │
│  ├── Wavtool (Emenda, Splicing & Envelopamento):                                │
│  │   ├── Andromeda (Nativo): Envelopes UTAU de 5 pontos & Equal-Power Crossfade │
│  │   └── wavtool-yawu / Utilitários Externos                                    │
│  ├── Alinhamento de Fase: Redução de cancelamentos harmônicos em vogais ligadas │
│  ├── Rack de Efeitos (FX): Equalizador Gráfico 31 bandas, Compressor, Reverb     │
│  └── Barramento de Saída: Streaming progressivo (AudioPlayer) / Exportação WAV   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Etapas do ciclo de processamento

1. **Estrutura de dados e eventos temporais:** O projeto decompõe composições em faixas e partes independentes. As notas musicais contêm posições e durações absolutas em milissegundos calculadas em função do andamento (BPM) e da métrica de compasso.
2. **Resolução fonética e particionamento silábico:** O fonemizador converte as letras digitadas em cadeias de fonemas individuais (como ataques de início de frase, transições consoante-vogal e consoantes de término). Para cada fonema, o sistema consulta a tabela `oto.ini` do cantor ativo e determina a amostra WAV correspondente de acordo com o tom da nota (`prefix.map`).
3. **Cálculo de temporização e sobreposição (*Timing Engine*):** A posição acústica de cada amostra é ajustada aplicando os valores de *preutterance* (para antecipar consoantes antes da batida musical) e *overlap* (definindo a zona de fusão com a nota precedente).
4. **Resampling e transposição tonal:** O motor de afinação (como o **VENUS** nativo ou o **straycat-rs**) isola o trecho útil de áudio delimitado por *offset* e *cutoff*, preserva a consoante fixa sem deformação temporal e estica ou comprime a vogal periódica, transpondo a frequência fundamental para acompanhar a curva contínua de pitch bend e os parâmetros de expressão.
5. **Emenda, envelopamento e splicing (*Wavtool & Mixing*):** O motor **Andromeda** aplica a curva de amplitude UTAU de 5 pontos a cada fatia e costura as amostras no buffer da faixa utilizando interpolação suave e *equal-power crossfade*.
6. **Streaming em tempo real e visualização:** O renderizador despacha tarefas em paralelo por blocos temporais (*progressive chunks*) através de um pool de threads gerenciado pelo Rayon. À medida que os blocos ficam prontos, eles são enviados diretamente para o dispositivo de áudio através de canais sincronizados sem bloqueio da interface gráfica, gerando simultaneamente a forma de onda de alta resolução exibida no fundo do piano roll.

## Formatos de arquivo suportados

O Kamafeu Studio lê e grava projetos em múltiplos formatos do ecossistema de síntese vocal:

| Formato | Extensão | Importação | Exportação | Descrição técnica |
| --- | --- | :---: | :---: | --- |
| **Arquivo de Projeto Saturno** | `.aps` | Sim | Sim | **Formato nativo do Kamafeu Studio.** Armazena em JSON estruturado todas as faixas vocais, partes de áudio WAV, curvas contínuas de pitch bend em alta resolução, envelopes de 5 pontos, configurações do FX Rack e metadados de projeto. |
| **OpenUtau Project** | `.ustx` | Sim | Sim | **Formato moderno do OpenUtau (YAML).** Preserva estrutura multifaixa, cantores atribuídos, expressões dinâmicas, curvas de afinação e parâmetros de respiração/gênero. |
| **UTAU Sequence Text** | `.ust` | Sim | Sim | **Formato clássico do UTAU.** Arquivo de texto estruturado por seções `[#0000]` contendo notas, letras, andamento (BPM), portamentos (`PBType`, `PBDst`) e envelopes de amplitude. |
| **Standard MIDI File** | `.mid`, `.midi` | Sim | Sim | **Padrão internacional MIDI.** Importa e exporta notas, tempos métricos, andamento e mensagens de pitch wheel para intercâmbio com DAWs como Reaper, FL Studio, Ableton e Logic Pro. |
| **UtaFormatix Data** | `.ufdata` | Sim | Sim | **Esquema universal de intercâmbio (JSON).** Padrão aberto criado para migração fiel de dados melódicos e temporais entre diversos sintetizadores de canto do mercado. |
| **Synthesizer V Project** | `.svp` | Sim | Sim | **Formato de projeto do Synthesizer V (Dreamtonics).** Conversão direta de trilhas melódicas, notas, letras e divisões silábicas. |
| **VOCALOID Sequence** | `.vsqx` | Sim | Sim | **Formato de sequência XML do VOCALOID (Yamaha).** Converte dados de trilhas de canto (`vocaloidStyleTrack`), notas musicais, letras e curvas de pitch bend. |
| **Kamafeu Voicebank** | `.kfv` | Sim | Sim | **Pacote de cantor do Kamafeu Studio (ZIP compactado).** Agrupa gravações de áudio WAV, arquivos `oto.ini`, `character.txt`, `prefix.map` e metadados de identificação do banco de voz. |

## Motores de áudio (Resamplers e Wavtools)

O Kamafeu Studio permite alternar entre o pipeline nativo de processamento e ferramentas clássicas de linha de comando:

### Motores de afinação (Resamplers)
- **VENUS (Nativo):** Motor nativo em Rust baseado em síntese formântica pura, análise de periodicidade com YIN e reconstrução espectral de fase mínima. Preserva a inteligibilidade de consoantes rápidas sem cortes de alias e mantém a ressonância natural da voz sem artefatos anasalados.
- **straycat-rs (UtaUtaUtau):** Motor padrão recomendado para máxima compatibilidade e clareza acústica.
- **Motores externos:** Suporte transparente a executáveis da comunidade como `Hifisampler`, `macres`, `moresampler`, `world4utau`, `TIPS` e `Organum`. No Windows, esses binários rodam nativamente por se tratarem de executáveis do sistema; no macOS e Linux, rodam perfeitamente ao detectarem o Wine instalado na máquina.

### Motores de junção e emenda de fonemas (Wavtools)
- **Andromeda (Nativo):** Motor de emenda e costura de fonemas interno de alta performance escrito em Rust. Realiza cálculo de envelope de amplitude, interpolação de amostras e crossfade de potência constante (*equal-power*) diretamente na memória.
- **wavtool-yawu / wavtool clássico:** Junção e costura de fonemas via linha de comando com suporte a fluxo contínuo por frases.

## Copaiba Voicebank Toolkit (Experimental)

> **Aviso de desenvolvimento:** O Copaiba Toolkit é um utilitário em estágio experimental e não está pronto para uso em produção. Sua interface, estrutura de dados e rotinas de gravação de arquivos ainda passam por alterações frequentes.

O **Copaiba** foi projetado para calibragem, teste e organização de bancos de voz UTAU. Ele pode ser aberto como utilitário independente (`copaiba`) ou acionado a partir de qualquer fonema na régua do Kamafeu para inspeção dos parâmetros da `oto.ini`:

- **Offset:** Início útil da amostra, descartando silêncios ou ruídos mecânicos de captação.
- **Consonant (Consoante fixa):** Intervalo temporal que não sofre estiramento durante alterações de andamento.
- **Cutoff:** Ponto de término da amostra. Valores positivos cortam a partir do final do arquivo; valores negativos determinam a extensão a partir do offset.
- **Preutterance:** Antecipação do ataque consonantal em relação à batida métrica da nota.
- **Overlap:** Extensão da sobreposição suave com o fonema anterior para evitar descontinuidades.

## Primeiros passos

1. **Obtenha o executável:** Baixe a versão correspondente ao seu sistema operacional na aba de [Releases](https://github.com/studiopomar/kamafeu/releases) ou realize a compilação local.
2. **Carregue um banco de voz:** Abra a aba de cantores no painel lateral e selecione uma pasta de voicebank contendo arquivos `.wav` e `oto.ini`.
3. **Crie ou importe sequências:** Desenhe notas com o lápis (`N`) ou importe projetos `.ustx`, `.ust` ou `.mid` pelo menu **Arquivo**.
4. **Configure a fonética:** Selecione o fonemizador apropriado no menu **Modos Vocais** ou insira aliases manualmente nas notas.
5. **Modele a afinação:** Utilize a ferramenta de pitch (`P`) para criar curvas de transição, portamentos e vibratos.
6. **Reproduza e exporte:** Pressione `Espaço` para ouvir a prévia e utilize o menu **Arquivo -> Exportar Áudio** para gerar o arquivo final em WAV ou FLAC.

## Versão mobile (Android e iOS)

O Kamafeu Studio conta com infraestrutura de código baseada em `winit` e `egui`, permitindo a compilação cruzada para dispositivos móveis (smartphones e tablets). No entanto, o ecossistema mobile impõe desafios arquiteturais e restrições técnicas significativas:

### Desafios de adaptação e build

1. **Impossibilidade de executar resamplers externos (Subprocessos CLI):**
   - Nos sistemas operacionais desktop (Windows, macOS, Linux), o Kamafeu pode invocar executáveis externos como `straycat-rs.exe` ou `hifisampler` através de processos filhos (`std::process::Command`) e Wine.
   - No Android e iOS, o isolamento rigoroso de processos (*sandbox*) e as políticas de segurança das lojas proíbem a criação de processos filhos arbitrários e a execução de binários Win32/x86.
   - **Solução no Kamafeu:** Dispositivos móveis dependem 100% dos motores nativos (**VENUS** para modulação de pitch e **Andromeda** para emenda e junção de fatias), que rodam diretamente linkados na mesma memória do aplicativo.

2. **Sistema de arquivos e permissões de armazenamento (*Scoped Storage*):**
   - No Android moderno (API 30+) e iOS, o acesso direto a caminhos tradicionais como `/sdcard/` ou pastas compartilhadas é bloqueado. O carregamento de voicebanks e projetos exige integração com *Storage Access Framework* (SAF) ou empacotamento prévio em arquivos `.kfv` / `.zip`.

3. **Adaptação de interface para toque (*Touch & Gesture Input*):**
   - O piano roll e a régua do UTAU foram historicamente projetados para cursor de mouse com precisão de pixel, botões secundários (clique direito) e atalhos de teclado.
   - A adaptação mobile exige zonas de toque ampliadas, gestos de pinça (*pinch-to-zoom*) para navegação temporal e vertical, e diálogos contextuais sensíveis ao toque.

4. **Gerenciamento de memória e limites de CPU:**
   - Em dispositivos móveis, a renderização multithread com Rayon precisa respeitar limites térmicos e de consumo de bateria, exigindo particionamento adaptativo de blocos de áudio e buffers menores de baixa latência (via AAudio/OpenSL ES no Android e CoreAudio no iOS).

### Compilação para Android

A compilação do APK do Kamafeu utiliza a ferramenta `cargo-apk` e o Android NDK:

```bash
# Compilar APK em modo release para arquitetura ARM64
cargo apk build --lib --release --target aarch64-linux-android
```

Para instruções detalhadas sobre assinatura criptográfica de APKs e variáveis de ambiente de keystore, consulte [docs/android-signing.md](docs/android-signing.md).

## Instruções de compilação

Requisitos básicos: **Rust 1.82 ou superior** e ferramenta **Cargo** instalados via [rustup.rs](https://rustup.rs/).

### Linux

Instale o compilador C, o gerenciador de pacotes `pkg-config`, os cabeçalhos do ALSA para áudio e as bibliotecas gráficas do X11/Wayland:

#### Ubuntu / Debian / Linux Mint / Pop!_OS
```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libasound2-dev \
  libxcb-render0-dev \
  libxcb-shape0-dev \
  libxcb-xfixes0-dev \
  libxkbcommon-dev \
  libfontconfig1-dev
```

#### Fedora / RHEL
```bash
sudo dnf install -y \
  gcc \
  pkg-config \
  alsa-lib-devel \
  libxcb-devel \
  libxkbcommon-devel \
  fontconfig-devel
```

#### Arch Linux / Manjaro
```bash
sudo pacman -S --needed \
  base-devel \
  pkgconf \
  alsa-lib \
  libxcb \
  libxkbcommon \
  fontconfig
```

### FreeBSD

No FreeBSD, instale o compilador LLVM/Clang, `pkgconf` e as bibliotecas do sistema gráfico X11 e ALSA:

```sh
sudo pkg install -y \
  rust \
  pkgconf \
  alsa-lib \
  libxcb \
  libxkbcommon \
  fontconfig
```

> **Nota:** Para habilitar a saída de áudio com compatibilidade ALSA no FreeBSD, certifique-se de que o pacote `alsa-plugins` esteja configurado ou utilize a camada de emulação de áudio OSS/sndio suportada pelo sistema.

### macOS

No macOS (Apple Silicon M1/M2/M3/M4 ou Intel x86_64), são necessárias apenas as ferramentas de linha de comando do Xcode (o backend gráfico Metal/Cocoa e o subsistema CoreAudio são nativos):

```bash
# Instalar ferramentas de compilação da Apple
xcode-select --install
```

### Windows

No Windows 10/11, utilize a cadeia de ferramentas MSVC recomendada pelo Rust:

1. Baixe e instale o [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
2. No instalador, marque a carga de trabalho **Desenvolvimento para Desktop com C++** (*Desktop development with C++*) e conclua a instalação dos componentes Windows 10/11 SDK.
3. Certifique-se de que o Rust esteja configurado para a toolchain MSVC:
   ```cmd
   rustup default stable-x86_64-pc-windows-msvc
   ```

---

### Compilação dos binários

Clone o repositório e compile todos os executáveis em modo otimizado (*release*):

```bash
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release --bins
```

Os executáveis gerados estarão localizados em `target/release/`:
- `kamafeu` (ou `kamafeu.exe` no Windows): Editor principal e sintetizador multifaixa.
- `copaiba` (ou `copaiba.exe` no Windows): Utilitário de calibração de voicebanks (`oto.ini`).

Para executar diretamente:

```bash
# Executar o editor Kamafeu
cargo run --release --bin kamafeu

# Executar o utilitário Copaiba
cargo run --release --bin copaiba
```

## Interface de linha de comando

O Kamafeu Studio disponibiliza comandos via terminal para tarefas em lote e checagem de arquivos:

```bash
# Exibir lista completa de comandos
cargo run --release --bin kamafeu -- --help

# Inspecionar dados e integridade de um banco de voz
cargo run --release --bin kamafeu -- voicebank-info "/caminho/do/voicebank"

# Renderizar um projeto diretamente para áudio WAV
cargo run --release --bin kamafeu -- render \
  --voicebank "/caminho/do/voicebank" \
  --input "projeto.aps" \
  --output "saida.wav" \
  --sample-rate 44100
```

## Tabela de atalhos de teclado

No macOS, utilize a tecla `Cmd` no lugar de `Ctrl`.

### Ferramentas de edição

| Tecla / Atalho | Ferramenta / Ação |
| --- | --- |
| `V` ou `1` | Ferramenta Ponteiro (seleção, movimentação e redimensionamento) |
| `N` ou `2` | Ferramenta Lápis (inserção e desenho contínuo de notas) |
| `P` ou `3` | Ferramenta Pitch (desenho livre, reta, vibrato e suavizador) |
| `Shift + P` | Alternar submodo de desenho de pitch (Livre, Reta, Vibrato, Suave) |
| `C` ou `4` | Ferramenta de corte e divisão de notas |
| `E` ou `5` | Ferramenta Borracha (exclusão de notas) |
| `Duplo-clique` / `Shift + Clique` na curva | Adicionar novo ponto de ancoragem no pitch |
| `Alt + Clique` / `Clique Direito` na âncora | Deletar ponto de ancoragem específico da curva |

### Transporte e navegação

| Atalho | Ação |
| --- | --- |
| `Espaço` | Iniciar ou pausar reprodução |
| `Esc` | Parar reprodução e retornar o cursor ao início (0 ms) |
| `Shift + Clique / Arraste na régua` | Definir região de repetição contínua [A ... B] |
| `M` | Alternar silenciamento (*Mute*) na faixa ativa |
| `Ctrl + =` / `Cmd + =` | Aumentar zoom horizontal da timeline |
| `Ctrl + -` / `Cmd + -` | Diminuir zoom horizontal da timeline |
| `Ctrl + 0` / `Cmd + 0` | Redefinir zoom padrão da timeline |

### Edição de notas e manipulação

| Atalho | Ação |
| --- | --- |
| `Ctrl + Z` / `Cmd + Z` | Desfazer última alteração |
| `Ctrl + Y` / `Ctrl + Shift + Z` / `Cmd + Shift + Z` | Refazer alteração desfeita |
| `Ctrl + X` / `Cmd + X` | Recortar notas selecionadas |
| `Ctrl + C` / `Cmd + C` | Copiar notas selecionadas |
| `Ctrl + V` / `Cmd + V` | Colar notas na posição do cursor de reprodução |
| `Ctrl + D` / `Cmd + D` | Duplicar notas selecionadas |
| `Ctrl + A` / `Cmd + A` | Selecionar todas as notas da faixa |
| `Ctrl + Shift + A` / `Cmd + Shift + A` | Desmarcar seleção de notas |
| `Delete` / `Backspace` | Excluir notas selecionadas |
| `↑` / `↓` | Transpor notas em semitons (+1 / -1 semitom) |
| `Shift + ↑` / `Shift + ↓` | Transpor notas em oitavas (+12 / -12 semitons) |
| `←` / `→` | Deslocar posição das notas no tempo (-50 ms / +50 ms) |
| `Shift + ←` / `Shift + →` | Alterar duração das notas (-50 ms / +50 ms) |

### Arquivo, janelas e painéis

| Atalho | Ação |
| --- | --- |
| `Ctrl + N` / `Cmd + N` | Criar novo projeto vazio |
| `Ctrl + O` / `Cmd + O` | Abrir projeto (`.aps`, `.ustx`, `.ust`, `.mid`) |
| `Ctrl + S` / `Cmd + S` | Salvar projeto ativo (`.aps`) |
| `Ctrl + Shift + S` / `Cmd + Shift + S` | Salvar projeto como novo arquivo |
| `Ctrl + E` / `Cmd + E` | Abrir diálogo de exportação de áudio (WAV / FLAC) |
| `Ctrl + ,` / `Cmd + ,` | Abrir diálogo de preferências e configurações |
| `Ctrl + Alt + P` / `Cmd + Alt + P` | Abrir janela do Pre-tunning (Afinador Orgânico / Auto-Pitch) |
| `Ctrl + Alt + T` / `Cmd + Alt + T` | Personalizar tema visual, cores de destaque e cantos da interface |
| `Ctrl + L` / `Cmd + L` | Exibir ou ocultar janela de log em tempo real do motor de áudio |
| `Tab` | Exibir ou ocultar gaveta inferior de parâmetros e expressões |
| `Alt + A` | Exibir ou ocultar painel de arranjo multifaixa |
| `Alt + O` | Exibir ou ocultar régua de fonemas e limites da OTO |
| `Ctrl + B` / `Cmd + B` | Exibir ou ocultar inspetor lateral direito |
| `F1` / `Cmd + ?` | Abrir guia interativo de teclas de atalho |
| `F11` | Alternar modo de tela cheia |

## Estrutura do código-fonte

```text
kamafeu/
├── assets/             # Ícones, fontes (Outfit), imagens e identidade visual
├── docs/               # Documentação técnica e guias de build/assinatura
├── resamplers/         # Binários de resamplers externos e scripts de integração
├── wavtools/           # Utilitários externos de junção e emenda de fatias (Wavtool)
├── tests/              # Suíte de testes unitários e de integração
└── src/
    ├── main.rs         # Ponto de entrada CLI, despachante de comandos e inicialização gráfica
    ├── lib.rs          # Exportação pública da biblioteca e módulos centrais
    ├── config.rs       # Preferências do usuário, caminhos de motores e persistência JSON
    ├── discord_rpc.rs  # Integração com Discord Rich Presence para status de edição
    ├── copaiba_bridge.rs # Ponte de integração entre o editor Kamafeu e o toolkit Copaiba
    ├── dialogs.rs      # Gerenciador unificado de caixas de diálogo e mensagens modais
    │
    ├── audio/          # Motor de áudio (Rodio/CoreAudio/ALSA), metrônomo e rack FX (EQ, Reverb, Comp)
    ├── bin/            # Executáveis autônomos (`kamafeu.rs` e `copaiba.rs`)
    ├── copaiba/        # Calibrador interativo de oto.ini, visualizador de onda e empacotador de voicebanks
    ├── drivers/        # Drivers de comunicação com resamplers (VENUS, straycat-rs, Wine) e wavtools (Andromeda, Yawu)
    ├── dsp/            # Processamento digital de sinais: VENUS, YIN pitch detection, envelopes UTAU e resample
    ├── formats/        # Parsers e conversores universais (.aps, .ustx, .ust, .mid, .ufdata, .svp, .vsqx, .kfv)
    ├── gui/            # Interface egui: piano roll, arranjo multifaixa, inspetor, régua de fonemas e diálogos
    ├── oto/            # Leitura/escrita de oto.ini (UTF-8/Shift-JIS), prefix.map e scanner de cantores
    ├── phonemizer/     # Motores fonéticos (Japonês CV/VCV/CVVC, Português BRAPA VCCV/G2P, Inglês VCCV)
    ├── project/        # Modelagem de dados: faixas, notas, curvas de pitch, envelopes e histórico (Undo/Redo)
    └── renderer/       # Pipeline multithread (Rayon), síntese paralela, alinhamento de fase e mixagem de faixas
```

## Licença

Distribuído sob os termos da [Licença MIT](LICENSE). Executáveis e ferramentas externas de terceiros mantêm suas respectivas licenças de distribuição (consulte [resamplers/README.md](resamplers/README.md)).

Desenvolvido pelo **Studio Pomar**.
