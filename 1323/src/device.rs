pub trait BlockDevice: Send + Send + 'static {
    fn read(&self, addr: usize, buf: &mut [u8]);
    fn write(&self, addr: usize, buf: &mut [u8]);
}