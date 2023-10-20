#![feature(stdsimd)] // for `mips::break_`. If desired, this could be replaced with asm.
#![no_std]
#![no_main]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]

use preimage_oracle::PreimageOracle;
#[cfg(feature = "mainnet")]
use zipline_spec::MainnetSpec as Spec;
#[cfg(feature = "minimal")]
use zipline_spec::MinimalSpec as Spec;
#[cfg(feature = "spec_test")]
use zipline_spec::SpecTestSpec as Spec;
use zipline_finality_client::input::ZiplineInput;
use zipline_finality_client::ssz_state_reader::{PatchedSszStateReader, SszStateReader};

extern crate alloc;
extern crate rlibc; // memcpy, and friends

mod heap;
mod iommu;

use alloc::string::ToString;
use log::{LevelFilter, Metadata, Record};
struct IommuLogger;
static LOGGER: IommuLogger = IommuLogger;
impl log::Log for IommuLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            iommu::print(&format_args!("{} - {}", record.level(), record.args()).to_string());
        }
    }

    fn flush(&self) {}
}

/// Main entrypoint for a verifiable computation
#[no_mangle]
pub extern "C" fn _start() {    
    unsafe { heap::init() }; // Please make sure not to delete this
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace));
    log::debug!("Zipline state transition start");

    let oracle = iommu::preimage_oracle();
    // load our input struct from the preimage oracle by its hash
    let input_bytes = oracle.get_cached(iommu::input_hash()).unwrap();
    let input = ZiplineInput::from_ssz_bytes(input_bytes);
    let state_reader = SszStateReader::<_, Spec>::new(oracle, input.state_root).unwrap();

    let result = zipline_finality_client::verify::<Spec, PatchedSszStateReader<_, Spec>, 2048, 10000, 256>(
        state_reader,
        input,
    );
    
    if result.is_ok() {
        iommu::output([0x00; 32]);
    } else {
        iommu::output([0xff; 32]);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    iommu::print(&msg);
    unsafe {
        core::arch::mips::break_();
    }
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: alloc::alloc::Layout) -> ! {
    // NOTE: avoid `panic!` here, technically, it might not be allowed to panic in an OOM situation.
    //       with panic=abort it should work, but it's no biggie use `break` here anyway.
    iommu::print("alloc error! probably OOM");
    unsafe {
        core::arch::mips::break_();
    }
}
