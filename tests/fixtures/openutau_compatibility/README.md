# Corpus sintético de compatibilidade OpenUtau

Este diretório contém somente contratos pequenos e reproduzíveis. Os arquivos
não são voicebanks proprietários: o `oto.ini` aponta para amostras sintéticas
geradas pelo teste, e o manifesto registra a expectativa sem exigir igualdade
de áudio bit a bit entre engines.

As primeiras famílias com contratos são japonês CV, VCV, CVVC e Presamp, além dos
contratos de aliases do Inglês Arpasing, dos modos chineses CVV/CVV+ e do
francês VCCV/CVVC. Cada caso
exercita início, transição, pausa, nota estendida (`+`), hint fonético e
ausência de alias. `prefix.map` também é incluído para manter explícita a
precedência pitch -> prefixo/sufixo -> alias do `oto.ini`.

O status “validado” no seletor da UI só pode ser usado por um modo com casos
equivalentes neste corpus e testes de contrato correspondentes.
