# Assinatura Android

O manifesto não deve conter senhas nem caminhos pessoais de keystore.
O workflow `build_android.yml` recebe os secrets `ANDROID_KEYSTORE_BASE64`
(keystore codificado em base64) e `ANDROID_KEYSTORE_PASSWORD` e remove o
arquivo temporário mesmo quando o build falha.

Para compilar localmente, forneça `CARGO_APK_RELEASE_KEYSTORE` com o caminho
absoluto de um keystore fora do repositório e `CARGO_APK_RELEASE_KEYSTORE_PASSWORD`
pelo gerenciador de secrets ou ambiente protegido. Execute:

```sh
cargo apk build --lib --release --target aarch64-linux-android
```

Essas variáveis são suportadas pelo [cargo-apk](https://github.com/rust-mobile/cargo-apk).
O cargo-apk usa caminho e senha do keystore; os campos `key_alias`, `key_name`
e `key_password` anteriormente escritos no manifesto não configuravam essa API.
Use um keystore compatível com a assinatura do cargo-apk.

## Credenciais anteriores

A configuração removida apontava para o keystore de depuração padrão do Android.
Remover essa configuração não rotaciona nenhuma chave nem apaga o histórico Git.
Antes de publicar, o responsável pela assinatura deve verificar se esse certificado
foi usado em releases, providenciar a substituição apropriada e atualizar os secrets
no GitHub. Não substitua uma chave de aplicativo publicado sem verificar o processo
de atualização de assinatura da loja. Nenhuma chave local ou remota foi rotacionada
por esta alteração.
