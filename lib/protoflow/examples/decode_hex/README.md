# Hex Decode Example

This is a simple four-block example program that starts with a constant string
containing hexadecimal-encoded data, encodes it into a byte stream,
decodes the hexadecimal back into raw bytes, and writes the decoded data to
standard output (stdout).

Note that this program demonstrates how to handle hexadecimal input without
relying on external files or sources, making it fully self-contained.

## Block Diagram

```mermaid
block-beta
    columns 10
    Const space:2 Encode space:2 DecodeHex space:2 WriteStdout
    Const-- "output → input" -->Encode
    Encode-- "output → input" -->DecodeHex
    DecodeHex-- "output → input" -->WriteStdout

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Const block
    class Encode block
    class DecodeHex block
    class WriteStdout block
```
