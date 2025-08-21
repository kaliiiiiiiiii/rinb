# RInb
A builder and downloader for windows images written in rust

[![.github/workflows/build.yml](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/build.yml/badge.svg)](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/build.yml)[![Security audit](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/audit-check.yml/badge.svg)](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/audit-check.yml)

# Usage
```bash
Usage: rinb.exe [OPTIONS] --config <CONFIG>

Options:
      --config <CONFIG>          Path to config file
      --cache-path <CACHE_PATH>  [default: ./.rinbcache/esd_cache]
  -h, --help                     Print help
  -V, --version                  Print version
```
A sample config file can be found at [rinb.json](rinb.json) ([json-schema](rinb_schema.json))

# TODO
- use [wimlib](https://codeberg.org/erin/toolsnt/src/branch/trunk/wimlib) and [hadris-iso](https://crates.io/crates/hadris-iso) to create a bootable iso for testing

# Building
See [BUILDING.md](./BUILDING.md)

## Licence

This project is licensed under the [Apache-2.0 license](./LICENSE.txt).

### Third-party software
This project depends on third-party crates under various licenses 
(including MIT, Apache-2.0, BSD-3-Clause, ISC, Unicode, etc.).  
A complete list of licenses is available in [NOTICE.md](./NOTICE.md).

# References
- [cargo-about-markdown-template.hbs](https://github.com/takkt-ag/persevere/blob/6e0f40d47a8ce5dd5ec83bc102053996f59b7291/.tools/cargo-about-markdown-template.hbs)