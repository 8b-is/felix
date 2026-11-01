# Felix v0.1

Two promises, both checked by tests:

1. **A `linear` value is consumed exactly once, or the program does not compile.**
2. **The ternary datapath is the `memnet` reference oracle, bit for bit** — Felix
   does not get its own opinion about packing or contraction.

It runs `.flx` files:

```bash
cargo run -q -- examples/01_hello_ternary.flx
```

```
word{gamma:2, re:+1/1, im:+0/0}
word{gamma:1, re:+4/0, im:+0/0}
(0, 0)
144
00+--+
```

And it refuses things:

```bash
cargo run -q -- examples/02_linear_rejected.flx
# error at 4:7: linear value `vault` is used here without being consumed
```

## The language, in full

```text
program    := statement* EOF
statement  := "linear"? "let" IDENT "=" expr ";"
            | "consume" "(" expr ")" ";"
            | "print"   "(" expr ")" ";"
expr       := primary ( "<~>" primary )*
primary    := INT | "[" [ expr ("," expr)* ] "]"
            | IDENT | IDENT "(" [ expr ("," expr)* ] ")"
```

Values: `Int`, `Word` (a packed complex ternary word plus its scale), `Pair`
(a contraction result), `Text`, `Vault`, `Unit`.

Builtins:

| Call | Meaning |
|---|---|
| `pack([samples...], gamma)` | dead-zone quantise integer samples into a word, imaginary quadrature zero |
| `a <~> b` | contract two words; returns `(re, im)` integer accumulator |
| `fib(n)` | `F(n)` by the NAND-only ripple-carry cascade |
| `bt(n)` | balanced ternary of `n`, least significant digit first |
| `new_vault()` | a fresh linear value |

## Why this is the whole language and not a scaffold

The repository this grew out of proposed a compiler, a VM, an LSP server, an MCP
surface, four stdlib modules and a triple-forge push, in one pass. None of those
can be called real until something pins their semantics, and a language whose
first commit is all of them is a specification in a repository's clothing.

So v0.1 ships the spine and says what it does not do:

| Piece | State |
|---|---|
| Lexer, parser, checker, evaluator | working, 27 tests |
| Linear-capability rule | enforced, with both use sites in the diagnostic |
| Ternary packing + contraction | delegated to `memnet`, differentially tested |
| NAND-only Fibonacci, balanced ternary | reachable from source, tested to `F(39)` |
| VM, LSP, MCP, stdlib modules | **absent, deliberately** |
| `mem8` / `engram` / `entheai` stdlib | absent — each needs its own acceptance tests |
| Second implementation (Zig/RTL) | absent — this is the reference the others get diffed against |

And one sentence that matters more than the rest: **a language runtime is not a
mind.** `Vault` exists so that a memory-owning value has an owner, not because
software has an interior. Anything that reads as a claim about consciousness is
a documentation bug and should be deleted.

## Layout

```
crates/felix-lang   lexer · ast · parser · check (linear types) · eval · diag
crates/felix-cli    the `felix` binary
examples/           01 runs, 02 is meant to fail
```

The differential tests (`packing_is_bit_identical_to_the_oracle`,
`contraction_is_bit_identical_to_the_oracle`) link the real `memnet` crate by
path, so if either side drifts the build breaks instead of the claim.

AGPL-3.0-only, matching the workspace it lives in.
