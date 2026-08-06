# Kamafeu

<img width="2132" height="1270" alt="image" src="https://github.com/user-attachments/assets/533c5016-f811-486a-b8f1-479d2c51248e" />

Sintetizador de voz e piano roll escrito em Rust, focado no fluxo de trabalho clássico do UTAU.

> **Status:** versão `0.2.0-alpha.1`. O formato de projeto e o motor de síntese ainda podem receber mudanças incompatíveis antes da versão estável.

O objetivo do Kamafeu é resgatar a sensação de ajustar e construir cada detalhe da afinação e fonética de forma artesanal e manual, unindo essa experiência tradicional a um ambiente moderno, rápido e estável.

O projeto permite a edição de sequências de áudio, importação de voicebanks UTAU e renderização offline ou em tempo real.

> **Curiosidade**: O Kamafeu possui uma integração histórica com o software [Copaíba NEO](https://github.com/studiopomar/Copaiba-NEO/). Ao realizar prévias de fonemas durante a edição no Copaíba NEO, quem processa e entrega o áudio em tempo real por trás dos panos é o próprio motor do Kamafeu desde o início!

---

## Filosofia e Conceito

O Kamafeu prioriza o controle absoluto e artesanal do usuário sobre a interpretação vocal:

* **Afinação Manuscrita**: O produtor molda manualmente os pontos de transição de pitch, vibratos, portamentos e modulações de envelope nota a nota.
* **Transparência Concatenativa**: As amostras de voz originais são preservadas e manipuladas diretamente por algoritmos de pitch-shift e time-stretch (SOLA ou resamplers UTAU tradicionais).
* **Desempenho Moderno**: O core em Rust oferece inicialização instantânea, renderização multithreaded e interface de alta taxa de quadros (60+ FPS) sem travamentos.
* **Arquitetura Flexível Multifaixa**: Suporte ao painel de arranjo multifaixa (*Arrangement View*) com isolamento de faixa (Solo), silenciamento (Mute) e ajuste individual de volume (dB) e pan.

---

## Recursos e Arquitetura

### 1. Suporte a Voicebanks UTAU e Fonemizador Integrado
* **Formatos de Amostra**: Compatível com voicebanks CV (Consonante-Vogal), VCV (Vogal-Consonante-Vogal) e CVVC.
* **Fonemizador Automático (Romaji / Hiragana / VCV)**:
  * Conversão automática de entradas em Romaji (`ka`, `ki`, `ku`, `ke`, `ko`) para caracteres Kana/Hiragana (`か`, `き`, `く`, `け`, `こ`).
  * Tratamento de fonemas iniciais com aliases de início de frase, como `- V` ou `- CV` (ex: `- あ`, `- か`).
  * Resolução de consoantes duplas (Sokuon `っ`) e consoantes nasais (`n` / `ん`).
* **Leitura de `oto.ini`**: Suporte nativo a codificações de texto Shift-JIS (Windows Japão) e UTF-8.
* **Configuração de Parâmetros de Amostra**:
  * **Offset**: Ponto de início do corte da amostra WAV.
  * **Consonant (Fixed Region)**: Região fixa que não sofre estiramento de tempo.
  * **Cutoff**: Ponto final do corte da amostra (suporte a valores negativos para marcação a partir do final do arquivo).
  * **Preutterance**: Pré-pronúncia (tempo de avanço do ataque em relação ao início da nota).
  * **Overlap**: Sobreposição suave com a nota anterior.
* **Prefix Map (`prefix.map`)**: Mapeamento automático de sufixos e prefixos de tom (ex: C4, G4) com base na escala da nota.

### 2. Pipeline de Renderização Concatenativa
O processo de síntese vocal no Kamafeu segue um pipeline rigoroso de processamento de sinal:

```
[Sequência de Notas USTX/UST]
           │
           ▼
 [Fonemizador & OTO Parser] ──► Mapeia aliasing (ex: "- ka", "a ka") e parâmetros de tempo (Offset/Overlap)
           │
           ▼
[Filtro de Janela (Chunking)] ──► Isola apenas as notas necessárias para a janela de prévia (baixa latência)
           │
           ▼
[Resampling / Pitch Shift] ──► Altera a afinação e aplica Time-Stretch (Motor SOLA nativo ou macres CLI)
           │
           ▼
 [Interpolação de Envelopes] ──► Aplica envelopes de amplitude UTAU e crossfade suave entre fragmentos
           │
           ▼
  [Mixer & Audio Player] ──► Recompõe as faixas e envia os buffers PCM para a placa de som (CoreAudio/ALSA)
```

### 3. Motores de Áudio (Resamplers & Wavtools)
O Kamafeu suporta tanto motores nativos em Rust quanto binários do ecossistema UTAU tradicional e moderno.

#### Motor Nativo (SOLA / DSP Interno)
Implementação nativa do algoritmo **SOLA (Synchronized Overlap-Add)** para alteração de altura e duração sem alterar o timbre base:
* **TD-PSOLA (Time-Domain)**: Alinhamento de marcadores de pitch no domínio do tempo para resposta rápida.
* **Análise pYIN & GCI**: Extração precisa de frequência fundamental $F_0$ e alinhamento de épocas glotais (Glottal Closure Instants).
* **Preservação de Formantes (LPC)**: Filtragem inversa por Predição Linear (Levinson-Durbin) para evitar o efeito "esquilo" (*chipmunk*) em transposições agudas.
* **Algoritmo WSOLA**: Janelamento por correlação cruzada no domínio do tempo para consoantes e trechos não-sonoros (*Unvoiced*).
* **Aceleração SIMD**: Operações de janelamento e Overlap-Add otimizadas para extensões AVX2 (x86_64) e NEON (ARM64).

#### Integração com Ferramentas Externas
O Kamafeu permite alternar dinamicamente entre os motores na interface gráfica ou selecionar executáveis localizados nos diretórios `./resamplers` e `./wavtools`:

* **[macres](https://github.com/titinko/macres)**: Motor de resampling multiplataforma baseado em `libpyin` e `libgvps`. É utilizado pelo Kamafeu para realizar a transposição de afinação (pitch-shifting) e o estiramento temporal (time-stretching) das amostras do voicebank.
* **[Organum](https://github.com/KakouLabs/Organum)**: Resampler WORLD escrito em Rust, com processamento paralelo e cache de análise. O pacote do Kamafeu inclui o build CPU para macOS/Apple Silicon.
* **[straycat-rs](https://github.com/UtaUtaUtau/straycat-rs)**: Resampler WORLD em Rust com suporte a bancos VCV, CVVC e VCCV. O adaptador compensa automaticamente a semântica própria de duração do motor.
* **[World4UTAU](https://github.com/xrdavies/world4utau)**: Perfil de compatibilidade e detecção automática para o port de macOS/Linux. O executável pode ser instalado em `./resamplers/world4utau`.
* **[wavtool-yawu](https://github.com/m13253/wavtool-yawu)** (*Yet Another Wavtool for UTAU*): Ferramenta de concatenação áudio moderna que substitui o `wavtool.exe` tradicional. No Kamafeu, ele processa os fragmentos gerados pelo resampler, aplicando envelopes de atenuação, interpolação de volume e *crossfading* entre as notas com suporte a amostragem em 32-bit e 64-bit float.

Todos esses motores usam o contrato CLI clássico do UTAU. O Kamafeu envia BPM (`!tempo`), velocidade de consoante, flags, recorte e pitch bend; se o executável estiver ausente, falhar ou produzir um WAV vazio, a renderização continua pelo motor nativo TD-PSOLA.

### 4. Parâmetros Vocais e Modulações
Os parâmetros podem ser ajustados por nota ou globalmente na faixa:
* **Gênero (`g`)**: Deslocamento de formantes para alterar o timbre entre vozes mais graves ou agudas.
* **Soprosidade (`B`)**: Adição de ruído filtrado para simular voz sussurrada ou aspirada.
* **Velocidade de Consoante (`VEL`)**: Ajuste do tempo de estiramento do ataque consonantal.
* **Modulação (`MOD`)**: Percentual de variação de pitch em relação à afinação original da amostra.
* **Volume & Dinâmica (`VOL` / `DYN`)**: `VOL` controla o nível do fonema e `DYN` usa a convenção OpenUtau de décimos de dB (`-240..120`).
* **Ataque e Decaimento (`ATK` / `DEC`)**: Controle dos níveis do envelope fonético, inclusive nas sobreposições VCV/CVVC.
* **Vibrato completo**: Comprimento, período, profundidade, fade-in, fade-out, fase, drift e vínculo de volume, preservados em UST/USTX.

O wavtool interno resolve `preutterance` e `overlap` por fonema, limita ataques que invadiriam notas curtas e alinha a fase das regiões sobrepostas por correlação cruzada antes do crossfade. Isso evita o efeito truncado comum ao concatenar cada nota como um bloco independente.

### 5. Curvas de Pitch (Mode 2 Pitch Bends)
O sistema implementa o modelo de pitch bend Mode 2 do UTAU e do OpenUTAU:
* **Pontos de Controle**: Adição e movimentação livre de nós de controle sobre o Piano Roll.
* **Interpolação Hermite Spline / Sigmoide**: Transições contínuas de frequência sem descontinuidades de derivada.
* **Formatos de Curva**:
  * **S-Curve / Spline (`s` / `io`)**: Transições em formato sigmoide (*SinEasingInOut*).
  * **Ease-In (`i`)**: Curva suave de aceleração inicial (*SinEasingIn*).
  * **Ease-Out (`o`)**: Curva de desaceleração final (*SinEasingOut*).
  * **Linear (`l` / `r`)**: Interpolação direta em linha reta.
* **Codificação UTAU Base64**: Compatibilidade com strings de parâmetro `#10#` e codificação de 12 bits para comunicação com resamplers CLI.

---

## Formatos de Arquivo Suportados

| Formato | Extensão | Leitura | Escrita | Descrição |
| --- | --- | --- | --- | --- |
| OpenUTAU | `.ustx` | Sim | Sim | Projeto multifaixa com partes, expressões e pitch bends |
| UTAU Sequence | `.ust` | Sim | Sim | Sequência clássica de faixa única e parâmetros do UTAU |
| Standard MIDI | `.mid`, `.midi` | Sim | Sim | Importação e exportação multifaixa, incluindo notas sobrepostas |
| Kamafeu Score | `.json` | Sim | Sim | Estrutura de dados interna serializada |

---

## Requisitos e Compilação

### Pré-requisitos
* **Rust**: Versão 1.82 ou superior (`rustup default stable`).

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
git clone https://github.com/studiopomar/kamafeu.git
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
Estrutura da interface:
* **Painel Superior (Arrangement View)**: Gerenciamento multifaixa com criação (`➕ Nova Track`), remoção (`🗑 Excluir`), Mute, Solo, nome e sliders de volume (dB).
* **Painel Esquerdo**: Seleção de Voicebank, parâmetros de Vocal Mode (Gênero, Soprosidade) e paleta visual de fonemas Kana.
* **Área Central**: Piano Roll interativo com grade de notas, régua temporal de compassos e ferramenta de desenho de pitch em tempo real.
* **Painel Direito**: Configurações da nota selecionada (Envelope, Vibrato, Flags) e seletor de executáveis resampler/wavtool.
* **Barra de Transporte**: Controles de reprodução (Play, Pause, Stop), leitura de tempo (`00:00.000`), seleção de snap de grade (1/4, 1/8, 1/16, Livre) e exportação WAV.

### 2. Configurando Resamplers Externos
Caso deseje adicionar ou substituir um resampler externo:
1. Baixe o binário correspondente ao seu sistema operacional.
2. Coloque os arquivos nas pastas do projeto:
   - `./resamplers/macres`
   - `./resamplers/organum-resampler`
   - `./resamplers/straycat-rs`
   - `./resamplers/world4utau` (opcional)
   - `./wavtools/wavtool-yawu`
3. Certifique-se de conceder permissão de execução (no macOS/Linux):
   ```bash
   chmod +x ./resamplers/* ./wavtools/wavtool-yawu
   ```
4. Na interface do Kamafeu, acesse **Configurações do Motor** e selecione `macres`, `Organum`, `straycat-rs`, `World4UTAU` ou **Procurar Resampler...** para qualquer outro executável compatível.

### 3. Renderização em Linha de Comando (CLI)
É possível renderizar projetos `.ust`, `.ustx`, `.mid`, `.midi` ou `.json` diretamente para um WAV estéreo sem iniciar a interface gráfica:

```bash
cargo run --release -- render \
  --voicebank ./caminho/do/voicebank \
  --input projeto.ustx \
  --output resultado.wav \
  --sample-rate 44100
```

Parâmetros do comando `render`:
* `-v`, `--voicebank <PATH>`: Diretório raiz do voicebank UTAU contendo `oto.ini`.
* `-i`, `--input <PATH>`: Arquivo de entrada (`.ustx`, `.ust`, `.mid`, `.midi` ou `.json`).
* `-o`, `--output <PATH>`: Caminho do arquivo WAV de saída (Padrão: `output.wav`).
* `-s`, `--sample-rate <HZ>`: Taxa de amostragem em Hz (Padrão: `44100`).

### 4. Outros Comandos CLI

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
├── melhorias.md              # Especificação técnica do motor DSP TD-PSOLA
├── src/
│   ├── main.rs               # Parser de argumentos CLI e inicialização da GUI
│   ├── lib.rs                # Declaração dos módulos da biblioteca
│   ├── audio/                # Streaming de áudio em tempo real via rodio/cpal
│   ├── drivers/              # Drivers de execução para resamplers/wavtools CLI
│   ├── dsp/                  # Algoritmos SOLA, pYIN, LPC, pitch bends e filtros
│   ├── formats/              # Parsers para formatos UST, USTX e MIDI
│   ├── gui/                  # Interface gráfica (Piano Roll, Arrangement, Inspectors)
│   ├── oto/                  # Leitura de oto.ini, character.txt e prefix.map
│   ├── phonemizer/           # Mapeamento e conversão de fonemas (Romaji/Kana/VCV)
│   ├── project/              # Modelos de dados do projeto (UProject, UNote, UPitch)
│   └── renderer/             # Pipeline de renderização concatenativa multithreaded
├── resamplers/               # Binários de resamplers externos
└── wavtools/                 # Binários de wavtools externos
```

---

## Testes

Para rodar a suíte completa de testes automatizados do projeto:
```bash
cargo test
```

As mesmas verificações executadas pela integração contínua podem ser reproduzidas localmente:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --release --all-targets
```

## Plataformas de Release

Os releases automatizados incluem o Kamafeu Studio e o Copaíba Toolkit para Windows x64, macOS Intel, macOS Apple Silicon e Linux x64. Android ainda não é distribuído: um APK só será publicado quando houver um projeto Android completo, assinatura e teste de instalação, em vez de renomear uma biblioteca nativa como APK.

---

## Licença

Este projeto está licenciado sob a Licença **MIT**. Veja o arquivo `LICENSE` para mais detalhes.
