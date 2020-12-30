use std::{
    path::{Path, PathBuf},
    collections::HashMap,
};

use crate::{
    handle::Address,
    datastore::{Storable, Cache, Delta, Branch}, 
    keys::KeyPublic,
};

// TODO: how to make it so the user does not have to have the whole history on-hahd
// but can still work with and modify that which they do have?

// TODO: how to make it so that user can modify agent without worrying about datastore?
// TODO: implement some form of logarithmic history,
// So changes are cheap and can be made often,
// but will be 'lost' (i.e. combined) over time as to preserve space.
// i.e. when editing, it's useful to be able to undo every character,
// but when looking at a five-year-old document,
// the exact timing of changes to every character is no longer as relevant.

// TODO: should a datastore be content?
// Probably not.
/// A datastore is a database of two parts:
/// The first part is the content-addressed storage.
/// This is on-disk, with a cache of commonly used items.
/// The second part is a tree of identities.
/// This is built out in-memory, from the relations contained from the content-addressed code.
#[derive(Debug)]
pub struct Datastore {
    keys:  HashMap<KeyPublic, Branch>,
    cache: Cache,
}

impl Datastore {
    pub fn new(path: &Path) -> Result<Datastore, String> {
        Ok(Datastore {
            local: Branch::new(&path.join("branches/local"))?,
            cache: Cache::new(&path.join("kv"))?,
        })
    }

    fn load(&self, address: &Address) -> Result<Content, String> {
        // TODO: schedule on network if not in cache?
        return self.cache.load(address);
    }

    fn store(&mut self, content: &Content) -> Result<Address, String> {
        // TODO: notify network after written locally?
        return self.cache.store(content);
    }

    // TODO: make more general than deltas
    fn resolve(&mut self, delta: &Delta) -> Result<Content, String> {
        match delta {
            Delta::Base { base, .. } => Ok(base.clone()),
            Delta::Tip  { previous, difference, checksum } => {
                // TODO: check datastore cache, then history.
                let prev_content = self.load(&previous)?;
                let next_content = Diff::apply(&prev_content, &difference)
                    .ok_or("Could not apply diff")?;
                // check the checksum
                if &Address::new(&Storable::try_to_bytes(&next_content)
                        .ok_or("Could not serialize content")?
                ) != checksum {
                    return Err("Checksum of delta does not match".to_string());
                }
                return Ok(next_content);
            }
        }
    }

    // TODO: commit
    // NOTE: just local for now!
    pub fn update(&mut self, content: &Content) -> Result<(), String> {
        self.store(content)?;
        let history  = self.local.history(&Contentable::location(content))?;
        let previous = self.load(&history.head)?;
        self.local.update(&previous, content)?;
        Ok(())
    }

    pub fn register(&mut self, content: &Content) -> Result<(), String> {
        self.store(content)?;
        self.local.register(content.clone())?;
        // todo!()
        // TODO: propogate changes up location chain?
        Ok(())
    }
}
