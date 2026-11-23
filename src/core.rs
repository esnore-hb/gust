

use std::{collections::BTreeMap, marker::PhantomData};

use crate::hash::{self, GitObject, HashStrategy, Serializable};

/// # Blob
/// Representa el contenido crudo de un archivo como secuencia de bytes (*Binary Large Object*)
/// 
/// Solo guarda los datos del archivo, sin su nombre ni permisos. El parámetro genérico `H`
/// define la [`HashStrategy`] con la que se calculará su id al usarlo como [`GitObject`].
pub struct Blob<H: HashStrategy> {
    /// Contenido crudo almacenado como un vector de bytes
    content: Vec<u8>,
    /// *HashStrategy* a utilizar
    _h: PhantomData<H>,
}

impl <H: HashStrategy>Serializable for Blob<H> {
    /// Devuelve una copia de los bytes crudos del archivo.
    fn serialize(&self) -> Vec<u8> {
        self.content.clone()
    }
}

impl <H: HashStrategy>GitObject for Blob<H> {
    type H = H;
    const KIND: &'static str = "blob";
}

/// # EntryKind
/// Indica el tipo de objeto al que apunta una entrada dentro de un [`Tree`]
pub enum EntryKind {
    /// Archivo individual con contenido crudo ([`Blob`])
    Blob,
    /// Un subdirectorio que agrupa otras entradas ([`Tree`])
    Tree,
}

/// # TreeEntry
/// Representa un elemento individual de un directorio ([`Tree`]).
/// 
/// Vincula un nombre con el tipo de objeto y el hash que lo identifica.
pub struct TreeEntry<H: HashStrategy> {
    /// Tipo de objeto que referencia (archivo o subdirectorio)
    pub kind: EntryKind,
    /// Nombre del archivo o subdirectorio (ej. `a.txt` o `src`)
    pub name: String,
    /// Identificador del objeto al que apunta esta entrada
    pub hash: H::Digest,
}

impl EntryKind {
    /// Devuelve el nombre del tipo, igual al `KIND` del objeto referenciado.
    fn as_str(&self) -> &'static str {
        match self {
            EntryKind::Blob => "blob",
            EntryKind::Tree => "tree",
        }
    }
}

impl<H: HashStrategy> TreeEntry<H> {
    /// Cantidad exacta de bytes que ocupa la entrada serializada.
    fn serialized_len(&self) -> usize {
        self.kind.as_str().len() + 1 + self.name.len() + 1 + self.hash.as_ref().len()
    }

    /// Escribe la entrada serializada al final de `buff`, sin reservas intermedias.
    fn write_into(&self, buff: &mut Vec<u8>) {
        buff.extend_from_slice(self.kind.as_str().as_bytes());
        buff.push(b' ');
        buff.extend_from_slice(self.name.as_bytes());
        buff.push(0);
        buff.extend_from_slice(self.hash.as_ref());
    }
}

impl<H: HashStrategy> Serializable for TreeEntry<H> {
    /// Formato: `<kind> <name>\0<digest crudo>`.
    ///
    /// El nombre termina en `\0` porque no puede aparecer en un nombre de archivo,
    /// y el digest tiene largo fijo, por lo que no necesita separador final.
    fn serialize(&self) -> Vec<u8> {
        let mut buff = Vec::with_capacity(self.serialized_len());
        self.write_into(&mut buff);
        buff
    }
}

/// # Tree
/// Representa un *snapshot* de un directorio mediante la lista de sus entradas.
/// 
/// Similar a Git, no guarda el contenido de los archivos directamente, sino referencias ([`TreeEntry`])
/// a cada archivo o subcarpeta.
/// ## Ejemplo
/// ```ignore
/// let file_entry = TreeEntry::<HashKetchum> {
///     kind: EntryKind::Blob,
///     name: String::from("README.md"),
///     hash: blob_id,
/// };
/// // No guarda archivos, ya que no almacena ningún blob. 
/// // En algún momento se procesó el blob del archivo y
/// // se calculó su hash.
/// 
/// let dir = Tree {
///     entries: vec![file_entry],
/// };
/// ```
pub struct Tree<H: HashStrategy> {
    pub entries: Vec<TreeEntry<H>>,
}

impl<H: HashStrategy> Serializable for Tree<H> {
    /// Concatena las entradas serializadas, ordenadas por los bytes de su nombre.
    ///
    /// El orden garantiza que el mismo directorio produzca siempre los mismos bytes,
    /// sin importar el orden en que se insertaron las entradas.
    fn serialize(&self) -> Vec<u8> {
        // Se ordenan referencias para no modificar ni clonar las entradas
        let mut sorted: Vec<&TreeEntry<H>> = self.entries.iter().collect();
        sorted.sort_unstable_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

        let total_len: usize = sorted.iter().map(|e| e.serialized_len()).sum();
        let mut buff = Vec::with_capacity(total_len);

        for entry in sorted {
            entry.write_into(&mut buff);
        }

        buff
    }
}

/// # Meta
/// Agrupa metadatos flexibles en pares clave-valor para describir un [`Commit`]
/// (ej. `"author"`, `"committer"` o `"date"`).
/// 
/// Usa un [`BTreeMap`] para mantener las claves **ordenadas** alfabéticamente,
/// lo que garantiza siempre la misma serialización de los mismos metadatos.
pub struct Meta {
    /// Diccionario clave valor: `{<data>:<description>}`
    pub fields: BTreeMap<String, String>,
}


impl Serializable for BTreeMap<String, String> {
    fn serialize(&self) -> Vec<u8> {
        let total_len: usize = self
            .iter()
            .map(|(k, v)| k.len() + v.len() + 2)
            .sum();
        let mut buff = Vec::with_capacity(total_len); 

        for (key, value) in self {
            buff.extend_from_slice(key.as_bytes());
            buff.push(b' ');
            buff.extend_from_slice(value.as_bytes());
            buff.push(b'\n');
        }
        
        buff
    }
}

impl Serializable for Meta {
    fn serialize(&self) -> Vec<u8> {
        self.fields.serialize()
    }
}


/// # Commit
/// Representa un *snapshot* del proyecto en un punto concreto del historial
/// 
/// Contiene el [`Tree`] de la raíz con los *commits* padres que le dieron origen,
/// junto a un mensaje descriptivo y sus metadatos (ej. `"author"`, `"committer"` o `"date"`).
/// 
/// ## Ejemplo
/// ```ignore
/// // Definimos los metadatos básicos del commit
/// let mut fields = BTreeMap::new();
/// fields.insert(String::from("author"), String::from("Linus Torvalds"));
///
/// // Creamos el commit apuntando al tree raíz y al commit anterior
/// let commit = Commit::<HashKetchum> {
///     root_hash: root_tree_id,
///     parents: vec![parent_commit_id],
///     message: String::from("feat: Agrega soporte inicial para commits"),
///     meta: Meta { fields },
/// }; 
/// ```
pub struct Commit<H: HashStrategy> {
    ///  Hash del directorio raíz del proyecto, es decir, del [`Tree`] raíz. Captura el estado completo del proyecto
    pub root_hash: H::Digest,
    /// Hash de los commits padres (vacío en el commit raíz, 2+ en un merge)
    pub parents: Vec<H::Digest>,
    /// Mensaje del commit
    pub message: String,
    /// Metadatos asociados al commit, como autor, fecha, etc.
    pub meta: Meta,
}
