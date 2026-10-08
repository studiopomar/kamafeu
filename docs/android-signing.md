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

### Verificação local sem assinar

Para validar somente a compilação do alvo, sem gerar APK nem usar keystore:

```sh
rustup target add aarch64-linux-android
cargo check --target aarch64-linux-android --no-default-features
```

O comando exige um NDK instalado e os compiladores exportados para o alvo. No
NDK 26.1, por exemplo:

```sh
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/26.1.10909125"
export PATH="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
export CC_aarch64_linux_android="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"
export CXX_aarch64_linux_android="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang++"
```

Sem esses executáveis, o erro `failed to find tool
aarch64-linux-android-clang` indica falta de toolchain local, não uma falha do
código Rust. O workflow de CI instala essa versão do NDK antes do build.

## Credenciais anteriores

A configuração removida apontava para o keystore de depuração padrão do Android.
Remover essa configuração não rotaciona nenhuma chave nem apaga o histórico Git.
Antes de publicar, o responsável pela assinatura deve verificar se esse certificado
foi usado em releases, providenciar a substituição apropriada e atualizar os secrets
no GitHub. Não substitua uma chave de aplicativo publicado sem verificar o processo
de atualização de assinatura da loja. Nenhuma chave local ou remota foi rotacionada
por esta alteração.
