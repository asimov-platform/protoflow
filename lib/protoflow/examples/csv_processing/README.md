# CSV Processing Example

This is a simple five-block example program that starts with a constant string
containing CSV data, encodes it into a byte stream, decodes it back into
structured CSV data consisting of headers and rows, re-encodes that structured
data back into CSV format, and writes the result to standard output (stdout).

Note that this program ensures the integrity of the CSV data during the decoding
and re-encoding process and demonstrates the handling of structured data in CSV format.

## Block Diagram

```mermaid
block-beta
    columns 13
    Const space:2 Encode space:2 DecodeCSV space:2 EncodeCSV space:2 WriteStdout
    Const-- "output → input" -->Encode
    Encode-- "output → input" -->DecodeCSV
    DecodeCSV-- "header,rows, → header,rows" -->EncodeCSV
    EncodeCSV-- "output → input" -->WriteStdout

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Const block
    class Encode block
    class DecodeCSV block
    class EncodeCSV block
    class WriteStdout block
```
