# Install dependencies

Install deps for build target
```bash
rustup target add x86_64-pc-windows-gnu
```

##### Windows
Following [wimlib/README.WINDOWS.md#building-from-source](https://github.com/ebiggers/wimlib/blob/master/README.WINDOWS.md#building-from-source)
1. install [mysys2](https://www.msys2.org/)
2. Run
```bash
pacman -Syu --noconfirm
```
3.
Install llvm for [rust-bindgen](https://github.com/rust-lang/rust-bindgen)
```bash
winget install LLVM.LLVM
```

Build and generate rinb_schema.json
```bash
cd rinb
cargo build --target x86_64-pc-windows-gnu --release --locked
```

# Audit Cargo.lock
Install dependencies
```bash
cargo install cargo-audit --locked
```

Audit
```bash
cd rinb
cargo audit
```

# Update third-party licences
Install dependencies
```bash
cargo install --locked cargo-about
```

Generate [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
```bash
cargo about generate --manifest-path ./rinb/Cargo.toml ./template/NOTICE.md.hbs --output-file ./NOTICE.md
cargo about generate --manifest-path ./toolsnt/Cargo.toml ./toolsnt/template/NOTICE.md.hbs --output-file ./toolsnt/NOTICE.md
```