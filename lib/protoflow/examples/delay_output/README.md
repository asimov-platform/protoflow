# Delayed Line Output Example

This is a simple five-block example program that starts with a constant string
containing multiple lines, splits the string into individual lines,
introduces a delay between processing each line, and writes the delayed lines
to standard output (stdout).

Note that this program demonstrates how to control the timing of data processing,
making it suitable for simulating real-time output or throttling data streams

## Block Diagram

```mermaid
block-beta
    columns 13
    Const space:2 SplitString space:2 Delay space:2 Encode space:2 WriteStdout
    Const-- "output → input" -->SplitString
    SplitString-- "output → input" -->Delay
    Delay-- "output → input" -->Encode
    Encode-- "output → input" -->WriteStdout

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Const block
    class SplitString block
    class Delay block
    class Encode block
    class WriteStdout block
```
