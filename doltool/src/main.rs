mod array_vec_slice;
mod dol;


use std::fs::File;
use std::io::Read;
use std::num::ParseIntError;
use std::path::PathBuf;

use clap::Parser;

use crate::dol::DolHeader;


/// Convert between DOL, ELF and raw binary formats.
///
/// Originally implemented by loser, TenOfTen, deufeufeu and Steve_-.
#[derive(Parser)]
enum Mode {
    /// Output information about a DOL file.
    Info(InfoOpts),

    /// Convert a DOL file into a raw binary file.
    Dol2Bin(Dol2BinOpts),

    /// Convert a DOL file into an ELF file.
    Dol2Elf(Dol2ElfOpts),

    /// Convert a BIN file into a DOL file.
    Bin2Dol(Bin2DolOpts),

    /// Convert an ELF file into a DOL file.
    Elf2Dol(Elf2DolOpts),
}


/// Default load address and entry point for bin2dol.
const DEFAULT_LOAD_ADDRESS: u32 = 0x80003100;


#[derive(Parser)]
struct InfoOpts {
    /// DOL file whose info to output.
    pub dol_path: PathBuf,
}

#[derive(Parser)]
struct Dol2BinOpts {
    /// DOL file to convert to a raw binary file.
    pub dol_path: PathBuf,

    /// Path for the output binary file.
    pub bin_path: PathBuf,
}

#[derive(Parser)]
struct Dol2ElfOpts {
    /// DOL file to convert to an ELF file.
    pub dol_path: PathBuf,

    /// Path for the output ELF file.
    pub elf_path: PathBuf,
}

#[derive(Parser)]
struct Bin2DolOpts {
    /// The hexadecimal memory address at which to load the binary data.
    #[arg(short, long, default_value_t = DEFAULT_LOAD_ADDRESS, value_parser = Bin2DolOpts::parse_hex)]
    pub load_address: u32,

    /// The hexadecimal memory address at which execution is to start.
    #[arg(short, long, default_value_t = DEFAULT_LOAD_ADDRESS, value_parser = Bin2DolOpts::parse_hex)]
    pub entry_point: u32,

    /// Raw binary file to convert to a DOL file.
    pub bin_path: PathBuf,

    /// Path for the output DOL file.
    pub dol_path: PathBuf,
}
impl Bin2DolOpts {
    fn parse_hex(s: &str) -> Result<u32, ParseIntError> {
        u32::from_str_radix(s, 16)
    }
}

#[derive(Parser)]
struct Elf2DolOpts {
    /// ELF file to convert to a DOL file.
    pub elf_path: PathBuf,

    /// Path for the output DOL file.
    pub dol_path: PathBuf,
}

fn main() {
    let mode = Mode::parse();

    match mode {
        Mode::Info(opts) => {
            let mut dol_file = File::open(&opts.dol_path)
                .expect("failed to open DOL file");
            let dol_header = DolHeader::try_read(&mut dol_file)
                .expect("failed to read DOL header");

            let text_sections = dol_header.text_addresses.iter()
                .zip(dol_header.text_offsets.iter())
                .zip(dol_header.text_sizes.iter())
                .enumerate()
                .map(|(i, ((a, o), s))| (i, *a, *o, *s));
            for (index, address, offset, size) in text_sections {
                println!("text section {}:", index);
                println!("  address:   {:#010X}", address);
                println!("  offset:    {:#010X}", offset);
                println!("  size:      {:#010X}", size);
            }

            let data_sections = dol_header.data_addresses.iter()
                .zip(dol_header.data_offsets.iter())
                .zip(dol_header.data_sizes.iter())
                .enumerate()
                .map(|(i, ((a, o), s))| (i, *a, *o, *s));
            for (index, address, offset, size) in data_sections {
                println!("data section {}:", index);
                println!("  address:   {:#010X}", address);
                println!("  offset:    {:#010X}", offset);
                println!("  size:      {:#010X}", size);
            }

            println!("bss section");
            println!("  address:   {:#010X}", dol_header.bss_address);
            println!("  size:      {:#010X}", dol_header.bss_size);

            println!("entry point: {:#010X}", dol_header.entry_point);

            if dol_header.padding.iter().copied().any(|e| e != 0) {
                println!("padding values:");
                for padding_value in dol_header.padding {
                    println!("  {:#010X}", padding_value);
                }
            }
        },
        other => todo!("not yet implemented"),
    }
}
