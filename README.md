An [org-mode](https://orgmode.org/) toolkit, including parser, renderer.

# Usage

## How to use?

``` bash
cargo install windancer
windancer --help
```

see [status](docs/status.org) for status.

## development

``` bash
cargo test

# bench
cargo bench
# performance
cargo flamegraph --example main
perf report
```

# change log

- 2026-01-15 13:42:49: Support parse/render from input directory to
output directory
- 2026-01-05 20:12:45: Remove Rollback state, using ctx and
**inp.full~slice~()** to visit PRE(previous char), parse speed is 2x
(for test file: 50ms -\> 26ms). Note since chumsky full~slice~() API
is not public, so only local version chumsky worked. See [chumsky
issue 946](https://github.com/zesterer/chumsky/issues/943)

# Planning

- support config of subscript
- Error: Rich maybe slow
- use group rather then nested then?

## PRE during parsing

<https://github.com/zesterer/chumsky/issues/943>

- get byte index of previous utf8 char from input, rather than using
state.
- require chumsky's have public APT to visit the raw~input~, such as
InputRef::full~slice~() be public.

# Reference

- [org-syntax](https://orgmode.org/worg/org-syntax.html)
- [chumsky](https://github.com/zesterer/chumsky)
- [orgize](https://github.com/tfeldmann/organize)
- mdbook
