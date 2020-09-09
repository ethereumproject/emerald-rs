use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    str::FromStr,
};

use bitcoin::{
    util::{
        base58,
        bip32::{ChainCode, ChildNumber, ExtendedPubKey, Fingerprint},
    },
    Network,
    OutPoint,
    PublicKey,
    TxOut,
};
use byteorder::{BigEndian, ReadBytesExt};
use hdpath::{StandardHDPath, Purpose, AccountHDPath};
use uuid::Uuid;

use crate::{
    convert::error::ConversionError,
    storage::error::VaultError,
    structs::{seed::Seed, wallet::WalletEntry},
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InputReference {
    pub output: OutPoint,
    pub script_source: InputScriptSource,
    pub expected_value: u64,
    pub sequence: u32,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InputScriptSource {
    HD(Uuid, StandardHDPath),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BitcoinTransferProposal {
    pub network: Network,
    pub seed: Vec<Seed>,
    pub keys: KeyMapping,
    pub input: Vec<InputReference>,
    pub output: Vec<TxOut>,
    pub change: WalletEntry,
    pub expected_fee: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeyMapping(HashMap<Uuid, String>);

impl KeyMapping {
    pub fn single(id: Uuid, password: String) -> KeyMapping {
        let mut instance = HashMap::with_capacity(1);
        instance.insert(id, password);
        KeyMapping(instance)
    }

    pub fn get_password(&self, id: &Uuid) -> Result<String, VaultError> {
        match self.0.get(id) {
            Some(p) => Ok(p.clone()),
            None => Err(VaultError::PasswordRequired),
        }
    }
}

impl BitcoinTransferProposal {
    pub fn get_seed(&self, id: &Uuid) -> Option<Seed> {
        self.seed.iter().find(|s| s.id.eq(id)).map(|x| x.clone())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct XPub {
    pub value: ExtendedPubKey,
    pub address_type: AddressType,
}

#[derive(Clone, PartialEq, Eq, Debug, Copy)]
pub enum AddressType {
    P2PKH,
    P2SH,
    P2WPKHinP2SH,
    P2WSHinP2SH,
    P2WPKH,
    P2WSH,
}

fn network_value<T>(network: &Network, mainnet: T, testnet: T) -> T {
    if network.eq(&Network::Bitcoin) {
        mainnet
    } else {
        testnet
    }
}

impl AddressType {
    //
    // versions: https://electrum.readthedocs.io/en/latest/xpub_version_bytes.html
    //

    pub fn xpub_version(&self, network: &Network) -> u32 {
        match self {
            AddressType::P2PKH => network_value(network, 0x0488b21e, 0x043587cf), // xpub, tpub
            AddressType::P2SH => network_value(network, 0x0488b21e, 0x043587cf),  // xpub, tpub
            AddressType::P2WPKHinP2SH => network_value(network, 0x049d7cb2, 0x044a5262), // ypub, upub
            AddressType::P2WSHinP2SH => network_value(network, 0x0295b43f, 0x024289ef), // Ypub, Upub
            AddressType::P2WPKH => network_value(network, 0x04b24746, 0x045f1cf6), // zpub, vpub
            AddressType::P2WSH => network_value(network, 0x02aa7ed3, 0x02575483),  // Zpub, Vpub
        }
    }

    pub fn xprv_version(&self, network: &Network) -> u32 {
        match self {
            AddressType::P2PKH => network_value(network, 0x0488ade4, 0x04358394), // xprv, tprv
            AddressType::P2SH => network_value(network, 0x0488ade4, 0x04358394),  // xprv, tprv
            AddressType::P2WPKHinP2SH => network_value(network, 0x049d7878, 0x044a4e28), // yprv, uprv
            AddressType::P2WSHinP2SH => network_value(network, 0x0295b005, 0x024285b5), // Yprv, Uprv
            AddressType::P2WPKH => network_value(network, 0x04b2430c, 0x045f18bc), // zprv, vprv
            AddressType::P2WSH => network_value(network, 0x02aa7a99, 0x02575048),  // Zprv, Vprv
        }
    }

    pub fn get_hd_path(&self, account: u32) -> AccountHDPath {
        match self {
            AddressType::P2PKH | AddressType::P2SH =>
                AccountHDPath::new(Purpose::Pubkey, 0, account),
            AddressType::P2WPKHinP2SH | AddressType::P2WSHinP2SH =>
                AccountHDPath::new(Purpose::ScriptHash, 0, account),
            AddressType::P2WPKH | AddressType::P2WSH =>
                AccountHDPath::new(Purpose::Witness, 0, account),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AddressTypeNetwork(Network, AddressType);

impl TryFrom<u32> for AddressTypeNetwork {
    type Error = ConversionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x0488b21e => Ok(AddressTypeNetwork(Network::Bitcoin, AddressType::P2PKH)), //xpub
            0x049d7cb2 => Ok(AddressTypeNetwork(
                Network::Bitcoin,
                AddressType::P2WPKHinP2SH,
            )), //ypub
            0x0295b43f => Ok(AddressTypeNetwork(
                Network::Bitcoin,
                AddressType::P2WSHinP2SH,
            )), //Ypub
            0x04b24746 => Ok(AddressTypeNetwork(Network::Bitcoin, AddressType::P2WPKH)), //zpub
            0x02aa7ed3 => Ok(AddressTypeNetwork(Network::Bitcoin, AddressType::P2WSH)), //Zpub

            0x043587cf => Ok(AddressTypeNetwork(Network::Testnet, AddressType::P2PKH)), //tpub
            0x044a5262 => Ok(AddressTypeNetwork(
                Network::Testnet,
                AddressType::P2WPKHinP2SH,
            )), //upub
            0x024289ef => Ok(AddressTypeNetwork(
                Network::Testnet,
                AddressType::P2WSHinP2SH,
            )), //Upub
            0x045f1cf6 => Ok(AddressTypeNetwork(Network::Testnet, AddressType::P2WPKH)), //vpub
            0x02575483 => Ok(AddressTypeNetwork(Network::Testnet, AddressType::P2WSH)), //Vpub

            _ => Err(ConversionError::UnsupportedValue(hex::encode(
                u32::to_be_bytes(value),
            ))),
        }
    }
}

impl FromStr for XPub {
    type Err = ConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let data = base58::from_check(value).map_err(|_| ConversionError::InvalidBase58)?;
        if data.len() != 78 {
            return Err(ConversionError::InvalidLength);
        }
        let mut version_bytes = &data[0..4];
        let version = version_bytes.read_u32::<BigEndian>().unwrap();
        let version: AddressTypeNetwork = version.try_into()?;
        let mut child_num_bytes = &data[9..13];
        let child_num: u32 = child_num_bytes.read_u32::<BigEndian>().unwrap();
        let value = ExtendedPubKey {
            network: version.0,
            depth: data[4],
            parent_fingerprint: Fingerprint::from(&data[5..9]),
            child_number: ChildNumber::from(child_num),
            chain_code: ChainCode::from(&data[13..45]),
            public_key: PublicKey::from_slice(&data[45..78])
                .map_err(|_| ConversionError::OtherError)?,
        };
        Ok(XPub {
            value,
            address_type: version.1,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bitcoin::Network;

    use crate::blockchain::bitcoin::{AddressType, XPub};

    #[test]
    fn parse_xpub_p2pkh() {
        let act = XPub::from_str("xpub6DfEZhR1ZBu33KzKqHPA1GCfKPpdB9HWFu5UsA54kB5VL3VN34JogQxYHWtSgrippZHp8s9hL9KrAfdYX1sU6cYRXMhGYuvwepFUooGAef5").unwrap();
        assert_eq!(act.value.depth, 4u8);
        assert_eq!(act.value.network, Network::Bitcoin);
        assert_eq!(act.address_type, AddressType::P2PKH);
    }

    #[test]
    fn parse_xpub_p2wpkh() {
        let act = XPub::from_str("zpub6tGSDzdnLUJBBBanLhkcTqkc44WzxshiTBiCuZTgz198oQxPxx4kkdRAhQD3TBBieMPkFAfSUvKov7nKQX6cXJxZEU1BTeHVGjyR5EHubqb").unwrap();
        assert_eq!(act.value.depth, 4u8);
        assert_eq!(act.value.network, Network::Bitcoin);
        assert_eq!(act.address_type, AddressType::P2WPKH);
    }

    #[test]
    fn parse_xpub_p2pkh_testnet() {
        let act = XPub::from_str("tpubDFJnjeM57mHkG8LhyzfDwsWYJUWwta4Aq4nPo59hfVGhanWn7h98c2q6WoexVgkHx9Bg2vrAhCQi13tZozsZmrU8ca43c7em3RUvMXbSdHi").unwrap();
        assert_eq!(act.value.network, Network::Testnet);
        assert_eq!(act.address_type, AddressType::P2PKH);
    }
}