#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Placeholder — parser fuzzing implemented in Phase 11 (US9).
    let _ = data;
});
