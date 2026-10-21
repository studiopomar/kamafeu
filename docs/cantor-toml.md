# Bancos nativos `cantor.toml`

O formato nativo atual do Kamafeu é identificado por um arquivo `cantor.toml` na raiz do banco. O KAMUSE cria e edita esse arquivo, importa os áudios sem substituir os originais e gera pacotes `.kfv` com o TOML e os arquivos referenciados.

```text
cantor-exemplo/
├── cantor.toml
├── src/
│   └── kam-a.wav
└── assets/
    ├── avatar.png
    └── portrait.png
```

## Exemplo

```toml
versao = 1
id = "org.exemplo.cantor"
nome = "Cantor de Exemplo"
idiomas = ["pt-BR"]
engine = "tainah"
imagem = "assets/avatar.png"
portrait = "assets/portrait.png"

[ajustes]
ganho_db = 0.0
transicao_ms = 20.0

[[tons]]
id = "midi-60"
midi = 60

[[tons.gravacoes]]
id = "kam-a"
arquivo = "src/kam-a.wav"
fonemas = ["k", "a", "m", "a"]
marcacoes_ms = [0.0, 90.0, 210.0, 330.0, 500.0]
```

Cada gravação tem uma sequência alternada de consoantes e vogais e uma marcação em cada limite, incluindo início e fim. As marcações são crescentes, finitas e expressas em milissegundos dentro da duração do arquivo. Em `k a m a`, o Kamafeu deriva `ka` (CV), `am` (VC) e `ma` (CV), mantendo todas as unidades ligadas à mesma gravação de origem.

WAV e MP3 são aceitos. O arquivo importado permanece intacto; MP3 é decodificado para PCM em memória/área temporária durante a preparação para síntese. Os tons usam número MIDI de 0 a 127. `ganho_db` aceita -24 a +24 dB e `transicao_ms`, 0 a 500 ms. A imagem de galeria precisa medir 200×200 pixels. O portrait precisa ter pixels transparentes.

## Renderização e compatibilidade

Tainah WORLD é a única engine nativa disponível. Uma falha de análise, síntese ou cobertura retorna um diagnóstico; o Kamafeu não troca silenciosamente para SOLA. Controles de banco e segmentação pertencem ao TOML; preferências globais da engine pertencem às preferências do Kamafeu; andamento, notas e controles musicais pertencem ao projeto.

Em notas conectadas, a cobertura de uma transição VC permite inserir a unidade na fronteira. Sem VC, o plano mantém a junção direta de CV e emite um aviso. A seleção de gravação considera a camada de tom mais apropriada para cada nota.

`cantor.toml` identifica o formato nativo e `oto.ini` identifica o formato UTAU. A presença dos dois na mesma pasta é um conflito explícito. Um TOML inválido não é reinterpretado como UTAU. Bancos nativos JSON KFV 2 antigos não são carregados pelo fluxo atual. O formato UTAU continua no fluxo de `oto.ini`, inclusive para áudio MP3.

Os perfis de lista de gravação disponíveis no KAMUSE são `pt-BR`, `en-US` e `ja-JP`. A lista ajuda a planejar as tomadas; a cobertura efetiva depende dos fonemas e gravações incluídos no banco.
