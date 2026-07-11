use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use gcst_common::{ByteStr, ReadExt};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};


/// Dumps the contents of a .bnk file.
#[derive(Parser)]
struct Opts {
    /// Path to the .bnk file.
    pub bnk_path: PathBuf,
}


#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct InstrumentBank {
    pub bank_id: u32,
    pub version: u32,
    pub padding: [u8; 16],
    pub sections: Vec<BankSection>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type")]
enum BankSection {
    Envelopes {
        envelopes: Vec<Envelope>,
    },
    Oscillators {
        oscillators: Vec<Oscillator>,
    },
    List {
        list_items: Vec<BankListItem>,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Envelope {
    pub mode: u16,
    pub delay: u16,
    pub value: i16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Oscillator {
    // id: u32, // "Osci"
    pub target: u32,
    pub rate: OrderedFloat<f32>,
    pub attack_envelope_offset: u32,
    pub release_envelope_offset: u32,
    pub scale: OrderedFloat<f32>,
    pub vertex: OrderedFloat<f32>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct PercussionMap {
    // id: u32, // "Pmap"
    pub volume: OrderedFloat<f32>,
    pub pitch: OrderedFloat<f32>,
    pub pan: u8,
    pub reserved0: u8,
    pub release: u16,
    pub oscillator_count: u32,
    // velocity_region_count: u32,
    pub velocity_regions: Vec<VelocityRegion>,
}

#[derive(Clone, Debug, Deserialize, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type")]
enum BankListItem {
    #[default]
    Invalid,
    Instrument(Instrument),
    Percussion(Percussion),
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Instrument {
    // oscillator_count: u32,
    pub oscillators: Vec<u32>,
    // random_effect_count: u32,
    pub random_effects: Vec<u32>,
    // key_region_count: u32,
    pub key_regions: Vec<KeyRegion>,
    pub volume: OrderedFloat<f32>,
    pub pitch: OrderedFloat<f32>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Percussion {
    pub percussion_maps: Vec<Option<PercussionMap>>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct KeyRegion {
    pub high_key_raw: u32,
    // velocity_region_count: u32,
    pub velocity_regions: Vec<VelocityRegion>,
}
impl KeyRegion {
    pub fn high_key(&self) -> u32 {
        self.high_key_raw >> 0x18
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct VelocityRegion {
    pub velocity: u8,
    pub padding: [u8; 3],
    pub wave_system_id: u16,
    pub wave_id: u16,
    pub volume: OrderedFloat<f32>,
    pub pitch: OrderedFloat<f32>,
}
impl VelocityRegion {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 16];
        reader.read_exact(&mut buf)?;
        Ok(VelocityRegion {
            velocity: buf[0],
            padding: buf[1..4].try_into().unwrap(),
            wave_system_id: u16::from_be_bytes(buf[4..6].try_into().unwrap()),
            wave_id: u16::from_be_bytes(buf[6..8].try_into().unwrap()),
            volume: OrderedFloat(f32::from_be_bytes(buf[8..12].try_into().unwrap())),
            pitch: OrderedFloat(f32::from_be_bytes(buf[12..16].try_into().unwrap())),
        })
    }
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
    let version = bnk_file.read_u32_be()
        .expect("failed to read version from .bnk file");

    let mut padding_buf = [0u8; 16];
    bnk_file.read_exact(&mut padding_buf)
        .expect("failed to read padding from .bnk file");
    if !padding_buf.iter().all(|b| *b == 0x00) {
        eprintln!("padding is not all zeroes: {:?}", padding_buf);
    }

    let mut sections = Vec::new();
    loop {
        let magic_opt = read_magic(&mut bnk_file)
            .expect("failed to read next section magic");
        let Some(magic) = magic_opt else { break };
        if magic == [0, 0, 0, 0] {
            // it's over, don't even bother reading the length
            // (there might be none)
            break;
        }

        let section_length: usize = bnk_file.read_u32_be()
            .expect("failed to read section length")
            .try_into().unwrap();

        let mut section_data = vec![0u8; section_length];
        if section_length > 0 {
            bnk_file.read_exact(&mut section_data)
                .expect("failed to read section data");
        }
        let mut section_cursor = Cursor::new(&section_data);

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
                for _ in 0..envelope_count {
                    let mode = section_cursor.read_u16_be()
                        .expect("failed to read envelope mode");
                    let delay = section_cursor.read_u16_be()
                        .expect("failed to read envelope delay");
                    let value = section_cursor.read_i16_be()
                        .expect("failed to read envelope value");
                    envelopes.push(Envelope {
                        mode,
                        delay,
                        value,
                    });
                }
                sections.push(BankSection::Envelopes { envelopes });
            },
            b"OSCT" => {
                // OSCillator Table
                let osc_count: usize = section_cursor.read_u32_be()
                    .expect("failed to read oscillator count from OSCT section of .bnk file")
                    .try_into().unwrap();
                let mut oscillators = Vec::with_capacity(osc_count);
                for _ in 0..osc_count {
                    section_cursor.read_exact(&mut magic_buf)
                        .expect("failed to read Osci magic from .bnk file");
                    if &magic_buf != b"Osci" {
                        eprintln!("OSCT section has non-Osci chunk; giving up on the section");
                        break;
                    }

                    let target = section_cursor.read_u32_be()
                        .expect("failed to obtain oscillator target");
                    let rate = OrderedFloat(
                        section_cursor.read_f32_be()
                            .expect("failed to obtain oscillator rate")
                    );
                    let attack_envelope_offset = section_cursor.read_u32_be()
                        .expect("failed to obtain oscillator attack envelope offset");
                    let release_envelope_offset = section_cursor.read_u32_be()
                        .expect("failed to obtain oscillator release envelope offset");
                    let scale = OrderedFloat(
                        section_cursor.read_f32_be()
                            .expect("failed to obtain oscillator scale")
                    );
                    let vertex = OrderedFloat(
                        section_cursor.read_f32_be()
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
                sections.push(BankSection::Oscillators { oscillators });
            },
            b"RAND" => {
                // RANDom effect
                let rand_count: usize = section_cursor.read_u32_be()
                    .expect("failed to read random-effect count from RAND section of .bnk file")
                    .try_into().unwrap();
                //let mut random_effects = Vec::with_capacity(rand_count);
                for _ in 0..rand_count {
                    panic!("don't know how to decode entries in the RAND section");
                }
            },
            b"SENS" => {
                // SENSor effect
                let sens_count: usize = section_cursor.read_u32_be()
                    .expect("failed to read random-effect count from SENS section of .bnk file")
                    .try_into().unwrap();
                //let mut sensor_effects = Vec::with_capacity(rand_count);
                for _ in 0..sens_count {
                    panic!("don't know how to decode entries in the SENS section");
                }
            },
            b"INST" => {
                // INSTruments
                // don't do anything yet; these are referenced from the LIST block
            },
            b"PMAP" => {
                // Percussion MAPs
                // don't do anything yet; these are referenced from the PERC block
            },
            b"PERC" => {
                // PERCussion
                // don't do anything yet; these are referenced from the LIST block
            },
            b"LIST" => {
                // LIST
                let item_count: usize = section_cursor.read_u32_be()
                    .expect("failed to read item count from LIST section of .bnk file")
                    .try_into().unwrap();
                let orig_pos = bnk_file.seek(io::SeekFrom::Current(0))
                    .expect("failed to remember current location within .bnk file");
                let mut list_items = Vec::with_capacity(item_count);

                for _ in 0..item_count {
                    let offset = section_cursor.read_u32_be()
                        .expect("failed to read a list offset from LIST section of .bnk file");
                    if offset == 0 {
                        list_items.push(BankListItem::Invalid);
                        continue;
                    }
                    bnk_file.seek(SeekFrom::Start(offset.into()))
                        .expect("failed to seek to offset pointed to by an entry of LIST section of .bnk file");
                    bnk_file.read_exact(&mut magic_buf)
                        .expect("failed to read magic from chunk pointed to by LIST section of .bnk file");
                    match magic_buf.as_slice() {
                        b"Inst" => {
                            let oscillator_count: usize = bnk_file.read_u32_be()
                                .expect("failed to read oscillator count from Inst chunk")
                                .try_into().unwrap();
                            let mut oscillators = Vec::with_capacity(oscillator_count);
                            for _ in 0..oscillator_count {
                                let oscillator_index = bnk_file.read_u32_be()
                                    .expect("failed to read oscillator index from Inst chunk");
                                oscillators.push(oscillator_index);
                            }

                            let random_effect_count: usize = bnk_file.read_u32_be()
                                .expect("failed to read random-effect count from Inst chunk")
                                .try_into().unwrap();
                            let mut random_effects = Vec::with_capacity(random_effect_count);
                            for _ in 0..random_effect_count {
                                let random_effect = bnk_file.read_u32_be()
                                    .expect("failed to read random effect index from Inst chunk");
                                random_effects.push(random_effect);
                            }

                            let key_region_count: usize = bnk_file.read_u32_be()
                                .expect("failed to read key-region count from Inst chunk")
                                .try_into().unwrap();
                            let mut key_regions = Vec::with_capacity(key_region_count);
                            for _ in 0..key_region_count {
                                let high_key_raw = bnk_file.read_u32_be()
                                    .expect("failed to read high-key value from Inst chunk");
                                let velocity_region_count: usize = bnk_file.read_u32_be()
                                    .expect("failed to read velocity region count from Inst chunk")
                                    .try_into().unwrap();
                                let mut velocity_regions = Vec::with_capacity(velocity_region_count);
                                for _ in 0..velocity_region_count {
                                    let velocity_region = VelocityRegion::read(&mut bnk_file)
                                        .expect("failed to read velocity region from Inst chunk");
                                    velocity_regions.push(velocity_region);
                                }
                                key_regions.push(KeyRegion {
                                    high_key_raw,
                                    velocity_regions,
                                });
                            }

                            let volume = OrderedFloat(
                                bnk_file.read_f32_be()
                                    .expect("failed to read volume from Inst chunk")
                            );
                            let pitch = OrderedFloat(
                                bnk_file.read_f32_be()
                                    .expect("failed to read pitch from Inst chunk")
                            );

                            list_items.push(BankListItem::Instrument(Instrument {
                                oscillators,
                                random_effects,
                                key_regions,
                                volume,
                                pitch,
                            }));
                        },
                        b"Perc" => {
                            let percussion_count: usize = bnk_file.read_u32_be()
                                .expect("failed to read percussion count from Perc chunk")
                                .try_into().unwrap();
                            let mut percussion_offsets = Vec::with_capacity(percussion_count);
                            for _ in 0..percussion_count {
                                let offset = bnk_file.read_u32_be()
                                    .expect("failed to read percussion offset from Perc chunk");
                                percussion_offsets.push(offset);
                            }

                            let mut percussion_maps = Vec::with_capacity(percussion_offsets.len());
                            for percussion_offset in &percussion_offsets {
                                if *percussion_offset == 0 {
                                    percussion_maps.push(None);
                                    continue;
                                }

                                bnk_file.seek(SeekFrom::Start((*percussion_offset).into()))
                                    .expect("failed to seek to percussion offset from Perc chunk");

                                bnk_file.read_exact(&mut magic_buf)
                                    .expect("failed to read magic of value pointed to by offset in Perc chunk");
                                match magic_buf.as_slice() {
                                    b"Pmap" => {
                                        let volume = OrderedFloat(
                                            bnk_file.read_f32_be()
                                                .expect("failed to read volume from Pmap chunk")
                                        );
                                        let pitch = OrderedFloat(
                                            bnk_file.read_f32_be()
                                                .expect("failed to read volume from Pmap chunk")
                                        );
                                        let pan = bnk_file.read_u8()
                                            .expect("failed to read pan value from Pmap chunk");
                                        let reserved0 = bnk_file.read_u8()
                                            .expect("failed to read reserved value from Pmap chunk");
                                        let release = bnk_file.read_u16_be()
                                            .expect("failed to read release from Pmap chunk");
                                        let oscillator_count = bnk_file.read_u32_be()
                                            .expect("failed to read oscillator count from Pmap chunk");
                                        let velocity_region_count: usize = bnk_file.read_u32_be()
                                            .expect("failed to read velocty region count from Pmap chunk")
                                            .try_into().unwrap();
                                        let mut velocity_regions = Vec::with_capacity(velocity_region_count);
                                        for _ in 0..velocity_region_count {
                                            let velocity_region = VelocityRegion::read(&mut bnk_file)
                                                .expect("failed to read velocity region from Inst chunk");
                                            velocity_regions.push(velocity_region);
                                        }

                                        percussion_maps.push(Some(PercussionMap {
                                            volume,
                                            pitch,
                                            pan,
                                            reserved0,
                                            release,
                                            oscillator_count,
                                            velocity_regions,
                                        }));
                                    },
                                    other => panic!("unknown magic at Perc offset: {:?}", other),
                                }
                            }

                            list_items.push(BankListItem::Percussion(Percussion {
                                percussion_maps,
                            }));
                        },
                        other => panic!("unknown chunk {:?} pointed to from LIST section of .bnk file", other),
                    }
                }
                sections.push(BankSection::List { list_items });
                bnk_file.seek(SeekFrom::Start(orig_pos))
                    .expect("failed to return to previous .bnk file location after jumping around due to LIST section");
            },
            other => {
                eprintln!("skipping unknown section {}", ByteStr(other));
            },
        }
    }

    let instrument_bank = InstrumentBank {
        bank_id,
        version,
        padding: padding_buf,
        sections,
    };
    let ibnk_json_string = serde_json::to_string_pretty(&instrument_bank)
        .expect("failed to serialize instrument bank to JSON");
    println!("{}", ibnk_json_string);
}
