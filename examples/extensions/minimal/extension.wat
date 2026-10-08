;; Compile with any WAT-to-WASM tool, for example:
;;   wat2wasm extension.wat -o extension.wasm
(module
  ;; The first four bytes are the ABI version contract.
  (func (export "kamafeu_extension_api_version") (result i32)
    (i32.const 1))

  ;; A read-only result buffer for invoke_wasm_bytes. The packed i64 contains
  ;; pointer=0 in the high half and length=5 in the low half.
  (memory (export "memory") 1)
  (data (i32.const 0) "hello")
  (func (export "analyze") (param i32) (result i64)
    (i64.const 5)))
