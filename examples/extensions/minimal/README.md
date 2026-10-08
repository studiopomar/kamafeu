# Extensão mínima executável

Este exemplo demonstra o menor plugin aceito pelo Kamafeu. Ele não acessa
arquivos, processos ou rede e não solicita permissões.

O diretório já inclui `extension.wasm`, gerado a partir de `extension.wat`.
Para regenerá-lo durante o desenvolvimento, use `wat2wasm` ou o conversor do
workspace. Com a extensão disponível:

```sh
kamafeu extensions examples/extensions/minimal --json --verify-wasm
kamafeu extensions examples/extensions/minimal --invoke-id org.kamafeu.example-minimal --invoke-export analyze --invoke-bytes
```

O catálogo deve encontrar `kamafeu-extension.json` no diretório atual e
reportar a extensão como compatível e com `entrypoint_status: available`.
Capacidades reais devem ser adicionadas
somente quando o módulo implementar o contrato correspondente.

Além do export de versão, o módulo demonstra o ABI de bytes: `analyze(i32) ->
i64` retorna o buffer `hello` na memória exportada. Um host pode chamar esse
export com `invoke_wasm_bytes`; o exemplo usa ponteiro zero e tamanho cinco no
valor empacotado. A descoberta continua apenas lendo o manifesto e validando o
arquivo, sem executar `analyze`.
