use crate::structs::crypto::Encrypted;
use crate::structs::types::HasUuid;
use crate::{Address, PrivateKey};
use uuid::Uuid;

pub struct PrivateKeyHolder {
    pub id: Uuid,
    pub pk: PrivateKeyType,
}

pub enum PrivateKeyType {
    EthereumPk(EthereumPk3),
}

pub struct EthereumPk3 {
    pub address: Option<Address>,
    pub key: Encrypted,
}

impl HasUuid for PrivateKeyHolder {
    fn get_id(&self) -> Uuid {
        self.id
    }
}

impl PrivateKeyHolder {
    pub fn generate_id(&mut self) -> Uuid {
        self.id = Uuid::new_v4();
        self.id.clone()
    }
}
