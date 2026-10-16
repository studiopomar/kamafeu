# Spike pareado SOLA/WORLD para KFV 2

`cargo run --bin kfv2-engine-spike -- <banco-ou-pasta.kfv> <unit-id> <midi> <duração-do-evento-ms> <pasta-de-saída>` renderiza cinco vezes cada backend com o mesmo `PhrasePlan`, sem normalização de loudness. Para casos multi-evento, passe um corpus JSON:

```sh
cargo run --bin kfv2-engine-spike -- <banco-ou-pasta.kfv> --corpus <corpus.json> <pasta-de-saída>
```

O formato está ilustrado em [`engine-spike-corpus.example.json`](engine-spike-corpus.example.json). Cada caso grava seu `phrase_plan.json`, `sola.wav`, `world.wav` e `report.json`; o diretório raiz recebe `corpus_report.json`. O report de cada caso inclui hashes de cada asset fonte e dos vetores de amostras `f32` little-endian antes da quantização WAV, traces por unidade, fallbacks/ajustes, duração, silêncio por janelas de 10 ms acima de -60 dBFS, pico, RMS, clipping, razão de janelas audíveis e latências individuais e mediana.

O campo de bytes estima somente o buffer mono de saída em `f32`; não mede memória de pico do processo. A latência é uma mediana de cinco chamadas sequenciais, sem controle de frequência da CPU, temperatura ou carga do sistema. Use-a para comparação exploratória no mesmo computador, não como benchmark entre máquinas.

O teste unitário do executável também monta uma fixture sintética com dois eventos, testa a execução em sequência e confere um trace para cada unidade. A fixture é empacotada como `.kfv`; a versão em pasta e a empacotada devem produzir o mesmo `PhrasePlan` e os mesmos hashes de saída nos dois backends. Isso valida carregamento e reprodução estrutural, sem representar material vocal real.

## Primeira execução local

Em 15/10/2026, o comando abaixo foi executado no rascunho local VIICTOR VCCV BRAPA. Os artefatos ficam em `target/kfv2-spike-viiictor/` e são regeneráveis:

```sh
cargo run --bin kfv2-engine-spike -- \
  "/Users/xiao/VIICTOR VCCV BRAPA KFV2 Draft" \
  legacy-000001 60 200 target/kfv2-spike-viiictor
```

Para essa unidade isolada (fone `a`, camada MIDI 60), a duração do evento pedida foi 200 ms, mas o planner produziu uma unidade de 350 ms para acomodar a âncora e o início de fonte; os dois WAVs têm os mesmos 350 ms e derivam do mesmo plano. A execução registrou:

| Métrica | SOLA | WORLD solicitado |
| --- | ---: | ---: |
| Backend efetivo | `kamafeu.sola` | `kamafeu.world-rs+kamafeu.resize-preserving-pitch` |
| Latência mediana (5 renders) | 14,01 ms | 38,77 ms |
| RMS | -23,95 dBFS | -24,05 dBFS |
| Pico | -13,88 dBFS | -12,65 dBFS |
| Janelas audíveis (> -60 dBFS) | 100% | 100% |
| Silêncio inicial/final medido | 0 / 0 ms | 0 / 0 ms |
| Amostras clipadas | 0 | 0 |

O caminho WORLD efetivo incluiu `resize-preserving-pitch`; não foi uma execução pura de WORLD. Essa única unidade e fixture não representam frases VCCV, inteligibilidade, continuidade, timbre ou preferência auditiva. As marcas do draft são inferidas e ainda não revisadas; a licença da origem não está definida. A execução é evidência técnica local, não valida autorização nem permite distribuição. Nenhum backend de referência foi escolhido por estes números.

### Smoke test multi-evento

O corpus [`viiictor-draft-vowels-smoke.json`](corpora/viiictor-draft-vowels-smoke.json) foi executado localmente no mesmo draft. Ele pede dois eventos V (`a` e `e`) sobrepostos em MIDI 60, e o planner selecionou `legacy-000001` e `legacy-000003`, com hashes dos dois WAVs registrados no relatório. Os WAVs SOLA/WORLD resultantes têm 490 ms. RMS: -23,07/-22,36 dBFS; pico: -13,95/-12,74 dBFS; zero amostras clipadas e 100% das janelas acima do limiar de audibilidade. Latência mediana: 25,71/88,27 ms. O WORLD passou por `resize-preserving-pitch` nas duas unidades. As duas unidades vêm de uma conversão sem revisão; este caso de duas vogais não cobre VCCV nem serve como evidência auditiva.

## Próxima coleta para fechar o spike

1. Revisar as unidades e marcas com a fonte e registrar as correções antes de compará-las.
2. Preparar corpus de pelo menos 20 frases curtas em material cuja autorização de uso esteja definida; preservar lista de eventos, pitch, duração e plano.
3. Gerar ambos os WAVs para cada plano, conferir os relatórios e exportar pares em loudness perceptivo comparável para escuta separada dos arquivos técnicos sem normalização.
4. Alternar a ordem e ocultar o backend na avaliação; coletar notas de inteligibilidade, continuidade e timbre, número de ouvintes e dispersão.
5. Só então decidir o backend de referência. Métricas e testes estruturais não substituem essa audição.
