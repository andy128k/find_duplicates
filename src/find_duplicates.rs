use crate::exclusion::Exclusion;
use humansize::{format_size, DECIMAL};
use lazy_static::lazy_static;
use sha2::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Digest, Sha256,
};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, Metadata};
use std::hash::Hash;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub size: u64,
    pub disk_usage: u64,
    pub device: u64,
    pub inode: u64,
}

impl FileInfo {
    pub fn from_path_and_metadata(
        path: impl Into<PathBuf>,
        metadata: Metadata,
    ) -> io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let modified = metadata.modified()?;
        let size = metadata.len();
        let disk_usage = metadata.blocks() * metadata.blksize();
        let device = metadata.dev();
        let inode = metadata.ino();
        Ok(FileInfo {
            path: path.into(),
            modified,
            size,
            disk_usage,
            device,
            inode,
        })
    }
}

fn find_files(
    dir: &Path,
    files: &mut Vec<FileInfo>,
    exclude: &[glob::Pattern],
    min_size: u64,
    recurse: bool,
) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            let skip = exclude.iter().any(|pattern| pattern.matches_path(&path));
            if skip {
                continue;
            }

            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                if recurse {
                    find_files(&path, files, exclude, min_size, recurse)?;
                }
            } else if metadata.is_file() {
                let fi = FileInfo::from_path_and_metadata(path, metadata)?;
                if fi.size >= min_size {
                    files.push(fi);
                }
            }
        }
    }
    Ok(())
}

fn find_files_in_dirs(
    dirs: &[PathBuf],
    exclude: &[glob::Pattern],
    min_size: u64,
    recurse: bool,
) -> io::Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    for dir in dirs {
        find_files(dir, &mut files, exclude, min_size, recurse)?;
    }
    Ok(files)
}

fn get_file_hash(
    fi: &FileInfo,
) -> io::Result<GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>> {
    lazy_static! {
        static ref EMPTY_HASH: GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize> =
            Sha256::new().finalize();
    }

    if fi.size > 0 {
        let mut hasher = Sha256::new();
        let file = std::fs::File::open(&fi.path)?;
        let mut reader = io::BufReader::new(file);
        io::copy(&mut reader, &mut hasher)?;
        let digest = hasher.finalize();
        Ok(digest)
    } else {
        Ok(*EMPTY_HASH)
    }
}

fn unique_by<K: Hash + Eq>(
    get_key: impl Fn(&FileInfo) -> io::Result<K>,
    fis: Vec<FileInfo>,
) -> io::Result<Vec<FileInfo>> {
    let mut map: HashMap<K, FileInfo> = HashMap::new();
    for fi in fis {
        let key = get_key(&fi)?;
        map.insert(key, fi);
    }
    Ok(map.drain().map(|(_k, v)| v).collect())
}

fn group_by<K: Hash + Eq>(
    get_key: impl Fn(&FileInfo) -> io::Result<K>,
    fis: Vec<FileInfo>,
) -> io::Result<Vec<Vec<FileInfo>>> {
    let mut groups: HashMap<K, Vec<FileInfo>> = HashMap::new();
    for fi in fis {
        let key = get_key(&fi)?;
        groups.entry(key).or_insert_with(Vec::new).push(fi);
    }
    groups.retain(|_, fis| fis.len() >= 2);
    Ok(groups.drain().map(|(_k, v)| v).collect())
}

fn group_by_size(fis: Vec<FileInfo>) -> io::Result<Vec<Vec<FileInfo>>> {
    group_by(|fi| Ok(fi.size), fis)
}

fn group_by_hash(fis: Vec<FileInfo>) -> io::Result<Vec<Vec<FileInfo>>> {
    group_by(get_file_hash, fis)
}

fn split(
    groups: Vec<Vec<FileInfo>>,
    fun: impl Fn(Vec<FileInfo>) -> io::Result<Vec<Vec<FileInfo>>>,
) -> io::Result<Vec<Vec<FileInfo>>> {
    let mut result = vec![];
    for group in groups {
        let more_groups = (fun)(group)?;
        result.extend(more_groups);
    }
    Ok(result)
}

fn find_duplicates(
    paths: &[PathBuf],
    exclude: &[glob::Pattern],
    min_size: u64,
    recurse: bool,
) -> io::Result<Vec<Vec<FileInfo>>> {
    let files = find_files_in_dirs(paths, exclude, min_size, recurse)?;

    let files = unique_by(|fi| Ok((fi.device, fi.inode)), files)?;
    let files = unique_by(|fi| Ok(fi.path.clone()), files)?;

    let mut groups: Vec<Vec<FileInfo>> = vec![files];
    groups = split(groups, group_by_size)?;
    groups = split(groups, group_by_size)?;
    groups = split(groups, group_by_hash)?;

    Ok(groups)
}

#[derive(Debug)]
pub struct DuplicatesGroup {
    pub files: Vec<FileInfo>,
}

impl DuplicatesGroup {
    pub fn size(&self) -> u64 {
        self.files[0].size
    }

    pub fn waste(&self) -> u64 {
        self.size() * self.files.len() as u64
    }
}

fn exclusion_to_pattern(exclusion: &Exclusion) -> Result<glob::Pattern, Box<dyn Error>> {
    let pattern = match exclusion {
        Exclusion::Directory(dir) => glob::Pattern::new(
            dir.to_str()
                .ok_or_else(|| format!("Cannot create glob pattern from {}.", dir.display()))?,
        )?,
        Exclusion::Pattern(pattern) => glob::Pattern::new(pattern)?,
    };
    Ok(pattern)
}

pub fn find_duplicate_groups(
    paths: &[PathBuf],
    exclude: &[Exclusion],
    min_size: u64,
    recurse: bool,
) -> Result<Vec<DuplicatesGroup>, Box<dyn Error>> {
    let exclude: Vec<glob::Pattern> = exclude
        .iter()
        .map(exclusion_to_pattern)
        .collect::<Result<_, _>>()?;

    let duplicates1 = find_duplicates(&paths, &exclude, min_size, recurse)?;

    let mut duplicates: Vec<DuplicatesGroup> = vec![];
    for dup in duplicates1 {
        duplicates.push(DuplicatesGroup { files: dup });
    }

    duplicates.sort_by_key(|group| group.waste());
    duplicates.reverse();

    Ok(duplicates)
}

pub fn duplication_status(dups: &[DuplicatesGroup]) -> String {
    let mut waste_bytes = 0;
    let mut waste_count = 0;
    for dup in dups {
        waste_bytes += dup.waste();
        waste_count += dup.files.len() - 1;
    }

    format!(
        "{} wasted in {} files (in {} groups)",
        format_size(waste_bytes, DECIMAL),
        waste_count,
        dups.len()
    )
}
