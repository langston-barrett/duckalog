# Duckalog

Duckalog is a [Datalog][datalog] engine built on [DuckDB][duckdb].

Currently only usable as a Rust library; there is no parser.

The language is 100% vanilla Datalog. There are no datatypes, no negation, no
aggregation. Relations are not declared separately from their uses. Facts are
part of the program, they are simply rules with empty bodies.

## Features

### Implemented

- DuckDB and SQLite backends

  - Dynamic join order (query planning)
  - Indices
  - Join algorithms

- Semi-naïve evaluation
- That's it!

### Roadmap

See [the issue tracker](https://github.com/langston-barrett/duckalog/issues).

### Not on the roadmap (for now)

- Syntax
- Aggregation
- Negation
- Datatypes, built-in functions

## Comparison to other tools

- Duckalog is like [Duckegg][duckegg], but the focus is on supporting
  moderately performant (semi-naïve) evaluation, rather than experimenting with
  e-graphs.
- Duckalog is to DuckDB what [RecStep][recstep] is to
  [QuickStep][quickstep]---though Duckalog is currently much less optimized.
- Duckalog is much simpler and less fully-featured than [Soufflé][souffle], and
  likely way slower. Duckalog only has an interpreter, not a compiler.
- Unlike [Ascent][ascent], Duckalog is an interpreter. Duckalog is also
  probably way slower, and certainly less featureful.
- Duckalog is meant to be run on a single node, unlike BigDatalog, Distributed
  SociaLite, etc.
- Duckalog is open source, unlike LogicBlox.
- I don't understand [DataFrog][datafrog] enough to draw a comparison

## FAQ

### Is it any good?

Not yet!

### Why another Datalog engine?

To provide a hackable platform for experimentation! 

[ascent]: https://github.com/s-arash/ascent
[datafrog]: https://github.com/rust-lang/datafrog
[datalog]: https://en.wikipedia.org/wiki/Datalog
[duckdb]: https://duckdb.org/
[duckegg]: https://github.com/philzook58/duckegg
[quickstep]: https://dl.acm.org/doi/abs/10.14778/3184470.3184471
[recstep]: https://arxiv.org/abs/1812.03975
[souffle]: https://souffle-lang.github.io/index.html