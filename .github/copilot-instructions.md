This project is based on the Risc V architecture. All assembly code is based on the Risc V 64-bit ISA. When the MMU is active, sv39 mode is used. The kernel code is assumed to be running in supervisor mode.

All Rust code uses the 2024 edition with the no_std and no_main options. The following guidelines should be used for Rust code:
  - Naming of structs, functions, and variables should be explicit, descriptive, and used spelled out words.
  - Blank lines should be used to separate logical blocks of code inside of functions.
  - Comments should use complete sentences and proper punctuation including periods at the end of sentences.
  - Complex statements should be broken out into a series of simpler statements using descriptive intermediate variables as needed.