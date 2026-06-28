use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;


/// Reads a .bst file and its corresponding .bstn file and outputs the directory structure.
///
/// .bst and .bstn files can be extracted from .baa files using the companion tool baad.
///
/// Originally implemented by hcs.
#[derive(Parser)]
struct Opts {
    /// Path to the .bst file.
    pub bst_path: PathBuf,

    /// Path to the .bstn file corresponding to the .bst file.
    pub bstn_path: PathBuf,
}


const FILENAME_SIZE: usize = 256;


fn end_at_first_zero(buf: &[u8]) -> &[u8] {
    let zero_pos = buf.iter().position(|b| *b == 0x00);
    match zero_pos {
        Some(pos) => &buf[0..pos],
        None => buf,
    }
}


fn traverse<B: Read + Seek, N: Read + Seek>(
    bst_reader: &mut B,
    mut bst_offset: u32,
    bstn_reader: &mut N,
    mut bstn_offset: u32,
    dir_name: &str,
    leaf_addr: u32,
) {
    if leaf_addr != 0 {
        bstn_reader.seek(SeekFrom::Start(bstn_offset.into()))
            .expect("failed to seek in .bstn file to file name");
        let mut filename_buf = [0u8; FILENAME_SIZE];
        bstn_reader.read_exact(&mut filename_buf)
            .expect("filename in .bstn file truncated");
        let filename_slice = end_at_first_zero(&filename_buf);
        let filename_str = std::str::from_utf8(filename_slice)
            .expect("filename is invalid UTF-8");

        bst_reader.seek(SeekFrom::Start(u64::from(bst_offset) & 0x00FF_FFFF))
            .expect("failed to seek in .bst file to file entry");
        let mut bst_number_buf = [0u8; 4];
        bst_reader.read_exact(&mut bst_number_buf)
            .expect("file entry in .bst file truncated");
        let bst_number = u32::from_be_bytes(bst_number_buf);
        bst_reader.read_exact(&mut bst_number_buf)
            .expect("file entry in .bst file truncated");
        let bst_other_number = u32::from_be_bytes(bst_number_buf);
        bst_reader.read_exact(&mut bst_number_buf)
            .expect("file entry in .bst file truncated");
        let bst_third_number = u32::from_be_bytes(bst_number_buf);
        println!(
            "[{:08X}]=>{:08X}: {:08X} {:08X} {:08X}\t{}{}",
            leaf_addr, bst_offset, bst_number, bst_other_number, bst_third_number, dir_name, filename_str,
        );
        return;
    }

    bst_reader.seek(SeekFrom::Start(bst_offset.into()))
        .expect("failed to seek in .bst file");
    bstn_reader.seek(SeekFrom::Start(bstn_offset.into()))
        .expect("failed to seek in .bstn file");

    let mut subdir_count_buf = [0u8; 4];

    bst_reader.read_exact(&mut subdir_count_buf)
        .expect("failed to read subdirectory count from .bst file");
    bst_offset += 4;
    let bst_subdir_count = u32::from_be_bytes(subdir_count_buf);

    bstn_reader.read_exact(&mut subdir_count_buf)
        .expect("failed to read subdirectory count from .bstn file");
    bstn_offset += 4;
    let bstn_subdir_count = u32::from_be_bytes(subdir_count_buf);

    if bst_subdir_count != bstn_subdir_count {
        panic!("inconsistency with {:?} subdirectory count: {} != {}", dir_name, bst_subdir_count, bstn_subdir_count);
    }

    // leaf directories start with 0 entry in .bst
    let mut is_leaf_buf = [0u8; 4];
    bst_reader.read_exact(&mut is_leaf_buf)
        .expect("failed to read zero status");
    let is_leaf = if u32::from_be_bytes(is_leaf_buf) == 0 {
        // yup, is a leaf
        bst_offset += 4;
        true
    } else {
        // we will re-read the value later; don't adjust bst_offset
        false
    };

    let filename = if dir_name == "" {
        // the root has no name field
        "/".to_owned()
    } else {
        let mut name_offset_buf = [0u8; 4];
        bstn_reader.read_exact(&mut name_offset_buf)
            .expect("failed to read name offset from .bstn file");
        bstn_offset += 4;
        let name_offset = u32::from_be_bytes(name_offset_buf);
        bstn_reader.seek(SeekFrom::Start(name_offset.into()))
            .expect("failed to seek to name in .bstn file");

        let mut filename_buf = [0u8; FILENAME_SIZE];
        bstn_reader.read_exact(&mut filename_buf)
            .expect("failed to read filename from .bstn file");
        let filename_slice = end_at_first_zero(&filename_buf);
        let filename_str = std::str::from_utf8(filename_slice)
            .expect("invalid UTF-8 file name");
        format!("{}{}/", dir_name, filename_str)
    };

    for i in 0..bst_subdir_count {
        bst_reader.seek(SeekFrom::Start(u64::from(bst_offset + i * 4)))
            .expect("failed to seek to child entry in .bst file");
        bstn_reader.seek(SeekFrom::Start(u64::from(bstn_offset + i * 4)))
            .expect("failed to seek to child entry in .bstn file");

        let mut next_offset_buf = [0u8; 4];
        bst_reader.read_exact(&mut next_offset_buf)
            .expect("failed to read next .bst file offset");
        let next_bst_offset = u32::from_be_bytes(next_offset_buf);
        bstn_reader.read_exact(&mut next_offset_buf)
            .expect("failed to read next .bstn file offset");
        let next_bstn_offset = u32::from_be_bytes(next_offset_buf);

        traverse(
            bst_reader, next_bst_offset,
            bstn_reader, next_bstn_offset,
            &filename,
            if is_leaf { bst_offset + i*4 } else { 0 },
        );
    }
}


fn main() {
    let opts = Opts::parse();

    let mut bst_file = File::open(&opts.bst_path)
        .expect("failed to open .bst file");
    let mut bstn_file = File::open(&opts.bstn_path)
        .expect("failed to open .bstn file");

    let mut bst_magic = [0u8; 4];
    bst_file.read_exact(&mut bst_magic)
        .expect("failed to read magic from .bst file");
    if &bst_magic != b"BST " {
        panic!(".bst file has unexpected format");
    }

    let mut bstn_magic = [0u8; 4];
    bstn_file.read_exact(&mut bstn_magic)
        .expect("failed to read magic from .bstn file");
    if &bstn_magic != b"BSTN" {
        panic!(".bstn file has unexpected format");
    }

    // the value after the magic must be 0
    let mut nonzero_buf = [0u8; 4];
    bst_file.read_exact(&mut nonzero_buf)
        .expect(".bst file ends too early");
    if nonzero_buf.iter().any(|nz| *nz != 0) {
        panic!(".bst file has unexpected format");
    }
    bstn_file.read_exact(&mut nonzero_buf)
        .expect(".bstn file ends too early");
    if nonzero_buf.iter().any(|nz| *nz != 0) {
        panic!(".bstn file has unexpected format");
    }

    // the next value must be 0x0100_0000
    let mut one_buf = [0u8; 4];
    bst_file.read_exact(&mut one_buf)
        .expect(".bst file ends too early");
    if u32::from_be_bytes(one_buf) != 0x0100_0000 {
        panic!(".bst file has unexpected format");
    }
    bstn_file.read_exact(&mut one_buf)
        .expect(".bstn file ends too early");
    if u32::from_be_bytes(one_buf) != 0x0100_0000 {
        panic!(".bstn file has unexpected format");
    }

    let mut offset_buf = [0u8; 4];
    bst_file.read_exact(&mut offset_buf)
        .expect(".bst file ends too early");
    let bst_root_offset = u32::from_be_bytes(offset_buf);
    bstn_file.read_exact(&mut offset_buf)
        .expect(".bstn file ends too early");
    let bstn_root_offset = u32::from_be_bytes(offset_buf);

    traverse(
        &mut bst_file, bst_root_offset,
        &mut bstn_file, bstn_root_offset,
        "", 0,
    )
}
