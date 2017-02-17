// Copyright 2016 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::vec::Vec;

use byteorder::{ByteOrder, BigEndian, WriteBytesExt};

use raftstore::Result;
use util::escape;
use kvproto::metapb::Region;
use std::mem;

pub const MIN_KEY: &'static [u8] = &[];
pub const MAX_KEY: &'static [u8] = &[0xFF];

pub const EMPTY_KEY: &'static [u8] = &[];

// local is in (0x01, 0x02);
pub const LOCAL_PREFIX: u8 = 0x01;
pub const LOCAL_MIN_KEY: &'static [u8] = &[LOCAL_PREFIX];
pub const LOCAL_MAX_KEY: &'static [u8] = &[LOCAL_PREFIX + 1];

pub const DATA_PREFIX: u8 = b'z';
pub const DATA_PREFIX_KEY: &'static [u8] = &[DATA_PREFIX];
pub const DATA_MIN_KEY: &'static [u8] = &[DATA_PREFIX];
pub const DATA_MAX_KEY: &'static [u8] = &[DATA_PREFIX + 1];

// Following keys are all local keys, so the first byte must be 0x01.
pub const STORE_IDENT_KEY: &'static [u8] = &[LOCAL_PREFIX, 0x01];
// We save two types region data in DB, for raft and other meta data.
// When the store starts, we should iterate all region meta data to
// construct peer, no need to travel large raft data, so we separate them
// with different prefixes.
pub const REGION_RAFT_PREFIX: u8 = 0x02;
pub const REGION_RAFT_PREFIX_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_RAFT_PREFIX];
pub const REGION_META_PREFIX: u8 = 0x03;
pub const REGION_META_PREFIX_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_META_PREFIX];
pub const REGION_META_MIN_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_META_PREFIX];
pub const REGION_META_MAX_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_META_PREFIX + 1];

// Following are the suffix after the local prefix.
// For region id
pub const RAFT_LOG_SUFFIX: u8 = 0x01;
pub const RAFT_STATE_SUFFIX: u8 = 0x02;
pub const APPLY_STATE_SUFFIX: u8 = 0x03;

// For region meta
pub const REGION_STATE_SUFFIX: u8 = 0x01;

pub fn store_ident_key() -> Vec<u8> {
    STORE_IDENT_KEY.to_vec()
}

fn make_region_id_key(region_id: u64, suffix: u8, extra_cap: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_RAFT_PREFIX_KEY.len() + mem::size_of::<u64>() +
                                     mem::size_of::<u8>() +
                                     extra_cap);
    key.extend_from_slice(REGION_RAFT_PREFIX_KEY);
    // no need check error here, can't panic;
    key.write_u64::<BigEndian>(region_id).unwrap();
    key.push(suffix);
    key
}

pub fn region_raft_prefix(region_id: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_RAFT_PREFIX_KEY.len() + mem::size_of::<u64>());
    key.extend_from_slice(REGION_RAFT_PREFIX_KEY);
    // no need check error here, can't panic;
    key.write_u64::<BigEndian>(region_id).unwrap();
    key
}

pub fn region_raft_prefix_len() -> usize {
    // REGION_RAFT_PREFIX_KEY + region_id + suffix
    REGION_RAFT_PREFIX_KEY.len() + mem::size_of::<u64>() + 1
}

pub fn raft_log_key(region_id: u64, log_index: u64) -> Vec<u8> {
    let mut key = make_region_id_key(region_id, RAFT_LOG_SUFFIX, mem::size_of::<u64>());
    // no need check error here, can't panic;
    key.write_u64::<BigEndian>(log_index).unwrap();
    key
}

pub fn raft_state_key(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_STATE_SUFFIX, 0)
}

pub fn apply_state_key(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, APPLY_STATE_SUFFIX, 0)
}

/// Get the log index from raft log key generated by `raft_log_key`.
pub fn raft_log_index(key: &[u8]) -> Result<u64> {
    let expect_key_len = REGION_RAFT_PREFIX_KEY.len() + mem::size_of::<u64>() +
                         mem::size_of::<u8>() + mem::size_of::<u64>();
    if key.len() != expect_key_len {
        return Err(box_err!("key {} is not a valid raft log key", escape(key)));
    }
    Ok(BigEndian::read_u64(&key[expect_key_len - mem::size_of::<u64>()..]))
}

/// Get the region id and index from raft log key generated by `raft_log_key`.
pub fn decode_raft_log_key(key: &[u8]) -> Result<(u64, u64)> {
    let suffix_idx = REGION_RAFT_PREFIX_KEY.len() + mem::size_of::<u64>();
    let expect_key_len = suffix_idx + mem::size_of::<u8>() + mem::size_of::<u64>();
    if key.len() != expect_key_len || !key.starts_with(REGION_RAFT_PREFIX_KEY) ||
       key[suffix_idx] != RAFT_LOG_SUFFIX {
        return Err(box_err!("key {} is not a valid raft log key", escape(key)));
    }
    let region_id = BigEndian::read_u64(&key[REGION_RAFT_PREFIX_KEY.len()..suffix_idx]);
    let index = BigEndian::read_u64(&key[suffix_idx + mem::size_of::<u8>()..]);
    Ok((region_id, index))
}

pub fn raft_log_prefix(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_LOG_SUFFIX, 0)
}

fn make_region_meta_key(region_id: u64, suffix: u8) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_META_PREFIX_KEY.len() + mem::size_of::<u64>() +
                                     mem::size_of::<u8>());
    key.extend_from_slice(REGION_META_PREFIX_KEY);
    // no need to check error here, can't panic;
    key.write_u64::<BigEndian>(region_id).unwrap();
    key.push(suffix);
    key
}

// Decode region meta key, return the region key and meta suffix type.
pub fn decode_region_meta_key(key: &[u8]) -> Result<(u64, u8)> {
    if REGION_META_PREFIX_KEY.len() + mem::size_of::<u64>() + mem::size_of::<u8>() != key.len() {
        return Err(box_err!("invalid region meta key length for key {}", escape(key)));
    }

    if !key.starts_with(REGION_META_PREFIX_KEY) {
        return Err(box_err!("invalid region meta prefix for key {}", escape(key)));
    }

    let region_id =
        BigEndian::read_u64(&key[REGION_META_PREFIX_KEY.len()..REGION_META_PREFIX_KEY.len() +
                                                               mem::size_of::<u64>()]);

    Ok((region_id, key[key.len() - 1]))
}

pub fn region_meta_prefix(region_id: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_META_PREFIX_KEY.len() + mem::size_of::<u64>());
    key.extend_from_slice(REGION_META_PREFIX_KEY);
    key.write_u64::<BigEndian>(region_id).unwrap();
    key
}

pub fn region_state_key(region_id: u64) -> Vec<u8> {
    make_region_meta_key(region_id, REGION_STATE_SUFFIX)
}

pub fn validate_data_key(key: &[u8]) -> bool {
    key.starts_with(DATA_PREFIX_KEY)
}

pub fn data_key(key: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(DATA_PREFIX_KEY.len() + key.len());
    v.extend_from_slice(DATA_PREFIX_KEY);
    v.extend_from_slice(key);
    v
}

pub fn origin_key(key: &[u8]) -> &[u8] {
    assert!(validate_data_key(key));
    &key[DATA_PREFIX_KEY.len()..]
}

/// Get the `start_key` of current region in encoded form.
pub fn enc_start_key(region: &Region) -> Vec<u8> {
    // only initialized region's start_key can be encoded, otherwise there must be bugs
    // somewhere.
    assert!(!region.get_peers().is_empty());
    data_key(region.get_start_key())
}

/// Get the `end_key` of current region in encoded form.
pub fn enc_end_key(region: &Region) -> Vec<u8> {
    // only initialized region's end_key can be encoded, otherwise there must be bugs
    // somewhere.
    assert!(!region.get_peers().is_empty());
    data_end_key(region.get_end_key())
}

#[inline]
pub fn data_end_key(region_end_key: &[u8]) -> Vec<u8> {
    if region_end_key.is_empty() {
        DATA_MAX_KEY.to_vec()
    } else {
        data_key(region_end_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};
    use kvproto::metapb::{Region, Peer};
    use std::cmp::Ordering;

    #[test]
    fn test_region_id_key() {
        let region_ids = vec![0, 1, 1024, ::std::u64::MAX];
        for region_id in region_ids {
            let prefix = region_raft_prefix(region_id);

            assert!(raft_log_prefix(region_id).starts_with(&prefix));
            assert!(raft_log_key(region_id, 1).starts_with(&prefix));
            assert!(raft_state_key(region_id).starts_with(&prefix));
            assert!(apply_state_key(region_id).starts_with(&prefix));
        }

        // test sort.
        let tbls = vec![(1, 0, Ordering::Greater), (1, 1, Ordering::Equal), (1, 2, Ordering::Less)];
        for (lid, rid, order) in tbls {
            let lhs = region_raft_prefix(lid);
            let rhs = region_raft_prefix(rid);
            assert_eq!(lhs.partial_cmp(&rhs), Some(order));

            let lhs = raft_state_key(lid);
            let rhs = raft_state_key(rid);
            assert_eq!(lhs.partial_cmp(&rhs), Some(order));

            let lhs = apply_state_key(lid);
            let rhs = apply_state_key(rid);
            assert_eq!(lhs.partial_cmp(&rhs), Some(order));
        }
    }

    #[test]
    fn test_raft_log_sort() {
        let tbls = vec![(1, 1, 1, 2, Ordering::Less),
                        (2, 1, 1, 2, Ordering::Greater),
                        (1, 1, 1, 1, Ordering::Equal)];

        for (lid, l_log_id, rid, r_log_id, order) in tbls {
            let lhs = raft_log_key(lid, l_log_id);
            let rhs = raft_log_key(rid, r_log_id);
            assert_eq!(lhs.partial_cmp(&rhs), Some(order));
        }
    }

    #[test]
    fn test_region_meta_key() {
        let ids = vec![1, 1024, u64::max_value()];
        for id in ids {
            let prefix = region_meta_prefix(id);
            let info_key = region_state_key(id);
            assert!(info_key.starts_with(&prefix));

            assert_eq!(decode_region_meta_key(&info_key).unwrap(),
                       (id, REGION_STATE_SUFFIX));
        }

        // test sort.
        let tbls: Vec<(u64, u64, Ordering)> = vec![
        (1, 2, Ordering::Less),
        (1, 1, Ordering::Equal),
        (2, 1, Ordering::Greater),
        ];

        for (lkey, rkey, order) in tbls {
            let lhs = region_state_key(lkey);
            let rhs = region_state_key(rkey);
            assert_eq!(lhs.partial_cmp(&rhs), Some(order));
        }
    }

    #[test]
    fn test_raft_log_key() {
        for region_id in 1..10 {
            for idx_id in 1..10 {
                let key = raft_log_key(region_id, idx_id);
                assert_eq!(idx_id, raft_log_index(&key).unwrap());
                assert_eq!((region_id, idx_id), decode_raft_log_key(&key).unwrap());
            }
        }

        let mut state_key = raft_state_key(1);
        // invalid length
        assert!(decode_raft_log_key(&state_key).is_err());

        state_key.write_u64::<BigEndian>(2).unwrap();
        // invalid suffix
        assert!(decode_raft_log_key(&state_key).is_err());

        let mut region_state_key = region_state_key(1);
        region_state_key.write_u64::<BigEndian>(2).unwrap();
        // invalid prefix
        assert!(decode_raft_log_key(&region_state_key).is_err());
    }

    #[test]
    fn test_data_key() {
        assert!(validate_data_key(&data_key(b"abc")));
        assert!(!validate_data_key(b"abc"));

        let mut region = Region::new();
        // uninitialised region should not be passed in `enc_start_key` and `enc_end_key`.
        assert!(recover_safe!(|| enc_start_key(&region)).is_err());
        assert!(recover_safe!(|| enc_end_key(&region)).is_err());

        region.mut_peers().push(Peer::new());
        assert_eq!(enc_start_key(&region), vec![DATA_PREFIX]);
        assert_eq!(enc_end_key(&region), vec![DATA_PREFIX + 1]);

        region.set_start_key(vec![1]);
        region.set_end_key(vec![2]);
        assert_eq!(enc_start_key(&region), vec![DATA_PREFIX, 1]);
        assert_eq!(enc_end_key(&region), vec![DATA_PREFIX, 2]);
    }
}
