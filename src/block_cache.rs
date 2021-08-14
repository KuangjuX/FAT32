use super::BUFFER_SIZE;
use super::BlockDevice;

use alloc::sync::Arc;
use alloc::collections::VecDeque;
use core::ptr;

use spin::Mutex;
use lazy_static::*;

pub struct BlockCache {
    cache: [u8; BUFFER_SIZE],
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
    modified: bool
}

impl BlockCache {
    /// Load a new BlockCache from disk.
    pub fn new(
        block_id: usize, 
        block_device: Arc<dyn BlockDevice>
    ) -> Self {
        let mut cache = [0u8; BUFFER_SIZE];
        block_device.read(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BUFFER_SIZE);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) } 
    }

    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BUFFER_SIZE);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset:usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write(self.block_id, &self.cache);
        }
    }

    pub fn get_cache(&self) -> &[u8] {
        &self.cache
    }

    pub fn get_cache_mut(&mut self) -> &mut [u8] {
        &mut self.cache
    }

    pub fn split(&self, start: usize, end: usize) -> &[u8] {
        &(self.get_cache())[start..end]
    }

    pub fn write_cache(&self, buf: &[u8]) {
        unsafe{
            ptr::copy(buf.as_ptr(), self.cache.as_ptr() as *mut u8, BUFFER_SIZE);
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self { queue: VecDeque::new() }
    }

    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue
            .iter()
            .find(|pair| pair.0 == block_id) {
                Arc::clone(&pair.1)
        } else {
            // substitute
            if self.queue.len() == BLOCK_CACHE_SIZE {
                // from front to tail
                if let Some((idx, _)) = self.queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1) {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // load block into mem and push back
            let block_cache = Arc::new(Mutex::new(
                BlockCache::new(block_id, Arc::clone(&block_device))
            ));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(
        BlockCacheManager::new()
    );
}

unsafe impl Send for BlockCacheManager{}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER.lock().get_block_cache(block_id, block_device)
}