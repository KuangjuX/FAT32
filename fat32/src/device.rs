pub trait BlockDevice: Send + Send + 'static {
    fn read(&self, addr: usize, buf: &mut [u8], number_of_block: usize);
    fn write(&self, addr: usize, buf: &[u8], number_of_block: usize);
}