use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use gcst_bms::{self, Event};


/// Dumps the contents of a .bsc file.
#[derive(Parser)]
struct Opts {
    /// Path to the .bsc file.
    pub bsc_path: PathBuf,

    /// Ignore this offset in the .bsc file when outputting all banks. Can be specified multiple times.
    #[arg(short, long)]
    pub ignore_offset: Vec<u32>,
}

fn print_depth_prefix(depth: usize) {
    for _ in 0..depth {
        print!("  ");
    }
}

fn output_sequence(bsc_file: &mut File) {
    let mut call_stack = Vec::new();
    let mut jumped_here_before = BTreeSet::new();
    loop {
        let ev = gcst_bms::read_event(bsc_file)
            .expect("failed to read BMS event");

        let depth = 2 + call_stack.len();
        print_depth_prefix(depth);
        println!("{:?}", ev);

        match &ev {
            Event::Call { target } => {
                if let Some(target_u32) = target.as_u32() {
                    let return_here = bsc_file.seek(SeekFrom::Current(0))
                        .expect("failed to obtain current position");
                    call_stack.push(return_here);
                    bsc_file.seek(SeekFrom::Start(target_u32.into()))
                        .expect("failed to seek to call destination");
                } else {
                    // we eventually continue from here,
                    // so no need to break out
                }
            },
            Event::Jump { target } => {
                if let Some(target_u32) = target.as_u32() {
                    if jumped_here_before.insert(target_u32) {
                        bsc_file.seek(SeekFrom::Start(target_u32.into()))
                            .expect("failed to seek to jump destination");
                    } else {
                        print_depth_prefix(depth);
                        println!("(and we loop)");
                        break;
                    }
                } else {
                    // we don't know where we jumped; it's over for us
                    print_depth_prefix(depth);
                    println!("the trail goes cold");
                    break;
                }
            },
            Event::JumpTable { .. } => {
                print_depth_prefix(depth);
                println!("the trail goes cold");
                break;
            },
            Event::Return => {
                let return_there = call_stack.pop()
                    .expect("returning from an empty call stack?!");
                bsc_file.seek(SeekFrom::Start(return_there))
                    .expect("failed to go back to return address");
            },
            Event::Finish => {
                if let Some(return_there) = call_stack.pop() {
                    bsc_file.seek(SeekFrom::Start(return_there))
                        .expect("failed to go back to return address at finish");
                } else {
                    break;
                }
            },
            Event::OpenTrack { track_pointer, .. } => {
                // act like this is a Call
                if let Some(target_u32) = track_pointer.as_u32() {
                    let return_here = bsc_file.seek(SeekFrom::Current(0))
                        .expect("failed to obtain current position");
                    call_stack.push(return_here);
                    bsc_file.seek(SeekFrom::Start(target_u32.into()))
                        .expect("failed to seek to open-track destination");
                } else {
                    // we eventually continue from here,
                    // so no need to break out
                }
            },
            _ => {},
        }
    }
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
            println!("  offset {:#010X} ({})", offset, offset);
            if opts.ignore_offset.contains(offset) {
                println!("    ignoring as requested");
                continue;
            }
            bsc_file.seek(SeekFrom::Start((*offset).into()))
                .expect("failed to seek to BMS data");
            output_sequence(&mut bsc_file);
        }
    }
}
