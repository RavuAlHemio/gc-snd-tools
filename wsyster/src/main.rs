use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use clap::Parser;


/// Reads a .wsys file and its assortment of .aw files, decoding each entry from AFC ADPCM to
/// regular PCM and storing it as .wav.
///
/// Originally implemented by hcs.
#[derive(Parser)]
struct Opts {
    /// Path to the .wsys file.
    pub wsys_path: PathBuf,

    /// Enables more detailed logging.
    #[arg(short, long)]
    pub verbose: bool,
}

const AW_FILENAME_LENGTH: usize = 112;

const AFC_COEFFICIENTS: [(i64, i64); 16] = [
    ( 0x0000,  0x0000),
    ( 0x0800,  0x0000),
    ( 0x0000,  0x0800),
    ( 0x0400,  0x0400),
    ( 0x1000, -0x0800),
    ( 0x0e00, -0x0600),
    ( 0x0c00, -0x0400),
    ( 0x1200, -0x0a00),
    ( 0x1068, -0x08c8),
    ( 0x12c0, -0x08fc),
    ( 0x1400, -0x0c00),
    ( 0x0800, -0x0800),
    ( 0x0400, -0x0400),
    (-0x0400,  0x0400),
    (-0x0400,  0x0000),
    (-0x0800,  0x0000),
];

fn end_at_first_zero(buf: &[u8]) -> &[u8] {
    let zero_pos = buf.iter().position(|b| *b == 0x00);
    match zero_pos {
        Some(pos) => &buf[0..pos],
        None => buf,
    }
}

/// Decodes a chunk of AFC ADPCM audio into linear PCM.
fn afc_decode_chunk(
    input_samples: &[u8],
    output_samples: &mut [i16],
    lookback1: &mut i16,
    lookback2: &mut i16,
) {
    assert_eq!(input_samples.len(), 9);
    assert_eq!(output_samples.len(), 16);

    let mut input_index = 0;
    let mut output_index = 0;

    let delta: u16 = 1 << ((input_samples[input_index] >> 4) & 0xF);
    let index = usize::from(input_samples[input_index] & 0xF);
    input_index += 1;

    let mut nibbles = [0i16; 16];
    for i in (0..16).step_by(2) {
        let j = input_samples[input_index] >> 4;
        nibbles[i] = j.into();
        let j = input_samples[input_index] & 0xF;
        nibbles[i+1] = j.into();
        input_index += 1;
    }

    for nibble in &mut nibbles {
        if *nibble >= 8 {
            *nibble -= 16;
        }
    }

    for i in 0..16 {
        let mut sample = (i64::from(delta) * i64::from(nibbles[i])) << 11;
        println!();
        println!("base sample: {}", sample);
        println!(
            "lb1={}, coef1={}, prod1={}, lb2={}, coef2={}, prod2={}",
            i64::from(*lookback1), AFC_COEFFICIENTS[index].0, i64::from(*lookback1) * AFC_COEFFICIENTS[index].0,
            i64::from(*lookback2), AFC_COEFFICIENTS[index].1, i64::from(*lookback2) * AFC_COEFFICIENTS[index].1,
        );
        sample += i64::from(*lookback1) * AFC_COEFFICIENTS[index].0;
        sample += i64::from(*lookback2) * AFC_COEFFICIENTS[index].1;
        println!("with lookback: {}", sample);
        sample >>= 11;
        println!("downshifted: {}", sample);

        // clamp
        let final_sample = if sample > 32767 {
            32767
        } else if sample < -32768 {
            -32768
        } else {
            sample as i16
        };
        output_samples[output_index] = final_sample;
        output_index += 1;

        *lookback2 = *lookback1;
        *lookback1 = final_sample;
    }
}

fn dump_afc<R: Read + Seek>(
    aw_file: &mut R,
    offset: u32,
    size: u32,
    sample_rate: u16,
    file_name: &str,
) {
    aw_file.seek(SeekFrom::Start(offset.into()))
        .expect("failed to seek to wave data in .aw file");

    let mut size_left = size + 9;
    let mut lookback1 = 0;
    let mut lookback2 = 0;

    let mut output_data = Vec::new();

    loop {
        size_left -= 9;
        if size_left < 9 {
            break;
        }

        let mut afc_buf = [0u8; 9];
        aw_file.read_exact(&mut afc_buf)
            .expect("failed to read AFC audio chunk from AW file");
        let mut pcm = [0i16; 16];
        afc_decode_chunk(
            &afc_buf,
            &mut pcm,
            &mut lookback1,
            &mut lookback2,
        );

        // spit out the samples as little-endian (because RIFF)
        for sample in pcm.iter() {
            output_data.extend_from_slice(&sample.to_le_bytes());
        }
    }

    let sample_rate_bytes = sample_rate.to_le_bytes();
    let bits_per_sample: u16 = 16;
    let bits_per_sample_bytes = bits_per_sample.to_le_bytes();
    let bytes_per_all_channels_sample: u16 = 1 * bits_per_sample / 8; // channels * bits per sample / 8
    let bytes_per_all_channels_sample_bytes = bytes_per_all_channels_sample.to_le_bytes();
    let bytes_per_sec = u32::from(sample_rate) * u32::from(bytes_per_all_channels_sample);
    let bytes_per_sec_bytes = bytes_per_sec.to_le_bytes();

    let mut wav_header = [
        b'R', b'I', b'F', b'F',
        0, 0, 0, 0, // RIFF data size placeholder (including b"WAVE" but excluding b"RIFF" and itself)
        b'W', b'A', b'V', b'E', // content type ("WAVE")

        // format block
        b'f', b'm', b't', b' ', // "fmt "
        0x10, 0x00, 0x00, 0x00, // format block data size
        0x01, 0x00, // format: u16 = 1 (integer PCM)
        0x01, 0x00, // channel_count: u16 = 1 (mono)
        sample_rate_bytes[0], sample_rate_bytes[1], 0x00, 0x00, // sample_rate: u32 (in Hz)
        bytes_per_sec_bytes[0], bytes_per_sec_bytes[1], bytes_per_sec_bytes[2], bytes_per_sec_bytes[3], // bytes_per_sec: u32 (sample_rate * bytes_per_all_channels_sample)
        bytes_per_all_channels_sample_bytes[0], bytes_per_all_channels_sample_bytes[1], // bytes_per_all_channels_sample: u16 (channels * bits_per_sample / 8)
        bits_per_sample_bytes[0], bits_per_sample_bytes[1], // bits_per_sample: u16

        // data block
        b'd', b'a', b't', b'a', // "data"
        0x00, 0x00, 0x00, 0x00, // data size placeholder
        // header ends, data begins

        // fortunately, RIFF doesn't have any trailing elements per chunk,
        // so we can consider its structure "header + data"
        // and not the encapsulation it actually is
    ];

    let data_size_u32_bytes = u32::try_from(output_data.len()).unwrap().to_le_bytes();
    let wav_size = (wav_header.len() - 8) + output_data.len();
    let wav_size_u32_bytes = u32::try_from(wav_size).unwrap().to_le_bytes();

    wav_header[4..8].copy_from_slice(&wav_size_u32_bytes);
    wav_header[40..44].copy_from_slice(&data_size_u32_bytes);

    let mut output_file = File::create(file_name)
        .expect("failed to create output file");
    output_file.write_all(&mut wav_header)
        .expect("failed to write .wav header");
    output_file.write_all(&mut output_data)
        .expect("failed to write .wav data");
    output_file.flush()
        .expect("failed to flush .wav file");
}

fn process_wsys<R: Read + Seek>(wsys: &mut R, verbose: bool) {
    let mut wsys_magic_buf = [0u8; 4];
    wsys.read_exact(&mut wsys_magic_buf)
        .expect("failed to read magic from .wsys file");
    if &wsys_magic_buf != b"WSYS" {
        panic!("unexpected magic; is the .wsys file a WSYS file?");
    }

    // skip three u32s we aren't interested in
    wsys.seek(SeekFrom::Current(12))
        .expect("failed to skip part of .wsys file header");

    // read, then seek to, WINF offset
    let mut winf_offset_buf = [0u8; 4];
    wsys.read_exact(&mut winf_offset_buf)
        .expect("failed to read WINF offset from .wsys file");
    let winf_offset = u32::from_be_bytes(winf_offset_buf);
    wsys.seek(SeekFrom::Start(winf_offset.into()))
        .expect("failed to seek to WINF offset within .wsys file");

    let mut winf_magic_buf = [0u8; 4];
    wsys.read_exact(&mut winf_magic_buf)
        .expect("failed to read WINF magic bytes from .wsys file");
    if &winf_magic_buf != b"WINF" {
        panic!("unexpected value at .wsys file offset {}; expected b'WINF'", winf_offset);
    }

    let mut aw_count_buf = [0u8; 4];
    wsys.read_exact(&mut aw_count_buf)
        .expect("failed to read number of AW entries");
    let aw_count = u32::from_be_bytes(aw_count_buf);
    if verbose {
        eprintln!("{} AW entries", aw_count);
    }

    for aw_i in 0..aw_count {
        wsys.seek(SeekFrom::Start(u64::from(winf_offset + 8 + aw_i*4)))
            .expect("failed to seek to AW entry");

        // each of these entries is itself an offset to data about the .aw
        let mut aw_offset_buf = [0u8; 4];
        wsys.read_exact(&mut aw_offset_buf)
            .expect("failed to read .aw data offset from .wsys");
        let aw_name_offset = u32::from_be_bytes(aw_offset_buf);
        let aw_table_offset = aw_name_offset + u32::try_from(AW_FILENAME_LENGTH).unwrap();

        wsys.seek(SeekFrom::Start(aw_name_offset.into()))
            .expect("failed to seek to AW metadata");

        // .aw file name first
        let mut aw_filename_buf = [0u8; AW_FILENAME_LENGTH];
        wsys.read_exact(&mut aw_filename_buf)
            .expect("failed to read AW filename");
        let aw_filename_slice = end_at_first_zero(&aw_filename_buf);
        let aw_filename_str = std::str::from_utf8(aw_filename_slice)
            .expect("AW filename is invalid UTF-8");

        let mut aw_file = match File::open(aw_filename_str) {
            Ok(af) => af,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    // it happens
                    eprintln!("{} not found", aw_filename_str);
                    continue;
                } else {
                    panic!("error opening {}: {}", aw_filename_str, e);
                }
            },
        };

        // after the filename is the number of waves
        let mut wave_count_buf = [0u8; 4];
        wsys.read_exact(&mut wave_count_buf)
            .expect("failed to read .aw wave count");
        let wave_count = u32::from_be_bytes(wave_count_buf);

        if verbose {
            println!("aw={}", aw_filename_str);
            println!("table at {:#X}, wav_count={:#X}", aw_table_offset, wave_count);
        }

        for wave_i in 0..wave_count {
            let mut wave_entry_offset_buf = [0u8; 4];
            wsys.seek(SeekFrom::Start(u64::from(aw_table_offset + 4 + wave_i*4)))
                .expect("failed to seek to wave entry offset");
            wsys.read_exact(&mut wave_entry_offset_buf)
                .expect("failed to read wave entry offset");
            let wave_entry_offset = u32::from_be_bytes(wave_entry_offset_buf);
            wsys.seek(SeekFrom::Start(wave_entry_offset.into()))
                .expect("failed to seek to wave entry");

            let mut wave_entry_buf = [0u8; 20];
            wsys.read_exact(&mut wave_entry_buf)
                .expect("failed to read wave entry");

            // ?? ?? ?? ?? ?? rr rr ?? oo oo oo oo ss ss ss ss ?? ?? ?? ??
            let sample_rate = u16::from_be_bytes(wave_entry_buf[5..7].try_into().unwrap()) >> 1;
            let afc_offset = u32::from_be_bytes(wave_entry_buf[8..12].try_into().unwrap());
            let afc_size = u32::from_be_bytes(wave_entry_buf[12..16].try_into().unwrap());
            if verbose {
                println!("index={:#010X}\toffset={:#X}\tsize={:#X}\tsrate={}", wave_i, afc_offset, afc_size, sample_rate);
            }

            let wav_filename = format!("{}_{:08X}.wav", aw_filename_str, wave_i);
            dump_afc(
                &mut aw_file,
                afc_offset,
                afc_size,
                sample_rate,
                &wav_filename,
            );
        }
    }
}

fn main() {
    let opts = Opts::parse();

    let mut wsys_file = File::open(&opts.wsys_path)
        .expect("failed to open .wsys file");
    process_wsys(&mut wsys_file, opts.verbose);
}
