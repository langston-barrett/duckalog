# Duckalog

Duckalog is a [Datalog][datalog] engine built on [DuckDB][duckdb].

Currently only usable as a Rust library; there is no parser.

## Features

Almost none!

## Language

The language is 100% vanilla Datalog. There are no datatypes, no declaring
relations, no negation, no aggregation. Facts are part of the program, they
are simply rules with empty bodies.

## Comparison to other tools

- Duckalog is like [Duckegg][duckegg], but the focus is on supporting
  moderately performant (semi-naïve) evaluation, rather than experimenting with
  e-graphs.
- Duckalog is to DuckDB what [RecStep][recstep] is to
  [QuickStep][quickstep]---though Duckalog is currently much less optimized.
- Duckalog is much simpler and less fully-featured than [Soufflé][souffle], and
  likely way slower.

<!-- TODO: SociaLite, BigDatalog, Ascent -->

[datalog]: https://en.wikipedia.org/wiki/Datalog
[duckdb]: https://duckdb.org/
[duckegg]: https://github.com/philzook58/duckegg
[quickstep]: https://dl.acm.org/doi/abs/10.14778/3184470.3184471
[recstep]: https://arxiv.org/abs/1812.03975
[souffle]: https://souffle-lang.github.io/index.html