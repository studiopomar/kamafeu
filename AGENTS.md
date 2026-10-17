# Repository Guidelines

Guia para contribuidores e agentes: preserve alterações locais e o escopo solicitado.

## Estrutura do Projeto

- `src/`: aplicação Rust por domínio: `gui/`, `dsp/`, `audio/`, `renderer/`, `project/`, `oto/`, `phonemizer/`, `formats/` e `voicebank/`.
- `src/bin/`: ferramentas auxiliares; `src/main.rs` inicia o desktop e `src/lib.rs` expõe o núcleo compartilhado.
- `tests/`: testes de integração e fixtures; testes unitários ficam junto aos módulos.
- `assets/`, `res/`, `native/`: recursos visuais e de plataforma, além de código nativo.
- `docs/`, `examples/`, `resamplers/`: documentação, extensões de exemplo e integrações de motores externos.

## Fluxo de Trabalho

1. Antes de editar, confira `git status --short`, leia o módulo afetado e consulte `README.md`, `docs/` ou fixtures. Preserve alterações alheias.
2. Localize a fronteira afetada: formato, fonemização, render/DSP, GUI, plataforma ou extensão. Mantenha regras de domínio no módulo apropriado.
3. Faça a menor alteração coerente e adicione regressão quando necessário.
4. Revise o diff. Relate verificações executadas e o que ainda exige validação manual.

## Contratos de Voz e Áudio

Trate `oto.ini` como autoridade final de aliases e offsets. Ao alterar fonemização, preserve letra → fonema → alias → amostra → timing → motor; cubra transições, alias ausente, subbancos, `prefix.map` e codificações com fixtures. Modo listado não significa compatibilidade validada. Em DSP/render, preserve duração, envelope, nível e junção; evite alocação, bloqueio ou I/O no callback de áudio. Mantenha leitura de projetos existentes ou documente migração.

## Build e Verificações

- `cargo run` inicia o app desktop.
- `cargo fmt --all -- --check` confere formatação.
- `cargo clippy --all-targets --all-features --locked` verifica avisos e problemas comuns.
- `cargo test --all-targets --all-features --locked` executa a suíte automatizada.
- `cargo build --release --bins --locked` compila os binários de distribuição.

Prefira verificações focadas; para mudanças amplas, siga `.github/workflows/ci.yml`. Em áudio, verifique duração, silêncio, pico, RMS e clipping; WAVs de motores distintos não precisam ser idênticos bit a bit. Builds/testes não comprovam comportamento visual, audição, drivers ou bancos reais. Relate limites sem alegar validação não realizada.

## Estilo e Convenções

Use Rust 2021 e `rustfmt` (quatro espaços). Siga os padrões: módulos/funções em `snake_case`, tipos/traits em `UpperCamelCase`, constantes em `SCREAMING_SNAKE_CASE`. Prefira APIs explícitas, propagação de erros e compatibilidade. Preserve fixtures; não invente dados ou alegações de compatibilidade.

## Commits e Pull Requests

O histórico usa Conventional Commits (`fix: ...`, `feat: ...`). Use texto imperativo e específico. PRs explicam motivação, listam verificações e limites, incluem capturas para mudanças visuais e vinculam issue quando houver.
