/// Convert to intel byte order
pub fn bytes_order_u16(x: u16) -> u16 {
    let mut y: [u8; 2];
    y = x.to_be_bytes();
    y.reverse();
    ((y[0] as u16) << 8) | y[1] as u16
    
 }
 
 pub fn bytes_order_u32(x: u32) -> u32 {
    let mut y: [u8; 4];
    y = x.to_be_bytes();
    y.reverse();
   ((y[0] as u32) << 24) | ((y[1] as u32) << 16) | ((y[2] as u32) << 8) | y[3] as u32
 }