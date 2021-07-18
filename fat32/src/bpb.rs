/// Define BIOS Parameters
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct BIOSParameterBlock {
    pub byte_per_sector: u16,
    pub sector_per_cluster: u8,
    pub reserved_sector: u16,
    pub num_fat: u8,
    pub total_sector: u32,
    pub sector_per_fat: u32,
    pub root_cluster: u32,
    pub id: u32,
    pub volume_label: [u8; 11],
    pub file_system: [u8; 8],
}

impl BIOSParameterBlock {
    /// Uninit
    pub fn uninit() -> Self {
        Self {
            byte_per_sector: 0,
            sector_per_cluster: 0,
            reserved_sector: 0,
            num_fat: 0,
            total_sector: 0,
            sector_per_fat: 0,
            root_cluster: 0,
            id: 0,
            volume_label: [0; 11],
            file_system: [0; 8]
        }
    }

    /// Get the first sector offset bytes of the cluster from the cluster number
    pub fn offset(&self, cluster: u32) -> usize {
        ((self.reserved_sector as usize)
            + (self.num_fat as usize) * (self.sector_per_fat as usize)
            + (cluster as usize - 2) * (self.sector_per_cluster as usize))
            * (self.byte_per_sector as usize)
    }

    /// Get FAT1 Offset
    pub fn fat1(&self) -> usize {
        (self.reserved_sector as usize) * (self.byte_per_sector as usize)
    }

    /// Get sector_per_cluster_usize as usize value
    pub fn sector_per_cluster_usize(&self) -> usize {
        self.sector_per_cluster as usize
    }

    /// Get the numbers of data clusters
    pub fn data_sectors(&self) -> usize {
        (self.total_sector - (self.reserved_sector as u32 + (self.num_fat as u32 * self.sector_per_fat) 
        + self.root_cluster)) as usize
    }

    /// Get the count 
    pub fn count_of_clusters(&self) -> usize {
        self.data_sectors() / self.sector_per_cluster as usize
    }
}
