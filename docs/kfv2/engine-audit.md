# Audit de engines para KFV 2

Snapshot de código auditado em 17/10/2026. Este documento descreve os caminhos
implementados no checkout; não é uma comparação de qualidade sonora.

## Identidades e rotas

| Seleção/uso | Identidade estável | Caminho executado | Estado |
| --- | --- | --- | --- |
| `Tainah` no desktop | `kamafeu.lessampler-tainah` | `NativeWorldResamplerDriver` chama a ABI C++ WORLD do Tainah; ataque, sustentação e cauda são definidos pelas marcas de `oto.ini` no mapa temporal dos frames. Falhas de carregamento, análise, cobertura ou síntese retornam erro. | O nome visível e os aliases salvos continuam aceitos. A superioridade auditiva sobre straycat-rs ainda depende da comparação pareada com voicebank autorizado. |
| Aliases antigos `WORLD (Nativo)`, `WORLD (Native)` ou `VENUS` | `kamafeu.lessampler-tainah` | `create_resampler_driver` reconhece os rótulos salvos e instancia `NativeWorldResamplerDriver`. | Compatibilidade de configuração; esses aliases não indicam outro motor. |
| Driver SOLA clássico | `kamafeu.sola` | `NativeResamplerDriver` chama `SolaResampler` diretamente. | Implementado, mas não escolhido como referência de produto. |
| Preview padrão e exportação KFV 2 | `kamafeu.sola` | `render_phrase_sola` chama SOLA explicitamente e não troca para WORLD. | Protótipo experimental; backend de referência pendente. |
| Preview comparativo do Copaíba | SOLA ou WORLD solicitado | O operador escolhe um backend para a mesma unidade/frase; WORLD passa pelo driver nativo. Erros de WORLD interrompem o preview em vez de mudar de backend. | Ferramenta de spike experimental; não altera exportação nem seleciona backend de produto. |
| Andromeda | `kamafeu.andromeda` | `NativeWavtoolDriver` aplica o envelope UTAU com curva cosseno. | Wavtool alternativo, sem alinhamento de fase. |
| Galapagos | `kamafeu.galapagos` | `GalapagosWavtoolDriver` usa curva linear de compatibilidade OpenUtau e estratégia `OpenUtauConvergence`. | Caminho nativo de alinhamento/mixagem; não é resampler. |

Os perfis externos conhecidos também têm IDs separados dos rótulos, sob o
namespace `external.resampler.*`; caminhos de executável personalizados sem um
perfil reconhecido permanecem `external.resampler.unknown`, com nome e comando
preservados nos demais campos do diagnóstico.

As rotas principais estão em `src/gui/engine_setup.rs`,
`src/drivers/resampler_driver.rs`, `src/drivers/tainah_ffi.rs`,
`src/drivers/wavtool_driver.rs` e `src/voicebank/render.rs`. Os diagnósticos por
fonema guardam a identidade estável do driver selecionado, o backend efetivo,
os estágios que contribuíram para as amostras, os motivos de fallback e os
ajustes aplicados, além da identidade do wavtool e dos rótulos apresentados.
Os mesmos dados ficam associados à entrada de cache em RAM e ao sidecar JSON do
cache persistente; um WAV sem metadados correspondentes é rejeitado e refeito.

## Tratamento de falhas WORLD

O Tainah devolve um código e uma mensagem para entrada inválida, região
sem cobertura, cancelamento, falha WORLD ou saída silenciosa. O driver Rust
rejeita também duração, taxa e cobertura incompatíveis; não substitui o
resultado por SOLA nem por resize.

Tainah mapeia os frames WORLD mantendo ataque e cauda e esticando o intervalo de
sustentação entre as marcas de loop de `oto.ini`. SOLA permanece como driver
separado e explícito para as rotas que a selecionam.

Os critérios externos são: pelo menos 95% do comprimento esperado, nenhuma
amostra não finita, pico acima de `1e-5`, pelo menos 75% das janelas de 50 ms
audíveis e janela final audível. São guardas de cobertura técnica, não um
indicador de qualidade perceptiva. A rota clássica mantém seu driver SOLA
separado para compatibilidade; selecionar Tainah nunca muda silenciosamente o
backend.

## Decisão do spike

**Nenhum backend foi selecionado como referência.** Os testes estruturais
demonstram contratos e saídas finitas/timadas com fixtures. A ferramenta
[`kfv2-engine-spike`](engine-spike.md) produziu um par técnico da mesma unidade
migrada VIICTOR com o mesmo `PhrasePlan`, mas esse render pertence ao código
anterior à política WORLD-only e registrou `resize-preserving-pitch`. Ele não
valida o comportamento atual e precisa ser refeito antes de qualquer comparação.
As marcas da migração não foram revisadas e a licença não está definida. Isso não
compara um corpus de gravações autorizadas nem frases representativas com
audição controlada. A antiga descrição de
VENUS como síntese formântica/YIN/fase mínima foi removida porque não descreve
nenhum dos caminhos encontrados.

Para fechar o spike, usar um corpus de gravações e marcações revisadas nas duas
rotas, além de renderizações de frase com a mesma pronúncia, curva de pitch,
duração e ganho percebido. Registrar duração, silêncio inicial/final, pico, RMS,
clipping, energia audível e latência. O utilitário atual estima apenas o tamanho
do vetor de saída; memória de pico do processo ainda precisa ser medida fora
dele. Preservar WAVs, parâmetros e IDs dos caminhos; e fazer escuta cega para
inteligibilidade, continuidade e timbre.
O banco VIICTOR convertido é um rascunho sem licença definida e com pitch de
camada não estimado; ele não sustenta uma alegação pública nem deve ser usado
como banco piloto distribuível até suas permissões e créditos serem definidos.
