pub trait BlockDevice {
    fn read(&self, buf: &mut [u8], addr: usize, block_number: usize);
    fn write(&self, buf: &[u8], addr: usize, block_number: usize);
}