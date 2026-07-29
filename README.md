# Kamafeu

Sintetizador de voz e piano roll escrito em Rust, focado no fluxo de trabalho clássico do UTAU.

O objetivo do Kamafeu é resgatar a sensação de ajustar e construir cada detalhe da afinação e fonética de forma artesanal e manual, unindo essa experiência tradicional a um ambiente moderno, rápido e estável.

O projeto permite a edição de sequências de áudio, importação de voicebanks UTAU e renderização offline ou em tempo real.

> **Curiosidade**: O Kamafeu possui uma integração histórica com o software [Copaíba NEO](https://github.com/studiopomar/Copaiba-NEO/). Ao realizar prévias de fonemas durante a edição no Copaíba NEO, quem processa e entrega o áudio em tempo real por trás dos panos é o próprio motor do Kamafeu desde o início!

---

## Filosofia e Conceito

O Kamafeu prioriza o controle absoluto e artesanal do usuário sobre a interpretação vocal:

* **Afinação Manuscrita**: O produtor molda manualmente os pontos de transição de pitch, vibratos, portamentos e modulações de envelope nota a nota.
* **Transparência Concatenativa**: As amostras de voz originais são preservadas e manipuladas diretamente por algoritmos de pitch-shift e time-stretch (SOLA ou resamplers UTAU tradicionais).
* **Desempenho Moderno**: O core em Rust oferece inicialização instantânea, renderização multithreaded e interface de alta taxa de quadros (60+ FPS) sem travamentos.

---

## Recursos e Arquitetura

### 1. Suporte a Voicebanks UTAU
* **Formatos de Amostra**: Compatível com voicebanks CV (Consonante-Vogal) e VCV (Vogal-Consonante-Vogal).
* **Leitura de `oto.ini`**: Suporte nativo a codificações de texto Shift-JIS (Windows Japão) e UTF-8.
* **Configuração de Parâmetros de Amostra**:
  * **Offset**: Ponto de início do corte da amostra WAV.
  * **Consonant (Fixed Region)**: Região fixa que não sofre estiramento de tempo.
  * **Cutoff**: Ponto final do corte da amostra.
  * **Preutterance**: Pré-pronúncia (tempo de avanço do ataque em relação ao início da nota).
  * **Overlap**: Sobreposição suave com a nota anterior.
* **Prefix Map (`prefix.map`)**: Mapeamento automático de sufixos de tom (ex: C4, G4) com base na altura da nota selecionada.

### 2. Processamento de Áudio e Motores Externos (Resamplers & Wavtools)
O Kamafeu suporta tanto motores nativos em Rust quanto binários do ecossistema UTAU tradicional e moderno.

#### Motor Nativo (SOLA)
Implementação nativa do algoritmo **SOLA (Synchronized Overlap-Add)** para alteração de altura e duração sem alterar o timbre base:
* **TD-PSOLA (Time-Domain)**: Alinhamento de marcadores de pitch no domínio do tempo para resposta rápida.
* **FD-PSOLA (Frequency-Domain)**: Análise espectral via FFT com preservação de formantes.
* **WSOLA (Waveform Similarity)**: Janelamento por autocorrelação cruzada para consoantes e transições complexas.
* **LP-PSOLA (Linear Predictive Coding)**: Separação de filtro de trato vocal e excitação glótica.

#### Integração com Ferramentas Externas
O Kamafeu permite alternar dinamicamente entre os motores na interface gráfica ou selecionar executáveis localizados nos diretórios `./resamplers` e `./wavtools`:

* **[macres](https://github.com/titinko/macres)**: Motor de resampling multiplataforma baseado em `libpyin` e `libgvps`. É utilizado pelo Kamafeu para realizar a transposição de afinação (pitch-shifting) e o estiramento temporal (time-stretching) das amostras do voicebank.
* **[wavtool-yawu](https://github.com/m13253/wavtool-yawu)** (*Yet Another Wavtool for UTAU*): Ferramenta de concatenação áudio moderna que substitui o `wavtool.exe` tradicional. No Kamafeu, ele processa os fragmentos gerados pelo resampler, aplicando envelopes de atenuação, interpolação de volume e *crossfading* entre as notas com suporte a amostragem em 32-bit e 64-bit float.

### 3. Parâmetros Vocais em Tempo Real
Os parâmetros podem ser ajustados por nota ou globalmente na faixa:
* **Gênero (`g`)**: Deslocamento de formantes para alterar o timbre entre vozes mais graves ou agudas.
* **Soprosidade (`B`)**: Adição de ruído filtrado para simular voz sussurrada ou aspirada.
* **Tensão (`t`)**: Compressão da forma de onda para alterar a intensidade do ataque.
* **Brilho (`b`)**: Equalização de altas frequências no espectro vocal.

### 4. Curvas de Pitch (Mode 2 Pitch Bends)
O sistema implementa o modelo de pitch bend Mode 2 do UTAU:
* **Pontos de Controle (P)**: Adição e movimentação de nós de controle ao longo da nota.
* **Tipos de Interpolação**:
  * **S-Curve (S)**: Transições suaves em formato sigmoide.
  * **Linear (L)**: Interpolação direta em linha reta.
  * **R-Curve (R)**: Curva parabólica de atenuação rápida.
  * **J-Curve (J)**: Curva de ataque acentuado.
* **Codificação UTAU**: Compatibilidade com as strings de parâmetro `PBS`, `PBW`, `PBY` e `PBM`.

---

## Formatos de Arquivo Suportados

| Formato | Extensão | Leitura | Escrita | Descrição |
| --- | --- | --- | --- | --- |
| OpenUTAU | `.ustx` | Sim | Sim | Formato de projeto moderno multifaixa |
| UTAU Sequence | `.ust` | Sim | Sim | Sequência clássica de notas e parâmetros do UTAU |
| Standard MIDI | `.mid`, `.midi` | Sim | Não | Importação de faixas de notas e tempos |
| Kamafeu Score | `.json` | Sim | Sim | Estrutura de dados interna serializada |

---

## Requisitos e Compilação

### Pré-requisitos
* **Rust**: Versão 1.75 ou superior (`rustup default stable`).

### Dependências de Sistema

#### Linux (Ubuntu/Debian/Fedora)
```bash
sudo apt update
sudo apt install build-essential libasound2-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

#### macOS
Não são necessárias dependências extras de sistema. O backend de áudio utiliza a CoreAudio nativa.

#### Windows
Certifique-se de ter as ferramentas de compilação C++ do MSVC instaladas via Visual Studio Installer.

### Compilando a Aplicação
Clone o repositório e compile a versão otimizada:
```bash
git clone https://github.com/dorayakito/kamafeu.git
cd kamafeu
cargo build --release
```
O executável compilado estará localizado em `target/release/kamafeu`.

---

## Guia de Uso

### 1. Interface Gráfica (Studio)
Para abrir o estúdio gráfico interativo:
```bash
cargo run --release -- gui
```
Na interface:
* **Painel Esquerdo**: Seleção de Voicebank, parâmetros de Vocal Mode e inspeção da faixa.
* **Área Central**: Piano Roll interativo e visualização de curvas de pitch.
* **Painel Direito**: Configurações avançadas da nota e seleção de motores resampler/wavtool.
* **Barra Superior**: Controles de reprodução (Play, Pause, Stop), ajuste de BPM e tempo.

### 2. Renderização em Linha de Comando (CLI)
É possível renderizar projetos `.ust`, `.ustx` ou `.json` diretamente para um arquivo WAV sem iniciar a interface gráfica:

```bash
cargo run --release -- render \
  --voicebank ./caminho/do/voicebank \
  --input projeto.ustx \
  --output resultado.wav \
  --sample-rate 44100
```

Parâmetros do comando `render`:
* `-v`, `--voicebank <PATH>`: Diretorio raiz do voicebank UTAU contendo `oto.ini`.
* `-i`, `--input <PATH>`: Arquivo de entrada (`.ustx`, `.ust` ou `.json`).
* `-o`, `--output <PATH>`: Caminho do arquivo WAV de saída (Padrão: `output.wav`).
* `-s`, `--sample-rate <HZ>`: Taxa de amostragem em Hz (Padrão: `44100`).

### 3. Outros Comandos CLI

#### Inspeção de Voicebank
Exibe o nome, autor, quantidade de entradas no `oto.ini` e amostra de aliases registrados:
```bash
cargo run --release -- voicebank-info ./caminho/do/voicebank
```

#### Geração de Voicebank Sintético para Testes
Cria um diretório de voicebank de teste com formas de onda senoidais e arquivo `oto.ini` estruturado:
```bash
cargo run --release -- gen-sample ./meu_voicebank_teste
```

---

## Atalhos de Teclado e Navegação

### Ferramentas do Piano Roll
| Atalho | Ferramenta | Descrição |
| --- | --- | --- |
| `V` | Ponteiro (Pointer) | Seleção, movimentação e redimensionamento de notas |
| `N` | Lápis (Pencil) | Inserção de novas notas na grade |
| `P` | Desenho de Pitch (Pitch Draw) | Edição direta da curva de afinação sobre as notas |
| `E` | Borracha (Eraser) | Remoção rápida de notas |

### Reprodução e Edição
| Atalho | Ação |
| --- | --- |
| `Espaço` | Iniciar ou pausar a reprodução de áudio |
| `Ctrl + Z` / `Cmd + Z` | Desfazer a última ação |
| `Ctrl + Y` / `Cmd + Shift + Z` | Refazer a ação desfeita |
| `Ctrl + C` / `Cmd + C` | Copiar notas selecionadas |
| `Ctrl + X` / `Cmd + X` | Recortar notas selecionadas |
| `Ctrl + V` / `Cmd + V` | Colar notas no ponto atual do cursor |
| `Delete` / `Backspace` | Apagar notas selecionadas |

### Controles de Navegação por Mouse
* **Scroll Vertical**: Desloca o piano roll para cima/baixo (notas agudas/graves).
* **Shift + Scroll**: Desloca a linha do tempo para a esquerda/direita.
* **Ctrl + Scroll** / **Cmd + Scroll**: Zoom horizontal no tempo.
* **Arrastar com Botão Central**: Pan livre pela área de trabalho.

---

## Estrutura do Código Fonte

```
kamafeu/
├── Cargo.toml                # Configuração do pacote e dependências Rust
├── src/
│   ├── main.rs               # Parser de argumentos CLI e inicialização da GUI
│   ├── lib.rs                # Declaração dos módulos da biblioteca
│   ├── audio/                # Streaming de áudio em tempo real via rodio
│   ├── drivers/              # Drivers de execução para resamplers/wavtools CLI
│   ├── dsp/                  # Algoritmos SOLA, envelopes, pitch bends e filtros
│   ├── formats/              # Parsers para formatos UST, USTX e MIDI
│   ├── gui/                  # Interface gráfica (Piano Roll, Toolbar, Inspectors)
│   ├── oto/                  # Leitura de oto.ini, character.txt e prefix.map
│   ├── phonemizer/           # Mapeamento e conversão de fonemas (Romaji/Kana/VCV)
│   ├── project/              # Modelos de dados do projeto (UProject, UNote, UPitch)
│   └── renderer/             # Pipeline de renderização concatenativa multithreaded
├── resamplers/               # Binários de resamplers externos
└── wavtools/                 # Binários de wavtools externos
```

---

## Testes

Para rodar a suíte de testes unitários do projeto:
```bash
cargo test
```

---

## Licença

Este projeto está licenciado sob a Licença **MIT**. Veja o arquivo `LICENSE` para mais detalhes.
