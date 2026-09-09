<div align="center">

<img src="https://raw.githubusercontent.com/studiopomar/pomar-lts/main/public/studio-pomar-icon-4096.png" alt="Logo do Studio Pomar" width="120" height="120" />

# Kamafeu Studio

**Síntese vocal e piano roll em Rust, com controle sobre cada nota.**

[![Versão](https://img.shields.io/badge/vers%C3%A3o-1.0.0--A-d7ff3f?style=flat-square)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange?style=flat-square&logo=rust)](Cargo.toml)
[![Licença: MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-yellow.svg?style=flat-square)](LICENSE)
[![Studio Pomar](https://img.shields.io/badge/Studio-Pomar-brightgreen?style=flat-square)](https://studiopomar.github.io/pomar-lts/)

[Downloads](https://github.com/studiopomar/kamafeu/releases) · [Primeiros passos](#primeiros-passos) · [Compilação](#compilação) · [Histórico de alterações](CHANGELOG.md)

<img src="assets/kamafeu_banner.png" alt="Kamafeu Studio" width="1200" />

</div>

O **Kamafeu Studio** é um editor e sintetizador de canto voltado para o ecossistema UTAU e OpenUtau. Ele combina composição no piano roll, desenho de afinação, ajustes fonéticos e renderização de áudio, permitindo trabalhar tanto com motores nativos em Rust quanto com resamplers e wavtools externos.

> **A origem do nome e a proposta do projeto:** O nome **Kamafeu** vem da joia tradicional em camafeu, uma peça esculpida pacientemente à mão, em relevo, camada por camada. O foco do software não é o uso de redes neurais ou motores gerados por inteligência artificial (como DiffSinger ou modelos de Deep Learning). Em vez disso, o Kamafeu foi concebido para o trabalho detalhado e artesanal de gravar, calibrar (*oto.ini*) e afinar bancos de voz concatenativos, oferecendo ao usuário controle total e direto sobre cada nota, transição, envelope e nuance acústica da música.

> **Em desenvolvimento:** O formato de projeto e o motor de síntese continuam evoluindo antes da versão estável. Consulte o [changelog](CHANGELOG.md) para acompanhar as novidades e correções.

## Navegação

- [Recursos](#recursos)
- [Filosofia e arquitetura](#filosofia-e-arquitetura)
- [Primeiros passos](#primeiros-passos)
- [Compilação](#compilação)
- [Linha de comando](#linha-de-comando)
- [Formatos suportados](#formatos-suportados)
- [Copaiba Voicebank Toolkit](#copaiba-voicebank-toolkit)
- [Motores de síntese](#motores-de-síntese)
- [Pipeline de renderização](#pipeline-de-renderização)
- [Atalhos de teclado](#atalhos-de-teclado)
- [Desenvolvimento e contribuições](#desenvolvimento-e-contribuições)
- [Glossário](#glossário)
- [Licença](#licença)

## Recursos

### Composição e edição vocal

- **Piano roll completo:** inserção, divisão, redimensionamento e movimentação de notas, com seleção múltipla e histórico de desfazer/refazer que grava ações contínuas por inteiro.
- **Edição de afinação:** desenho livre de pitch, suavização de curvas, transições lineares ou Bézier, portamento entre notas vizinhas e vibrato com ajuste de profundidade, período, fase e fades de entrada/saída.
- **Expressão por nota:** controle de volume, ataque, decaimento, velocidade de consoante, modulação, sopro (*breathiness*), gênero (*gender*), envelopes UTAU de 5 pontos e crossfades manuais.
- **Diagnóstico de fonemas e oto.ini:** painel lateral e tooltips na régua que mostram o alias solicitado vs. o alias mapeado no `oto.ini`, arquivo WAV utilizado, conferência de existência no disco e tempos calculados de preutterance e overlap.
- **Arranjo e FX Rack:** suporte a faixas vocais e faixas de áudio com solo, mute, ganho e pan, além de rack de efeitos integrado (equalizador gráfico de 31 bandas, chorus, compressor, delay e reverb) e prévia da forma de onda.
- **Macros de edição:** diálogo para inserir e dividir letras automaticamente (por espaços ou hífens), humanização de tempo, afinação e dinâmica, e atalhos para presets de vibrato.

### Bancos de voz e suporte fonético

- **Bancos UTAU e OpenUtau:** leitura de arquivos `oto.ini` em UTF-8 e Shift-JIS, além de mapeamento multitom por `prefix.map`.
- **Galeria de cantores:** busca rápida, avatares e detecção de pastas de voicebanks do sistema e do OpenUtau.
- **Fonemizadores integrados:** suporte para japonês (CV, VCV, CVVC com conversão Romaji/Kana), português (BRAPA/CVVC) e inglês (VCCV completo com preservação de encontros consonantais).
- **Régua de fonemas:** visualização gráfica das transições em X, facilitando o ajuste de preutterance e overlap diretamente na linha do tempo.
- **Pacotes `.kfv`:** suporte ao formato empacotado de cantores, gerado e editado pelo Copaiba Toolkit.

## Filosofia e arquitetura

O Kamafeu preserva o caráter artesanal da síntese concatenativa: em vez de delegar a voz a modelos automáticos de Deep Learning, o software valoriza a construção meticulosa do canto a partir de gravações reais.

Assim como esculpir uma joia camafeu, moldar a interpretação vocal é um processo detalhado: a letra digitada resolve os fonemas, o banco de voz fornece as amostras com seus limites de temporização (`oto.ini`) e o afinador ajusta a curva de cada passagem no piano roll.

```text
Projeto (.aps, USTX, UST, MIDI e outros)
        │
        ├── Faixas e partes vocais
        │       └── Notas, letras, pitch, expressões e FX Rack
        │
        ├── Voicebank
        │       └── oto.ini, aliases, prefix.map e arquivos WAV
        │
        └── Renderizador
                ├── Fonemizador e temporização unificada
                ├── Resampler (nativo em Rust ou externo via UTAU CLI/Wine)
                ├── Wavtool, alinhamento de fase e envelopes
                └── Mixagem com efeitos e saída de áudio
```

O núcleo é desenvolvido em Rust, aproveitando paralelismo com Rayon para renderização e cache de amostras em memória para evitar leituras repetidas de disco. A mesma base de cálculo é utilizada tanto na prévia durante a reprodução quanto na exportação final em WAV.

## Primeiros passos

Baixe um instalador na página de [Releases](https://github.com/studiopomar/kamafeu/releases) ou [compile o código-fonte](#compilação). O software roda em Windows, macOS e Linux.

1. **Abra o Kamafeu Studio:** ao iniciar sem argumentos, a interface gráfica abre com um projeto padrão.
2. **Escolha um cantor:** na aba de voicebank, abra a galeria ou aponte para a pasta de um banco de voz.
3. **Crie ou importe notas:** use o lápis (`N`) para desenhar notas ou abra arquivos `.ustx`, `.ust`, `.mid` ou `.aps` pelo menu **Arquivo**.
4. **Ajuste letra e afinação:** edite as sílabas, selecione o fonemizador apropriado e use a ferramenta de pitch (`P`) para modelar a afinação.
5. **Ouça e refine:** aperte `Espaço` para reproduzir. A forma de onda renderizada aparece no fundo do piano roll para orientar os ajustes.
6. **Exporte:** salve o projeto em `.aps` ou exporte o áudio final em WAV, FLAC ou PCM.

Se nenhum banco de voz estiver selecionado, o motor nativo gera tons sintéticos para permitir a composição melódica imediata.

## Compilação

É necessário ter o Rust (versão 1.82 ou superior) e o Cargo instalados.

### Dependências no Linux

Em distribuições baseadas em Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

### Compilar e rodar

```bash
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release --bins
```

Para abrir o editor:

```bash
cargo run --release --bin kamafeu
```

Para abrir o editor de voicebanks (Copaiba):

```bash
cargo run --release --bin copaiba
```

## Linha de comando

O executável também funciona via terminal para tarefas de renderização e checagem:

```bash
# Ver opções disponíveis
cargo run --release --bin kamafeu -- --help

# Inspecionar informações de um voicebank
cargo run --release --bin kamafeu -- voicebank-info "/caminho/do/voicebank"

# Renderizar um projeto diretamente para WAV
cargo run --release --bin kamafeu -- render \
  --voicebank "/caminho/do/voicebank" \
  --input "musica.aps" \
  --output "musica.wav" \
  --sample-rate 44100
```

## Formatos suportados

| Formato | Extensão | Importação | Exportação | Descrição |
| --- | --- | :---: | :---: | --- |
| Arquivo Projeto Saturno | `.aps` | Sim | Sim | Formato nativo do Kamafeu Studio |
| OpenUtau | `.ustx` | Sim | Sim | Projetos, faixas e curvas do OpenUtau |
| UTAU Sequence | `.ust` | Sim | Sim | Projetos clássicos do UTAU |
| Standard MIDI | `.mid`, `.midi` | Sim | Sim | Sequências MIDI padrão |
| UtaFormatix Data | `.ufdata` | Sim | Sim | Intercâmbio universal de canto |
| Synthesizer V | `.svp` | Sim | Sim | Conversão de dados de notas |
| VOCALOID | `.vsqx` | Sim | Sim | Conversão de sequências Vocaloid |
| Kamafeu Voicebank | `.kfv` | Sim | Pelo Copaiba | Pacote de cantor compactado |

## Copaiba Voicebank Toolkit

O **Copaiba** é a ferramenta integrada para calibrar e organizar bancos de voz. Ele pode ser aberto diretamente a partir de um fonema no Kamafeu ou como aplicativo independente (`copaiba`).

| Parâmetro | Função |
| --- | --- |
| **Offset** | Início útil da amostra, descartando silêncio ou ruídos de ataque. |
| **Consonant** | Trecho fixo da consoante que não é esticado durante a mudança de tempo. |
| **Cutoff** | Limite final da amostra (valores positivos cortam do fim; negativos definem comprimento a partir do offset). |
| **Preutterance** | Quanto o ataque do fonema antecede a posição musical da nota. |
| **Overlap** | Região de transição e fusão com o fonema anterior. |

## Motores de síntese

### Motor nativo (Rust)

O motor padrão do Kamafeu utiliza processamento em Rust:
- Análise de pitch com YIN e marcas de período coerentes.
- Trechos periódicos sintetizados com TD-PSOLA e trechos inarmônicos (consoantes e ruídos) via WSOLA.
- Alinhamento de fase em regiões vocálicas para reduzir cancelamentos e oscilações de volume durante crossfades.

### Motores externos (UTAU CLI)

O Kamafeu suporta resamplers e wavtools externos seguindo a convenção de linha de comando do UTAU:
- No macOS e Linux, executáveis Windows (`.exe`) são chamados automaticamente via Wine quando presentes.
- O caminho de executáveis pode ser configurado na aba de motor ou colocado nas pastas `resamplers/` e `wavtools/`.
- Suporte a `wavtool-yawu` com concatenação contínua por frase.

## Atalhos de teclado

No macOS, utilize `Cmd` no lugar de `Ctrl`.

### Ferramentas do Piano Roll

| Tecla | Ferramenta |
| --- | --- |
| `V` ou `1` | Seleção e movimentação de notas |
| `N` ou `2` | Lápis para desenhar notas |
| `P` ou `3` | Pincel de curvas de pitch |
| `Shift + P` | Alternar modo do pincel de pitch |
| `C` ou `4` | Cortar / dividir nota |
| `E` ou `5` | Borracha para apagar notas |

### Transporte e edição geral

| Atalho | Ação |
| --- | --- |
| `Espaço` | Reproduzir / Pausar |
| `Ctrl + N` / `Ctrl + O` / `Ctrl + S` | Novo / Abrir / Salvar projeto |
| `Ctrl + Shift + S` | Salvar como |
| `Ctrl + E` | Exportar áudio |
| `Ctrl + Z` / `Ctrl + Shift + Z` | Desfazer / Refazer |
| `Ctrl + C` / `X` / `V` / `D` | Copiar / Recortar / Colar / Duplicar |
| `Ctrl + A` / `Ctrl + Shift + A` | Selecionar tudo / Limpar seleção |
| `Delete` / `Backspace` | Excluir notas selecionadas |
| `↑` / `↓` | Transpor semitom |
| `Shift + ↑` / `Shift + ↓` | Transpor oitava |
| `←` / `→` | Mover notas no tempo (passos de 50 ms) |
| `Shift + ←` / `Shift + →` | Alterar duração das notas (passos de 50 ms) |
| `Ctrl + =` / `Ctrl + -` / `Ctrl + 0` | Zoom horizontal (+ / - / reset) |
| `F1` | Ajuda e documentação |

## Desenvolvimento

### Discord Rich Presence

O Kamafeu pode exibir o projeto ativo, faixa, cantor e status de reprodução no Discord. Os assets visuais necessários estão documentados em [assets/discord](assets/discord/README.md).

### Estrutura do projeto

```text
src/
├── audio/        # Reprodução e rack de efeitos (FX)
├── bin/          # Executável do Copaiba
├── copaiba/      # Edição e empacotamento de voicebanks
├── drivers/      # Drivers de resampler e wavtool
├── dsp/          # Processamento de sinal, pitch e envelopes
├── formats/      # Importação e exportação de projetos
├── gui/          # Interface gráfica egui, piano roll e timeline
├── oto/          # Voicebanks, oto.ini e prefix.map
├── phonemizer/   # Conversão de letras em fonemas (JP/EN/PT)
├── project/      # Estrutura de dados do projeto e notas
└── renderer/     # Renderização de áudio, mixagem e exportação
```

### Testes e qualidade

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Licença

Distribuído sob a [licença MIT](LICENSE). Resamplers e ferramentas externas de terceiros mantêm suas respectivas licenças (consulte [resamplers/README.md](resamplers/README.md)).

Desenvolvido pelo **Studio Pomar**.
