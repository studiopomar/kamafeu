# Audit de engines para KFV 2

Snapshot de código auditado em 15/10/2026. Este documento descreve os caminhos
implementados no checkout; não é uma comparação de qualidade sonora.

## Identidades e rotas

| Seleção/uso | Identidade estável | Caminho executado | Estado |
| --- | --- | --- | --- |
| `Tainah` no desktop | `kamafeu.world-rs` | `NativeWorldResamplerDriver` chama `WorldResampler::render_sample_with_oto_traced`; ataques, sustentações e caudas OTO orientam o mapa temporal dos frames WORLD. Casos sem análise útil podem usar SOLA ou resize preservando pitch. | O nome visível foi atualizado; o ID e aliases salvos foram preservados. A superioridade auditiva sobre straycat-rs ainda depende da comparação pareada com voicebank autorizado. |
| Aliases antigos `WORLD (Nativo)`, `WORLD (Native)` ou `VENUS` | `kamafeu.world-rs` | `create_resampler_driver` reconhece os rótulos salvos e instancia `NativeWorldResamplerDriver`. | Compatibilidade de configuração; esses aliases não indicam outro motor. |
| Driver SOLA clássico | `kamafeu.sola` | `NativeResamplerDriver` chama `SolaResampler` diretamente. | Implementado, mas não escolhido como referência de produto. |
| Preview padrão e exportação KFV 2 | `kamafeu.sola` | `render_phrase_sola` chama SOLA explicitamente e não troca para WORLD. | Protótipo experimental; backend de referência pendente. |
| Preview comparativo do Copaíba | SOLA ou WORLD solicitado | O operador escolhe um backend para a mesma unidade/frase; WORLD passa pelo driver nativo, e o backend efetivo/fallback é reportado por unidade. | Ferramenta de spike experimental; não altera exportação nem seleciona backend de produto. |
| Andromeda | `kamafeu.andromeda` | `NativeWavtoolDriver` aplica o envelope UTAU com curva cosseno. | Wavtool alternativo, sem alinhamento de fase. |
| Galapagos | `kamafeu.galapagos` | `GalapagosWavtoolDriver` usa curva linear de compatibilidade OpenUtau e estratégia `OpenUtauConvergence`. | Caminho nativo de alinhamento/mixagem; não é resampler. |

Os perfis externos conhecidos também têm IDs separados dos rótulos, sob o
namespace `external.resampler.*`; caminhos de executável personalizados sem um
perfil reconhecido permanecem `external.resampler.unknown`, com nome e comando
preservados nos demais campos do diagnóstico.

As rotas principais estão em `src/gui/engine_setup.rs`,
`src/drivers/resampler_driver.rs`, `src/dsp/world_resampler.rs`,
`src/drivers/wavtool_driver.rs` e `src/voicebank/render.rs`. Os diagnósticos por
fonema guardam a identidade estável do driver selecionado, o backend efetivo,
os estágios que contribuíram para as amostras, os motivos de fallback e os
ajustes aplicados, além da identidade do wavtool e dos rótulos apresentados.
Os mesmos dados ficam associados à entrada de cache em RAM e ao sidecar JSON do
cache persistente; um WAV sem metadados correspondentes é rejeitado e refeito.

## Fallbacks WORLD observados

`WorldResampler::render_sample_with_oto` retorna silêncio para duração, entrada
ou região inválidas; usa `resize_preserving_pitch` quando a síntese WORLD não
produz um resultado utilizável; e usa SOLA quando a análise DIO/HARVEST, o
refinamento StoneMask, CheapTrick ou a cobertura audível falham. Mesmo após uma
saída WORLD válida, SOLA pode ser aplicada como correção de pitch ou quando há
marcas explícitas de sustain/final. Tainah mapeia os frames WORLD mantendo
ataque e cauda e esticando o intervalo de sustentação entre as marcas OTO; a
correção SOLA de pitch não reprocessa essas marcas. Em seguida, `NativeWorldResamplerDriver`
envolve a chamada em `catch_unwind` e repete SOLA caso o processo entre em
panic ou a saída falhe nos critérios externos.

Os critérios externos são: pelo menos 95% do comprimento esperado, nenhuma
amostra não finita, pico acima de `1e-5`, pelo menos 75% das janelas de 50 ms
audíveis e janela final audível. São guardas de cobertura técnica, não um
indicador de qualidade perceptiva. A rota clássica preserva esse fallback por
compatibilidade; KFV 2 não o utiliza silenciosamente porque seu renderizador
solicita SOLA diretamente.

## Decisão do spike

**Nenhum backend foi selecionado como referência.** Os testes estruturais
demonstram contratos e saídas finitas/timadas com fixtures. A ferramenta
[`kfv2-engine-spike`](engine-spike.md) também produziu um par técnico da mesma
unidade migrada VIICTOR, com o mesmo `PhrasePlan`; nessa unidade WORLD usou
`resize-preserving-pitch`. As marcas da migração não foram revisadas e a licença
não está definida. Isso não compara um corpus de gravações autorizadas nem
frases representativas com audição controlada. A antiga descrição de
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
