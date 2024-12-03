# Write File Example

This is a simple three-block example program that reads input from standard input
(stdin) and writes the content directly to a file (`output.txt`).

Note that this program demonstrates how to capture user input or piped data
and store it persistently in a file.

## Block Diagram

```mermaid
block-beta
    columns 4
    Const space:3
    space:3 WriteFile
    ReadStdin
    ReadStdin-- "output → input" -->WriteFile
    Const-- "output → path" -->WriteFile

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class ReadStdin block
    class Const block
    class WriteFile block
```
