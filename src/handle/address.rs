use sha3::{Digest, Sha3_256};
use serde::{Serialize, Deserialize};

// TODO: fixed size?
/// An `Address` is the immutable hash of some `Data`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(Vec<u8>);

impl Address {
    pub fn new(content: &[u8]) -> Address {
        let mut hasher = Sha3_256::new();
        hasher.update(content);
        let result = hasher.finalize();
        Address(result.to_vec())
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}
