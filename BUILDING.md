# build rinb
And generate rinb_schema.json
```bash
cd rinb
cargo build --release --locked
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