# Gust

An user-friendly Git terminal written in Rust

## How to run

Al compilar el proyecto, va a lanzar un interfaz donde se puede ingresar la ruta
de un archivo, por ejemplo `src/poem.txt` y le va a calcular el SHA-1. Luego
verifica que sea igual a una constante que se tiene en `main.rs`. También se
puede probar con `src/lorem.txt`, el cual muestra su SHA-1, pero no es el mismo
que del anterior archivo.

```bash
cargo run
```

## Goals (Minimum Viable Scope)

- Add files to the staging area and create commits.
- Store content-addressed objects (blob, tree, commit) identified by a hash, following Git's structural model.
- Browse the commit timeline and history (equivalent to `git log`).
- Inspect what a specific commit changed (equivalent to `git show`).
- Undo a commit.
- Provide a TUI as the primary interface, implemented in pure Rust.

## 1. Architecture

Estos son módulos que se van separar.

```py
gust (bin)  ──►  gust-tui  ──►  gust-repo  ───►  gust-diff
                                    │                │
                                    └─►  gust-core ◄─┘
```

| Module | Responsibility |
| --- | --- |
| `gust-core` | Storage model: object IDs, blob/tree/commit encoding, object database. No knowledge of the working tree. |
| `gust-diff` | Pure algorithms with no I/O: line diff and tree-to-tree diff. |
| `gust-repo` | All version-control semantics: index, refs, add, commit, log, show, undo. Exposes the `Repository` facade. |
| `gust-tui` | Presentation only: state machine, events, views. Contains no version-control logic. |
| `gust` | Thin binary that opens the repository and launches the TUI. |

Paquetes extra que ayudarán

```toml
crossterm = "Sirve para poder ejecutar en cualquier terminal de cualquier OS"
flate2 = "Para comprimir/descomprimir (blobs supongo)"
hex = "manipular hexadecimales, que de eso tratan los SHA-1"
ratatui = "Un paquete que permite hacer TUI"
sha1 = ":v"
similar = "Lo que permite hacer diff's y +12 -10 Lines added"
```

## 3. Directory Structure

```python
gust/
├── Cargo.toml
└── crates/
    ├── gust-core/          # Storage model (no knowledge of the working tree)
    │   └── src/
    │       ├── lib.rs
    │       ├── hash.rs     # ObjectId (SHA-1 wrapper), hex encode/decode
    │       ├── object/
    │       │   ├── mod.rs  # enum Object { Blob, Tree, Commit }
    │       │   ├── blob.rs
    │       │   ├── tree.rs # TreeEntry { mode, name, id }
    │       │   └── commit.rs # author, committer, parents, message, timestamp
    │       ├── odb.rs      # Object database: read/write, zlib, .gust/objects/ab/cdef...
    │       └── error.rs
    │
    ├── gust-diff/          # Pure algorithms, no I/O
    │   └── src/
    │       ├── lib.rs
    │       ├── text.rs     # Line diff (Myers, via `similar`), hunks
    │       └── tree.rs     # Tree-to-tree diff: added/modified/deleted files
    │
    ├── gust-repo/          # All version-control semantics
    │   └── src/
    │       ├── lib.rs      # struct Repository: the public facade
    │       ├── init.rs     # Creates .gust/{objects,refs/heads}, HEAD
    │       ├── index.rs    # Staging area (read/write the index file)
    │       ├── refs.rs     # HEAD, branches, resolving references
    │       ├── workdir.rs  # Scanning the working tree, ignore rules, status
    │       └── ops/
    │           ├── add.rs
    │           ├── commit.rs   # index -> trees -> commit object -> move ref
    │           ├── log.rs      # Commit history walk (iterator)
    │           ├── show.rs     # Commit + diff against parent
    │           └── undo.rs     # revert / reset
    │
    ├── gust-tui/           # Presentation only
    │   └── src/
    │       ├── lib.rs      # run(repo) entry point
    │       ├── app.rs      # State machine: current screen, selection, messages
    │       ├── event.rs    # Key events -> Actions
    │       ├── theme.rs
    │       └── views/
    │           ├── status.rs    # Working tree + staging (add/unstage)
    │           ├── commit.rs    # Message editor
    │           ├── log.rs       # Timeline of commits
    │           ├── show.rs      # Diff viewer for one commit
    │           └── confirm.rs   # Confirmation dialog (undo)
    │
    └── gust/               # Thin binary
        └── src/main.rs     # Opens the repository and calls gust_tui::run
```

## 4. How Each Feature Maps to the Structure

| Feature | Where it lives | Notes |
| --- | --- | --- |
| Hashing and object structure | `gust-core` | Same format as Git: `"<type> <size>\0<content>"`, hashed, zlib-compressed, stored by hash. |
| Add files | `gust-repo::ops::add` + `index` | Writes blobs to the object database and updates the index. |
| Commit | `gust-repo::ops::commit` | Builds trees from the index, creates the commit object, advances the current branch ref. |
| Log / timeline | `gust-repo::ops::log` | Expose it as an `Iterator<Item = Commit>`. The TUI consumes it lazily, so large histories stay fast. |
| Show | `gust-repo::ops::show` + `gust-diff` | Compare the commit's tree with its parent's tree, then text-diff the changed blobs. |
| Undo | `gust-repo::ops::undo` | See the design note below. |
| Interface | `gust-tui` | Contains no version-control logic, only calls to `Repository`. |

### Design note: undoing a commit

Two distinct operations exist, and they should be separate functions:

- `revert(id)` creates a new commit that inverts the changes. It is safe and preserves history.
- `reset(id)` moves the branch pointer backward. It is destructive to the visible history, so the TUI should require explicit confirmation.

## 5. Internal Design Recommendations

1. **A single facade.** Expose `Repository` from `gust-repo` with methods such as `add()`, `commit()`, `log()`, `show()`, and `revert()`. The TUI should never touch the object database directly.
2. **TUI architecture (Elm-style).** Keep an `App` state, translate key events into `Action` values, and apply them in a pure `update(state, action)` function. Rendering is then a pure function of state, which makes the interface unit-testable.
3. **Typed errors in libraries, `anyhow` in binaries.** Use `thiserror` in `gust-core`, `gust-diff`, and `gust-repo`, and `anyhow` only in `gust-tui` and `gust`.
4. **Git-compatible object format at first.** Using SHA-1 and Git's exact encoding lets you validate your implementation against the real tool (for example, compare the output of `git hash-object` with yours). Keep the hash behind the `ObjectId` type so that migrating to SHA-256 later is localized.
5. **Use your own metadata directory** (`.gust/`) to avoid colliding with an existing `.git/` repository.
6. **Test with `tempfile`.** Each integration test creates a temporary directory, initializes a repository, and exercises real operations end to end.

## 6. Suggested Implementation Order

1. **`gust-core`:** `ObjectId`, blob read/write in the object database, then tree and commit. Verify against `git cat-file`.
2. **`gust-repo`:** `init`, `index`, `add`, `commit`, and refs. At this point you have a functioning version-control core.
3. **`gust-repo::log`**, then `gust-diff` and `show`.
4. **`undo`:** `revert` first, then `reset`.
5. **`gust-tui`:** start with the log view (the simplest read-only screen), then status/add, commit, show, and finally the undo confirmation.
