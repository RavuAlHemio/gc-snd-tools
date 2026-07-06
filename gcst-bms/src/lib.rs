mod max_array;


use std::{io::{self, Read, Seek}, sync::LazyLock};

use crate::max_array::MaxArray;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValueType {
    Immediate8,
    Immediate16,
    Immediate24,
    RegisterRead,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Immediate8(u8),
    Immediate16(u16),
    Immediate24(u32),
    RegisterRead(u8),
}
impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Immediate8(_) => ValueType::Immediate8,
            Self::Immediate16(_) => ValueType::Immediate16,
            Self::Immediate24(_) => ValueType::Immediate24,
            Self::RegisterRead(_) => ValueType::RegisterRead,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::Immediate8(val) => Some((*val).into()),
            Self::Immediate16(val) => Some((*val).into()),
            Self::Immediate24(val) => Some((*val).into()),
            Self::RegisterRead(_) => None,
        }
    }
}


/// Parameter format definitions for commands 0xA0 and beyond.
///
/// Parsing must be done through this indirection because commands 0x90-0x9F may override some
/// parameters to be register reads instead.
static COMMAND_PARAMS: LazyLock<[Option<MaxArray<ValueType, 5>>; 96]> = LazyLock::new(|| [
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(max_array![ // NoteOn
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::Immediate8,
    ]),
    Some(max_array![ // NoteOff
        ValueType::Immediate8,
    ]),
    Some(max_array![ // Note
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // SetLastNote
        ValueType::Immediate8,
    ]),
    None,
    None,
    None,
    Some(max_array![ // ParamE
        ValueType::Immediate8,
        ValueType::Immediate8,
    ]),
    Some(max_array![ // ParamI
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // ParamEI
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // ParamII
        ValueType::Immediate8,
        ValueType::Immediate16,
        ValueType::Immediate16,
    ]),
    None,
    None,
    None,
    None,
    None,
    Some(max_array![ // OpenTrack
        ValueType::Immediate8,
        ValueType::Immediate24,
    ]),
    Some(max_array![ // CloseTrack
        ValueType::Immediate8,
    ]),
    Some(max_array![ // Call
        ValueType::Immediate24,
    ]),
    Some(max_array![ // CallF
        ValueType::Immediate8,
        ValueType::Immediate24,
    ]),
    Some(max_array![]), // Ret
    Some(max_array![ // RetF
        ValueType::Immediate8,
    ]),
    Some(max_array![ // Jmp
        ValueType::Immediate24,
    ]),
    Some(max_array![ // JmpF
        ValueType::Immediate8,
        ValueType::Immediate24,
    ]),
    Some(max_array![ // JmpTable
        ValueType::RegisterRead,
        ValueType::Immediate24,
    ]),
    Some(max_array![ // CallTable
        ValueType::RegisterRead,
        ValueType::Immediate24,
    ]),
    Some(max_array![ // LoopS
        ValueType::Immediate16,
    ]),
    Some(max_array![]), // LoopE
    None,
    None,
    None,
    Some(max_array![ // ReadPort
        ValueType::Immediate8,
        ValueType::Immediate8,
    ]),
    Some(max_array![ // WritePort
        ValueType::Immediate8,
        ValueType::RegisterRead,
    ]),
    Some(max_array![ // CheckPortImport
        ValueType::Immediate8,
    ]),
    Some(max_array![ // CheckPortExport
        ValueType::Immediate8,
    ]),
    Some(max_array![ // ParentWritePort
        ValueType::Immediate8,
        ValueType::RegisterRead,
    ]),
    Some(max_array![ // ChildWritePort
        ValueType::Immediate8,
        ValueType::RegisterRead,
    ]),
    Some(max_array![ // ParentReadPort
        ValueType::Immediate8,
        ValueType::Immediate8,
    ]),
    Some(max_array![ // ChildReadPort
        ValueType::Immediate8,
        ValueType::Immediate8,
    ]),
    Some(max_array![ // RegLoad
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // Reg (register)
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::RegisterRead,
    ]),
    Some(max_array![ // Reg (immediate)
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // RegUni
        ValueType::Immediate8,
        ValueType::Immediate8,
    ]),
    Some(max_array![ // RegTblLoad
        ValueType::Immediate8,
        ValueType::Immediate8,
        ValueType::Immediate24,
        ValueType::RegisterRead,
    ]),
    None,
    None,
    None,
    Some(max_array![ // Tempo
        ValueType::Immediate16,
    ]),
    Some(max_array![ // BankPrg
        ValueType::Immediate16,
    ]),
    Some(max_array![ // Bank
        ValueType::Immediate8,
    ]),
    Some(max_array![ // Prg
        ValueType::Immediate8,
    ]),
    None,
    None,
    None,
    Some(max_array![ // EnvScaleSet
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // EnvSet
        ValueType::Immediate8,
        ValueType::Immediate24,
    ]),
    Some(max_array![ // SimpleADSR
        ValueType::Immediate16,
        ValueType::Immediate16,
        ValueType::Immediate16,
        ValueType::Immediate16,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // BusConnect
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // IIRCutOff
        ValueType::Immediate8,
    ]),
    Some(max_array![ // IIRSet
        ValueType::Immediate16,
        ValueType::Immediate16,
        ValueType::Immediate16,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // FIRSet
        ValueType::Immediate16,
    ]),
    None,
    None,
    Some(max_array![]), // Wait
    Some(max_array![ // WaitByte
        ValueType::Immediate8,
    ]),
    None,
    Some(max_array![ // SetIntTable
        ValueType::Immediate24,
    ]),
    Some(max_array![ // SetInterrupt
        ValueType::Immediate16,
    ]),
    Some(max_array![ // DisInterrupt
        ValueType::Immediate16,
    ]),
    Some(max_array![]), // RetI
    Some(max_array![]), // ClrI
    Some(max_array![ // IntTimer
        ValueType::Immediate8,
        ValueType::Immediate16,
    ]),
    Some(max_array![ // SyncCPU
        ValueType::Immediate16,
    ]),
    None,
    None,
    None,
    Some(max_array![]), // Printf
    Some(max_array![]), // Nop
    Some(max_array![]), // Finish
]);


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Event {
    DirectNoteOn {
        // 0x00-0x7F
        pitch: u8,
        voice: u8,
        velocity: u8,
        duration: Option<u128>,
    },
    // 0x80: NULL
    DirectNoteOff {
        // 0x81-0x87
        voice: u8,
    },
    // 0x88-0x9F: unknown
    // 0xA0-0xAF: NULL
    // 0xB0: extended opcodes (see bottom of enum)
    IndirectNoteOn {
        // 0xB1
        pitch: Value,
        voice: Value,
        velocity: Value,
    },
    IndirectNoteOff {
        // 0xB2
        voice: Value,
    },
    Note {
        // 0xB3
        flags: Value,
        pitch: Value,
        velocity: Value,
        time: Value,
    },
    SetLastNote {
        // 0xB4
        pitch: Value,
    },
    // 0xB5-0xB7: NULL
    Control8 {
        // 0xB8
        parameter: Value,
        value: Value,
    },
    Control16 {
        // 0xB9
        // Control8 and Control16 are differentiated
        // because some parameters interpret a 16-bit value differently
        // (e.g. 8-bit pitches are always on the chromatic scale,
        // while 16-bit pitches are top byte = chromatic, bottom byte = pitch bend)
        parameter: Value,
        value: Value,
    },
    Control8Gradual {
        // 0xBA
        parameter: Value,
        target_value: Value,
        gradual_duration: Value,
    },
    Control16Gradual {
        // 0xBB
        // see also: comment at Control16
        parameter: Value,
        target_value: Value,
        gradual_duration: Value,
    },
    // 0xBC-0xC0: NULL
    OpenTrack {
        // 0xC1
        channel_number: Value,
        track_pointer: Value, // u24
    },
    CloseTrack {
        // 0xC2
        channel_number: Value,
    },
    Call {
        // 0xC3
        target: Value,
    },
    ConditionalCall {
        // 0xC4
        condition: Value,
        target: Value,
    },
    Return, // 0xC5
    ConditionalReturn {
        // 0xC6
        condition: Value,
    },
    Jump {
        // 0xC7
        target: Value,
    },
    ConditionalJump {
        // 0xC8
        condition: Value,
        target: Value,
    },
    JumpTable {
        // 0xC9
        target: Value,
        offset: Value,
    },
    CallTable {
        // 0xCA
        target: Value,
        offset: Value,
    },
    LoopStart {
        // 0xCB
        loop_count: Value,
    },
    LoopEnd, // 0xCC
    // 0xCD-0xCF: NULL
    ReadPort {
        // 0xD0
        source_port: Value,
        destination_register: Value,
    },
    WritePort {
        // 0xD1
        source_register: Value,
        destination_port: Value,
    },
    CheckPortImport {
        // 0xD2
        port: Value,
    },
    CheckPortExport {
        // 0xD3
        port: Value,
    },
    ParentWritePort {
        // 0xD4
        port: Value,
        value: Value,
    },
    ChildWritePort {
        // 0xD5
        child_and_port: Value,
        value: Value,
    },
    ParentReadPort {
        // 0xD6
        port: Value,
        destination_register: Value,
    },
    ChildReadPort {
        // 0xD7
        child_and_port: Value,
        destination_register: Value,
    },
    RegisterLoad {
        // 0xD8
        register: Value,
        value: Value,
    },
    RegisterBinaryOperation {
        // 0xD9, 0xDA
        operation: Value,
        register: Value,
        operand: Value,
    },
    RegisterUnaryOperation {
        // 0xDB
        operation: Value,
        register: Value,
    },
    RegisterTableLoad {
        // 0xDC
        access_mode: Value,
        destination: Value,
        offset: Value,
        index: Value,
    },
    // 0xDD-0xDF: NULL
    Tempo {
        // 0xE0
        bpm: Value,
    },
    SwitchBankAndProgram {
        // 0xE1
        bank_and_program: Value,
    },
    SwitchBank {
        // 0xE2
        bank: Value,
    },
    SwitchProgram {
        // 0xE3
        program: Value,
    },
    // 0xE4-0xE6: NULL
    SetOscillatorScale {
        // 0xE7
        oscillator_number: Value,
        scale: Value,
    },
    SetOscillatorTable {
        // 0xE8
        oscillator_number: Value,
        table_pointer: Value,
    }, 
    SimpleAsdr {
        // 0xE9
        attack: Value,
        sustain: Value,
        decay: Value,
        amplitude: Value,
        release: Value,
    },
    BusConnect {
        // 0xEA
        line: Value,
        destination: Value,
    },
    SetIirToCutoff {
        // 0xEB
        cutoff_value: Value,
    },
    SetIir {
        // 0xEC
        params: [Value; 4],
    },
    SetFirFromTable {
        // 0xED
        table_offset: Value, // pointing to [i16; 8]
    },
    // 0xEE-0xEF: NULL
    WaitTicksFromStream {
        // 0xF0
        ticks: u128, // variable; hopefully u128 is enough
    },
    WaitTicksFromValue {
        // 0xF1
        ticks: Value,
    },
    // 0xF2: NULL
    SetInterruptTable {
        // 0xF3
        table_pointer: Value,
    },
    EnableInterrupts {
        // 0xF4
        mask_to_enable: Value,
    },
    DisableInterrupts {
        // 0xF5
        mask_to_disable: Value,
    }, 
    ReturnFromInterrupt, // 0xF6
    ClearInterrupt, // 0xF7
    InterruptTimer {
        // 0xF8
        timer_count: Value,
        time: Value,
    },
    SyncCpu {
        // 0xF9
        callback_value: Value,
    },
    // 0xFA-0xFC: NULL
    Print {
        // 0xFD
        string: Vec<u8>, // NUL-terminated
    },
    Nop, // 0xFE
    Finish, // 0xFF
    Dump, // 0xB001
}
impl Default for Event {
    fn default() -> Self {
        Self::Nop
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RegisterOperation {
    #[default] NoOp,
    Add,
    Subtract,
    SubtractInto3,
    MultiplyInto33,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Rand,
    ShiftLeft,
    ShiftRight,
    Other(u8),
}
impl From<u8> for RegisterOperation {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::NoOp,
            1 => Self::Add,
            2 => Self::Subtract,
            3 => Self::SubtractInto3,
            4 => Self::MultiplyInto33,
            5 => Self::BitwiseAnd,
            6 => Self::BitwiseOr,
            7 => Self::BitwiseXor,
            8 => Self::Rand,
            9 => Self::ShiftLeft,
            10 => Self::ShiftRight,
            other => Self::Other(other),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ControlParameter {
    Volume,
    Pitch,
    Reverb,
    Pan,
    Other(u8),
}
impl From<u8> for ControlParameter {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Volume,
            1 => Self::Pitch,
            2 => Self::Reverb,
            3 => Self::Pan,
            other => Self::Other(other),
        }
    }
}

fn read_midi_variable_length_int<R: Read + Seek>(reader: &mut R) -> Result<u128, io::Error> {
    let mut byte = 0u8;
    let mut value = 0u128;
    loop {
        reader.read_exact(std::slice::from_mut(&mut byte))?;
        value <<= 7;
        value |= (byte & 0b0111_1111) as u128;
        if (byte & 0b1000_0000) == 0 {
            break;
        }
    }
    Ok(value)
}

pub fn read_event<R: Read + Seek>(reader: &mut R) -> Result<Event, io::Error> {
    let mut cmd = 0;
    reader.read_exact(std::slice::from_mut(&mut cmd))?;
    match cmd {
        0x00..=0x7F => {
            let pitch = cmd;
            let mut voice_velocity_buf = [0u8; 2];
            reader.read_exact(&mut voice_velocity_buf)?;
            let voice = voice_velocity_buf[0];
            let velocity = voice_velocity_buf[1];
            let duration = if (voice & 7) == 0 {
                // we also have a duration
                Some(read_midi_variable_length_int(reader)?)
            } else {
                None
            };
            Ok(Event::DirectNoteOn { pitch, voice, velocity, duration })
        },
        0x80..=0x8F => {
            let voice = cmd & 0xF;
            Ok(Event::DirectNoteOff { voice })
        },
        0x90..=0x9F => {
            // command with register indirection
            let param_count = (cmd & 0b111) + 1;
            // (yes, bit 1<<3 is dropped)
            let mut params = [0u8; 2];
            reader.read_exact(&mut params)?;
            let mut reg_bits_msb = params[0];
            let inner_cmd = params[1];

            let mut reg_bits_lsb: u8 = 0;
            for _ in 0..param_count {
                reg_bits_lsb = reg_bits_lsb.unbounded_shl(1);
                if reg_bits_msb & 0x80 != 0 {
                    reg_bits_lsb |= 1;
                }
                reg_bits_msb = reg_bits_msb.unbounded_shl(1);
            }
            read_command(reader, inner_cmd, reg_bits_lsb)
        },
        _ => {
            read_command(reader, cmd, 0)
        },
    }
}

fn read_command<R: Read + Seek>(reader: &mut R, cmd: u8, reg_bits_lsb: u8) -> Result<Event, io::Error> {
    // what parameters does this command expect?
    assert!(cmd >= 0xA0);

    if cmd == 0xB0 {
        // two-byte extended command
        let mut ext_cmd = 0;
        reader.read_exact(std::slice::from_mut(&mut ext_cmd))?;
        return match ext_cmd {
            0x01 => Ok(Event::Dump),
            _ => panic!("unknown extended command 0xB0{:02X}", ext_cmd),
        };
    }

    let cmd_params_index = usize::from(cmd - 0xA0);
    let mut cmd_params = match &COMMAND_PARAMS[cmd_params_index] {
        Some(cp) => cp.clone(),
        None => panic!("unknown command {:04X}", cmd),
    };

    // are any of them being overridden?
    if reg_bits_lsb != 0 {
        for (i, par) in cmd_params.as_slice_mut().iter_mut().enumerate() {
            if reg_bits_lsb & (1 << i) != 0 {
                *par = ValueType::RegisterRead;
            }
        }
    }

    // read the parameters as specified
    let mut param_values = Vec::with_capacity(cmd_params.len());
    for par in cmd_params.as_slice() {
        match par {
            ValueType::Immediate8 => {
                let mut value = 0;
                reader.read_exact(std::slice::from_mut(&mut value))?;
                param_values.push(Value::Immediate8(value));
            },
            ValueType::Immediate16 => {
                let mut buf = [0u8; 2];
                reader.read_exact(&mut buf)?;
                let value = u16::from_be_bytes(buf);
                param_values.push(Value::Immediate16(value));
            },
            ValueType::Immediate24 => {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf[1..4])?;
                let value = u32::from_be_bytes(buf);
                param_values.push(Value::Immediate24(value));
            },
            ValueType::RegisterRead => {
                let mut reg = 0;
                reader.read_exact(std::slice::from_mut(&mut reg))?;
                param_values.push(Value::RegisterRead(reg));
            },
        }
    }

    match cmd {
        // beware: most of these simply take their defined parameters,
        // but some read additional data from the program stream
        0xB1 => {
            assert_eq!(param_values.len(), 3);
            Ok(Event::IndirectNoteOn {
                pitch: param_values[0],
                voice: param_values[1],
                velocity: param_values[2],
            })
        },
        0xB2 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::IndirectNoteOff {
                voice: param_values[0],
            })
        },
        0xB3 => {
            assert_eq!(param_values.len(), 4);
            Ok(Event::Note {
                flags: param_values[0],
                pitch: param_values[1],
                velocity: param_values[2],
                time: param_values[3],
            })
        },
        0xB4 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SetLastNote {
                pitch: param_values[0],
            })
        },
        0xB8 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::Control8 {
                parameter: param_values[0],
                value: param_values[1],
            })
        },
        0xB9 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::Control16 {
                parameter: param_values[0],
                value: param_values[1],
            })
        },
        0xBA => {
            assert_eq!(param_values.len(), 3);
            Ok(Event::Control8Gradual {
                parameter: param_values[0],
                target_value: param_values[1],
                gradual_duration: param_values[2],
            })
        },
        0xBB => {
            assert_eq!(param_values.len(), 3);
            Ok(Event::Control16Gradual {
                parameter: param_values[0],
                target_value: param_values[1],
                gradual_duration: param_values[2],
            })
        },
        0xC1 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::OpenTrack {
                channel_number: param_values[0],
                track_pointer: param_values[1],
            })
        },
        0xC2 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::CloseTrack {
                channel_number: param_values[0],
            })
        },
        0xC3|0xC7 => {
            assert_eq!(param_values.len(), 1);
            let target = param_values[0];
            Ok(if cmd == 0xC3 {
                Event::Call { target }
            } else {
                Event::Jump { target }
            })
        },
        0xC4|0xC8 => {
            assert_eq!(param_values.len(), 2);
            let condition = param_values[0];
            let target = param_values[1];
            Ok(if cmd == 0xC4 {
                Event::ConditionalCall { condition, target }
            } else {
                Event::ConditionalJump { condition, target }
            })
        },
        0xC5|0xF6|0xF7 => {
            assert_eq!(param_values.len(), 0);
            Ok(match cmd {
                0xC5 => Event::Return,
                0xF6 => Event::ReturnFromInterrupt,
                0xF7 => Event::ClearInterrupt,
                _ => unreachable!(),
            })
        },
        0xC6 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::ConditionalReturn {
                condition: param_values[0],
            })
        },
        0xC9|0xCA => {
            assert_eq!(param_values.len(), 2);
            let target = param_values[0];
            let offset = param_values[1];
            Ok(if cmd == 0xC9 {
                Event::JumpTable { target, offset }
            } else {
                Event::CallTable { target, offset }
            })
        },
        0xCB => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::LoopStart { loop_count: param_values[0] })
        },
        0xCC => {
            assert_eq!(param_values.len(), 0);
            Ok(Event::LoopEnd)
        },
        0xD0 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::ReadPort {
                source_port: param_values[0],
                destination_register: param_values[1],
            })
        },
        0xD1 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::WritePort {
                source_register: param_values[0],
                destination_port: param_values[1],
            })
        },
        0xD2|0xD3 => {
            assert_eq!(param_values.len(), 1);
            let port = param_values[0];
            Ok(if cmd == 0xD2 {
                Event::CheckPortImport { port }
            } else {
                Event::CheckPortExport { port }
            })
        },
        0xD4|0xD5 => {
            assert_eq!(param_values.len(), 2);
            let port = param_values[0];
            let value = param_values[1];
            Ok(if cmd == 0xD4 {
                Event::ParentWritePort { port, value }
            } else {
                Event::ChildWritePort { child_and_port: port, value }
            })
        },
        0xD6|0xD7 => {
            assert_eq!(param_values.len(), 2);
            let port = param_values[0];
            let destination_register = param_values[1];
            Ok(if cmd == 0xD4 {
                Event::ParentReadPort { port, destination_register }
            } else {
                Event::ChildReadPort { child_and_port: port, destination_register }
            })
        },
        0xD8 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::RegisterLoad {
                register: param_values[0],
                value: param_values[1],
            })
        },
        0xD9|0xDA => {
            // different parameter types, otherwise the same operation
            assert_eq!(param_values.len(), 3);
            Ok(Event::RegisterBinaryOperation {
                operation: param_values[0],
                register: param_values[1],
                operand: param_values[2],
            })
        },
        0xDB => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::RegisterUnaryOperation {
                operation: param_values[0],
                register: param_values[1],
            })
        },
        0xE0 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::Tempo {
                bpm: param_values[0],
            })
        },
        0xE1 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SwitchBankAndProgram {
                bank_and_program: param_values[0],
            })
        },
        0xE2 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SwitchBank {
                bank: param_values[0],
            })
        },
        0xE3 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SwitchProgram {
                program: param_values[0],
            })
        },
        0xE7 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::SetOscillatorScale {
                oscillator_number: param_values[0],
                scale: param_values[1],
            })
        },
        0xE8 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::SetOscillatorTable {
                oscillator_number: param_values[0],
                table_pointer: param_values[1],
            })
        },
        0xE9 => {
            assert_eq!(param_values.len(), 5);
            Ok(Event::SimpleAsdr {
                attack: param_values[0],
                sustain: param_values[1],
                decay: param_values[2],
                amplitude: param_values[3],
                release: param_values[4],
            })
        },
        0xEA => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::BusConnect {
                line: param_values[0],
                destination: param_values[1],
            })
        },
        0xEB => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SetIirToCutoff {
                cutoff_value: param_values[0],
            })
        },
        0xEC => {
            assert_eq!(param_values.len(), 4);
            Ok(Event::SetIir {
                params: [
                    param_values[0],
                    param_values[1],
                    param_values[2],
                    param_values[3],
                ],
            })
        },
        0xED => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SetFirFromTable {
                table_offset: param_values[0],
            })
        },
        0xF0 => {
            // !!! this operation is a stream reader:
            // wait duration is taken directly from stream
            // (as a MIDI variable-length integer)
            assert_eq!(param_values.len(), 0);
            let ticks = read_midi_variable_length_int(reader)?;
            Ok(Event::WaitTicksFromStream { ticks })
        },
        0xF1 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::WaitTicksFromValue { ticks: param_values[0] })
        },
        0xF3 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SetInterruptTable { table_pointer: param_values[0] })
        },
        0xF4|0xF5 => {
            assert_eq!(param_values.len(), 1);
            let mask = param_values[0];
            Ok(if cmd == 0xF4 {
                Event::EnableInterrupts { mask_to_enable: mask }
            } else {
                Event::DisableInterrupts { mask_to_disable: mask }
            })
        },
        0xF8 => {
            assert_eq!(param_values.len(), 2);
            Ok(Event::InterruptTimer {
                timer_count: param_values[0],
                time: param_values[1],
            })
        },
        0xF9 => {
            assert_eq!(param_values.len(), 1);
            Ok(Event::SyncCpu {
                callback_value: param_values[0],
            })
        },
        0xFD => {
            // !!! this operation is a stream reader:
            // the format string is taken from the stream
            assert_eq!(param_values.len(), 0);
            let mut string = Vec::new();
            loop {
                let mut b = 0;
                reader.read_exact(std::slice::from_mut(&mut b))?;
                if b == 0x00 {
                    break;
                }
                string.push(b);
            }
            Ok(Event::Print { string })
        },
        0xFE => {
            assert_eq!(param_values.len(), 0);
            Ok(Event::Nop)
        },
        0xFF => {
            assert_eq!(param_values.len(), 0);
            Ok(Event::Finish)
        },
        other => panic!("unhandled command {:#04X}", other),
    }
}
