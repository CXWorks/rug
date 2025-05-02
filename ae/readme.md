# AE of RUG

These are the generated tests from RUG we use for comparison with other tools.

[rug](rug) contains the crates for LLM evaluation

[rulf](rulf) contains the crates for fuzzing evaluation against rulf and rpg

[rustyunit_rug](rustyunit_rug) contains the crates comparison with rustyunit

[syrust](syrust) contains the crates comparison with syrust


We usually have multiple folders: the test folder, the fuzzing harness folder(ends with _fuzz), the coverage folder(ends with _cov) and fuzzing corpus replay folder(ends with _replay)

Please use a very good tool, cargo bolero for fuzzing.