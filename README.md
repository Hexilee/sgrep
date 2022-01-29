<div align="center">
  <h1>Super Grep</h1>
  <p><strong>Search words in everything. </strong> </p>
  <p>

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/Hexilee/sgrep/blob/master/LICENSE)

  </p>
</div>
<br>


## Build & Usage

```bash
> make release
> target/release/sgrep -h
sgrep 0.1.0
hexilee <i@hexilee.me>
Super Grep, search words in everything

USAGE:
    sgrep [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -v, --verbose    Verbose level of logs
    -V, --version    Print version information

SUBCOMMANDS:
    grep      Precisely match words by regex
    help      Print this message or the help of the given subcommand(s)
    index     Manage indexes
    search    Fuzzy search words
```
