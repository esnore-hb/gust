//! Define los objetos que forman el historial de un repositorio: archivos ([`Blob`]),
//! directorios ([`Tree`], compuesto por [`TreeEntry`]) y *snapshots* del proyecto
//! ([`Commit`], con sus metadatos [`Meta`]).
//!
//! Los objetos son **inmutables**: un cambio nunca modifica uno existente, sino que crea otro.
//! Lo que sí avanza con cada *commit* (las ramas y `HEAD`) vive en [`crate::refs`], y el
//! almacenamiento de los objetos, en [`crate::store`].
//!
//! ## Repaso
//! ### Los objetos de Git
//! Git representa un proyecto completo con solo tres tipos de objetos:
//! - **blob:** el contenido de un archivo y nada más. Como no guarda el nombre ni los
//!   permisos, dos archivos con el mismo contenido comparten *blob*, que se almacena una vez.
//! - **tree:** un directorio. Cada una de sus entradas asocia un nombre con el hash de un
//!   *blob* (archivo) o de otro *tree* (subdirectorio).
//! - **commit:** una "foto" del proyecto. Apunta al *tree* de la carpeta raíz y a los *commits*
//!   que lo preceden (sus padres), y añade un mensaje y metadatos como autor o fecha.
//!
//! ```text
//! commit c2 ──parent──▶ commit c1
//!   │
//!   └─ tree (raíz)
//!        ├─ README.md ─▶ blob
//!        └─ src        ─▶ tree
//!                           └─ main.rs ─▶ blob
//! ```
//!
//! ### El grafo de objetos
//! Los objetos no se contienen entre sí: se **referencian por su hash**. El hash de un *tree*
//! depende de los hash de sus entradas, y el de un *commit*, del de su *tree* raíz. Por eso, al
//! modificar un solo archivo cambian en cadena el id de su *blob*, el de cada *tree* que lo
//! contiene y el del *commit*. Una estructura así se llama árbol de Merkle y tiene dos ventajas:
//! - **Reutilización:** un subdirectorio sin cambios conserva su id, así que el nuevo
//!   *commit* apunta al mismo *tree* que el anterior.
//! - **Integridad:** el id de un *commit* resume todo el proyecto en ese momento. No se puede
//!   alterar un archivo antiguo sin que cambien los ids de todos los *commits* posteriores.
//!
//! El historial forma además un grafo dirigido acíclico (*DAG*): cada *commit* apunta a sus
//! padres (ninguno el primero, dos o más un *merge*). No puede haber ciclos, porque para
//! apuntar a un *commit* hay que conocer su hash y, por lo tanto, ya debe existir.
//!
//! ### Representación única
//! Para que un objeto tenga un único id, debe haber una única secuencia de bytes válida que lo
//! represente. Por eso la serialización ordena las entradas de un [`Tree`] por nombre y las
//! claves de [`Meta`] alfabéticamente. La deserialización, a su vez, **rechaza** los bytes que
//! no respetan ese orden o que repiten nombres o claves: si los aceptara, al volver a
//! serializar el objeto obtendríamos bytes distintos y, por lo tanto, otro id.
//!
//! ## Diseño
//! [`Blob`], [`Tree`], [`TreeEntry`] y [`Commit`] son genéricos sobre
//! `H: HashStrategy` (ver [`crate::hash`]), así que no dependen de ningún algoritmo concreto.
//! Los tres que se almacenan por sí mismos ([`Blob`], [`Tree`] y [`Commit`]) implementan
//! [`Serializable`], [`Deserializable`] y [`GitObject`], cada uno con su `KIND` (`"blob"`,
//! `"tree"` o `"commit"`). [`TreeEntry`] y [`Meta`] solo existen como parte de otro objeto.
//!
//! El cuerpo serializado tiene este formato:
//!
//! | Tipo          | Formato                                                                 |
//! |---------------|-------------------------------------------------------------------------|
//! | [`Blob`]      | `<content>`, los bytes del archivo tal cual                             |
//! | [`TreeEntry`] | `<kind> <name>\0<digest crudo>`                                         |
//! | [`Tree`]      | sus entradas concatenadas, ordenadas por **nombre**                     |
//! | [`Meta`]      | una línea `<key> <value>\n` por campo, ordenadas por clave              |
//! | [`Commit`]    | `tree <digest>\n`, `parent <digest>\n` por padre, `\n`, meta, `\n`, mensaje |
//!
//! Para simplificar, nos apartamos de Git en algunos puntos:
//! - Los *digests* se escriben siempre en bytes crudos (Git usa hexadecimal en los *commits*).
//! - [`TreeEntry`] guarda su tipo ([`EntryKind`]) en lugar de un modo numérico como `100644`.
//! - [`Meta`] es un mapa clave-valor libre, sin líneas fijas `author` y `committer`.
//!
//! Además, [`validate_entry_name`] rechaza nombres como `".."` o con `'/'`, para que un *tree*
//! malicioso no pueda escribir archivos fuera del repositorio al restaurarlo.
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **Copia completa en `Blob::serialize`:** la firma actual de [`Serializable`] obliga a
//!   copiar todo el contenido en un `Vec<u8>` nuevo. Es aceptable para el MVP, pero costoso
//!   con archivos grandes.
//! - **Caracteres reservados en [`Meta`]:** la serialización asume que las claves no contienen
//!   `' '` ni `'\n'`, y que los valores no contienen `'\n'`. De lo contrario, el objeto no se
//!   lee igual que se escribió o se confunde el separador `\n\n` de [`Commit`]. Falta validar
//!   al insertar o escapar esos caracteres.
//! - **Alcance de [`EntryKind`]:** solo distingue archivos regulares y directorios; no modela
//!   permisos de ejecución ni enlaces simbólicos (*symlinks*).
//! - **Recorridos del historial:** [`Commit`] no tiene un número de generación topológica, así
//!   que buscar ancestros puede obligar a recorrer ramas completas del *DAG*.
//!
//! [`Serializable`]: crate::hash::Serializable
//! [`Deserializable`]: crate::hash::Deserializable
//! [`GitObject`]: crate::hash::GitObject

mod blob;
mod commit;
mod meta;
mod tree;

// Reexporta los tipos públicos con la ruta de siempre
pub use blob::Blob;
pub use commit::Commit;
pub use meta::Meta;
pub use tree::{EntryKind, Tree, TreeEntry, validate_entry_name};