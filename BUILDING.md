# build rinb
And generate rinb_schema.json
```bash
cd rinb
cargo build --release --locked
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
cd rinb
cargo about generate ./../template/NOTICE.md.hbs --output-file ./../NOTICE.md 
```