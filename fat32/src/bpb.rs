#[derive(Default, Debug, Clone)]
pub struct BiosParameterBlock {
    pub(crate) bytes_per_sector: u16,
    pub(crate) sectors_per_cluster: u16,
    pub(crate) reversed_sector: u16,
    pub(crate) fats: u8,
    pub(crate) root_entries: u16,
    pub(crate) total_sectors_16: u16,
    pub(crate) media: u8,
    pub(crate) sectors_per_fat_16: u16,
    pub(crate) sectors_per_track: u16,
    pub(crate) heads: u16,
    pub(crate) hidden_sectors: u32,
    pub(crate) total_sectors_32: u32,

    // Extended BIOS Paramter Block
    pub(crate) sectors_per_fat_32: u32,
    pub(crate) extended_flags: u16,
    pub(crate) fs_version: u16,
    pub(crate) root_dir_first_cluster: u32,
    pub(crate) fs_info_sector: u16,
    pub(crate) backup_boot_sector: u16,
    pub(crate) reserved_0: [u8;12],
    pub(crate) drive_num: u8,
    pub(crate) ext_sig: u8,
    pub(crate) volume_id: u32,
    pub(crate) volume_label: [u8;11],
    pub(crate) fs_type_label: [u8;8]
}

impl BiosParameterBlock {
    /// Get the first sector offset bytes of the cluster from the cluster number
    pub(crate) fn offset(&self, cluster: u32) -> usize {
        ((self.reversed_sector as usize)
        + (self.fats as usize) * (self.sectors_per_fat_32 as usize)
        + (cluster as usize - 2) * (self.sectors_per_cluster as usize))
        * (self.bytes_per_sector as usize)
    }

    /// Get fat1 offset
    pub(crate) fn fat1(&self) -> usize {
        (self.reversed_sector as usize) * (self.bytes_per_sector as usize)
    }

    /// Get sector_per_cluster_usize as usize value
    pub(crate) fn sector_per_cluster_size(&self) -> usize {
        self.sectors_per_cluster as usize
    }
}