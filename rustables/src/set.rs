// Fork change 2026-06-19: SetElement gains key_end / timeout / expiration so
// interval (CIDR-range) and per-element-timeout sets can be managed; SetBuilder
// gains add methods for those. See FORK-CHANGES.md.

use rustables_macros::nfnetlink_struct;

use crate::data_type::DataType;
use crate::error::BuilderError;
use crate::nlmsg::NfNetlinkObject;
use crate::parser_impls::{NfNetlinkData, NfNetlinkList};
use crate::sys::{
    NFTA_SET_ELEM_EXPIRATION, NFTA_SET_ELEM_FLAGS, NFTA_SET_ELEM_KEY, NFTA_SET_ELEM_KEY_END,
    NFTA_SET_ELEM_LIST_ELEMENTS, NFTA_SET_ELEM_LIST_SET, NFTA_SET_ELEM_LIST_TABLE,
    NFTA_SET_ELEM_TIMEOUT, NFTA_SET_FLAGS, NFTA_SET_ID, NFTA_SET_KEY_LEN, NFTA_SET_KEY_TYPE,
    NFTA_SET_NAME, NFTA_SET_TABLE, NFTA_SET_USERDATA, NFT_MSG_DELSET, NFT_MSG_DELSETELEM,
    NFT_MSG_NEWSET, NFT_MSG_NEWSETELEM, NFT_SET_ELEM_INTERVAL_END,
};
use crate::table::Table;
use crate::ProtocolFamily;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[nfnetlink_struct(derive_deserialize = false)]
pub struct Set {
    pub family: ProtocolFamily,
    #[field(NFTA_SET_TABLE)]
    pub table: String,
    #[field(NFTA_SET_NAME)]
    pub name: String,
    #[field(NFTA_SET_FLAGS)]
    pub flags: u32,
    #[field(NFTA_SET_KEY_TYPE)]
    pub key_type: u32,
    #[field(NFTA_SET_KEY_LEN)]
    pub key_len: u32,
    #[field(NFTA_SET_ID)]
    pub id: u32,
    #[field(NFTA_SET_USERDATA)]
    pub userdata: Vec<u8>,
}

impl NfNetlinkObject for Set {
    const MSG_TYPE_ADD: u32 = NFT_MSG_NEWSET;
    const MSG_TYPE_DEL: u32 = NFT_MSG_DELSET;

    fn get_family(&self) -> ProtocolFamily {
        self.family
    }

    fn set_family(&mut self, family: ProtocolFamily) {
        self.family = family;
    }
}

pub struct SetBuilder<K: DataType> {
    inner: Set,
    list: SetElementList,
    _phantom: PhantomData<K>,
}

impl<K: DataType> SetBuilder<K> {
    pub fn new(name: impl Into<String>, table: &Table) -> Result<Self, BuilderError> {
        let table_name = table.get_name().ok_or(BuilderError::MissingTableName)?;
        let set_name = name.into();
        let set = Set::default()
            .with_key_type(K::TYPE)
            .with_key_len(K::LEN)
            .with_table(table_name)
            .with_name(&set_name);

        Ok(SetBuilder {
            inner: set,
            list: SetElementList {
                table: Some(table_name.clone()),
                family: table.get_family(),
                set: Some(set_name),
                elements: Some(SetElementListElements::default()),
            },
            _phantom: PhantomData,
        })
    }

    pub fn add(&mut self, key: &K) {
        self.push(SetElement {
            key: Some(NfNetlinkData::default().with_value(key.data())),
            ..Default::default()
        });
    }

    /// Add a single key with a per-element timeout (milliseconds). The set must
    /// have been created with `flags timeout`.
    pub fn add_with_timeout(&mut self, key: &K, timeout_ms: u64) {
        self.push(SetElement {
            key: Some(NfNetlinkData::default().with_value(key.data())),
            timeout: Some(timeout_ms),
            ..Default::default()
        });
    }

    /// Add a half-open interval `[start, end_exclusive)` to an rbtree `interval`
    /// set, as the kernel represents it: a start boundary carrying the optional
    /// timeout, plus an end boundary flagged INTERVAL_END. For a CIDR this is
    /// `start = network`, `end_exclusive = broadcast + 1`. The set must have
    /// `flags interval` (and `flags timeout` for a timeout).
    pub fn add_interval(&mut self, start: &K, end_exclusive: &K, timeout_ms: Option<u64>) {
        self.push(SetElement {
            key: Some(NfNetlinkData::default().with_value(start.data())),
            timeout: timeout_ms,
            ..Default::default()
        });
        self.push(SetElement {
            key: Some(NfNetlinkData::default().with_value(end_exclusive.data())),
            flags: Some(NFT_SET_ELEM_INTERVAL_END as u32),
            ..Default::default()
        });
    }

    fn push(&mut self, elem: SetElement) {
        self.list.elements.as_mut().unwrap().add_value(elem);
    }

    pub fn finish(self) -> (Set, SetElementList) {
        (self.inner, self.list)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[nfnetlink_struct(nested = true, derive_deserialize = false)]
pub struct SetElementList {
    pub family: ProtocolFamily,
    #[field(NFTA_SET_ELEM_LIST_TABLE)]
    pub table: String,
    #[field(NFTA_SET_ELEM_LIST_SET)]
    pub set: String,
    #[field(NFTA_SET_ELEM_LIST_ELEMENTS)]
    pub elements: SetElementListElements,
}

impl NfNetlinkObject for SetElementList {
    const MSG_TYPE_ADD: u32 = NFT_MSG_NEWSETELEM;
    const MSG_TYPE_DEL: u32 = NFT_MSG_DELSETELEM;

    fn get_family(&self) -> ProtocolFamily {
        self.family
    }

    fn set_family(&mut self, family: ProtocolFamily) {
        self.family = family;
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[nfnetlink_struct(nested = true)]
pub struct SetElement {
    #[field(NFTA_SET_ELEM_KEY)]
    pub key: NfNetlinkData,
    /// Single-element inclusive range end (NFTA_SET_ELEM_KEY_END). Accepted by
    /// concat/pipapo sets only — rbtree interval sets instead use the half-open
    /// two-element form (see `flags` / `SetBuilder::add_interval`).
    #[field(NFTA_SET_ELEM_KEY_END)]
    pub key_end: NfNetlinkData,
    /// Element flags, e.g. NFT_SET_ELEM_INTERVAL_END to mark the end boundary of
    /// a half-open interval in an rbtree `interval` set.
    #[field(NFTA_SET_ELEM_FLAGS)]
    pub flags: u32,
    /// Per-element timeout in milliseconds (sets with `flags timeout`).
    #[field(NFTA_SET_ELEM_TIMEOUT)]
    pub timeout: u64,
    /// Remaining time in milliseconds before expiry; reported by the kernel on
    /// dump (read-only — don't set it when adding).
    #[field(NFTA_SET_ELEM_EXPIRATION)]
    pub expiration: u64,
}

type SetElementListElements = NfNetlinkList<SetElement>;
