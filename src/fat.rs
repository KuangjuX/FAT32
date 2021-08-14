use super::BlockDevice;
use crate::{BUFFER_SIZE, get_block_cache};
use crate::tool::read_le_u32;

use alloc::sync::Arc;

use core::ptr;

pub struct FAT {
    block_device: Arc<dyn BlockDevice>,
    fat_offset: usize,
    bytes_per_sector: u16,
    sectors_per_cluster:u8,
    start_cluster: u32,
    previous_cluster: u32,
    pub(crate) current_cluster: u32,
    next_cluster: Option<u32>,
    buffer: [u8; BUFFER_SIZE],
}

impl FAT {
    pub fn new(cluster: u32, fat_offset: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        Self {
            block_device,
            fat_offset,
            bytes_per_sector: 0,
            sectors_per_cluster: 1,
            start_cluster: cluster,
            previous_cluster: 0,
            current_cluster: 0,
            next_cluster: None,
            buffer: [0; BUFFER_SIZE],
        }
    }

    pub(crate) fn blank_cluster(&mut self) -> u32 {
        let mut cluster = 0;
        let mut done = false;
        let count_block_id: usize = 0;
        let base_block_id = self.fat_offset/(self.bytes_per_sector as usize * self.sectors_per_cluster as usize);
        loop {
            let block_id = base_block_id + count_block_id;
            let block_cache = unsafe{ get_block_cache(block_id, self.block_device) };
            for i in (0..BUFFER_SIZE).step_by(4) {
                if read_le_u32(block_cache.lock().split(i, i+4)) == 0 {
                    done = true;
                    break;
                } else {
                    cluster += 1;
                }
                if done {break};
            }
            count_block_id += 1;

        }

        cluster
    }

    pub fn write(&mut self, cluster: u32, value: u32) {
        let offset = (cluster as usize) * 4;
        let block_offset = offset / BUFFER_SIZE;
        let offset_left = offset % BUFFER_SIZE;
        let offset = self.fat_offset + block_offset * BUFFER_SIZE;
        let mut value: [u8; 4] = value.to_be_bytes();
        value.reverse();


        let block_cache = unsafe{ get_block_cache(block_offset, self.block_device).lock() };
        let cache = block_cache.get_cache_mut();
        cache[offset_left..offset + 4].copy_from_slice(&value);
        // Write Cache
        block_cache.write_cache(cache);
        // Write back Disk
        drop(block_cache);
        
    }

    pub(crate) fn refresh(&mut self, start_cluster: u32) {
        self.current_cluster = 0;
        self.start_cluster = start_cluster;
    }

    pub(crate) fn previous(&mut self) {
        if self.current_cluster != 0 {
            self.next_cluster = Some(self.current_cluster);
            self.current_cluster = self.previous_cluster;
        }
    }

    pub(crate) fn next_is_none(&self) -> bool {
        self.next_cluster.is_none()
    }

    fn current_cluster_usize(&self) -> usize {
        self.current_cluster as usize
    }
}

impl Iterator for FAT
{
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_cluster == 0 {
            self.current_cluster = self.start_cluster;
        } else {
            let next_cluster = self.next_cluster;
            if next_cluster.is_some() {
                self.previous_cluster = self.current_cluster;
                self.current_cluster = next_cluster.unwrap();
            } else {
                return None;
            }
        }

        let offset = self.current_cluster_usize() * 4;
        let block_offset = offset / BUFFER_SIZE;
        let offset_left = offset % BUFFER_SIZE;

        // self.device.read(&mut self.buffer,
        //                  self.fat_offset + block_offset * BUFFER_SIZE,
        //                  1);
        let block_id = self.fat_offset/BUFFER_SIZE + block_offset;
        let block_cache = unsafe{
             get_block_cache(block_id, self.block_device) 
             .lock()
            };
        let cache = block_cache.get_cache();
        // Copy FAT Buffer into self.buffer
        unsafe{
            ptr::copy(cache.as_ptr(), self.buffer.as_mut_ptr(), BUFFER_SIZE);
        }
        

        let next_cluster = read_le_u32(&self.buffer[offset_left..offset_left + 4]);
        let next_cluster = if next_cluster == 0x0FFFFFFF {
            None
        } else {
            Some(next_cluster)
        };

        self.next_cluster = next_cluster;

        Some(Self {
            next_cluster,
            block_device: self.block_device,
            fat_offset: self.fat_offset,
            bytes_per_sector: self.bytes_per_sector,
            sectors_per_cluster: self.sectors_per_cluster,
            start_cluster: self.start_cluster,
            previous_cluster: self.previous_cluster,
            current_cluster: self.current_cluster,
            buffer: self.buffer,
        })
    }
}
