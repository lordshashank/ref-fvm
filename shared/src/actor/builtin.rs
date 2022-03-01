use std::collections::BTreeMap;

use cid::Cid;
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::version::NetworkVersion;

/// Identifies the builtin actor types for usage with the
/// actor::resolve_builtin_actor_type syscall.
#[derive(
    PartialEq,
    Eq,
    Clone,
    Copy,
    PartialOrd,
    Ord,
    FromPrimitive,
    Debug,
    Deserialize_repr,
    Hash,
    Serialize_repr,
)]
#[repr(i32)]
pub enum Type {
    System = 1,
    Init = 2,
    Cron = 3,
    Account = 4,
    Power = 5,
    Miner = 6,
    Market = 7,
    PaymentChannel = 8,
    Multisig = 9,
    Reward = 10,
    VerifiedRegistry = 11,
}

pub const CALLER_TYPES_SIGNABLE: &[Type] = &[Type::Account, Type::Multisig];

impl TryFrom<&str> for Type {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let ret = match value {
            "system" => Type::System,
            "init" => Type::Init,
            "cron" => Type::Cron,
            "account" => Type::Account,
            "storagepower" => Type::Power,
            "storageminer" => Type::Miner,
            "storagemarket" => Type::Market,
            "paymentchannel" => Type::PaymentChannel,
            "multisig" => Type::Multisig,
            "reward" => Type::Reward,
            "verifiedregistry" => Type::VerifiedRegistry,
            _ => return Err(String::from("unrecognized actor type")),
        };
        Ok(ret)
    }
}

/// Returns true if the actor kind represents a singleton actor. That is, an actor
/// that cannot be constructed by a user.
pub fn is_singleton_actor(typ: Type) -> bool {
    typ == Type::System
        || typ == Type::Init
        || typ == Type::Reward
        || typ == Type::Cron
        || typ == Type::Power
        || typ == Type::Market
        || typ == Type::VerifiedRegistry
}

/// Returns true if the code belongs to an account actor.
pub fn is_account_actor(typ: Type) -> bool {
    typ == Type::Account
}

/// Tests whether an actor type represents an actor that can be an external
/// principal: i.e. an account or multisig.
pub fn is_principal(typ: Type) -> bool {
    typ == Type::Account || typ == Type::Multisig
}

/// A mapping of builtin actor CIDs to their respective types.
pub type Manifest = BTreeMap<Cid, Type>;

/// A mapping of network versions to builtin actor Manifests.
pub type NetworksManifests = BTreeMap<NetworkVersion, Manifest>;
