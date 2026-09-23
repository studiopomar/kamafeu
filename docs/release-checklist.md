# Checklist de release do Kamafeu Studio

Use este checklist para transformar uma release candidate em uma publicação reproduzível.

## Antes da tag

- [ ] `git status` contém apenas alterações intencionais.
- [ ] `Cargo.toml`, janela, Android, badge do menu e changelog exibem a mesma versão.
- [ ] A tela Sobre/tooltips exibem `Codinome interno: projeto_saturno` sem alterar o nome principal do produto.
- [ ] `cargo fmt --all -- --check` passa.
- [ ] `cargo test --all-targets --all-features` passa.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passa.
- [ ] `cargo audit` foi revisado; avisos de manutenção foram registrados no changelog.
- [ ] README e instruções de instalação correspondem aos artefatos reais.

## Validação de áudio

- [ ] Renderizar uma frase VCV, uma CVVC/VCCV, uma BRAPA e uma inglesa.
- [ ] Repetir os renders com o cache limpo.
- [ ] Ouvir as junções de vogal, VC, clusters e finais de frase.
- [ ] Confirmar que não há silêncio, truncamento, clipping ou descontinuidade audível.
- [ ] Conferir que o WAV final incorpora o resultado do wavtool selecionado.

## Artefatos

- [ ] Windows x64 abre e renderiza um projeto mínimo.
- [ ] Windows x86 é mantido somente se houver demanda de suporte.
- [ ] O `.dmg` contém `Kamafeu.app` e o binário Copaiba.
- [ ] O pacote Linux contém permissões de execução e instruções de dependências.
- [ ] Resamplers externos empacotados estão presentes ou a release falha explicitamente.
- [ ] `SHA256SUMS.txt` acompanha todos os arquivos publicados.
- [ ] Assinatura/notarização macOS e assinatura Android foram verificadas quando aplicável.

## Após publicar

- [ ] Baixar cada artefato a partir da página pública, sem usar a árvore de build.
- [ ] Executar `--help`, `voicebank-info` e um render curto em cada plataforma disponível.
- [ ] Registrar problemas conhecidos na release e manter o changelog atualizado.
