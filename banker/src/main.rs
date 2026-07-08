use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::PathBuf;

use clap::Parser;
use gcst_common::ReadExt;
use ordered_float::OrderedFloat;


/// Dumps the contents of a .bnk file.
#[derive(Parser)]
struct Opts {
    /// Path to the .bnk file.
    pub bnk_path: PathBuf,
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct InstrumentBank {
    pub bank_id: u32,
    pub version: u32,
    pub padding: [u8; 16],
    pub sections: Vec<BankSection>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum BankSection {
    Envelopes {
        envelopes: Vec<Envelope>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Envelope {
    pub mode: u16,
    pub delay: u16,
    pub value: i16,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Oscillator {
    // id: u32, // "Osci"
    target: u32,
    rate: OrderedFloat<f32>,
    attack_envelope_offset: u32,
    release_envelope_offset: u32,
    scale: OrderedFloat<f32>,
    vertex: OrderedFloat<f32>,
}


fn read_magic<R: Read + Seek>(reader: &mut R) -> Result<Option<[u8; 4]>, io::Error> {
    let mut ret = [0u8; 4];

    // if the first read fails with EOF, return Ok(None)
    let read_count = match reader.read(&mut ret) {
        Ok(n) if n == ret.len() => {
            // we read the whole thing in one shot, nice
            return Ok(Some(ret));
        },
        Ok(0) => {
            // end of file
            return Ok(None);
        },
        Ok(n) => n,
        Err(e) => return Err(e),
    };

    // try reading the rest
    reader.read_exact(&mut ret[read_count..])?;

    // if we got this far, we succeeded
    Ok(Some(ret))
}

fn realign32<R: Read>(reader: &mut R, previous_data_length: usize) -> Result<(), io::Error> {
    let misalignment = previous_data_length % 4;
    if misalignment > 0 {
        let mut buf = [0u8; 4];
        let compensation = 4 - misalignment;
        reader.read_exact(&mut buf[..compensation])?;
    }
    Ok(())
}


fn main() {
    let opts = Opts::parse();

    let mut bnk_file = File::open(&opts.bnk_path)
        .expect("failed to open .bnk file");

    let mut magic_buf = [0u8; 4];
    bnk_file.read_exact(&mut magic_buf)
        .expect("failed to read IBNK magic from .bnk file");
    if &magic_buf != b"IBNK" {
        panic!(".bnk file has unexpected format");
    }

    bnk_file.read_u32_be()
        .expect("failed to read length from .bnk file");
    let bank_id = bnk_file.read_u32_be()
        .expect("failed to read bank ID from .bnk file");
    println!("bank ID: {}", bank_id);
    let version = bnk_file.read_u32_be()
        .expect("failed to read version from .bnk file");
    println!("version: {}", version);

    let mut padding_buf = [0u8; 16];
    bnk_file.read_exact(&mut padding_buf)
        .expect("failed to read padding from .bnk file");
    if !padding_buf.iter().all(|b| *b == 0x00) {
        println!("padding is not all zeroes: {:?}", padding_buf);
    }

    let mut section = Vec::new();
    loop {
        let magic_opt = read_magic(&mut bnk_file)
            .expect("failed to read next section magic");
        let Some(magic) = magic_opt else { break };

        let section_length: usize = bnk_file.read_u32_be()
            .expect("failed to read section length")
            .try_into().unwrap();

        let mut section_data = vec![0u8; section_length];
        if section_length > 0 {
            bnk_file.read_exact(&mut section_data)
                .expect("failed to read section data");
        }

        realign32(&mut bnk_file, section_length)
            .expect("failed to realign stream after section");

        match &magic {
            b"ENVT" => {
                // ENVelope Table
                let envelope_count = section_length / 6;
                if section_length % 6 != 0 {
                    eprintln!("ENVT section has invalid length (must be divisible by 6); rounding down");
                }
                let mut envelopes = Vec::with_capacity(envelope_count);
                for i in 0..envelope_count {
                    let offset = 6 * i;
                    let mode = u16::from_be_bytes(section_data[offset+0..offset+2].try_into().unwrap());
                    let delay = u16::from_be_bytes(section_data[offset+2..offset+4].try_into().unwrap());
                    let value = i16::from_be_bytes(section_data[offset+4..offset+6].try_into().unwrap());
                    envelopes.push(Envelope {
                        mode,
                        delay,
                        value,
                    });
                }
                section.push(BankSection::Envelopes { envelopes });
            },
            b"OSCT" => {
                // OSCillator Table
                let osc_count: usize = bnk_file.read_u32_be()
                    .expect("failed to read oscillator count from OSCT section of .bnk file")
                    .try_into().unwrap();
                let mut oscillators = Vec::with_capacity(osc_count);
                for _ in 0..osc_count {
                    bnk_file.read_exact(&mut magic_buf)
                        .expect("failed to read Osci magic from .bnk file");
                    if &magic_buf != b"Osci" {
                        eprintln!("OSCT section has non-Osci chunk; giving up on the section");
                        break;
                    }

                    let target = bnk_file.read_u32_be()
                        .expect("failed to obtain oscillator target");
                    let rate = OrderedFloat(
                        bnk_file.read_f32_be()
                            .expect("failed to obtain oscillator rate")
                    );
                    let attack_envelope_offset = bnk_file.read_u32_be()
                        .expect("failed to obtain oscillator attack envelope offset");
                    let release_envelope_offset = bnk_file.read_u32_be()
                        .expect("failed to obtain oscillator release envelope offset");
                    let scale = OrderedFloat(
                        bnk_file.read_f32_be()
                            .expect("failed to obtain oscillator scale")
                    );
                    let vertex = OrderedFloat(
                        bnk_file.read_f32_be()
                            .expect("failed to obtain oscillator vertex")
                    );
                    oscillators.push(Oscillator {
                        target,
                        rate,
                        attack_envelope_offset,
                        release_envelope_offset,
                        scale,
                        vertex,
                    });
                }
            },
            b"RAND" => {
                // RANDom effect
                todo!("decode RAND");
            },
            b"SENS" => {
                // SENSor effect
                todo!("decode SENS");
            },
            b"INST" => {
                // INSTruments
                todo!("decode INST");
            },
            b"PMAP" => {
                // Percussion MAPs
                todo!("decode PMAP");
            },
            b"PERC" => {
                // PERCussion
                todo!("decode PERC");
            },
            b"LIST" => {
                // LIST
                todo!("decode LIST");
            },
            &[0, 0, 0, 0] => {
                // end of file marker
                break;
            },
            other => {
                print!("skipping unknown section \"");
                for b in other {
                    if *b >= b' ' && *b <= b'~' {
                        print!("{}", char::from_u32((*b).into()).unwrap());
                    } else {
                        print!("\\x{:02X}", *b);
                    }
                }
                println!("\"\n");
            },
        }
    }

    bnk_file.read_exact(&mut magic_buf)
        .expect("failed to read INST magic from .bnk file");
    if &magic_buf != b"INST" {
        panic!(".bnk INST section has unexpected magic");
    }

    let mut nfi_buf = [0u8; 12];
    bnk_file.read_exact(&mut nfi_buf)
        .expect("failed to read INST header fields from .bnk file");

    let keyboard_count = bnk_file.read_u32_be()
        .expect("failed to read INST keyboard count from .bnk file");
}
