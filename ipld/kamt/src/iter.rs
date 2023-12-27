// Copyright 2021-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use std::borrow::Borrow;
use std::iter::FusedIterator;

use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::de::DeserializeOwned;
use fvm_ipld_encoding::CborStore;

use crate::hash_bits::HashBits;
use crate::node::Node;
use crate::pointer::Pointer;
use crate::{AsHashedKey, Config, Error, KeyValuePair};

#[doc(hidden)]
pub struct IterImpl<'a, BS, V, K, H, const N: usize = 32> {
    store: &'a BS,
    stack: Vec<std::slice::Iter<'a, Pointer<K, V, H, N>>>,
    current: std::slice::Iter<'a, KeyValuePair<K, V>>,
}

pub type Iter<'a, BS, V, K, H, const N: usize = 32> = IterImpl<'a, BS, V, K, H, N>;
impl<'a, K, V, BS, H, const N: usize> IterImpl<'a, BS, V, K, H, N>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
    BS: Blockstore,
{
    pub(crate) fn new(store: &'a BS, root: &'a Node<K, V, H, N>) -> Self {
        Self {
            store,
            stack: vec![root.pointers.iter()],
            current: [].iter(),
        }
    }

    pub(crate) fn new_from<Q: Sized>(
        store: &'a BS,
        root: &'a Node<K, V, H, N>,
        key: &Q,
        conf: &'a Config,
    ) -> Result<Self, Error>
    where
        K: Borrow<Q>,
        Q: PartialEq,
        H: AsHashedKey<Q, N>,
    {
        let hashed_key = H::as_hashed_key(key);
        let mut hash = HashBits::new(&hashed_key);
        let mut node = root;
        let mut stack = Vec::new();
        loop {
            let idx = hash.next(conf.bit_width)?;
            stack.push(node.pointers[node.index_for_bit_pos(idx)..].iter());
            node = match stack.last_mut().unwrap().next() {
                Some(p) => match p {
                    Pointer::Link { cid, cache, .. } => {
                        if let Some(cached_node) = cache.get() {
                            cached_node
                        } else {
                            let node =
                                if let Some(node) = store.get_cbor::<Node<K, V, H, N>>(cid)? {
                                    node
                                } else {
                                    #[cfg(not(feature = "ignore-dead-links"))]
                                    return Err(Error::CidNotFound(cid.to_string()));

                                    #[cfg(feature = "ignore-dead-links")]
                                    continue;
                                };

                            // Ignore error intentionally, the cache value will always be the same
                            cache.get_or_init(|| Box::new(node))
                        }
                    }
                    Pointer::Dirty { node, .. } => node,
                    Pointer::Values(values) => {
                        return match values.iter().position(|kv| kv.key().borrow() == key) {
                            Some(offset) => Ok(Self {
                                store,
                                stack,
                                current: values[offset..].iter(),
                            }),
                            None => Err(Error::StartKeyNotFound),
                        }
                    }
                },
                None => continue,
            };
        }
    }
}
impl<'a, K, V, BS, H, const N: usize> Iterator for IterImpl<'a, BS, V, K, H, N>
where
    BS: Blockstore,
    K: DeserializeOwned + PartialOrd,
    V: DeserializeOwned,
{
    type Item = Result<(&'a K, &'a V), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.current.next() {
            return Some(Ok((v.key(), v.value())));
        }
        loop {
            let Some(next) = self.stack.last_mut()?.next() else {
                self.stack.pop();
                continue;
            };
            match next {
                Pointer::Link { cid, cache, .. } => {
                    let node = if let Some(cached_node) = cache.get() {
                        cached_node
                    } else {
                        let node = match self.store.get_cbor::<Node<K, V, H, N>>(cid) {
                            Ok(Some(node)) => node,
                            #[cfg(not(feature = "ignore-dead-links"))]
                            Ok(None) => return Some(Err(Error::CidNotFound(cid.to_string()))),
                            #[cfg(feature = "ignore-dead-links")]
                            Ok(None) => continue,
                            Err(err) => return Some(Err(err.into())),
                        };

                        // Ignore error intentionally, the cache value will always be the same
                        cache.get_or_init(|| Box::new(node))
                    };
                    self.stack.push(node.pointers.iter())
                }
                Pointer::Dirty { node, .. } => self.stack.push(node.pointers.iter()),
                Pointer::Values(kvs) => {
                    self.current = kvs.iter();
                    if let Some(v) = self.current.next() {
                        return Some(Ok((v.key(), v.value())));
                    }
                }
            }
        }
    }
}

impl<'a, K, V, BS, H, const N: usize> FusedIterator for IterImpl<'a, BS, V, K, H, N>
where
    K: DeserializeOwned + PartialOrd,
    V: DeserializeOwned,
    BS: Blockstore,
{
}
