use core::convert::TryInto;
use core::ptr::copy;


/// BIOSParameterBlock Offset
pub enum BPBOffset {
    BytesPerSector = 11,
    SectorsPerCluster = 13,
    ReservedSectorCount = 14,
    FatNums = 16, 
    TotalSector32 = 32,
    FatSize32 = 36,
    RootSector = 44,
    VolumeID = 67,
    VolumeLabel = 71,
    FileSystemType = 82
}

impl BPBOffset {
    pub fn split(start: usize, end: usize, sector: &[u8]) -> &[u8] {
        &sector[start ..end]
    }

    /// Get bytes_per_sector
    pub fn bytes_per_sector(sector: &[u8]) -> u16 {
        u16::from_le_bytes(
            Self::split(
                Self::BytesPerSector as usize, 
                Self::SectorsPerCluster as usize, 
                sector
            )
            .try_into()
            .unwrap(),
        )
    }

    /// Get sectors_per_cluster
    pub fn sectors_per_cluster(sector: &[u8]) -> u8 {
        u8::from_le_bytes(
            Self::split(
                Self::SectorsPerCluster as usize, 
                Self::SectorsPerCluster as usize + 1, 
                sector
            )
            .try_into()
            .unwrap(),
        )
    }

    /// Get reversed_sector_count
    pub fn reversed_sector_count(sector: &[u8]) -> u16 {
        u16::from_le_bytes(
            Self::split(
                Self::ReservedSectorCount as usize,
                 Self::ReservedSectorCount as usize + 2, 
                 sector
                )
                .try_into()
                .unwrap(),
        )
    }

    /// Get fat_nums
    pub fn fat_nums(sector: &[u8]) -> u8 {
        u8::from_le_bytes(
            Self::split(
                Self::FatNums as usize, 
                Self::FatNums as usize + 1, 
                sector
            )
            .try_into()
            .unwrap()
        )
    }

    /// Get the total sector
    pub fn total_sector(sector: &[u8]) -> u32 {
        u32::from_le_bytes(
            Self::split(
                Self::TotalSector32 as usize, 
                Self::TotalSector32 as usize + 4, 
                sector
            )
            .try_into()
            .unwrap(),
        )
    }

    /// Get sector_per_fat
    pub fn sectors_per_fat(sector: &[u8]) -> u32 {
        u32::from_le_bytes(
            Self::split(
                Self::FatSize32 as usize, 
                Self::FatSize32 as usize + 4, 
                sector
            )
            .try_into()
            .unwrap(),
        )
    }

    /// Get root sector
    pub fn root_sector(sector: &[u8]) -> u32 {
        u32::from_le_bytes(
            Self::split(
                Self::RootSector as usize, 
                Self::RootSector as usize + 4, 
                sector
            )
            .try_into()
            .unwrap(),
        )
    }

    /// Get volume_id
    pub fn volume_id(sector: &[u8]) -> u32 {
        u32::from_le_bytes(
            Self::split(
                Self::VolumeID as usize, 
                Self::VolumeID as usize + 4, 
                sector
            )
            .try_into()
            .unwrap(),
        )
    }

    /// Get volume_label
    pub fn volume_label(sector: &[u8]) -> [u8; 11] {
        let mut ret: [u8;11] = [0;11];
        unsafe{
            copy(
                sector.as_ptr().offset(Self::VolumeLabel as isize), 
                ret.as_mut_ptr(), 
                11
            );
        }
        ret
    }

    /// Get file_system
    pub fn file_system(sector: &[u8]) -> [u8;8] {
        let mut ret: [u8;8] = [0;8];
        unsafe{
            copy(
                sector.as_ptr().offset(Self::FileSystemType as isize), 
                ret.as_mut_ptr(), 
                11
            );
        }
        ret
    }
}

/// Define BIOS Parameters
#[derive(Debug, Copy, Clone)]
pub struct BIOSParameterBlock {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector: u16,
    pub num_fat: u8,
    pub total_sector: u32,
    pub sectors_per_fat: u32,
    pub root_cluster: u32,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub file_system: [u8; 8],
}

impl BIOSParameterBlock {
    /// Uninit
    pub fn uninit() -> Self {
        Self {
            bytes_per_sector: 0,
            sectors_per_cluster: 0,
            reserved_sector: 0,
            num_fat: 0,
            total_sector: 0,
            sectors_per_fat: 0,
            root_cluster: 0,
            volume_id: 0,
            volume_label: [0; 11],
            file_system: [0; 8]
        }
    }


    /// read secotr to init bpb
    pub fn read(&mut self, buf: &[u8]) {
        self.bytes_per_sector = BPBOffset::bytes_per_sector(buf);
        self.sectors_per_cluster = BPBOffset::sectors_per_cluster(buf);
        self.reserved_sector = BPBOffset::reversed_sector_count(buf);
        self.num_fat = BPBOffset::fat_nums(buf);
        self.total_sector = BPBOffset::total_sector(buf);
        self.sectors_per_fat = BPBOffset::sectors_per_fat(buf);
        self.root_cluster = BPBOffset::root_sector(buf);
        self.volume_id = BPBOffset::volume_id(buf);
        self.volume_label = BPBOffset::volume_label(buf);
        self.file_system = BPBOffset::file_system(buf);
    }

    /// Get the first sector offset bytes of the cluster from the cluster number
    pub fn offset(&self, cluster: u32) -> usize {
        ((self.reserved_sector as usize)
            + (self.num_fat as usize) * (self.sectors_per_fat as usize)
            + (cluster as usize - 2) * (self.sectors_per_cluster as usize))
            * (self.bytes_per_sector as usize)
    }

    /// Get FAT1 Offset
    pub fn fat1(&self) -> usize {
        (self.reserved_sector as usize) * (self.bytes_per_sector as usize)
    }

    /// Get sector_per_cluster_usize as usize value
    pub fn sector_per_cluster_usize(&self) -> usize {
        self.sectors_per_cluster as usize
    }

    /// Get the numbers of data clusters
    pub fn data_sectors(&self) -> usize {
        (self.total_sector - (self.reserved_sector as u32 + (self.num_fat as u32 * self.sectors_per_fat) 
        + self.root_cluster)) as usize
    }

    /// Get the count 
    pub fn count_of_clusters(&self) -> usize {
        self.data_sectors() / self.sectors_per_cluster as usize
    }
}
