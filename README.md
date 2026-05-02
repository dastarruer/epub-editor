# epub-editor

Edit EPUBs in a Google Docs-like interface.

## Development

To install required developer dependencies:

```sh
nix develop
```

To serve the Tauri app, first install node dependencies in `frontend/`:

```sh
cd frontend
pnpm i
```

Then, put a (valid) `test.epub` file in the root of the project for testing purposes:

```text
.
├── flake.lock
├── flake.nix
├── frontend
├── README.md
├── src-tauri
└── test.epub
```

Finally, from the root of the project, run the following to serve the app:

```sh
cargo tauri dev
```
