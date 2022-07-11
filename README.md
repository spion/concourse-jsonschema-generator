# concourse-jsonschema-generator

Transforms the booklit documentation of Concourse into a detailed JSONSchema

# usage

```
cargo build --release
./target/release/concourse-jsonschema-generator path/to/concourse/docs/lit/docs/**/*.lit | jq > schema.json
```
