# Sistema visual do Kamafeu Studio

O Kamafeu é uma estação de trabalho vocal maximalista. A interface não deve
esconder controles avançados para parecer simples. Ela deve organizar uma
grande quantidade de informação para que músicos experientes encontrem o que
precisam rapidamente.

## Princípios

- **Densidade com hierarquia:** muitos controles podem existir juntos, mas
  títulos, valores, rótulos e estados precisam ter pesos visuais diferentes.
- **Identidade antes de decoração:** o acento do tema é usado em foco,
  seleção, reprodução e orientação. Não é usado em todos os elementos ao
  mesmo tempo.
- **Superfícies em camadas:** canvas, painel, superfície elevada e estado
  ativo devem ser distinguíveis por contraste sutil, não por caixas pesadas.
- **Movimento funcional:** animações comunicam reprodução, troca de aba,
  foco, carregamento e mudanças de estado. Nenhum elemento deve piscar sem
  significado.
- **Acesso direto:** animação ou agrupamento visual não pode esconder funções
  avançadas atrás de fluxos para iniciantes.

## Escala visual

O editor usa uma escala consistente para diferenciar os níveis de informação:

- texto principal para valores e edição;
- texto de botão para ações;
- texto pequeno para metadados e rótulos auxiliares;
- títulos para grupos de parâmetros;
- acento do tema para seleção, foco e orientação.

Em telas estreitas, a escala e a área de toque aumentam. A quantidade de
recursos permanece a mesma; apenas a disposição muda.

## Estados

Use os tokens semânticos de `ThemeConfig`:

- `success_c32()` para pronto, concluído e saudável;
- `warning_c32()` para atenção e condições não fatais;
- `danger_c32()` para parar, erro e ações destrutivas;
- `info_c32()` para informação neutra;
- `accent_c32()` para identidade, seleção e foco.

Evite cores RGB fixas em novos módulos. A exceção são gráficos musicais em
que a cor representa um dado específico, como pitch, dinâmica ou envelope.

## Animações

O tempo global de transição é controlado por `ui_animations_enabled` e
`ui_animation_speed`. Estados contínuos, como reprodução, podem solicitar
repaint em pequenos intervalos; estados estáticos devem usar as transições do
egui e parar de repintar quando a animação terminar.

Boas animações no Kamafeu são discretas: uma aba muda de superfície, o
transporte pulsa durante a reprodução e o playhead acompanha o áudio. O
editor não deve usar movimento permanente para chamar atenção.

## Componentes

Ao criar um painel novo:

1. agrupe controles relacionados em uma superfície elevada;
2. use `ui_rounding()` e `card_stroke()` em vez de raios e bordas fixos;
3. dê ao grupo um título curto e um estado claro;
4. preserve acesso por teclado e tooltips para controles não óbvios;
5. valide desktop, WASM e layout touch-first.

O objetivo não é deixar o Kamafeu minimalista. É fazer uma ferramenta grande
parecer intencional, legível e autoral.
