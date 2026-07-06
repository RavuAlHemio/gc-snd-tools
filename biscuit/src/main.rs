use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use gcst_bms;


/// Dumps the contents of a .bsc file.
#[derive(Parser)]
struct Opts {
    /// Path to the .bsc file.
    pub bsc_path: PathBuf,
}


fn main() {
    let opts = Opts::parse();

    let mut bsc_file = File::open(&opts.bsc_path)
        .expect("failed to open .bsc file");

    let mut bsc_magic = [0u8; 2];
    bsc_file.read_exact(&mut bsc_magic)
        .expect("failed to read magic from .bsc file");
    if &bsc_magic != b"SC" {
        panic!(".bsc file has unexpected format");
    }

    let mut category_count_buf = [0u8; 2];
    bsc_file.read_exact(&mut category_count_buf)
        .expect("failed to read category count from .bsc file");
    let category_count = u16::from_be_bytes(category_count_buf);

    let mut size_buf = [0u8; 4];
    bsc_file.read_exact(&mut size_buf)
        .expect("failed to read size from .bsc file");

    let mut offset_table_offsets = Vec::with_capacity(category_count.into());
    for _ in 0..category_count {
        let mut offset_buf = [0u8; 4];
        bsc_file.read_exact(&mut offset_buf)
            .expect("failed to read offset from .bsc file");
        let offset = u32::from_be_bytes(offset_buf);
        offset_table_offsets.push(offset);
    }

    let mut per_table_offsets = Vec::with_capacity(offset_table_offsets.len());
    for offset_table_offset in &offset_table_offsets {
        bsc_file.seek(SeekFrom::Start((*offset_table_offset).into()))
            .expect("failed to seek to offset table");
        let mut count_buf = [0u8; 4];
        bsc_file.read_exact(&mut count_buf)
            .expect("failed to read number of entries in offset table");
        let count_u32 = u32::from_be_bytes(count_buf);
        let count: usize = count_u32.try_into().unwrap();
        let mut this_table_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let mut offset_buf = [0u8; 4];
            bsc_file.read_exact(&mut offset_buf)
                .expect("failed to read offset from offset table");
            let offset = u32::from_be_bytes(offset_buf);
            this_table_offsets.push(offset);
        }
        per_table_offsets.push(this_table_offsets);
    }

    for table in &per_table_offsets {
        println!("table start");
        for offset in table {
            println!("  offset {:010X}", offset);
            bsc_file.seek(SeekFrom::Start((*offset).into()))
                .expect("failed to seek to BMS data");
            loop {
                let ev = gcst_bms::read_event(&mut bsc_file)
                    .expect("failed to read BMS event");
                println!("    {:?}", ev);
                if ev.ends_song() {
                    break;
                }
            }
        }
    }
}
