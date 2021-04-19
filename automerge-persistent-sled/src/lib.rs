#![warn(missing_docs)]
#![warn(missing_crate_level_docs)]
#![warn(missing_doc_code_examples)]

//! A persister targetting [Sled](https://github.com/spacejam/sled).
//!
//! # Single persister
//!
//! ```rust
//! # use automerge_persistent::PersistentBackend;
//! # use automerge_persistent_sled::SledPersister;
//! # fn main() -> Result<(), sled::Error> {
//! let db = sled::Config::new().temporary(true).open()?;
//! let changes_tree = db.open_tree("changes")?;
//! let documents_tree = db.open_tree("documents")?;
//!
//! let persister = SledPersister::new(changes_tree, documents_tree, String::new());
//! let backend = PersistentBackend::load(persister);
//! # Ok(())
//! # }
//! ```
//!
//! # Multiple persisters sharing the same trees
//!
//! ```rust
//! # use automerge_persistent::PersistentBackend;
//! # use automerge_persistent_sled::SledPersister;
//! # fn main() -> Result<(), sled::Error> {
//! let db = sled::Config::new().temporary(true).open()?;
//! let changes_tree = db.open_tree("changes")?;
//! let documents_tree = db.open_tree("documents")?;
//!
//! let persister1 =
//!     SledPersister::new(changes_tree.clone(), documents_tree.clone(), "1".to_owned());
//! let backend1 = PersistentBackend::load(persister1);
//!
//! let persister2 = SledPersister::new(changes_tree, documents_tree, "2".to_owned());
//! let backend2 = PersistentBackend::load(persister2);
//! # Ok(())
//! # }
//! ```

use automerge_protocol::ActorId;

/// The key to use to store the document in the document tree
const DOCUMENT_KEY: &[u8] = b"document";

/// The persister that stores changes and documents in sled trees.
///
/// Changes and documents are kept in separate trees.
///
/// An optional prefix can be used in case multiple persisters may share the same trees.
#[derive(Debug)]
pub struct SledPersister {
    changes_tree: sled::Tree,
    document_tree: sled::Tree,
    prefix: String,
}

/// Possible errors from persisting.
#[derive(Debug, thiserror::Error)]
pub enum SledPersisterError {
    /// Internal errors from sled.
    #[error(transparent)]
    SledError(#[from] sled::Error),
}

impl SledPersister {
    /// Construct a new persister.
    pub fn new(changes_tree: sled::Tree, document_tree: sled::Tree, prefix: String) -> Self {
        Self {
            changes_tree,
            document_tree,
            prefix,
        }
    }

    /// Make a key from the prefix, actor_id and sequence_number.
    ///
    /// Converts the actor_id to bytes and appends the sequence_number in big endian form.
    fn make_key(&self, actor_id: &ActorId, seq: u64) -> Vec<u8> {
        let mut key = self.prefix.as_bytes().to_vec();
        key.extend(&actor_id.to_bytes());
        key.extend(&seq.to_be_bytes());
        key
    }

    fn make_document_key(&self) -> Vec<u8> {
        let mut key = self.prefix.as_bytes().to_vec();
        key.extend(DOCUMENT_KEY);
        key
    }
}

impl automerge_persistent::Persister for SledPersister {
    type Error = SledPersisterError;

    /// Get all of the current changes.
    fn get_changes(&self) -> Result<Vec<Vec<u8>>, Self::Error> {
        self.changes_tree
            .iter()
            .values()
            .map(|v| v.map(|v| v.to_vec()).map_err(Self::Error::SledError))
            .collect()
    }

    /// Insert all of the given changes into the tree.
    fn insert_changes(&mut self, changes: Vec<(ActorId, u64, Vec<u8>)>) -> Result<(), Self::Error> {
        for (a, s, c) in changes {
            let key = self.make_key(&a, s);
            self.changes_tree.insert(key, c)?;
        }
        Ok(())
    }

    /// Remove all of the given changes from the tree.
    fn remove_changes(&mut self, changes: Vec<(&ActorId, u64)>) -> Result<(), Self::Error> {
        for (a, s) in changes {
            let key = self.make_key(a, s);
            self.changes_tree.remove(key)?;
        }
        Ok(())
    }

    /// Retrieve the document from the tree.
    fn get_document(&self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self
            .document_tree
            .get(self.make_document_key())?
            .map(|v| v.to_vec()))
    }

    /// Set the document in the tree.
    fn set_document(&mut self, data: Vec<u8>) -> Result<(), Self::Error> {
        self.document_tree.insert(self.make_document_key(), data)?;
        Ok(())
    }
}
