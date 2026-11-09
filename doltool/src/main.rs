mod array_vec_slice;
mod dol;
mod elf;


use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::num::ParseIntError;
use std::path::PathBuf;

use clap::Parser;
use goblin::container::{Container, Ctx, Endian};
use goblin::elf::header::Header;
use goblin::elf32::header::{EM_PPC, ET_EXEC};
use goblin::elf32::section_header::{
    SectionHeader, SHF_ALLOC, SHF_EXECINSTR, SHF_WRITE, SHN_ABS, SHN_UNDEF, SHT_NOBITS, SHT_NULL,
    SHT_PROGBITS, SHT_STRTAB, SHT_SYMTAB,
};
use goblin::elf32::sym::{STB_GLOBAL, STT_NOTYPE, STV_DEFAULT, Sym};
use scroll::ctx::SizeWith;

use crate::dol::{DolFile, DolHeader};
use crate::elf::{ElfWriteExt, PadExt};


/// Convert between DOL, ELF and raw binary formats.
///
/// Originally implemented by loser, TenOfTen, deufeufeu and Steve_-.
#[derive(Parser)]
enum Mode {
    /// Output information about a DOL file.
    Info(InfoOpts),

    /// Convert a DOL file into a raw binary file.
    #[command(name = "dol2bin")]
    Dol2Bin(Dol2BinOpts),

    /// Convert a DOL file into an ELF file.
    #[command(name = "dol2elf")]
    Dol2Elf(Dol2ElfOpts),

    /// Convert a BIN file into a DOL file.
    #[command(name = "bin2dol")]
    Bin2Dol(Bin2DolOpts),

    /// Convert an ELF file into a DOL file.
    #[command(name = "elf2dol")]
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
    /// Text file mapping memory addresses to names.
    #[arg(short = 's', long = "symbol-map")]
    pub symbol_map_path: Option<PathBuf>,

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
        Mode::Dol2Elf(opts) => {
            let mut dol_file = File::open(&opts.dol_path)
                .expect("failed to open DOL file");
            let dol_file = DolFile::try_read(&mut dol_file)
                .expect("failed to read DOL file");

            let sym_map = if let Some(sym_file_path) = opts.symbol_map_path.as_ref() {
                let mut sym_map = BTreeMap::new();
                let sym_file = File::open(sym_file_path)
                    .expect("failed to open symbol map file");
                let mut sym_buf = BufReader::new(sym_file);
                let mut line = String::new();
                loop {
                    line.clear();
                    match sym_buf.read_line(&mut line) {
                        Ok(0) => {
                            // EOF
                            break;
                        },
                        Err(e) => panic!("failed to read line from symbol map file: {}", e),
                        Ok(_) => {},
                    }
                    let trimmed_line = line.trim_end_matches(['\r', '\n']);
                    if trimmed_line.starts_with('#') || trimmed_line.len() == 0 {
                        // comment or empty line
                        continue;
                    }

                    let (addr_str, name_str) = match trimmed_line.split_once(' ') {
                        Some(an) => an,
                        None => {
                            // invalid line
                            continue;
                        },
                    };
                    let addr = u32::from_str_radix(addr_str, 16)
                        .expect("invalid symbol address");

                    let name_opt = if name_str.starts_with("0x") && &name_str[2..] == addr_str {
                        // unnamed entry
                        None
                    } else {
                        Some(name_str.to_owned())
                    };

                    sym_map.insert(addr, name_opt);
                }
                Some(sym_map)
            } else {
                None
            };

            // prepare a minimal ELF file
            let ctx = Ctx::new(Container::Little, Endian::Big);
            let mut header = Header::new(ctx);
            header.e_type = ET_EXEC;
            header.e_machine = EM_PPC;
            header.e_entry = dol_file.entry_point.into();
            header.e_shstrndx = 1;

            // ELF sections:
            // 0: header extension section
            // 1: string table
            // 2...: text sections
            // ...: data sections
            // ...: BSS section
            // ...: symbol table (if any)
            // ...: symbol string table (if symbol table exists)
            // (section name index entries are +1 because 0 is the empty string)

            let mut section_name_buf = String::new();
            let mut section_headers = Vec::with_capacity(2 + dol_file.text_sections.len() + dol_file.data_sections.len());

            section_headers.push(SectionHeader {
                sh_name: section_name_buf.len().try_into().unwrap(),
                sh_type: SHT_NULL,
                sh_flags: 0,
                sh_addr: 0,
                sh_offset: 0,
                sh_size: 0,
                sh_link: 0,
                sh_info: 0,
                sh_addralign: 0,
                sh_entsize: 0,
            });
            section_name_buf.push_str("\0");

            section_headers.push(SectionHeader {
                sh_name: section_name_buf.len().try_into().unwrap(),
                sh_type: SHT_STRTAB,
                sh_flags: 0,
                sh_addr: 0,
                sh_offset: 0,
                sh_size: 0,
                sh_link: 0,
                sh_info: 0,
                sh_addralign: 1,
                sh_entsize: 0,
            });
            section_name_buf.push_str(".shstrtab\0");

            for (i, text_section) in dol_file.text_sections.iter().enumerate() {
                section_headers.push(SectionHeader {
                    sh_name: section_name_buf.len().try_into().unwrap(),
                    sh_type: SHT_PROGBITS,
                    sh_flags: SHF_ALLOC | SHF_EXECINSTR,
                    sh_addr: text_section.address,
                    sh_offset: 0,
                    sh_size: text_section.data.len().try_into().unwrap(),
                    sh_link: 0,
                    sh_info: 0,
                    sh_addralign: 32,
                    sh_entsize: 0,
                });
                section_name_buf.push_str(&format!(".text{}\0", i));
            }

            for (i, text_section) in dol_file.data_sections.iter().enumerate() {
                section_headers.push(SectionHeader {
                    sh_name: section_name_buf.len().try_into().unwrap(),
                    sh_type: SHT_PROGBITS,
                    sh_flags: SHF_WRITE | SHF_ALLOC,
                    sh_addr: text_section.address,
                    sh_offset: 0,
                    sh_size: text_section.data.len().try_into().unwrap(),
                    sh_link: 0,
                    sh_info: 0,
                    sh_addralign: 32,
                    sh_entsize: 0,
                });
                section_name_buf.push_str(&format!(".data{}\0", i));
            }

            section_headers.push(SectionHeader {
                sh_name: section_name_buf.len().try_into().unwrap(),
                sh_type: SHT_NOBITS,
                sh_flags: SHF_WRITE | SHF_ALLOC,
                sh_addr: dol_file.bss.address,
                sh_offset: 0,
                sh_size: dol_file.bss.size,
                sh_link: 0,
                sh_info: 0,
                sh_addralign: 1,
                sh_entsize: 0,
            });
            section_name_buf.push_str(".bss\0");

            if sym_map.is_some() {
                section_headers.push(SectionHeader {
                    sh_name: section_name_buf.len().try_into().unwrap(),
                    sh_type: SHT_SYMTAB,
                    sh_flags: 0,
                    sh_addr: 0,
                    sh_offset: 0, // to be filled in later
                    sh_size: 0, // to be filled in later
                    sh_link: u32::try_from(section_headers.len()).unwrap() + 1, // index of the symbol name table
                    sh_info: 0,
                    sh_addralign: 1,
                    sh_entsize: 0x10,
                });
                section_name_buf.push_str(".symtab\0");

                section_headers.push(SectionHeader {
                    sh_name: section_name_buf.len().try_into().unwrap(),
                    sh_type: SHT_STRTAB,
                    sh_flags: 0,
                    sh_addr: 0,
                    sh_offset: 0, // to be filled in later
                    sh_size: 0, // to be filled in later
                    sh_link: 0,
                    sh_info: 0,
                    sh_addralign: 1,
                    sh_entsize: 0x10,
                });
                section_name_buf.push_str(".strtab\0");
            }

            // the section header table starts right after the header
            let mut offset = Header::size(ctx);
            header.e_shoff = offset.try_into().unwrap();

            // get section header table sizing
            let section_header_entry_size = SectionHeader::size_with(&ctx.le);
            let section_header_count = section_headers.len();

            // update the global header with information about the section table
            header.e_shentsize = section_header_entry_size.try_into().unwrap();
            header.e_shnum = section_header_count.try_into().unwrap();

            // add size of section header table to offset
            offset += section_header_entry_size * section_header_count;

            // round up to 32 bits
            while offset % 4 != 0 {
                offset += 1;
            }

            // update the section entry for the section name table
            section_headers[1].sh_offset = offset.try_into().unwrap();
            section_headers[1].sh_size = section_name_buf.len().try_into().unwrap();
            offset += section_name_buf.len();

            // round up to 32 bits
            while offset % 4 != 0 {
                offset += 1;
            }

            let mut section_header_index = 2;
            for text_section in &dol_file.text_sections {
                // update the section entry
                section_headers[section_header_index].sh_offset = offset.try_into().unwrap();
                section_headers[section_header_index].sh_size = text_section.data.len().try_into().unwrap();
                offset += text_section.data.len();

                // round up to 32 bits
                while offset % 4 != 0 {
                    offset += 1;
                }

                section_header_index += 1;
            }
            for data_section in &dol_file.data_sections {
                // update the section entry
                section_headers[section_header_index].sh_offset = offset.try_into().unwrap();
                section_headers[section_header_index].sh_size = data_section.data.len().try_into().unwrap();
                offset += data_section.data.len();

                // round up to 32 bits
                while offset % 4 != 0 {
                    offset += 1;
                }

                section_header_index += 1;
            }

            // the BSS section does not exist in the file; skip it
            section_header_index += 1;

            let mut symbols_opt = None;
            if let Some(sm) = sym_map {
                let mut symbol_table_bs = Vec::new();
                let mut symbol_table_cursor = Cursor::new(&mut symbol_table_bs);
                let mut symbol_name_buf = String::new();

                let null_entry = Sym {
                    st_name: symbol_name_buf.len().try_into().unwrap(),
                    st_value: 0,
                    st_size: 0,
                    st_info: 0,
                    st_other: 0,
                    st_shndx: SHN_UNDEF.try_into().unwrap(),
                };
                symbol_name_buf.push_str("\0");
                null_entry.write_elf(&mut symbol_table_cursor, ctx)
                    .expect("failed to write null symbol table entry");

                for (addr, name_opt) in &sm {
                    let entry = Sym {
                        st_name: if name_opt.is_some() {
                            symbol_name_buf.len().try_into().unwrap()
                        } else {
                            0
                        },
                        st_value: *addr,
                        st_size: 0,
                        st_info: (STB_GLOBAL << 4) | STT_NOTYPE,
                        st_other: STV_DEFAULT,
                        st_shndx: SHN_ABS.try_into().unwrap(),
                    };
                    if let Some(name) = name_opt.as_ref() {
                        symbol_name_buf.push_str(name);
                        symbol_name_buf.push('\0');
                    }
                    entry.write_elf(&mut symbol_table_cursor, ctx)
                        .expect("failed to write concrete symbol table entry");
                }

                symbol_table_cursor.flush()
                    .expect("failed to flush symbol table writer");
                drop(symbol_table_cursor);

                section_headers[section_header_index].sh_offset = offset.try_into().unwrap();
                section_headers[section_header_index].sh_size = symbol_table_bs.len().try_into().unwrap();
                section_headers[section_header_index].sh_entsize = Sym::size_with(&ctx.le).try_into().unwrap();
                offset += symbol_table_bs.len();

                // advance to the symbol name table
                section_header_index += 1;

                section_headers[section_header_index].sh_offset = offset.try_into().unwrap();
                section_headers[section_header_index].sh_size = symbol_name_buf.len().try_into().unwrap();

                symbols_opt = Some((symbol_table_bs, symbol_name_buf));
            }

            // now spit it all out
            let mut elf_buf = Vec::new();
            let mut elf_cur = Cursor::new(&mut elf_buf);
            header.write_elf(&mut elf_cur, ctx)
                .expect("failed to write ELF header");
            for section_header in &section_headers {
                section_header.write_elf(&mut elf_cur, ctx)
                    .expect("failed to write ELF header");
            }
            elf_cur.pad_to(4)
                .expect("failed to pad");

            elf_cur.write_all(section_name_buf.as_bytes())
                .expect("failed to write section name data");
            elf_cur.pad_to(4)
                .expect("failed to pad");

            for text_section in &dol_file.text_sections {
                elf_cur.write_all(&text_section.data)
                    .expect("failed to write out section data");
                elf_cur.pad_to(4)
                    .expect("failed to pad");
            }
            for data_section in &dol_file.data_sections {
                elf_cur.write_all(&data_section.data)
                    .expect("failed to write out section data");
                elf_cur.pad_to(4)
                    .expect("failed to pad");
            }
            if let Some((symbol_table_bs, symbol_name_buf)) = symbols_opt {
                elf_cur.write_all(&symbol_table_bs)
                    .expect("failed to write out symbol table data");
                elf_cur.write_all(symbol_name_buf.as_bytes())
                    .expect("failed to write out symbol name data");
            }

            // done
            elf_cur.flush()
                .expect("failed to flush output");
            drop(elf_cur);

            // output all that
            std::fs::write(&opts.elf_path, &elf_buf)
                .expect("failed to write ELF output");
        },
        _ => todo!("not yet implemented"),
    }
}
