use std::{collections::BTreeMap, marker::PhantomData};

use crate::hash::{Deserializable, DeserializeError, GitObject, HashStrategy, Serializable};

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

impl<H: HashStrategy> Serializable for Blob<H> {
    /// Devuelve una copia de los bytes crudos del archivo.
    ///
    /// Formato: `<content>`, sin header ni separadores.
    fn serialize(&self) -> Vec<u8> {
        self.content.clone()
    }
}

impl<H: HashStrategy> Deserializable for Blob<H> {
    /// Reconstruye el [`Blob`] copiando los bytes crudos recibidos.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        Ok(Self {
            content: bytes.to_vec(),
            _h: PhantomData,
        })
    }
}

impl<H: HashStrategy> GitObject for Blob<H> {
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

    /// Reconstruye el tipo a partir de su representación en bytes (`"blob"` o `"tree"`).
    fn from_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        match bytes {
            b"blob" => Ok(EntryKind::Blob),
            b"tree" => Ok(EntryKind::Tree),
            _ => Err(DeserializeError::InvalidFormat(
                "tipo de entrada desconocido",
            )),
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

    /// Lee una entrada desde el inicio de `bytes` y devuelve la entrada junto al resto del slice.
    fn parse_prefix(bytes: &[u8]) -> Result<(Self, &[u8]), DeserializeError> {
        let space = bytes
            .iter()
            .position(|&b| b == b' ')
            .ok_or(DeserializeError::UnexpectedEof)?;
        let kind = EntryKind::from_bytes(&bytes[..space])?;
        let rest = &bytes[space + 1..];

        let nul = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(DeserializeError::UnexpectedEof)?;
        let name = std::str::from_utf8(&rest[..nul])?;
        let after_nul = &rest[nul + 1..];

        if after_nul.len() < H::DIGEST_LEN {
            return Err(DeserializeError::UnexpectedEof);
        }
        let (digest_bytes, remaining) = after_nul.split_at(H::DIGEST_LEN);
        let hash = H::digest_from_bytes(digest_bytes)
            .ok_or(DeserializeError::InvalidFormat("largo de digest inválido"))?;

        Ok((
            Self {
                kind,
                name: name.to_owned(),
                hash,
            },
            remaining,
        ))
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

impl<H: HashStrategy> Deserializable for TreeEntry<H> {
    /// Reconstruye una entrada a partir del formato `<kind> <name>\0<digest crudo>`.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let (entry, remaining) = Self::parse_prefix(bytes)?;
        if !remaining.is_empty() {
            return Err(DeserializeError::InvalidFormat(
                "bytes sobrantes tras TreeEntry",
            ));
        }
        Ok(entry)
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
    /// Formato: `<entry_1><entry_2>...<entry_n>`, ver [`TreeEntry`].
    ///
    /// Concatena las entradas serializadas, ordenadas por los bytes de su nombre.
    /// Un [`Tree`] vacío produce 0 bytes.
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

impl<H: HashStrategy> Deserializable for Tree<H> {
    /// Reconstruye un [`Tree`] leyendo entradas consecutivas hasta consumir todos los bytes.
    ///
    /// Una secuencia vacía produce un [`Tree`] sin entradas.
    fn deserialize(mut bytes: &[u8]) -> Result<Self, DeserializeError> {
        let mut entries = Vec::new();
        while !bytes.is_empty() {
            let (entry, remaining) = TreeEntry::parse_prefix(bytes)?;
            entries.push(entry);
            bytes = remaining;
        }
        Ok(Self { entries })
    }
}

impl<H: HashStrategy> GitObject for Tree<H> {
    type H = H;
    const KIND: &'static str = "tree";
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
    /// Formato: una línea `<key> <value>\n` por cada par, en orden alfabético de clave.
    fn serialize(&self) -> Vec<u8> {
        let total_len: usize = self.iter().map(|(k, v)| k.len() + v.len() + 2).sum();
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

impl Deserializable for BTreeMap<String, String> {
    /// Reconstruye el mapa a partir de líneas `<key> <value>\n`.
    ///
    /// Una secuencia vacía produce un mapa vacío.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let mut rest = std::str::from_utf8(bytes)?;
        let mut map = BTreeMap::new();

        while !rest.is_empty() {
            let (line, tail) = rest
                .split_once('\n')
                .ok_or(DeserializeError::UnexpectedEof)?;
            let (key, value) = line.split_once(' ').ok_or(DeserializeError::InvalidFormat(
                "línea de metadato sin separador",
            ))?;
            map.insert(key.to_owned(), value.to_owned());
            rest = tail;
        }

        Ok(map)
    }
}

impl Serializable for Meta {
    /// Formato: el mismo que su [`BTreeMap`], `<key> <value>\n` por cada campo.
    fn serialize(&self) -> Vec<u8> {
        self.fields.serialize()
    }
}

impl Deserializable for Meta {
    /// Reconstruye [`Meta`] usando el mismo formato `<key> <value>\n` de su [`BTreeMap`].
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        Ok(Self {
            fields: BTreeMap::deserialize(bytes)?,
        })
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

impl<H: HashStrategy> Serializable for Commit<H> {
    /// Formato:
    /// ```text
    /// tree <digest raíz>\n
    /// parent <digest>\n      (0 o más, en orden de inserción)
    /// \n
    /// <meta: key value\n ...>
    /// \n
    /// <message>
    /// ```
    /// Los prefijos y las líneas en blanco separan cada sección sin ambigüedad,
    /// incluso si un digest crudo contiene el byte `\n`. El mensaje va al final,
    /// por lo que puede contener cualquier texto.
    fn serialize(&self) -> Vec<u8> {
        const TREE: &[u8] = b"tree ";
        const PARENT: &[u8] = b"parent ";

        let meta = self.meta.serialize();
        let parents_len: usize = self
            .parents
            .iter()
            .map(|p| PARENT.len() + p.as_ref().len() + 1)
            .sum();
        let total_len = TREE.len()
            + self.root_hash.as_ref().len()
            + 1
            + parents_len
            + 1
            + meta.len()
            + 1
            + self.message.len();
        let mut buff = Vec::with_capacity(total_len);

        buff.extend_from_slice(TREE);
        buff.extend_from_slice(self.root_hash.as_ref());
        buff.push(b'\n');

        // Orden de inserción: el primer padre es la rama donde se hizo el merge
        for parent in &self.parents {
            buff.extend_from_slice(PARENT);
            buff.extend_from_slice(parent.as_ref());
            buff.push(b'\n');
        }
        buff.push(b'\n');

        buff.extend_from_slice(&meta);
        buff.push(b'\n');

        buff.extend_from_slice(self.message.as_bytes());

        buff
    }
}

impl<H: HashStrategy> GitObject for Commit<H> {
    type H = H;
    const KIND: &'static str = "commit";
}
