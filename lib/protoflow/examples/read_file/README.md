# Read File Example

This is a simple seven-block example program that begins with a constant string
containing multiple lines of text, writes the content to a file, reads the file
back into the system, and writes the file's content to standard output (stdout).

Note that this program is fully self-contained, creating and processing the file
entirely within the flow, making it suitable for demonstrating file handling
in a standalone scenario.

## Block Diagram

```mermaid
block-beta
    columns 7
    space:3 Const_wpath space:3
    space:6 WriteFile
    Const_content space:2 Encode space:3
    space:7
    Const_rpath space:2 ReadFile space:2 WriteStdout
    Const_wpath-- "output → path" -->WriteFile
    Const_content-- "output → input" -->Encode
    Encode-- "output → input" -->WriteFile
    Const_rpath-- "output → path" -->ReadFile
    ReadFile-- "output → input" -->WriteStdout

    classDef block height:48px,padding:8px;
    classDef hidden visibility:none;
    class Const_wpath block
    class Const_content block
    class Const_rpath block
    class Encode block
    class WriteFile block
    class ReadFile block
    class WriteStdout block
```
