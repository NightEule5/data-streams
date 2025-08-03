# Data Streams

Data Streams provides stream extension traits for reading and writing data with streams.

## Usage

Add `data-streams` to your dependencies with `cargo add data-streams`, or manually in your `Cargo.toml`:

```toml
[dependencies]
data-streams = "2.0.0"
```

```rust
use data_streams::{DataSource, DataSink, Result};

fn read(source: &mut impl DataSource) -> Result<()> {
	let int: i32 = source.read_i32()?; // or use generic read_int()
	let str: &str = source.read_utf8_to_end(&mut String::default())?;
	let bytes: &[u8] = source.read_bytes(&mut [0; 128])?;
}

fn write(source: &mut impl DataSink) -> Result<()> {
	source.write_i32(12345)?; // or use generic write_int()
	source.write_utf8("something")?;
	source.write_bytes(&[1, 2, 3, 4, 5])?;
}
```

## Feature flags

The `utf8` feature enables reading UTF-8 bytes, with high-performance validation from the [`simdutf8`]
crate.

[`simdutf8`]: https://github.com/rusticstuff/simdutf8

### `no_std` Support

This crate supports `no_std` and environments without `alloc`. These are toggled by the `std` and
`alloc` features respectively. `no_std` allows use in embedded environments, at the expense of lost
implementations for types that would be provided by `std::io`. Disabling `alloc` removes reading
into vectors/strings.
