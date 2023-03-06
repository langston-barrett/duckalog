# Duckalog

Duckalog is a database engine built on DuckDB.

Currently only usable as a Rust library; there is no parser.

## Features

Almost none!

## Language

The language is 100% vanilla Datalog. There are no datatypes, no declaring
relations, no negation, no aggregation. Facts are part of the program, they
are simply rules with empty bodies.