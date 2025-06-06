use std::{
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::{Result, eyre::bail};
use tokio::sync::{Mutex, RwLock};

/// A simple ring buffer to maintain the last `CAPACITY` message IDs/users in a channel
/// This is only used to keep a VERY small cache of the last few users in the channel
/// we technically only use the last one, but more are kept in case a message is deleted
// May god forgive me for this half-assed implementation I'm so sorry
#[derive(Clone, Debug)]
pub struct MessageBuffer<const CAPACITY: usize> {
    /// points to the last element insterted into the list
    ptr: usize,
    size: usize,
    /// (user id, msg id, timestamp)
    data: Vec<(u64, u64, u64)>,
}

impl<const CAPACITY: usize> MessageBuffer<CAPACITY> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ptr: 0,
            size: 0,
            data: vec![(0, 0, 0); CAPACITY],
        }
    }

    /// Returns the last user to send a message in the channel
    /// returns (`user id`, `message id`, `timestamp` (unix secs))
    #[must_use]
    pub fn get_last_message(&self) -> Option<(u64, u64, u64)> {
        if self.size == 0 {
            return None;
        }

        Some(self.data[self.ptr])
    }

    /// Appends a message to the ring buffer.
    pub fn push(&mut self, author_id: u64, msg_id: u64, timestamp: u64) {
        if self.size != 0 {
            self.ptr = Self::wrapping_inc(self.ptr);
        }

        if self.size < CAPACITY {
            self.size += 1
        }

        self.data[self.ptr] = (author_id, msg_id, timestamp)
    }

    /// Removes a message by ID. Retains at least one message.
    /// Returns false if the message was not deleted.
    pub fn delete(&mut self, msg_id: u64) -> Result<()> {
        if self.size <= 1 {
            bail!(
                "Removal requested but ring buffer has {} element(s)",
                self.size
            );
        }

        let oldest_index = Self::wrapping_inc(self.ptr);

        let Some(msg_index) = self.data.iter().position(|m| m.1 == msg_id) else {
            // the message wasn't in the buffer
            bail!("Message {} not found in buffer", msg_id);
        };

        // shift every message back down to msg_index
        let mut next_msg_ptr = Self::wrapping_inc(msg_index);
        while next_msg_ptr != oldest_index && self.index_of(next_msg_ptr) <= self.size {
            // wrapping_add ensures ptr-1 and ptr both exist
            self.data[next_msg_ptr - 1] = self.data[next_msg_ptr];
            next_msg_ptr = Self::wrapping_inc(next_msg_ptr);
        }

        self.size -= 1;

        Ok(())
    }

    /// returns a number [0, CAPACITY) as if this were a stack
    fn index_of(&self, ptr: usize) -> usize {
        CAPACITY + ptr - self.ptr
    }

    #[inline]
    fn wrapping_inc(idx: usize) -> usize {
        Self::wrapping_add(idx, 1)
    }

    fn wrapping_add(lhs: usize, rhs: usize) -> usize {
        let idx = lhs + rhs;
        if idx >= CAPACITY { idx - CAPACITY } else { idx }
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl<const CAPACITY: usize> Default for MessageBuffer<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

/// A RwLock<HashMap<T, Arc<Mutex<U>>>> with some nice wrapper functions
/// this isn't a super small data structure by any means but it's certainly smaller than enabling
/// the entire Discord cache
pub struct SyncMap<K, V>(RwLock<HashMap<K, Arc<Mutex<V>>>>);

impl<K, V> SyncMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Default,
{
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    /// uses a read lock on the map. only writes if the key does not yet exist.
    /// Returns the mutex wrapping the value. Intentionally doesn't lock to allow the user to
    /// decide the semantics of allat
    pub async fn get_or_insert(&self, key: &K) -> Arc<Mutex<V>> {
        let mut map = self.0.read().await;

        let v = match map.get(key) {
            Some(key) => key,
            None => {
                drop(map);
                let mut map_write = self.0.write().await;
                map_write.insert(key.clone(), Arc::new(Mutex::new(V::default())));
                drop(map_write);
                map = self.0.read().await;
                map.get(key).unwrap()
            }
        };

        v.clone()
    }

    pub async fn set(&self, key: &K, value: V) {
        let v = self.get_or_insert(key).await;
        let mut v = v.lock().await;
        *v = value;
    }

    pub async fn get(&self, key: &K) -> Option<Arc<Mutex<V>>> {
        let map = self.0.read().await;
        map.get(key).cloned()
    }
}
