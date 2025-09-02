# RInb
A builder and downloader for windows images written in rust

[![.github/workflows/build.yml](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/build.yml/badge.svg)](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/build.yml)[![Security audit](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/audit-check.yml/badge.svg)](https://github.com/kaliiiiiiiiii/rinb/actions/workflows/audit-check.yml)

# Usage
```bash
Usage: rinb.exe [OPTIONS]

Options:
      --config <CONFIG>          Path to config file [default: rinb.json]
      --out <OUT>                [default: out/devwin.iso]
      --cache-path <CACHE_PATH>  [default: ./.rinbcache/esd_cache]
  -h, --help                     Print help
  -V, --version                  Print version
```
A sample config file can be found at [rinb.json](rinb.json) ([json-schema](rinb_schema.json))

# TODO
- support caching built base `install.wim`, `boot.wim` and `base.wim`
- build raw disk image (see [rinb/src/rdisk_pack.rs](rinb/src/rdisk_pack.rs)) based on install.wim for testing.
- support other targets than ["x86_64-pc-windows-gnu", "x86_64-unknown-linux-gnu"]

# Building
See [BUILDING.md](./BUILDING.md)

## Licence

This project is licensed under the [EUPL-1.2](./LICENSE.txt).

### Third-party software
This project depends on third-party crates under various licenses 
(including MIT, Apache-2.0, BSD-3-Clause, ISC, Unicode, etc.).  
A complete list of licenses is available in [NOTICE.md](./NOTICE.md).

Especially take note of
- [NOTICE.md#wimlib](./NOTICE.md#wimlib)

# References
- [cargo-about-markdown-template.hbs](https://github.com/takkt-ag/persevere/blob/6e0f40d47a8ce5dd5ec83bc102053996f59b7291/.tools/cargo-about-markdown-template.hbs)