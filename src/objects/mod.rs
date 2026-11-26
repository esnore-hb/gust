//! Define los objetos con los que se guarda el historial de un repositorio: archivos
//! ([`Blob`]), directorios ([`Tree`] y [`TreeEntry`]) y *snapshots* del proyecto ([`Commit`]
//! junto a sus metadatos [`Meta`]).
//!
//! Estos objetos son **inmutables**: una vez creados no cambian, solo se crean otros nuevos.
//! Lo que sí cambia con cada *commit* (ramas y `HEAD`) vive en [`crate::refs`], y el lugar
//! donde se guardan los objetos se define en [`crate::store2`].
//!
//! ## Repaso
//! ### Los objetos de Git
//! Git representa un proyecto completo con solo tres tipos de objetos:
//! - **blob:** el contenido de un archivo, nada más. No guarda su nombre ni sus permisos, por
//!   lo que dos archivos con el mismo contenido comparten el mismo *blob* (y se guarda una vez).
//! - **tree:** un directorio. Es una lista de entradas, donde cada una asocia un nombre con el
//!   hash de un *blob* (archivo) o de otro *tree* (subdirectorio).
//! - **commit:** una "foto" del proyecto. Apunta al *tree* de la carpeta raíz, a los *commits*
//!   padres que lo precedieron, y agrega un mensaje y metadatos (autor, fecha, etc.).
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
//! ### El grafo de objetos (DAG)
//! Los objetos no se contienen entre sí: se **referencian por su hash**. Como el hash de un
//! *tree* depende de los hashes de sus entradas, y el de un *commit* depende del hash de su
//! *tree* raíz, modificar un solo archivo cambia en cadena el id de su *blob*, el de cada
//! *tree* que lo contiene y el del *commit*. A esta estructura se le llama árbol de Merkle, y
//! tiene dos ventajas:
//! - **Reutilización:** un subdirectorio que no cambió conserva su id, así que el nuevo
//!   *commit* simplemente apunta al mismo *tree* que el anterior.
//! - **Integridad:** el id de un *commit* "resume" todo el proyecto en ese momento; es
//!   imposible alterar un archivo antiguo sin cambiar el id de todos los *commits* posteriores.
//!
//! El historial, por su parte, forma un grafo dirigido acíclico (*DAG*): cada *commit* apunta
//! a sus padres (ninguno en el primer *commit*, dos o más en un *merge*) y nunca puede haber
//! ciclos, pues para apuntar a un *commit* hay que conocer su hash, es decir, debe existir antes.
//!
//! ### Forma canónica
//! Para que un objeto tenga un único id, debe tener una única representación en bytes válida
//! (su *forma canónica*). Por eso la serialización ordena las entradas de un [`Tree`] por nombre
//! y las claves de [`Meta`] alfabéticamente, y la deserialización **rechaza** bytes que no estén
//! en forma canónica (ej. entradas desordenadas o claves repetidas): si los aceptara, al volver
//! a serializar el objeto obtendríamos bytes distintos y, por lo tanto, otro id.
//!
//! ## Diseño
//! Todos los objetos son genéricos sobre `H: HashStrategy` (ver [`crate::hash`]), por lo que
//! nunca dependen de un algoritmo concreto. Cada objeto almacenable implementa
//! [`Serializable`], [`Deserializable`] y [`GitObject`] con su `KIND` (`"blob"`, `"tree"` o
//! `"commit"`). El formato del cuerpo de cada uno es:
//!
//! | Objeto        | Formato del cuerpo                                                      |
//! |---------------|-------------------------------------------------------------------------|
//! | [`Blob`]      | `<content>`, los bytes del archivo tal cual                             |
//! | [`TreeEntry`] | `<kind> <name>\0<digest crudo>`                                         |
//! | [`Tree`]      | sus entradas concatenadas, ordenadas por **nombre**                     |
//! | [`Meta`]      | una línea `<key> <value>\n` por campo, ordenadas por clave              |
//! | [`Commit`]    | `tree <digest>\n`, `parent <digest>\n` por padre, `\n`, meta, `\n`, mensaje |
//!
//! Algunas diferencias deliberadas con Git, en favor de la simplicidad:
//! - Los *digests* se escriben en bytes crudos en todos los objetos (Git usa hexadecimal en
//!   los *commits*).
//! - [`TreeEntry`] guarda su tipo ([`EntryKind`]) en vez de un modo numérico como `100644`.
//! - [`Meta`] es un mapa clave-valor libre en lugar de líneas fijas `author`/`committer`.
//! - Al deserializar un [`TreeEntry`] se rechazan nombres como `".."` o con `'/'`, para que un
//!   *tree* malicioso no pueda escribir archivos fuera del repositorio al restaurarlo.
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **Clonación completa en `Blob::serialize`:** Por la firma actual de `Serializable`,
//!   serializar un `Blob` copia todo `self.content` en un nuevo `Vec<u8>` (aceptable para el
//!   MVP; costoso en archivos grandes).
//! - **Caracteres reservados en `Meta`:** `BTreeMap::serialize` asume que las claves no contienen
//!   `' '` ni `'\n'` y que los valores no contienen `'\n'`, ya que romperían la simetría con
//!   `deserialize` o el separador `\n\n` de `Commit`. Pendiente validar al insertar o escapar.
//! - **Alcance de `EntryKind`:** Solo modela archivos regulares (`Blob`) y directorios (`Tree`);
//!   aún no distingue permisos de ejecución ni enlaces simbólicos (*symlinks*).
//! - **Aceleración de recorridos:** `Commit` aún no incluye un número de generación topológica
//!   para podar búsquedas de ancestros sin recorrer ramas completas del DAG.


mod blob;
mod commit;
mod meta;
mod tree;

// Reexporta los tipos públicos con la ruta de siempre
pub use blob::Blob;
pub use commit::Commit;
pub use meta::Meta;
pub use tree::{EntryKind, Tree, TreeEntry, validate_entry_name};