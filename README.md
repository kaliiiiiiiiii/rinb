# RInb
A builder and downloader for windows images written in rust

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