use alloc::sync::Arc;
use alloc::vec::Vec;

use super::device::BlockDevice;
use super::superblock::SuperBlock;
pub struct FileEnrty{
    device: Arc<dyn BlockDevice>,
    clusters: Vec<usize>,
    size: usize,
    seek_at: usize,
    addr: usize,
    sblock: SuperBlock
}