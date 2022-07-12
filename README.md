# concourse-jsonschema-generator

Transforms the booklit documentation of Concourse into a detailed JSONSchema with docs

![image](https://user-images.githubusercontent.com/502412/178381984-97687890-668f-4acb-b8de-9c5b308536f9.png)


# usage

You can regenerate the schema by passing it the contents of the [concourse docs repo][concourse-docs] and globbing the `.lit` files

```
cargo build --release
./target/release/concourse-jsonschema-generator \
  path/to/concourse/docs/lit/docs/**/*.lit | jq > schema.json
```

You can also use the pre-generated shema from this repo directly.

First, install the [vscode yaml extension][yaml-extension] (or the [redhat yaml LSP server][yaml-lsp])
for your editor.

Then add the schema with a directive at the top of the pipeline file

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/spion/concourse-jsonschema-generator/main/schema.json

resource_types:
  # ...
resources:
  # ...
jobs:
  # ...
```

[concourse-docs]: https://github.com/concourse/docs/
[yaml-extension]: https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml
[yaml-lsp]: https://github.com/redhat-developer/yaml-language-server