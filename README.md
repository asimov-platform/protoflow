# Protoflow

[![License](https://img.shields.io/badge/license-Public%20Domain-blue.svg)](https://unlicense.org)
[![Compatibility](https://img.shields.io/badge/rust-1.70%2B-blue)](https://rust-lang.org)
[![Package](https://img.shields.io/crates/v/protoflow)](https://crates.io/crates/protoflow)
[![Documentation](https://img.shields.io/docsrs/protoflow?label=docs.rs)](https://docs.rs/protoflow/latest/protoflow/)

_"Τὰ πάντα ῥεῖ καὶ οὐδὲν μένει" — Heraclitus_

**Protoflow** is a [Rust] implementation of [flow-based programming] (FBP),
with messages encoded as [Protocol Buffers]. It can be used to implement
dataflow systems consisting of interconnected blocks that process messages.

> [!WARNING]
> **Protoflow has been discontinued** and archived in favor of its successors
> [Async-Flow](https://github.com/artob/async-flow) and
> [Flows.rs](https://github.com/artob/flows.rs), which represent the future of
> flow-based programming in Rust and offer a substantially better developer
> experience as well as simply higher development velocity. See also
> [Awesome FBP](https://github.com/artob/awesome-fbp) for the long tail of
> miscellaneous Rust implementations of flow-based programming.

- [Features](#-features)
- [Prerequisites](#%EF%B8%8F-prerequisites)
- [Installation](#%EF%B8%8F-installation)
- [Examples](#-examples)
- [Reference](#-reference)
- [Development](#-development)

## ✨ Features

- Implements a flow-based programming (FBP) dataflow scheduler.
- Constructs systems by connecting reusable components called blocks.
- Uses Protocol Buffers messages for inter-block communication.
- Currently offers a threaded runtime with an in-process transport.
- Planned support for pluggable runtimes (threaded, async, etc).
- Planned support for pluggable transports (in-process, socket, etc).
- Includes a command-line interface (CLI) for executing Protoflow blocks.
- Supports opting out of any feature using comprehensive feature flags.
- Adheres to the Rust API Guidelines in its [naming conventions].
- 100% free and unencumbered public domain software.

## 🛠️ Prerequisites

- [Rust](https://rust-lang.org) 1.70+

## ⬇️ Installation

### Installation via Cargo

```bash
cargo install protoflow
```

### Installation via Homebrew

```bash
brew tap asimov-platform/tap
brew install protoflow --HEAD
```

## 👉 Examples

### Examples for Rust

For Rust examples, see the [`examples`] directory. Good places to start are
the [`echo_lines`] and [`count_lines`] examples:

```bash
cargo run --example echo_lines < CHANGES.md
cargo run --example count_lines < README.md
```

#### The [`count_lines`] example

```rust filename="lib/protoflow/examples/count_lines/main.rs"
use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let stdin = s.read_stdin();

        let line_decoder = s.decode_lines();
        s.connect(&stdin.output, &line_decoder.input);

        let counter = s.count::<String>();
        s.connect(&line_decoder.output, &counter.input);

        let count_encoder = s.encode_lines();
        s.connect(&counter.count, &count_encoder.input);

        let stdout = s.write_stdout();
        s.connect(&count_encoder.output, &stdout.input);
    })
}
```

## 📚 Reference

### Glossary

- **System**: A collection of blocks that are connected together.
  Systems are the top-level entities in a Protoflow program.

- **Block**: An encapsulated system component that processes messages.
  Blocks are the autonomous units of computation in a system.

- **Port**: A named connection point on a block that sends or receives
  messages. Ports are the only interfaces through which blocks communicate
  with each other.

- **Message**: A unit of data that flows between blocks in a system.
  Messages are Protocol Buffers packets that are processed by blocks.

### Blocks

The built-in blocks provided by Protoflow are listed below:

| Block             | Description                                                                                                                    |
|:------------------|:-------------------------------------------------------------------------------------------------------------------------------|
| [`Buffer`]        | Stores all messages it receives.                                                                                               |
| [`ConcatStrings`] | Concatenates the received string messages, with an optional delimiter string inserted between each message.                    |
| [`Const`]         | Sends a constant value.                                                                                                        |
| [`Count`]         | Counts the number of messages it receives, while optionally passing them through.                                              |
| [`Decode`]        | Decodes messages from a byte stream.                                                                                           |
| [`DecodeCSV`]     | Decodes the received input bytes message into a structured CSV format, separating the header and rows as `prost_types::Value`. |
| [`DecodeHex`]     | Decodes hexadecimal stream to byte stream.                                                                                     |
| [`DecodeJSON`]    | Decodes JSON messages from a byte stream.                                                                                      |
| [`Delay`]         | Passes messages through while delaying them by a fixed or random duration.                                                     |
| [`Drop`]          | Discards all messages it receives.                                                                                             |
| [`Encode`]        | Encodes messages to a byte stream.                                                                                             |
| [`EncodeCSV`]     | Encodes the provided header and rows, given as `prost_types::Value`, into a CSV-formatted byte stream.                         |
| [`EncodeHex`]     | Encodes a byte stream into hexadecimal form.                                                                                   |
| [`EncodeJSON`]    | Encodes messages into JSON format.                                                                                             |
| [`Hash`]          | Computes the cryptographic hash of a byte stream.                                                                              |
| [`Random`]        | Generates and sends a random value.                                                                                            |
| [`ReadDir`]       | Reads file names from a file system directory.                                                                                 |
| [`ReadEnv`]       | Reads the value of an environment variable.                                                                                    |
| [`ReadFile`]      | Reads bytes from the contents of a file.                                                                                       |
| [`ReadSocket`]    | Reads bytes from a TCP socket.                                                                                                 |
| [`ReadStdin`]     | Reads bytes from standard input (aka stdin).                                                                                   |
| [`SplitString`]   | Splits the received input message, with an optional delimiter string parameter.                                                |
| [`WriteFile`]     | Writes or appends bytes to the contents of a file.                                                                             |
| [`WriteSocket`]   | Writes bytes to a TCP socket                                                                                                   |
| [`WriteStderr`]   | Writes bytes to standard error (aka stderr).                                                                                   |
| [`WriteStdout`]   | Writes bytes to standard output (aka stdout).                                                                                  |

#### [`Buffer`]

A block that simply stores all messages it receives.

```mermaid
block-beta
    columns 4
    Source space:2 Buffer
    Source-- "input" -->Buffer

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Buffer block
    class Source hidden
```

```bash
protoflow execute Buffer
```

#### [`ConcatStrings`]

A block for concatenating all string messages it receives, with an optional delimiter string inserted between each message

```mermaid
block-beta
    columns 7
    Source space:2 ConcatStrings space:2 Sink
    Source-- "input" -->ConcatStrings
    ConcatStrings-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ConcatStrings block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute ConcatStrings delimiter=","
```

#### [`Const`]

A block for sending a constant value.

```mermaid
block-beta
    columns 4
    Const space:2 Sink
    Const-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Const block
    class Sink hidden
```

```bash
protoflow execute Const value=Hello
```

#### [`Count`]

A block that counts the number of messages it receives, while optionally passing them through.

```mermaid
block-beta
    columns 7
    Source space:2 Count space:2 Sink
    space:7
    space:7
    space:3 Result space:3
    Source-- "input" -->Count
    Count-- "output" -->Sink
    Count-- "count" -->Result

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Count block
    class Source hidden
    class Sink hidden
    class Result hidden
```

```bash
protoflow execute Count
```

#### [`Decode`]

A block that decodes `T` messages from a byte stream.

```mermaid
block-beta
    columns 7
    Source space:2 Decode space:2 Sink
    Source-- "input" -->Decode
    Decode-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Decode block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute Decode encoding=text
```

#### [`DecodeCSV`]

A block that decodes CSV files from a byte stream into a header and rows represented as `prost_types::Value`

```mermaid
block-beta
    columns 7
    space:5 Sink1 space:1
    space:1 Source space:1 DecodeCSV space:3
    space:5 Sink2 space:1
    Source-- "input" -->DecodeCSV
    DecodeCSV-- "header" -->Sink1
    DecodeCSV-- "content" -->Sink2

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class DecodeCSV block
    class Source hidden
    class Sink1 hidden
    class Sink2 hidden
```

```bash
protoflow execute DecodeCSV
```

#### [`DecodeHex`]

A block that decodes a hexadecimal byte stream into bytes

```mermaid
block-beta
    columns 7
    Source space:2 DecodeHex space:2 Sink
    Source-- "input" -->DecodeHex
    DecodeHex-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class DecodeHex block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute DecodeHex
```

#### [`DecodeJSON`]

A block that decodes JSON messages from a byte stream.

```mermaid
block-beta
    columns 7
    Source space:2 DecodeJSON space:2 Sink
    Source-- "input" -->DecodeJSON
    DecodeJSON-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class DecodeJSON block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute DecodeJSON
```

#### [`Delay`]

A block that passes messages through while delaying them by a fixed or random duration.

```mermaid
block-beta
    columns 7
    Source space:2 Delay space:2 Sink
    Source-- "input" -->Delay
    Delay-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Delay block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute Delay fixed=2
```

#### [`Drop`]

A block that simply discards all messages it receives.

```mermaid
block-beta
    columns 4
    Source space:2 Drop
    Source-- "input" -->Drop

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Drop block
    class Source hidden
```

```bash
protoflow execute Drop
```

#### [`Encode`]

A block that encodes `T` messages to a byte stream.

```mermaid
block-beta
    columns 7
    Source space:2 Encode space:2 Sink
    Source-- "input" -->Encode
    Encode-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Encode block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute Encode encoding=text
protoflow execute Encode encoding=protobuf
```

#### [`EncodeCSV`]

A block that encodes CSV files by converting a header and rows, provided as `prost_types::Value` streams, into a byte stream

```mermaid
block-beta
    columns 7
    space:1 Source1 space:5
    space:3 EncodeCSV space:1 Sink space:1
    space:1 Source2 space:5
    Source1-- "header" -->EncodeCSV
    Source2-- "rows" -->EncodeCSV
    EncodeCSV-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class EncodeCSV block
    class Source1 hidden
    class Source2 hidden
    class Sink hidden
```

```bash
protoflow execute EncodeCSV
```

#### [`EncodeHex`]

A block that encodes a byte stream into hexadecimal form.

```mermaid
block-beta
    columns 7
    Source space:2 EncodeHex space:2 Sink
    Source-- "input" -->EncodeHex
    EncodeHex-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class EncodeHex block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute EncodeHex
```

#### [`EncodeJSON`]

A block that encodes messages into JSON format.

```mermaid
block-beta
    columns 7
    Source space:2 EncodeJSON space:2 Sink
    Source-- "input" -->EncodeJSON
    EncodeJSON-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class EncodeJSON block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute EncodeJSON
```

#### [`Hash`]

A block that computes the cryptographic hash of a byte stream, while optionally
passing it through.

```mermaid
block-beta
    columns 7
    Source space:2 Hash space:2 Sink
    space:7
    space:7
    space:3 Result space:3
    Source-- "input" -->Hash
    Hash-- "output" -->Sink
    Hash-- "hash" -->Result

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Hash block
    class Source hidden
    class Sink hidden
    class Result hidden
```

```bash
protoflow execute Hash algorithm=blake3
```

#### [`Random`]

A block for generating and sending a random value.

```mermaid
block-beta
    columns 4
    Random space:2 Sink
    Random-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Random block
    class Sink hidden
```

```bash
protoflow execute Random seed=42
```

#### [`ReadDir`]

A block that reads file names from a file system directory.

```mermaid
block-beta
    columns 4
    Config space:3
    space:4
    space:4
    ReadDir space:2 Sink
    Config-- "path" -->ReadDir
    ReadDir-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ReadDir block
    class Config hidden
    class Sink hidden
```

```bash
protoflow execute ReadDir path=/tmp
```

#### [`ReadEnv`]

A block that reads the value of an environment variable.

```mermaid
block-beta
    columns 4
    Config space:3
    space:4
    space:4
    ReadEnv space:2 Sink
    Config-- "name" -->ReadEnv
    ReadEnv-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ReadEnv block
    class Config hidden
    class Sink hidden
```

```bash
protoflow execute ReadEnv name=TERM
```

#### [`ReadFile`]

A block that reads bytes from the contents of a file.

```mermaid
block-beta
    columns 4
    Config space:3
    space:4
    space:4
    ReadFile space:2 Sink
    Config-- "path" -->ReadFile
    ReadFile-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ReadFile block
    class Config hidden
    class Sink hidden
```

```bash
protoflow execute ReadFile path=/tmp/file.txt
```

#### [`ReadSocket`]

A block that reads bytes from a TCP socket.

```mermaid
block-beta
    columns 4
    ReadSocket space:2 Sink
    ReadSocket-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ReadSocket block
    class Sink hidden
```

```bash
protoflow execute ReadSocket connection=tcp://127.0.0.1:7077 buffer_size=1024
```

#### [`ReadStdin`]

A block that reads bytes from standard input (aka stdin).

```mermaid
block-beta
    columns 4
    ReadStdin space:2 Sink
    ReadStdin-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ReadStdin block
    class Sink hidden
```

```bash
protoflow execute ReadStdin < input.txt
```

#### [`SplitString`]

A block that splits the received input message, with an optional delimiter string parameter

```mermaid
block-beta
    columns 7
    Source space:2 SplitString space:2 Sink
    Source-- "input" -->SplitString
    SplitString-- "output" -->Sink

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class SplitString block
    class Source hidden
    class Sink hidden
```

```bash
protoflow execute SplitString delimiter=","
```

#### [`WriteFile`]

A block that writes or appends bytes to the contents of a file.

```mermaid
block-beta
    columns 4
    space:3 Config
    space:4
    space:4
    Source space:2 WriteFile
    Config-- "path" -->WriteFile
    Source-- "input" -->WriteFile

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class WriteFile block
    class Config hidden
    class Source hidden
```

```bash
protoflow execute WriteFile path=/tmp/file.txt
```

#### [`WriteSocket`]

A block that writes bytes to TCP socket.

```mermaid
block-beta
    columns 4
    Source space:2 WriteSocket
    Source-- "input" -->WriteSocket

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class WriteSocket block
    class Source hidden
```

```bash
protoflow execute WriteSocket connection=tcp://127.0.0.1:7077 buffer_size=1024
```

#### [`WriteStderr`]

A block that writes bytes to standard error (aka stderr).

```mermaid
block-beta
    columns 4
    Source space:2 WriteStderr
    Source-- "input" -->WriteStderr

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class WriteStderr block
    class Source hidden
```

```bash
protoflow execute WriteStderr < input.txt 2> output.txt
```

#### [`WriteStdout`]

A block that writes bytes to standard output (aka stdout).

```mermaid
block-beta
    columns 4
    Source space:2 WriteStdout
    Source-- "input" -->WriteStdout

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class WriteStdout block
    class Source hidden
```

```bash
protoflow execute WriteStdout < input.txt > output.txt
```

## 👨‍💻 Development

```bash
git clone https://github.com/asimov-platform/protoflow.git
```

### Guidelines

#### Contributing a pull request

- Do your best to adhere to the existing coding conventions and idioms.
- Make sure to run `cargo fmt` prior to submitting your pull request.
- Don't leave trailing whitespace on any line, and make sure all text files
  include a terminating newline character.

#### Adding a new block type

To add a new block type implementation, make sure to examine and amend:

- The block type reference (table and subsections) in this README.
- The appropriate subdirectory under [`lib/protoflow-blocks/src/blocks/`],
  such as `core`, `flow`, `hash`, `io`, `math`, `sys`, or `text`.
- The `BlockTag` enum in [`lib/protoflow-blocks/src/block_tag.rs`],
  which lists the names of all available block types.
- The `BlockConfig` enum in [`lib/protoflow-blocks/src/block_config.rs`],
  which implements block instantiation and Serde deserialization.
- The system-building DSL in [`lib/protoflow-blocks/src/system.rs`],
  which provides convenience builder methods for system definition.
- The `build_stdio_system()` function in [`lib/protoflow-blocks/src/lib.rs`],
  which is used by the CLI to instantiate blocks for standard I/O.
- The documented block diagrams and sequence diagrams under
  [`lib/protoflow-blocks/doc/`], which are embedded in the README and docs.

> [!NOTE]
> If a block implementation requires additional crate dependencies, it may
> be appropriate for that block availability to be featured-gated so as to
> enable developers to opt out of those dependencies.

#### Block implementation notes

- Blocks must not panic; use other error-handling strategies. Ideally, block
  implementations should be robust and infallible. When that's not possible,
  consider encoding errors by having the output message type be an enum (cf.
  Rust's `Result`) or consider having a dedicated error output port. If truly
  necessary, abort block execution by returning a `BlockError`.
- Blocks should not generally spawn threads.
- Blocks should document their system resource requirements, if any.
- Blocks should use the [`tracing`] crate for logging any errors, warnings,
  and debug output. However, since tracing is an optional feature and
  dependency, do make sure to feature-gate any use of tracing behind a
  `#[cfg(feature = "tracing")]` guard.

- - -

[![Share on Twitter](https://img.shields.io/badge/share%20on-twitter-03A9F4?logo=twitter)](https://twitter.com/share?url=https://github.com/asimov-platform/protoflow&text=Protoflow)
[![Share on Reddit](https://img.shields.io/badge/share%20on-reddit-red?logo=reddit)](https://reddit.com/submit?url=https://github.com/asimov-platform/protoflow&title=Protoflow)
[![Share on Hacker News](https://img.shields.io/badge/share%20on-hacker%20news-orange?logo=ycombinator)](https://news.ycombinator.com/submitlink?u=https://github.com/asimov-platform/protoflow&t=Protoflow)
[![Share on Facebook](https://img.shields.io/badge/share%20on-facebook-1976D2?logo=facebook)](https://www.facebook.com/sharer/sharer.php?u=https://github.com/asimov-platform/protoflow)

[Protocol Buffers]: https://protobuf.dev
[Rust]: https://rust-lang.org
[flow-based programming]: https://jpaulm.github.io/fbp/
[naming conventions]: https://rust-lang.github.io/api-guidelines/naming.html
[`tracing`]: https://crates.io/crates/tracing

[`count_lines`]: lib/protoflow/examples/count_lines
[`echo_lines`]: lib/protoflow/examples/echo_lines
[`examples`]: lib/protoflow/examples

[`Buffer`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Buffer.html
[`ConcatStrings`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.ConcatStrings.html
[`Const`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Const.html
[`Count`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Count.html
[`Decode`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Decode.html
[`DecodeCSV`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.DecodeCsv.html
[`DecodeHex`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.DecodeHex.html
[`DecodeJSON`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.DecodeJson.html
[`Delay`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Delay.html
[`Drop`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Drop.html
[`Encode`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Encode.html
[`EncodeCSV`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.EncodeCsv.html
[`EncodeHex`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.EncodeHex.html
[`EncodeJSON`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.EncodeJson.html
[`Hash`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Hash.html
[`Random`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.Random.html
[`ReadDir`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.ReadDir.html
[`ReadEnv`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.ReadEnv.html
[`ReadFile`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.ReadFile.html
[`ReadSocket`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.ReadSocket.html
[`ReadStdin`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.ReadStdin.html
[`SplitString`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.SplitString.html
[`WriteFile`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.WriteFile.html
[`WriteSocket`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.WriteSocket.html
[`WriteStderr`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.WriteStderr.html
[`WriteStdout`]: https://docs.rs/protoflow-blocks/latest/protoflow_blocks/struct.WriteStdout.html

[`lib/protoflow-blocks/doc/`]: https://github.com/asimov-platform/protoflow/tree/master/lib/protoflow-blocks/doc
[`lib/protoflow-blocks/src/blocks/`]: https://github.com/asimov-platform/protoflow/tree/master/lib/protoflow-blocks/src/blocks
[`lib/protoflow-blocks/src/lib.rs`]: https://github.com/asimov-platform/protoflow/blob/master/lib/protoflow-blocks/src/lib.rs
[`lib/protoflow-blocks/src/block_config.rs`]: https://github.com/asimov-platform/protoflow/blob/master/lib/protoflow-blocks/src/block_config.rs
[`lib/protoflow-blocks/src/block_tag.rs`]: https://github.com/asimov-platform/protoflow/blob/master/lib/protoflow-blocks/src/block_tag.rs
[`lib/protoflow-blocks/src/system.rs`]: https://github.com/asimov-platform/protoflow/blob/master/lib/protoflow-blocks/src/system.rs
