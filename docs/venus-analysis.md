# Análise por alias do VENUS

O VENUS cria um arquivo de análise ao lado de cada WAV usado pelo motor:

```text
ka.wav
ka.venus
```

O `.venus` é JSON legível, contém a versão e o hash da gravação e é refeito se
o WAV mudar. Ele guarda F0 por quadro, regiões vozeadas, RMS e marcas de pitch,
além dos ajustes manuais por alias.

Os controles que afetam a síntese atual são:

- `gain_db` (`-24` a `+24` dB): corrige volume do alias após a síntese.
- `formant_shift_cents` (`-2400` a `+2400`): corrige o envelope de formantes.
- `breathiness` (`0` a `100`): adiciona respiração somente a este alias.

Antes desses ajustes manuais, o VENUS mede o corpo sonoro com RMS em janelas de
20 ms, ignora silêncio e escolhe um único ganho limitado para todo o alias. O
ganho não acompanha o envelope durante a nota — isso evitaria oscilações e
"pumping" em loops longos — e o limiter de pico atua apenas depois dessa
normalização estável.

Os dados de F0, vozeamento e marcas de pitch já são persistidos para inspeção
no Copaiba NEO e serão a base da edição de análise avançada. Não edite o WAV
sem reanalisar: o hash impede que ajustes antigos sejam aplicados a outra
gravação.

## Utilitário

```bash
cargo run --bin venus-editor -- path/do/alias.wav analyze
cargo run --bin venus-editor -- path/do/alias.wav show
cargo run --bin venus-editor -- path/do/alias.wav set gain-db 2.5
cargo run --bin venus-editor -- path/do/alias.wav set formant-cents -120
cargo run --bin venus-editor -- path/do/alias.wav set breathiness 8
```

No Copaiba NEO, abra o alias e use a seção **Análise VENUS (.venus)** no painel
direito. Alterações são salvas imediatamente e o cache de renderização inclui o
`.venus`, portanto uma alteração não reutiliza áudio sintetizado com controles
antigos.

## Corpus de validação

Para medir um voicebank real em C3, C4 e C5 antes e depois de mudanças no
motor, gere um relatório JSON:

```bash
cargo run --bin venus-corpus -- /caminho/do/voicebank venus-corpus.json 40
```

O relatório contém F0 solicitado e medido, erro em cents, RMS, pico, maior
descontinuidade e descritores espectrais por alias/tom. O limite opcional torna
possível começar com um subconjunto representativo antes de analisar o banco
inteiro.
