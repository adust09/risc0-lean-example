use sha2::{Digest, Sha256};

use ir_trace_common::trace_types::{Trace, TraceHeader, TRACE_MAGIC};
use ir_trace_common::value::Value;

use crate::interpreter::Interpreter;

pub fn build_trace(
    interpreter: &Interpreter,
    ir_program_bytes: &[u8],
    input_bytes: &[u8],
    output_value: &Value,
    output_value_id: u32,
) -> Trace {
    let ir_program_hash = sha256(ir_program_bytes);
    let input_hash = sha256(input_bytes);
    let output_bytes = output_value.serialize_to_bytes();
    let output_hash = sha256(&output_bytes);

    let header = TraceHeader {
        magic: TRACE_MAGIC,
        ir_program_hash,
        input_hash,
        output_hash,
        value_count: interpreter.value_table.len() as u32,
        step_count: interpreter.trace_steps.len() as u64,
    };

    Trace {
        header,
        value_table: interpreter.value_table.clone(),
        steps: interpreter.trace_steps.clone(),
        fn_name_table: interpreter.fn_name_table.clone(),
        output_value_id,
    }
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
