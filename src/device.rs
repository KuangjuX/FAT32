// pub trait BlockDevice {
//     fn read(&self, buf: &mut [u8], addr: usize, block_number: usize);
//     fn write(&self, buf: &[u8], addr: usize, block_number: usize);
// }

/// Trait for Block Device I/O
pub trait BlockDevice {
    fn read(&self, block_id: usize, buf: &mut [u8]);
    fn write(&self, block_id: usize, buf: &[u8]);
}