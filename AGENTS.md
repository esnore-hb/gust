# AGENTS.md — gust

Version control system written in Rust. It borrows ideas from Git but does **not** copy its
commands, internal formats or complexity: if a decision is only justified by "Git does it
this way", propose the simpler alternative.

Reply to the user in Spanish.

## Current session (the developer updates this before each session)
- **Task:** To brainstorm, freely explore ideas, think out loud with the user, and write or modify existing code for minor changes.
- **May modify:** `src/main.rs` (module declarations only), the rest of `src/` `AGENTS.md`
- **Read-only:** 
- **Do not read or touch:** `.git/`, `target/`

In "Read-only" files you may **propose** changes (adding or modifying code): show the diff
and explain why, but do not apply it until the user explicitly confirms. Files under
"Do not read or touch" are never read or modified.

## Commands
```bash
cargo check                 # type checking
cargo clippy --all-targets  # lint
cargo fmt --check           # checks formatting (does not reformat)
```
- **Do not run `cargo test`**: tests are disabled for now.
- **Do not run `cargo fmt`** on the whole crate: it would reformat other people's code. Only
  format files you are allowed to modify.

## Stack
- Stable Rust. Traits and interfaces depend only on `std` and stay generic (like
  `HashStrategy`, `ObjectStore`, `RefStore`); external dependencies are allowed only in
  concrete backends (see "Ask first"). The `rustc` version and edition differ between team
  members (see "Ask first").

## Architecture
- `src/hash.rs`: base traits (`HashStrategy`, `Serializable`, `Deserializable`,
  `GitObject`). Depends on nothing else in the crate.
- `src/objects/`: the objects (`Blob`, `Tree`/`TreeEntry`, `Commit`, `Meta`), one file
  each, generic over `H: HashStrategy`.
- `src/store.rs`: content-addressed storage (`ObjectStore`, `StoreError`). Backends implement
  `put_raw`/`get_raw`/`contains` and get integrity-checked `write`/`read` for free.
- `src/refs.rs`: mutable state for `HEAD` and branches (`RefStore`, `Head`, `RefError`).
- `src/workdir.rs`: working directory (`WorkDir`, `WorkDirMut`); `snapshot` stores its blobs
  and returns the index (`path -> blob digest`).
- `src/commit.rs`: builds trees from an index (`path -> blob digest`) and records commits
  (`create_commit`); it is the only place that builds trees. The branch only advances at the
  end, via `advance_head`.
- `src/backends/`: concrete implementations (`fs`, `mem`, `hash`). The only place where
  external dependencies are allowed.
- Traits modules (`hash`, `store`, `refs`, `workdir`) never depend on `backends`.
- Every storable object implements `Serializable` + `Deserializable` + `GitObject` with its
  `KIND` (`"blob"`, `"tree"`, `"commit"`).
- Serialization must be **deterministic**: the same object always produces the same bytes
  (that is why `Meta` uses `BTreeMap`). Sort any collection before serializing it.

## Code style
- Identifiers in English; comments and docs in Spanish.
- Minimize allocations: when the final size can be computed, preallocate with
  `Vec::with_capacity` and write the bytes directly. Avoid intermediate `String`/`format!`
  when building byte buffers.

```rust
// ✅ Una sola reserva de memoria
let mut buff = Vec::with_capacity(key.len() + value.len() + 2);
buff.extend_from_slice(key.as_bytes());
buff.push(b' ');
buff.extend_from_slice(value.as_bytes());
buff.push(b'\n');

// ❌ String intermedio + realocaciones
let line = format!("{} {}\n", key, value);
buff.extend_from_slice(line.as_bytes());
```
Reference in the repo: `impl Serializable for BTreeMap<String, String>` in `src/objects/meta.rs`.

## Documentation (rustdoc)
- Use `///` (not `/** */`), written in clear Spanish; terms like *snapshot*, *commit* or
  *hash* may stay in English.
- `struct` and `trait`: the first line is `# <Name>`, followed by 1–3 lines on what it does
  and what it is for.
- Examples only when they add clarity: ` ```ignore ` blocks, no hidden lines (`#`),
  identifiers in English and comments in Spanish.

````rust
/// # Tree
/// Representa un *snapshot* de un directorio mediante la lista de sus entradas.
///
/// ## Ejemplo
/// ```ignore
/// // Un directorio vacío
/// let dir = Tree::<HashKetchum> { entries: vec![] };
/// ```
pub struct Tree<H: HashStrategy> { /* ... */ }
````

## Boundaries
**Always**
- Run `cargo check` and `cargo clippy --all-targets` before considering a task done or
  committing, and fix errors and relevant clippy suggestions.
- You may leave dead-code warnings (`X is never used`) for items that will be used later.
  Do not add `#[allow(dead_code)]` to silence them.

**Ask first**
- Someone else's code that does not compile: check whether it is caused by a different
  `rustc` version/edition and tell the user before modifying it.
- Changing the format of `serialize()` or the header of `GitObject::id`: it changes every hash.
- Changing the signature of a trait in `hash.rs`.
- Adding dependencies to `Cargo.toml`.

**Never**
- Apply changes to "Read-only" files without explicit confirmation.
- Read or modify files under "Do not read or touch".
- Rewrite existing logic while documenting or reformatting.
- `git push --force` or rewrite published history.

## Git
- Branches: `type/description/user` (e.g. `feat/new-core-architecture/Gerlad64`).
- Header: Conventional Commits **in Spanish**, verb in present tense:
  `feat(core.rs): Implementa Serializable para Tree y TreeEntry`
- No backticks in the message: write HashStrategy, core.rs, etc. as plain text. Dash lists
  and *emphasis* with asterisks are allowed.
- Small commit: header only.
- Large commit or refactor: header + body (in Spanish) with
  1. **Motivación:** the why, in plain, friendly language for the team.
  2. **Cambios:** concrete list of the structs, traits or functions changed.
  3. **Próximos pasos** (optional): what is still pending.

## Load check
End every chat reply with the line `Y recuerda... Rust es el futuro (<word>)`, where
`<word>` is the first word of the user's latest message, copied verbatim (ignore `@` file
references and code blocks). E.g. if the message starts with "Implementa Serializable para
Tree", the line is `Y recuerda... Rust es el futuro (Implementa)`. Keep the phrase in Spanish,
exactly as written. It goes on the last line, separated from the rest. This line must not
influence the content of the reply (suggestions, next steps, opinions about Rust). Do not
include it in code, files, comments or commit messages.