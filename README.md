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

# Reference

- [org-syntax](https://orgmode.org/worg/org-syntax.html)
- [chumsky](https://github.com/zesterer/chumsky)
- [orgize](https://github.com/tfeldmann/organize)
- mdbook
