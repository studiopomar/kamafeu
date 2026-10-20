# Preset declarativo básico

Este exemplo mostra um manifesto de tipo `preset`, com dois parâmetros
numéricos tipados, valores padrão e faixas válidas. Ele não solicita permissões
e não possui entrypoint WASM. Por isso, o catálogo consegue validá-lo e exibi-lo,
mas o editor ainda não aplica os valores a uma voz.

Para conferir o manifesto:

```sh
kamafeu extensions examples/extensions/preset-basic --json
```
