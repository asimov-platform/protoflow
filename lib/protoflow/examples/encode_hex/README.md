# Hex Encode Example

This is a simple four-block example program that starts with a constant string,
encodes it into a byte stream, converts the bytes into hexadecimal format,
and writes the hexadecimal-encoded output to standard output (stdout).

Note that this program demonstrates how to transform human-readable text into
its hexadecimal representation, which can be useful for encoding data for secure
transmission or storage.

## Block Diagram

```mermaid
block-beta
    columns 10
    Const space:2 Encode space:2 EncodeHex space:2 WriteStdout
    Const-- "output → input" -->Encode
    Encode-- "output → input" -->EncodeHex
    EncodeHex-- "output → input" -->WriteStdout

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Const block
    class Encode block
    class EncodeHex block
    class WriteStdout block
```
