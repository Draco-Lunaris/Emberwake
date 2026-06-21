#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // T072: Fuzz the JSON import parser with arbitrary bytes.
    // The parser must never panic/OOM on hostile input — all errors are returned as Err.
    let _ = app::server::importer::json::parse_json(data);
});
