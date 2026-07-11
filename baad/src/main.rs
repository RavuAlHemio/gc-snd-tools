use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use gcst_common::ReadExt;


/// Reads and extracts the components of a .baa file, including .bst and .bstn, into the directory
/// containing the .baa file.
///
/// Originally implemented by hcs.
#[derive(Parser)]
struct Opts {
    /// Path to the .baa file, e.g. Z2Sound.baa.
    pub baa_path: PathBuf,
}


fn extract_range<R: Read + Seek, W: Write>(
    reader: &mut R,
    writer: &mut W,
    chunk_type: &str,
    start_offset: u32,
    end_offset: u32,
    custom_return_pos: Option<u64>,
) {
    if start_offset > end_offset {
        panic!(
            "{} chunk start offset ({} = {:#X}) is beyond end offset ({} = {:#X})",
            chunk_type,
            start_offset, start_offset,
            end_offset, end_offset,
        );
    }
    let chunk_length = end_offset - start_offset;

    let return_pos = match custom_return_pos {
        Some(crp) => crp,
        None => {
            match reader.stream_position() {
                Ok(cp) => cp,
                Err(e) => {
                    panic!(
                        "failed to obtain current stream position when extracting {} chunk: {}",
                        chunk_type, e,
                    );
                },
            }
        },
    };
    if let Err(e) = reader.seek(SeekFrom::Start(start_offset.into())) {
        panic!(
            "failed to seek to start {:#X} of {} chunk: {}",
            start_offset, chunk_type, e,
        );
    }

    let mut buf = vec![0u8; 4*1024*1024];
    let mut remaining_bytes = chunk_length;
    while remaining_bytes > 0 {
        let this_time_bytes_u32 = remaining_bytes
            .min(buf.len().try_into().unwrap());
        let this_time_bytes_usize: usize = this_time_bytes_u32.try_into().unwrap();
        let actually_read_usize = match reader.read(&mut buf[..this_time_bytes_usize]) {
            Ok(ar) => ar,
            Err(e) => {
                panic!(
                    "failed to read up to {} bytes from {}: {}",
                    this_time_bytes_usize, chunk_type, e,
                );
            },
        };
        if let Err(e) = writer.write_all(&buf[..actually_read_usize]) {
            panic!(
                "failed to write {} bytes from {}: {}",
                actually_read_usize, chunk_type, e,
            );
        }
        let actually_read_u32: u32 = actually_read_usize.try_into().unwrap();
        remaining_bytes -= actually_read_u32;
    }

    if let Err(e) = reader.seek(SeekFrom::Start(return_pos)) {
        panic!(
            "failed to seek back to {:#X} after extracting {} chunk: {}",
            return_pos, chunk_type, e,
        );
    }
}


fn extract_start_end_offset<R: Read + Seek, W: Write>(
    reader: &mut R,
    writer: &mut W,
    chunk_type: &str,
) {
    let mut offsets_buf = [0u8; 8];
    if let Err(e) = reader.read_exact(&mut offsets_buf) {
        panic!("failed to read {} offsets: {}", chunk_type, e);
    }
    let start_offset = u32::from_be_bytes(offsets_buf[0..4].try_into().unwrap());
    let end_offset = u32::from_be_bytes(offsets_buf[4..8].try_into().unwrap());
    extract_range(reader, writer, chunk_type, start_offset, end_offset, None);
}


fn extract_type_offset<R: Read + Seek>(
    reader: &mut R,
    chunk_type: &str,
    third_value: bool,
    reader_path: &Path,
    counter: Option<&mut u64>,
    extension: &str,
) {
    let mut offsets_buf = [0u8; 12];
    let read_res = if third_value {
        reader.read_exact(&mut offsets_buf[0..12])
    } else {
        reader.read_exact(&mut offsets_buf[0..8])
    };
    if let Err(e) = read_res {
        panic!("failed to read {} offsets: {}", chunk_type, e);
    }
    let chunk_subtype = u32::from_be_bytes(offsets_buf[0..4].try_into().unwrap());
    let start_offset = u32::from_be_bytes(offsets_buf[4..8].try_into().unwrap());
    // we don't actually care about the third value

    // seek to the start offset, skip 4 bytes and read 4 bytes to obtain the length
    let return_pos = match reader.stream_position() {
        Ok(rp) => rp,
        Err(e) => {
            panic!(
                "failed to obtain stream position before seeking for {} chunk length: {}",
                chunk_type, e,
            );
        },
    };
    if let Err(e) = reader.seek(SeekFrom::Start(u64::from(start_offset) + 4)) {
        panic!(
            "failed to seek to {} chunk length position: {}",
            chunk_type, e,
        );
    }

    let data_length = match reader.read_u32_be() {
        Ok(l) => l,
        Err(e) => panic!(
            "failed to read data length of {} chunk: {}",
            chunk_type, e,
        ),
    };

    let subtype_string = if let Some(ctr) = counter {
        // append current counter value to subtype, then increment
        let sts = format!("{}_{}", chunk_subtype, *ctr);
        *ctr += 1;
        sts
    } else {
        chunk_subtype.to_string()
    };
    let writer_path = reader_path
        .with_added_extension(&subtype_string)
        .with_added_extension(extension);
    let mut writer = match File::create(&writer_path) {
        Ok(w) => w,
        Err(e) => {
            panic!(
                "failed to create output file {} for {} chunk: {}",
                writer_path.display(), chunk_type, e,
            );
        },
    };

    // don't forget the magic and data length fields!
    let length = 4 + 4 + data_length;

    extract_range(
        reader, &mut writer,
        chunk_type,
        start_offset, start_offset + length,
        Some(return_pos),
    );
}


fn main() {
    let opts = Opts::parse();

    let mut baa_file = File::open(&opts.baa_path)
        .expect("failed to open .baa file");

    let mut magic = [0u8; 4];
    baa_file.read_exact(&mut magic)
        .expect("failed to read magic from .baa file");
    if &magic != b"AA_<" {
        panic!(".baa file has unexpected format");
    }

    let mut bank_counter = 0;

    loop {
        let mut chunk_name = [0u8; 4];
        baa_file.read_exact(&mut chunk_name)
            .expect("failed to read chunk name");

        if &chunk_name == b"bst " {
            eprintln!("bst");
            // file.baa -> file.bst
            let bst_path = opts.baa_path
                .with_added_extension("bst");
            let mut bst_file = match File::create(&bst_path) {
                Ok(bf) => bf,
                Err(e) => {
                    panic!(
                        "failed to create output file {} for bst chunk: {}",
                        bst_path.display(), e,
                    );
                },
            };
            extract_start_end_offset(
                &mut baa_file,
                &mut bst_file,
                "bst",
            );
        } else if &chunk_name == b"bstn" {
            eprintln!("bstn");
            // file.baa -> file.bstn
            let bstn_path = opts.baa_path
                .with_added_extension("bstn");
            let mut bstn_file = match File::create(&bstn_path) {
                Ok(bf) => bf,
                Err(e) => {
                    panic!(
                        "failed to create output file {} for bstn chunk: {}",
                        bstn_path.display(), e,
                    );
                },
            };
            extract_start_end_offset(
                &mut baa_file,
                &mut bstn_file,
                "bstn",
            );
        } else if &chunk_name == b"ws  " {
            eprintln!("ws");
            // file.baa -> file.0.wsys
            extract_type_offset(
                &mut baa_file,
                "ws",
                true,
                &opts.baa_path,
                None,
                "wsys",
            )
        } else if &chunk_name == b"bnk " {
            eprintln!("bnk");
            // file.baa -> file.0_0.bnk
            extract_type_offset(
                &mut baa_file,
                "bnk",
                false,
                &opts.baa_path,
                Some(&mut bank_counter),
                "bnk",
            )
        } else if &chunk_name == b"bsc " {
            eprintln!("bsc");
            // file.baa -> file.bst
            let bsc_path = opts.baa_path
                .with_added_extension("bsc");
            let mut bsc_file = match File::create(&bsc_path) {
                Ok(bf) => bf,
                Err(e) => {
                    panic!(
                        "failed to create output file {} for bsc chunk: {}",
                        bsc_path.display(), e,
                    );
                },
            };
            extract_start_end_offset(
                &mut baa_file,
                &mut bsc_file,
                "bsc",
            );
        } else if &chunk_name == b"bfca" {
            eprintln!("bfca");
            let mut buf = [0u8; 4];
            baa_file.read_exact(&mut buf)
                .expect("failed to read value of bfca chunk");
            // nothing to extract here
        } else if &chunk_name == b">_AA" {
            // and we're done
            eprintln!(">_AA");
            break;
        } else {
            panic!(
                "unrecognized chunk: {:#04X} {:#04X} {:#04X} {:#04X}",
                chunk_name[0], chunk_name[1], chunk_name[2], chunk_name[3],
            );
        }
    }
}
