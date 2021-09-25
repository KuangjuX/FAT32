use super::{
    get_block_cache, get_info_cache, set_start_sec, write_to_dev, BlockDevice, CacheMode, FSInfo,
    FatBS, FatExtBS, FAT,
};
use crate::{layout::*, VFile};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

pub struct FAT32Manager {
    block_device: Arc<dyn BlockDevice>,
    fsinfo: Arc<FSInfo>,
    sectors_per_cluster: u32,
    bytes_per_sector: u32,
    bytes_per_cluster: u32,
    fat: Arc<RwLock<FAT>>,
    root_sec: u32,
    #[allow(unused)]
    total_sectors: u32, //总扇区数
    vroot_dirent: Arc<RwLock<ShortDirEntry>>,
}

//type DataBlock = [u8; BLOCK_SZ];

pub fn create_fat(block_id: usize, device: Arc<dyn BlockDevice>) {
    let cache = get_info_cache(block_id, device, CacheMode::WRITE);
    let mut guard = cache.write();
    guard.modify(0, |fat: &mut u64| {
        *fat = 0xFFFFFFFFFFFFFFFF;
    });
    guard.modify(8, |fat: &mut u32| {
        *fat = 0x0FFFFFFF;
    });
    drop(guard);
}

impl FAT32Manager {
    /// 创建FAT32管理者
    pub fn create(block_device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>> {
        // 初始化文件系统镜像
        // FatBS::init_boot_sector(Arc::clone(&block_device));
        // FatExtBS::init_ext_bs(Arc::clone(&block_device));
        // 初始化Fat表
        // create_fat(1, Arc::clone(&block_device));
        // 初始化FsInfo
        // let fs_info = FSInfo::new(1);
        // fs_info.init_fsinfo(Arc::clone(&block_device));
        // 打开文件系统镜像
        Self::open(Arc::clone(&block_device))
    }

    pub fn sectors_per_cluster(&self) -> u32 {
        self.sectors_per_cluster
    }

    pub fn bytes_per_sector(&self) -> u32 {
        self.bytes_per_sector
    }

    pub fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_cluster
    }

    /// 第一个数据簇的扇区
    pub fn first_data_sector(&self) -> u32 {
        self.root_sec
    }

    /// 给定簇的第一个扇区
    pub fn first_sector_of_cluster(&self, cluster: u32) -> usize {
        //println!("first_sector_of_cluster: cluster = {}", cluster);
        (cluster as usize - 2) * self.sectors_per_cluster as usize + self.root_sec as usize
    }

    /// 打开现有的FAT32
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>> {
        // 读入分区偏移
        // println!("[fs]: Load FAT32");
        // let start_sector:u32 = get_info_cache(
        //     0,
        //     Arc::clone(&block_device),
        //     CacheMode::READ )
        // .read()
        // .read(0x1c6, |ssec_bytes:&[u8;4]|{
        //     // DEBUG
        //     let mut start_sec:u32 = 0;
        //     for i in 0..4 {
        //         let tmp = ssec_bytes[i] as u32;
        //         start_sec = start_sec + (tmp << (8*i));
        //         //println!("start sec = {}, buf = {}", start_sec , ssec_bytes[i])
        //     }
        //     start_sec
        // });
        let start_sector = 0;
        set_start_sec(start_sector as usize);

        // 读入 Boot Sector
        let boot_sec: FatBS = get_info_cache(0, Arc::clone(&block_device), CacheMode::READ)
            .read()
            .read(0, |bs: &FatBS| *bs);
        // 读入 Extended Boot Sector
        let ext_boot_sec: FatExtBS = get_info_cache(0, Arc::clone(&block_device), CacheMode::READ)
            .read()
            .read(36, |ebs: &FatExtBS| {
                *ebs // DEBUG
            });
        // 读入 FSInfo
        let fsinfo = FSInfo::new(ext_boot_sec.fat_info_sec());
        // 校验签名
        assert!(
            fsinfo.check_signature(Arc::clone(&block_device)),
            "Error loading fat32! Illegal signature"
        );

        let sectors_per_cluster = boot_sec.sectors_per_cluster as u32;
        let bytes_per_sector = boot_sec.bytes_per_sector as u32;
        let bytes_per_cluster = sectors_per_cluster * bytes_per_sector;

        // 读取FAT表信息
        let fat_n_sec = ext_boot_sec.fat_size();
        let fat1_sector = boot_sec.first_fat_sector();
        let fat2_sector = fat1_sector + fat_n_sec;
        // 用来计算一共有多少entry
        let fat_n_entry = fat_n_sec * bytes_per_sector / 4;

        let fat = FAT::new(fat1_sector, fat2_sector, fat_n_sec, fat_n_entry);

        // 保留扇区数 + 所有FAT表的扇区数
        let root_sec =
            boot_sec.table_count as u32 * fat_n_sec + boot_sec.reserved_sector_count as u32;

        // 初始化root_dirent
        let mut root_dirent = ShortDirEntry::new(
            &[0x2F, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20],
            &[0x20, 0x20, 0x20],
            ATTRIBUTE_DIRECTORY,
        );
        root_dirent.set_first_cluster(2);

        let fat32_manager = Self {
            block_device,
            fsinfo: Arc::new(fsinfo),
            sectors_per_cluster,
            bytes_per_sector,
            bytes_per_cluster,
            fat: Arc::new(RwLock::new(fat)),
            root_sec,
            total_sectors: boot_sec.total_sectors(),
            vroot_dirent: Arc::new(RwLock::new(root_dirent)),
        };
        Arc::new(RwLock::new(fat32_manager))
    }

    pub fn get_root_vfile(&self, fs_manager: &Arc<RwLock<Self>>) -> VFile {
        let long_pos_vec: Vec<(usize, usize)> = Vec::new();
        VFile::new(
            String::from("/"),
            0,
            0,
            long_pos_vec,
            ATTRIBUTE_DIRECTORY,
            0,
            Arc::clone(fs_manager),
            self.block_device.clone(),
        )
    }

    pub fn get_root_dirent(&self) -> Arc<RwLock<ShortDirEntry>> {
        self.vroot_dirent.clone()
    }

    /// 分配簇，会填写FAT，成功返回第一个簇号，失败返回None
    pub fn alloc_cluster(&self, num: u32) -> Option<u32> {
        let free_clusters = self.free_clusters();
        if num > free_clusters {
            return None;
        }
        // 获取FAT写锁
        let fat_writer = self.fat.write();
        let prev_cluster = self.fsinfo.first_free_cluster(self.block_device.clone());

        let first_cluster: u32 =
            fat_writer.next_free_cluster(prev_cluster, self.block_device.clone());
        let mut current_cluster = first_cluster;

        // 搜索可用簇，同时写表项
        #[allow(unused)]
        for i in 1..num {
            self.clear_cluster(current_cluster);
            let next_cluster =
                fat_writer.next_free_cluster(current_cluster, self.block_device.clone());
            assert_ne!(next_cluster, 0);
            fat_writer.set_next_cluster(current_cluster, next_cluster, self.block_device.clone());

            current_cluster = next_cluster;
        }
        self.clear_cluster(current_cluster);
        // 填写最后一个表项
        fat_writer.set_end(current_cluster, self.block_device.clone());
        // 修改FSINFO
        self.fsinfo
            .write_free_clusters(free_clusters - num, self.block_device.clone());
        // 写入分配的最后一个簇
        self.fsinfo
            .write_first_free_cluster(current_cluster, self.block_device.clone());
        self.cache_write_back();
        Some(first_cluster)
    }

    pub fn dealloc_cluster(&self, clusters: Vec<u32>) {
        let fat_writer = self.fat.write();
        let free_clusters = self.free_clusters();
        let num = clusters.len();
        for i in 0..num {
            // 将FAT对应表项清零
            fat_writer.set_next_cluster(clusters[i], FREE_CLUSTER, self.block_device.clone())
        }
        // 修改FSINFO
        if num > 0 {
            self.fsinfo
                .write_free_clusters(free_clusters + num as u32, self.block_device.clone());
            // 如果释放的簇号小于开始空闲簇字段，更新该字段
            if clusters[0] > 2
                && clusters[0] < self.fsinfo.first_free_cluster(self.block_device.clone())
            {
                self.fsinfo
                    .write_first_free_cluster(clusters[0] - 1, self.block_device.clone());
            }
        }
    }

    pub fn clear_cluster(&self, cluster_id: u32) {
        let start_sec = self.first_sector_of_cluster(cluster_id);
        for i in 0..self.sectors_per_cluster {
            get_block_cache(
                start_sec + i as usize,
                self.block_device.clone(),
                CacheMode::WRITE,
            )
            .write()
            .modify(0, |blk: &mut [u8; 512]| {
                for j in 0..512 {
                    blk[j] = 0;
                }
            });
        }
    }

    pub fn get_fat(&self) -> Arc<RwLock<FAT>> {
        Arc::clone(&self.fat)
    }

    /// 计算扩大至new_size(B)需要多少个簇
    pub fn cluster_num_needed(
        &self,
        old_size: u32,
        new_size: u32,
        is_dir: bool,
        first_cluster: u32,
    ) -> u32 {
        if old_size >= new_size {
            0
        } else {
            if is_dir {
                let old_clusters = self
                    .fat
                    .read()
                    .count_claster_num(first_cluster, self.block_device.clone());
                self.size_to_clusters(new_size) - old_clusters
            } else {
                self.size_to_clusters(new_size) - self.size_to_clusters(old_size)
            }
        }
    }

    /// 字节转化为所需的簇数
    pub fn size_to_clusters(&self, size: u32) -> u32 {
        (size + self.bytes_per_cluster - 1) / self.bytes_per_cluster
    }

    /// 计算当前偏移量在第几个簇
    pub fn cluster_of_offset(&self, offset: usize) -> u32 {
        offset as u32 / self.bytes_per_cluster
    }

    pub fn free_clusters(&self) -> u32 {
        self.fsinfo.read_free_clusters(self.block_device.clone())
    }

    /// 将长文件名拆分，并且补全0
    pub fn long_name_split(&self, name: &str) -> Vec<String> {
        let len = name.len() as u32; // 要有\0
        let name_bytes = name.as_bytes();
        let mut name_vec: Vec<String> = Vec::new();
        // 计算需要几个目录项
        let n_ent = (len + LONG_NAME_LEN - 1) / LONG_NAME_LEN;
        let mut temp_buffer = String::new();
        for i in 0..n_ent {
            temp_buffer.clear();
            for j in i * LONG_NAME_LEN..i * LONG_NAME_LEN + LONG_NAME_LEN {
                if j < len {
                    temp_buffer.push(name_bytes[j as usize] as char);
                } else if j > len {
                    temp_buffer.push(0xFF as char); //填充
                } else {
                    temp_buffer.push(0x00 as char);
                }
            }
            name_vec.push(temp_buffer.clone());
        }
        name_vec
    }

    /// 拆分文件名和后缀
    pub fn split_name_ext<'a>(&self, name: &'a str) -> (&'a str, &'a str) {
        let mut name_and_ext: Vec<&str> = name.split(".").collect();
        let name_ = name_and_ext[0];
        if name_and_ext.len() == 1 {
            name_and_ext.push("");
        }
        let ext_ = name_and_ext[1];
        (name_, ext_)
    }

    /// 将短文件名格式化为目录项存储的内容
    pub fn short_name_format(&self, name: &str) -> ([u8; 8], [u8; 3]) {
        let (mut name_, mut ext_) = self.split_name_ext(name);
        if name == "." || name == ".." {
            name_ = name;
            ext_ = ""
        }
        let name_bytes = name_.as_bytes();
        let ext_bytes = ext_.as_bytes();
        let mut f_name = [0u8; 8];
        let mut f_ext = [0u8; 3];
        for i in 0..8 {
            if i >= name_bytes.len() {
                f_name[i] = 0x20;
            } else {
                f_name[i] = (name_bytes[i] as char).to_ascii_uppercase() as u8;
            }
        }
        for i in 0..3 {
            if i >= ext_bytes.len() {
                f_ext[i] = 0x20;
            } else {
                f_ext[i] = (ext_bytes[i] as char).to_ascii_uppercase() as u8;
            }
        }
        (f_name, f_ext)
    }

    /// 由长文件名生成短文件名
    pub fn generate_short_name(&self, long_name: &str) -> String {
        // 目前仅支持【name.extension】 或者 【没有后缀】 形式的文件名！
        // 暂时不支持重复检测，即默认生成序号为~1
        // 无后缀
        let (name_, ext_) = self.split_name_ext(long_name);
        let name = name_.as_bytes();
        let extension = ext_.as_bytes();
        let mut short_name = String::new();
        for i in 0..6 {
            //fill name
            short_name.push((name[i] as char).to_ascii_uppercase())
        }
        short_name.push('~');
        short_name.push('1');
        let ext_len = extension.len();
        for i in 0..3 {
            //fill extension
            if i >= ext_len {
                short_name.push(0x20 as char); //填充
            } else {
                short_name.push((name[i] as char).to_ascii_uppercase()); //需要为大写
            }
        }
        short_name
    }

    pub fn cache_write_back(&self) {
        write_to_dev();
    }
}
