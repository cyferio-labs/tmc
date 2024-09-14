use bincode;
use serde::{Deserialize, Serialize};
use tfhe::{
    generate_keys, prelude::*,
    shortint::parameters::PARAM_GPU_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_3_KS_PBS, ClientKey,
    CompressedPublicKey, CompressedServerKey, ConfigBuilder, PublicKey, ServerKey,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerializationStatus {
    NotSerialized,
    PartiallySerialized,
    FullySerialized,
}

#[derive(Serialize, Deserialize)]
pub struct FheKeyConfig {
    pub fhe_public_key: Vec<u8>,
    pub fhe_server_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FheKeyGenConfig {
    pub public_key: Vec<u8>,
    pub server_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub serialization_status: SerializationStatus,
}

impl FheKeyGenConfig {
    pub fn new() -> Self {
        Self {
            public_key: Vec::new(),
            server_key: Vec::new(),
            private_key: Vec::new(),
            serialization_status: SerializationStatus::NotSerialized,
        }
    }

    pub fn serialize_keys(
        &mut self,
        client_key: &ClientKey,
        server_key: &CompressedServerKey,
        public_key: &CompressedPublicKey,
    ) {
        let public_key_result = serialize_key(public_key, "public");
        let server_key_result = serialize_key(server_key, "server");
        let private_key_result = serialize_key(client_key, "private");

        self.serialization_status = match (
            public_key_result.is_ok(),
            server_key_result.is_ok(),
            private_key_result.is_ok(),
        ) {
            (true, true, true) => SerializationStatus::FullySerialized,
            (false, false, false) => SerializationStatus::NotSerialized,
            _ => SerializationStatus::PartiallySerialized,
        };

        self.public_key = public_key_result.unwrap_or_else(|_| Vec::new());
        self.server_key = server_key_result.unwrap_or_else(|_| Vec::new());
        self.private_key = private_key_result.unwrap_or_else(|_| Vec::new());
    }

    pub fn deserialize_keys(&self) -> Option<(ClientKey, ServerKey, PublicKey)> {
        let client_key: ClientKey = deserialize_key(&self.private_key, "private")?;
        let compressed_server_key: CompressedServerKey =
            deserialize_key(&self.server_key, "server")?;
        let compressed_public_key: CompressedPublicKey =
            deserialize_key(&self.public_key, "public")?;

        // decompress keys
        let server_key = compressed_server_key.decompress();
        let public_key = compressed_public_key.decompress();

        Some((client_key, server_key, public_key))
    }

    pub fn is_fully_serialized(&self) -> bool {
        self.serialization_status == SerializationStatus::FullySerialized
    }
}

pub fn fhe_key_gen() -> FheKeyGenConfig {
    let config = ConfigBuilder::with_custom_parameters(
        PARAM_GPU_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_3_KS_PBS,
        None,
    )
    .build();
    let client_key = ClientKey::generate(config);
    let compressed_public_key = CompressedPublicKey::new(&client_key);
    let compressed_server_key = CompressedServerKey::new(&client_key);

    let mut fhe_keygen_config = FheKeyGenConfig::new();
    fhe_keygen_config.serialize_keys(&client_key, &compressed_server_key, &compressed_public_key);
    fhe_keygen_config
}

fn serialize_key<T: ?Sized + serde::Serialize>(key: &T, key_name: &str) -> Result<Vec<u8>, ()> {
    bincode::serialize(key).map_err(|_| {
        eprintln!("Failed to serialize {} key", key_name);
        ()
    })
}

fn deserialize_key<'a, T>(data: &'a [u8], key_name: &str) -> Option<T>
where
    T: serde::de::Deserialize<'a>,
{
    bincode::deserialize(data)
        .map_err(|_| {
            eprintln!("Failed to deserialize {} key", key_name);
        })
        .ok()
}
