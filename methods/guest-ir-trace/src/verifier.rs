use ir_trace_common::primitives::eval_primitive;
use ir_trace_common::trace_types::{Trace, TraceStep};
use ir_trace_common::value::{reconstruct, FlatValue};

pub fn verify_trace(trace: &Trace) {
    let values = &trace.value_table;

    for (i, step) in trace.steps.iter().enumerate() {
        match step {
            TraceStep::PrimResult { op, args, result } => {
                // Reconstruct to Value for primitive evaluation
                let arg_values: Vec<_> =
                    args.iter().map(|id| reconstruct(values, *id)).collect();
                let computed = eval_primitive(op, &arg_values);
                let expected = reconstruct(values, *result);
                assert_eq!(
                    computed, expected,
                    "Primitive verification failed at step {}: {:?}",
                    i, op
                );
            }
            TraceStep::Branch {
                scrutinee,
                chosen_tag,
            } => {
                let val = &values[*scrutinee as usize];
                assert_eq!(
                    val.tag(),
                    *chosen_tag,
                    "Branch verification failed at step {}: expected tag {}, got {}",
                    i,
                    chosen_tag,
                    val.tag()
                );
            }
            TraceStep::CtorCreate {
                tag,
                fields,
                scalar_data,
                result,
            } => {
                let obj = &values[*result as usize];
                assert_eq!(
                    obj.tag(),
                    *tag,
                    "Ctor tag mismatch at step {}",
                    i
                );
                match obj {
                    FlatValue::Object {
                        fields: obj_fields,
                        scalars,
                        ..
                    } => {
                        assert_eq!(
                            obj_fields.len(),
                            fields.len(),
                            "Ctor field count mismatch at step {}",
                            i
                        );
                        for (j, field_id) in fields.iter().enumerate() {
                            assert_eq!(
                                obj_fields[j], *field_id,
                                "Ctor field {} mismatch at step {}",
                                j, i
                            );
                        }
                        assert_eq!(
                            scalars.len(),
                            scalar_data.len(),
                            "Ctor scalar size mismatch at step {}",
                            i
                        );
                    }
                    _ => panic!("Ctor result is not an Object at step {}", i),
                }
            }
            TraceStep::ProjResult { obj, idx, result } => {
                let obj_val = &values[*obj as usize];
                assert_eq!(
                    obj_val.field_id(*idx as usize),
                    Some(*result),
                    "Proj verification failed at step {}",
                    i
                );
            }
            TraceStep::SetResult {
                obj,
                idx,
                val,
                result,
            } => {
                let obj_val = &values[*obj as usize];
                let expected = obj_val.with_field_set(*idx as usize, *val);
                assert_eq!(
                    expected,
                    values[*result as usize],
                    "Set verification failed at step {}",
                    i
                );
            }
            TraceStep::Call { .. } => {
                // User function calls: verified via their sub-steps
                // Extern calls (crypto stubs): trusted as axioms
            }
        }
    }
}
