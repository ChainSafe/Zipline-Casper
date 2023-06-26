#![allow(dead_code)]
#![allow(clippy::all)] // TODO: Remove this after we restart work on unicorn

use crate::oracle_provider::OracleProvider;
use crate::ram::Ram;

use std::sync::Arc;

use byteorder::{ByteOrder, BE};
use eth_trie::MemoryDB;
use log::debug;
use preimage_oracle::H256;
use unicorn_engine::unicorn_const::{Arch, HookType, Mode, Permission};
use unicorn_engine::RegisterMIPS;
use unicorn_engine::Unicorn;

pub struct ExecutionData<O, R> {
    steps: u64,
    heap_start: u64,
    /// if this should insert a fault into the resulting trace by
    /// messing with the output value
    output_fault: bool,
    ram: R,
    oracle: O,

    /// data needed to capture trace sections
    trace_type: TraceConfig,
    trie_db: Arc<MemoryDB>,
    pub snapshots: Vec<(u64, H256)>,
}

pub enum TraceConfig {
    // Runs the execution fully to get the final snapshot
    NewChallenge,
    // Runs the execution from start step until end step with n_sections amount of snapshots between
    DissectExecution {
        start: u64,
        end: u64,
        n_sections: usize,
        fuckup_step: Option<u64>,
    },
    // Runs the execution until step and then returns the snapshot
    OneStepProof {
        step: u64,
    },
    // No tracing, just run the execution
    Turbo,
}

impl<O, R> ExecutionData<O, R>
where
    R: Ram,
    O: OracleProvider,
{
    fn new(ram: R, oracle: O, trie_db: Arc<MemoryDB>, trace_type: TraceConfig) -> Self {
        Self {
            steps: 0,
            heap_start: 0,
            output_fault: false,
            oracle,
            ram,

            trace_type,
            trie_db,
            snapshots: Vec::new(),
        }
    }
}

pub fn new_cannon_unicorn<'a, R, O>(
    ram: R,
    oracle: O,
    trie_db: Option<Arc<MemoryDB>>,
    trace_type: TraceConfig,
) -> Unicorn<'a, ExecutionData<O, R>>
where
    R: Ram + 'a,
    O: OracleProvider,
{
    let trie_db = trie_db.unwrap_or_else(|| Arc::new(MemoryDB::new(false)));
    let mut mu = Unicorn::new_with_data(
        Arch::MIPS,
        Mode::MIPS32 | Mode::BIG_ENDIAN,
        ExecutionData::new(ram, oracle, trie_db, trace_type),
    )
    .unwrap();

    mu.add_intr_hook(|mu, intno| {
        match intno {
            17 => {
                // EXCP_SYSCALL,
                let syscall_no = mu.reg_read(RegisterMIPS::V0).unwrap();
                let mut v0 = 0u64;

                match syscall_no {
                    4020 => {
                        let mut oracle_hash = [0u8; 0x20];
                        mu.mem_read(0x30001000, &mut oracle_hash).unwrap();
                        let mut value = mu.get_data().oracle.get(&oracle_hash).to_vec();

                        let mut length = [0u8; 4];
                        // pray conversion no panic xD
                        BE::write_u32(&mut length, value.len() as u32);

                        mu.mem_write(0x31000000, &length).unwrap();
                        mu.mem_write(0x31000004, &value).unwrap();

                        mu.get_data().ram.write(0x31000000, value.len() as u32);
                        value.extend_from_slice(&[0, 0, 0]);

                        // In Go, they start loop to read 4 bytes at a time, but in Rust we can just use chunks
                        // Furthermore, we know the exact thing stored at 0x31000000, so no need to retrieve from Ram again
                        let mut i = 0;
                        let mut value_chunk_iter = value.chunks_exact(4);
                        while let Some(chunk) = value_chunk_iter.next() {
                            mu.get_data()
                                .ram
                                .write(0x31000004 + (i as u32 * 4), BE::read_u32(chunk));
                            i += 1;
                        }
                        let rem = value_chunk_iter.remainder();
                        if !rem.is_empty() {
                            let mut chunk = [0; 4];
                            chunk[..rem.len()].copy_from_slice(rem);
                            mu.get_data()
                                .ram
                                .write(0x31000004 + (i as u32 * 4), BE::read_u32(&chunk));
                        }
                    }
                    4004 => {
                        let _fd = mu.reg_read(RegisterMIPS::A0).unwrap();
                        let buf = mu.reg_read(RegisterMIPS::A1).unwrap();
                        let count = mu.reg_read(RegisterMIPS::A2).unwrap();
                        // conversion panic alert! (shouldnt happen if youre on a 64 bit machine though)
                        let mut bytes = vec![0u8; count as usize];
                        mu.mem_read(buf, &mut bytes).unwrap();

                        debug!(
                            "[{}] Unicorn: {}",
                            chrono::Utc::now().time(),
                            String::from_utf8_lossy(&bytes)
                        );
                    }
                    4090 => {
                        let a0 = mu.reg_read(RegisterMIPS::A0).unwrap();
                        let sz = mu.reg_read(RegisterMIPS::A1).unwrap();
                        if a0 == 0 {
                            let h_start = mu.get_data().heap_start;
                            v0 = 0x20000000 + h_start;
                            mu.get_data_mut().heap_start = h_start + sz;
                        } else {
                            v0 = a0;
                        }
                    }
                    4045 => {
                        v0 = 0x40000000;
                    }
                    4120 => {
                        v0 = 1;
                    }
                    4246 => {
                        // exit group
                        mu.reg_write(RegisterMIPS::PC, 0x5ead0000).unwrap();
                    }
                    _ => {
                        debug!("unrecognised syscall number: {}", syscall_no);
                    }
                }
                mu.reg_write(RegisterMIPS::V0, v0).unwrap();
                mu.reg_write(RegisterMIPS::A3, 0).unwrap();
            }
            18 => {
                // EXCP_BREAK,
                let pc = mu.pc_read().unwrap();
                debug!("Break interrupt detected");
                panic!("break at step: {}, pc: {}", mu.get_data().steps, pc);
            }
            _ => {
                let pc = mu.pc_read().unwrap();
                debug!("Interrupt: {}, PC: {}", intno, pc);
                debug!(
                    "Reading Cause Register: {:x?}",
                    mu.reg_read(RegisterMIPS::R13).unwrap()
                );
                debug!(
                    "Reading EPC Register: {:x?}",
                    mu.reg_read(RegisterMIPS::R14).unwrap()
                );
                panic!(
                    "invalid interrupt {} at step {}",
                    intno,
                    mu.get_data().steps
                )
            }
        }
    })
    .unwrap();

    mu.add_mem_hook(
        HookType::MEM_WRITE,
        0,
        0x80000000,
        |mu, _access, addr64, size, value| {
            let mut rt = value;
            let rs = addr64 & 3;
            // lmao is this really not going to panic????????
            let addr = (addr64 & 0xfffffffc) as u32;

            // if we want to write a fault into the output trace do it now
            // this is useful for challenge game testing where we want one party to
            // make an error
            if mu.get_data().output_fault && addr == 0x30000804 {
                debug!("injecting output fault over {:x}", rt);
                rt = 0xbabababa;
            }

            if size == 1 {
                let mem = mu.get_data().ram.read_or_default(addr);
                let val = ((rt & 0xff) << (24 - (rs & 3) * 8)) as u32;
                let mask = 0xFFFFFFFF ^ ((0xFF << (24 - (rs & 3) * 8)) as u32);
                mu.get_data().ram.write(addr, (mem & mask) | val);
            } else if size == 2 {
                let mem = mu.get_data().ram.read_or_default(addr);
                let val = ((rt & 0xffff) << (16 - (rs & 2) * 8)) as u32;
                let mask = 0xFFFFFFFF ^ ((0xFFFF << (16 - (rs & 2) * 8)) as u32);
                mu.get_data().ram.write(addr, (mem & mask) | val);
            } else if size == 4 {
                mu.get_data().ram.write(addr, rt as u32);
            } else {
                panic!("bad size write to ram");
            }

            // TODO: So this callback is expected to return a boolean, but i have no idea why...
            true
        },
    )
    .unwrap();

    // add the correct type of hook depending on the mode
    match mu.get_data().trace_type {
        TraceConfig::Turbo => {
            // don't even add a code hook!
        }
        TraceConfig::NewChallenge => {
            mu.add_code_hook(0, 0x80000000, move |muu, _addr, _size| {
                muu.get_data_mut().steps += 1;
                if muu.get_data().steps % 1_000_000_000 == 0 {
                    debug!("Step: {}", muu.get_data().steps);
                }
            })
            .unwrap();
        }
        TraceConfig::DissectExecution {
            start,
            end,
            n_sections,
            fuckup_step,
        } => {
            let section_size = (end - start) / n_sections as u64;

            mu.add_code_hook(0, 0x80000000, move |muu, _addr, _size| {
                let steps = muu.get_data().steps;
                // special case we are now in the last stages of dissection so return a snapshot for every step
                if (end - start) < n_sections as u64 {
                    let mut snapshot = get_snapshot(muu);

                    if let Some(fuckup_step) = fuckup_step {
                        if steps >= fuckup_step {
                            snapshot[0] = 0xff;
                        }
                    }

                    muu.get_data_mut().snapshots.push((steps, snapshot));

                    debug!(
                        "special: Creating snapshot {} at step: {}: {}",
                        muu.get_data_mut().snapshots.len(),
                        steps,
                        hex::encode(snapshot)
                    );

                    if steps == end {
                        muu.emu_stop().unwrap();
                    }
                } else {
                    let current_section = ((steps - start) / section_size) as usize;

                    debug!("{} :: {}", steps, hex::encode(get_snapshot(muu)));

                    // debug!("current section: {}", current_section);
                    // debug!("n_sections: {}", n_sections);
                    // debug!("steps: {}", steps);
                    // debug!("end: {}", end);
                    // debug!("start: {}", start);
                    // debug!("section_size: {}", section_size);
                    if steps >= start {
                        let a = (steps - start) % section_size == 0;
                        let b = current_section < n_sections;
                        if a && b {
                            let mut snapshot = get_snapshot(muu);

                            if let Some(fuckup_step) = fuckup_step {
                                if steps >= fuckup_step {
                                    snapshot[0] = 0xff;
                                }
                            }

                            muu.get_data_mut().snapshots.push((steps, snapshot));
                            debug!(
                                "Creating snapshot {} at step: {}: {}",
                                current_section,
                                steps,
                                hex::encode(snapshot)
                            );
                        }
                    }
                    if steps == (end - 1) {
                        let mut snapshot = get_snapshot(muu);

                        if let Some(fuckup_step) = fuckup_step {
                            if steps >= fuckup_step {
                                snapshot[0] = 0xff;
                            }
                        }

                        muu.get_data_mut().snapshots.push((steps, snapshot));
                        debug!(
                            "Creating FINAL snapshot at step: {}: {}",
                            steps,
                            hex::encode(snapshot)
                        );
                        muu.emu_stop().unwrap();
                    }
                }

                if muu.get_data().steps % 1_000_000_000 == 0 {
                    debug!("Step: {}", muu.get_data().steps);
                }
                muu.get_data_mut().steps += 1;
            })
            .unwrap();
        }
        TraceConfig::OneStepProof { step } => {
            mu.add_code_hook(0, 0x80000000, move |muu, _addr, _size| {
                let steps = muu.get_data().steps;
                // debug!("{} :: {}", steps, hex::encode(get_snapshot(muu)));
                if steps == step {
                    let snapshot = get_snapshot(muu);
                    muu.get_data_mut().snapshots.push((steps, snapshot));
                    debug!(
                        "Creating snapshot at step: {}: {}",
                        steps,
                        hex::encode(snapshot)
                    );
                    muu.emu_stop().unwrap();
                }
                muu.get_data_mut().steps += 1;
                if muu.get_data().steps % 1_000_000_000 == 0 {
                    debug!("Step: {}", muu.get_data().steps);
                }
            })
            .unwrap();
        }
    }
    // TODO: Check these permissions are correct
    mu.mem_map(0, 0x80000000, Permission::ALL).unwrap();

    mu
}

pub fn write_program<R: Ram, O>(mu: &mut Unicorn<ExecutionData<O, R>>, program: &[u8]) -> H256 {
    mu.mem_write(0, program).unwrap();
    debug!("program size: {}", program.len());
    mu.get_data().ram.load_data(program, 0);
    get_snapshot(mu)
}

pub fn write_input<R: Ram, O>(mu: &mut Unicorn<ExecutionData<O, R>>, input: &[u8; 32]) -> H256 {
    let mut input_extended = [0; 0xc0];
    input_extended[0..32].copy_from_slice(input);
    mu.mem_write(0x30000000, &input_extended).unwrap();
    mu.get_data().ram.load_data(input, 0x30000000);
    get_snapshot(mu)
}

/// Run the program to the given number of steps
/// If steps == 0 then run until completion
pub fn run<R: Ram, O>(
    mu: &mut Unicorn<ExecutionData<O, R>>,
    steps: u64,
) -> (H256, u64, [u8; 0x44]) {
    // actually start the program emulation with inputs and outputs!
    debug!("starting emulation");
    mu.emu_start(0, 0x5ead0004, 0, steps as usize).unwrap();

    // read the output
    let mut emulation_output = [0u8; 0x44];
    mu.mem_read(0x30000800, &mut emulation_output).unwrap();

    // get the final snapshot and step count
    let snapshot = get_snapshot(mu);
    let steps = mu.get_data().steps;

    (snapshot, steps, emulation_output)
}

pub fn get_snapshot<O, R: Ram>(mu: &mut Unicorn<ExecutionData<O, R>>) -> H256 {
    sync_regs(mu);
    mu.get_data()
        .ram
        .ram_to_trie(&mu.get_data().trie_db)
        .unwrap()
}

pub fn get_trie_db<O, R: Ram>(mu: &mut Unicorn<ExecutionData<O, R>>) -> Arc<MemoryDB> {
    sync_regs(mu);
    let _ = mu
        .get_data()
        .ram
        .ram_to_trie(&mu.get_data().trie_db)
        .unwrap();
    mu.get_data().trie_db.clone()
}

const REG_OFFSET: u32 = 0xc0000000;
// const REG_PC: u32 = REG_OFFSET + 0x20*4;
const REG_HEAP: u32 = REG_OFFSET + 0x23 * 4;

pub fn sync_regs<O, R: Ram>(mu: &mut Unicorn<ExecutionData<O, R>>) {
    let pc = mu.reg_read(RegisterMIPS::PC).unwrap();
    debug!("pc: {}", pc);
    let ram = &mu.get_data().ram;
    ram.write(0xc0000080, pc as u32);

    let mut addr = 0xc0000000;
    for i in RegisterMIPS::ZERO as u32..RegisterMIPS::ZERO as u32 + 32 {
        let reg = mu.reg_read(i32_to_register_mips(i as i32)).unwrap();
        ram.write(addr, reg as u32);
        addr += 4;
    }

    let reg_hi = mu.reg_read(RegisterMIPS::HI).unwrap();
    let reg_lo = mu.reg_read(RegisterMIPS::LO).unwrap();
    ram.write(REG_OFFSET + 0x21 * 4, reg_hi as u32);
    ram.write(REG_OFFSET + 0x22 * 4, reg_lo as u32);
    ram.write(REG_HEAP, mu.get_data().heap_start as u32)
}

fn i32_to_register_mips(value: i32) -> RegisterMIPS {
    match value {
        0 => RegisterMIPS::INVALID,

        // General purpose registers
        1 => RegisterMIPS::PC,
        2 => RegisterMIPS::R0,
        3 => RegisterMIPS::R1,
        4 => RegisterMIPS::R2,
        5 => RegisterMIPS::R3,
        6 => RegisterMIPS::R4,
        7 => RegisterMIPS::R5,
        8 => RegisterMIPS::R6,
        9 => RegisterMIPS::R7,
        10 => RegisterMIPS::R8,
        11 => RegisterMIPS::R9,
        12 => RegisterMIPS::R10,
        13 => RegisterMIPS::R11,
        14 => RegisterMIPS::R12,
        15 => RegisterMIPS::R13,
        16 => RegisterMIPS::R14,
        17 => RegisterMIPS::R15,
        18 => RegisterMIPS::R16,
        19 => RegisterMIPS::R17,
        20 => RegisterMIPS::R18,
        21 => RegisterMIPS::R19,
        22 => RegisterMIPS::R20,
        23 => RegisterMIPS::R21,
        24 => RegisterMIPS::R22,
        25 => RegisterMIPS::R23,
        26 => RegisterMIPS::R24,
        27 => RegisterMIPS::R25,
        28 => RegisterMIPS::R26,
        29 => RegisterMIPS::R27,
        30 => RegisterMIPS::R28,
        31 => RegisterMIPS::R29,
        32 => RegisterMIPS::R30,
        33 => RegisterMIPS::R31,

        // DSP registers
        34 => RegisterMIPS::DSPCCOND,
        35 => RegisterMIPS::DSPCARRY,
        36 => RegisterMIPS::DSPEFI,
        37 => RegisterMIPS::DSPOUTFLAG,
        38 => RegisterMIPS::DSPOUTFLAG16_19,
        39 => RegisterMIPS::DSPOUTFLAG20,
        40 => RegisterMIPS::DSPOUTFLAG21,
        41 => RegisterMIPS::DSPOUTFLAG22,
        42 => RegisterMIPS::DSPOUTFLAG23,
        43 => RegisterMIPS::DSPPOS,
        44 => RegisterMIPS::DSPSCOUNT,

        // ACC registers
        45 => RegisterMIPS::AC0,
        46 => RegisterMIPS::AC1,
        47 => RegisterMIPS::AC2,
        48 => RegisterMIPS::AC3,

        // COP registers
        49 => RegisterMIPS::CC0,
        50 => RegisterMIPS::CC1,
        51 => RegisterMIPS::CC2,
        52 => RegisterMIPS::CC3,
        53 => RegisterMIPS::CC4,
        54 => RegisterMIPS::CC5,
        55 => RegisterMIPS::CC6,
        56 => RegisterMIPS::CC7,

        // FPU registers
        57 => RegisterMIPS::F0,
        58 => RegisterMIPS::F1,
        59 => RegisterMIPS::F2,
        60 => RegisterMIPS::F3,
        61 => RegisterMIPS::F4,
        62 => RegisterMIPS::F5,
        63 => RegisterMIPS::F6,
        64 => RegisterMIPS::F7,
        65 => RegisterMIPS::F8,
        66 => RegisterMIPS::F9,
        67 => RegisterMIPS::F10,
        68 => RegisterMIPS::F11,
        69 => RegisterMIPS::F12,
        70 => RegisterMIPS::F13,
        71 => RegisterMIPS::F14,
        72 => RegisterMIPS::F15,
        73 => RegisterMIPS::F16,
        74 => RegisterMIPS::F17,
        75 => RegisterMIPS::F18,
        76 => RegisterMIPS::F19,
        77 => RegisterMIPS::F20,
        78 => RegisterMIPS::F21,
        79 => RegisterMIPS::F22,
        80 => RegisterMIPS::F23,
        81 => RegisterMIPS::F24,
        82 => RegisterMIPS::F25,
        83 => RegisterMIPS::F26,
        84 => RegisterMIPS::F27,
        85 => RegisterMIPS::F28,
        86 => RegisterMIPS::F29,
        87 => RegisterMIPS::F30,
        88 => RegisterMIPS::F31,
        89 => RegisterMIPS::FCC0,
        90 => RegisterMIPS::FCC1,
        91 => RegisterMIPS::FCC2,
        92 => RegisterMIPS::FCC3,
        93 => RegisterMIPS::FCC4,
        94 => RegisterMIPS::FCC5,
        95 => RegisterMIPS::FCC6,
        96 => RegisterMIPS::FCC7,

        // AFPR128
        97 => RegisterMIPS::W0,
        98 => RegisterMIPS::W1,
        99 => RegisterMIPS::W2,
        100 => RegisterMIPS::W3,
        101 => RegisterMIPS::W4,
        102 => RegisterMIPS::W5,
        103 => RegisterMIPS::W6,
        104 => RegisterMIPS::W7,
        105 => RegisterMIPS::W8,
        106 => RegisterMIPS::W9,
        107 => RegisterMIPS::W10,
        108 => RegisterMIPS::W11,
        109 => RegisterMIPS::W12,
        110 => RegisterMIPS::W13,
        111 => RegisterMIPS::W14,
        112 => RegisterMIPS::W15,
        113 => RegisterMIPS::W16,
        114 => RegisterMIPS::W17,
        115 => RegisterMIPS::W18,
        116 => RegisterMIPS::W19,
        117 => RegisterMIPS::W20,
        118 => RegisterMIPS::W21,
        119 => RegisterMIPS::W22,
        120 => RegisterMIPS::W23,
        121 => RegisterMIPS::W24,
        122 => RegisterMIPS::W25,
        123 => RegisterMIPS::W26,
        124 => RegisterMIPS::W27,
        125 => RegisterMIPS::W28,
        126 => RegisterMIPS::W29,
        127 => RegisterMIPS::W30,
        128 => RegisterMIPS::W31,
        129 => RegisterMIPS::HI,
        130 => RegisterMIPS::LO,
        131 => RegisterMIPS::P0,
        132 => RegisterMIPS::P1,
        133 => RegisterMIPS::P2,
        134 => RegisterMIPS::MPL0,
        135 => RegisterMIPS::MPL1,
        136 => RegisterMIPS::MPL2,
        137 => RegisterMIPS::CP0_CONFIG3,
        138 => RegisterMIPS::CP0_USERLOCAL,
        139 => RegisterMIPS::CP0_STATUS,
        140 => RegisterMIPS::ENDING,
        _ => panic!("Invalid register number"),
    }
}
