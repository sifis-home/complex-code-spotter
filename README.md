# Complex-code-spotter

This tool extracts snippets of code deemed complex according to the following
complexity metrics:

- Cyclomatic
- Cognitive

When the value associated to each of the metrics exceeds a preset threshold,
a snippet of code is automatically extracted.

**Note: Duplicated snippets could be contained in the final output when their
complexity value exceeds more than one threshold.**

## Build

To build both `lib` and `code-complex-spotter` code:

```
cargo build
```

## Run tests

Verify whether all tests pass with the command:

```
cargo test
```

## Produce docs

Generate the final documentation with the command:

```
cargo doc --open --no-deps
```

Remove the `--no-deps` option to build documentation for each dependency.

## View options

To view the list of `code-complex-spotter` options, run:

```
cargo run -- --help
```

## Usage

The default configuration extracts snippets of code for *cyclomatic* and
*cognitive* metrics, both with an empirical threshold of *15*, and defines
*markdown* as output format.

```
cargo run -- /path/to/your/file/or/directory /output/path
```

### Metrics

To choose complexity metrics and the relative thresholds,
use the *complexity* `c` option:
It supports only these values: *cyclomatic*, *cognitive*, *cyclomatic:threshold*, *cognitive:threshold*.

For example, to set up a threshold for each complexity metric:

```
cargo run -- -c cyclomatic:3 -c cognitive:16 /path/to/your/file/or/directory /output/path
```

**Note: When a threshold is not defined, a value of 15 is used for each
complexity metric!**

### Output

To output in different formats, use the *output* `O` option.
It supports only these values: *markdown*, *html*, *json*, *all*.

For example, to use *html* as output format:

```
cargo run -- -O html /path/to/your/file/or/directory /output/path
```

The *all* option saves the extracted snippets in each supported output format.

### Filter

It is possible to filter input source files using `I` and `X` options.
The *input* `-I` option is a glob filter that considers **only** the files with
a determined file extension.
The *exclude* `-X` option instead is a glob filter that **does** not consider
**only** the files with a determined file extension.

To consider only Rust `*.rs` files:

```
cargo run -- -I "*.rs" /path/to/your/file/or/directory /output/path
```

To exclude only Rust `*.rs` files:

```
cargo run -- -X "*.rs" /path/to/your/file/or/directory /output/path
```

Both these options can be used more than once.

## License

Released under the [MIT License](LICENSE).

## Acknowledgements

This software has been developed in the scope of the H2020 project SIFIS-Home with GA n. 952652.
