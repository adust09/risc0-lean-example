use crate::trace_types::PrimOp;
use crate::value::Value;

/// Skip leading Irrelevant args (erased type parameters in Lean4 λRC IR).
fn skip_erased_prefix(args: &[Value]) -> &[Value] {
    if matches!(args.first(), Some(Value::Irrelevant)) { &args[1..] } else { args }
}

pub fn eval_primitive(op: &PrimOp, args: &[Value]) -> Value {
    match op {
        // Nat arithmetic (simplified: use u64 for small nats)
        PrimOp::NatAdd => nat_binop(args, |a, b| a.wrapping_add(b)),
        PrimOp::NatSub => nat_binop(args, |a, b| a.saturating_sub(b)),
        PrimOp::NatMul => nat_binop(args, |a, b| a.wrapping_mul(b)),
        PrimOp::NatDiv => nat_binop(args, |a, b| if b == 0 { 0 } else { a / b }),
        PrimOp::NatMod => nat_binop(args, |a, b| if b == 0 { 0 } else { a % b }),
        PrimOp::NatBeq => nat_cmp(args, |a, b| a == b),
        PrimOp::NatBlt => nat_cmp(args, |a, b| a < b),
        PrimOp::NatBle => nat_cmp(args, |a, b| a <= b),
        PrimOp::NatDecEq => nat_cmp(args, |a, b| a == b),
        PrimOp::NatLand => nat_binop(args, |a, b| a & b),
        PrimOp::NatLor => nat_binop(args, |a, b| a | b),
        PrimOp::NatXor => nat_binop(args, |a, b| a ^ b),
        PrimOp::NatShiftLeft => nat_binop(args, |a, b| a.wrapping_shl(b as u32)),
        PrimOp::NatShiftRight => nat_binop(args, |a, b| a.wrapping_shr(b as u32)),

        // UInt8
        PrimOp::UInt8Add => uint_binop::<u8>(args, |a, b| a.wrapping_add(b)),
        PrimOp::UInt8Sub => uint_binop::<u8>(args, |a, b| a.wrapping_sub(b)),
        PrimOp::UInt8Mul => uint_binop::<u8>(args, |a, b| a.wrapping_mul(b)),
        PrimOp::UInt8Div => uint_binop::<u8>(args, |a, b| if b == 0 { 0 } else { a / b }),
        PrimOp::UInt8Mod => uint_binop::<u8>(args, |a, b| if b == 0 { 0 } else { a % b }),
        PrimOp::UInt8Land => uint_binop::<u8>(args, |a, b| a & b),
        PrimOp::UInt8Lor => uint_binop::<u8>(args, |a, b| a | b),
        PrimOp::UInt8Xor => uint_binop::<u8>(args, |a, b| a ^ b),
        PrimOp::UInt8ShiftLeft => uint_binop::<u8>(args, |a, b| a.wrapping_shl(b as u32)),
        PrimOp::UInt8ShiftRight => uint_binop::<u8>(args, |a, b| a.wrapping_shr(b as u32)),
        PrimOp::UInt8DecEq | PrimOp::UInt8Beq => uint_cmp::<u8>(args, |a, b| a == b),
        PrimOp::UInt8Blt => uint_cmp::<u8>(args, |a, b| a < b),
        PrimOp::UInt8Ble => uint_cmp::<u8>(args, |a, b| a <= b),

        // UInt16
        PrimOp::UInt16Add => uint_binop::<u16>(args, |a, b| a.wrapping_add(b)),
        PrimOp::UInt16Sub => uint_binop::<u16>(args, |a, b| a.wrapping_sub(b)),
        PrimOp::UInt16Mul => uint_binop::<u16>(args, |a, b| a.wrapping_mul(b)),
        PrimOp::UInt16Div => uint_binop::<u16>(args, |a, b| if b == 0 { 0 } else { a / b }),
        PrimOp::UInt16Mod => uint_binop::<u16>(args, |a, b| if b == 0 { 0 } else { a % b }),
        PrimOp::UInt16Land => uint_binop::<u16>(args, |a, b| a & b),
        PrimOp::UInt16Lor => uint_binop::<u16>(args, |a, b| a | b),
        PrimOp::UInt16Xor => uint_binop::<u16>(args, |a, b| a ^ b),
        PrimOp::UInt16ShiftLeft => uint_binop::<u16>(args, |a, b| a.wrapping_shl(b as u32)),
        PrimOp::UInt16ShiftRight => uint_binop::<u16>(args, |a, b| a.wrapping_shr(b as u32)),
        PrimOp::UInt16DecEq | PrimOp::UInt16Beq => uint_cmp::<u16>(args, |a, b| a == b),
        PrimOp::UInt16Blt => uint_cmp::<u16>(args, |a, b| a < b),
        PrimOp::UInt16Ble => uint_cmp::<u16>(args, |a, b| a <= b),

        // UInt32
        PrimOp::UInt32Add => uint_binop::<u32>(args, |a, b| a.wrapping_add(b)),
        PrimOp::UInt32Sub => uint_binop::<u32>(args, |a, b| a.wrapping_sub(b)),
        PrimOp::UInt32Mul => uint_binop::<u32>(args, |a, b| a.wrapping_mul(b)),
        PrimOp::UInt32Div => uint_binop::<u32>(args, |a, b| if b == 0 { 0 } else { a / b }),
        PrimOp::UInt32Mod => uint_binop::<u32>(args, |a, b| if b == 0 { 0 } else { a % b }),
        PrimOp::UInt32Land => uint_binop::<u32>(args, |a, b| a & b),
        PrimOp::UInt32Lor => uint_binop::<u32>(args, |a, b| a | b),
        PrimOp::UInt32Xor => uint_binop::<u32>(args, |a, b| a ^ b),
        PrimOp::UInt32ShiftLeft => uint_binop::<u32>(args, |a, b| a.wrapping_shl(b)),
        PrimOp::UInt32ShiftRight => uint_binop::<u32>(args, |a, b| a.wrapping_shr(b)),
        PrimOp::UInt32DecEq | PrimOp::UInt32Beq => uint_cmp::<u32>(args, |a, b| a == b),
        PrimOp::UInt32Blt => uint_cmp::<u32>(args, |a, b| a < b),
        PrimOp::UInt32Ble => uint_cmp::<u32>(args, |a, b| a <= b),

        // UInt64
        PrimOp::UInt64Add => uint_binop::<u64>(args, |a, b| a.wrapping_add(b)),
        PrimOp::UInt64Sub => uint_binop::<u64>(args, |a, b| a.wrapping_sub(b)),
        PrimOp::UInt64Mul => uint_binop::<u64>(args, |a, b| a.wrapping_mul(b)),
        PrimOp::UInt64Div => uint_binop::<u64>(args, |a, b| if b == 0 { 0 } else { a / b }),
        PrimOp::UInt64Mod => uint_binop::<u64>(args, |a, b| if b == 0 { 0 } else { a % b }),
        PrimOp::UInt64Land => uint_binop::<u64>(args, |a, b| a & b),
        PrimOp::UInt64Lor => uint_binop::<u64>(args, |a, b| a | b),
        PrimOp::UInt64Xor => uint_binop::<u64>(args, |a, b| a ^ b),
        PrimOp::UInt64ShiftLeft => uint_binop::<u64>(args, |a, b| a.wrapping_shl(b as u32)),
        PrimOp::UInt64ShiftRight => uint_binop::<u64>(args, |a, b| a.wrapping_shr(b as u32)),
        PrimOp::UInt64DecEq | PrimOp::UInt64Beq => uint_cmp::<u64>(args, |a, b| a == b),
        PrimOp::UInt64Blt => uint_cmp::<u64>(args, |a, b| a < b),
        PrimOp::UInt64Ble => uint_cmp::<u64>(args, |a, b| a <= b),

        // Array operations
        PrimOp::ArrayMkEmpty => Value::Array(Vec::new()),
        PrimOp::ArraySize => {
            let a = skip_erased_prefix(args);
            assert!(a.len() >= 1, "ArraySize: expected >=1 args after skip, got {}", a.len());
            match &a[0] {
                Value::Array(v) => Value::Scalar(v.len() as u64),
                other => panic!("ArraySize: expected Array, got {:?}", other),
            }
        }
        PrimOp::ArrayGet => {
            let a = skip_erased_prefix(args);
            assert!(a.len() >= 2, "ArrayGet: expected >=2 args after skip, got {}", a.len());
            let idx = a[1].as_u64() as usize;
            match &a[0] {
                Value::Array(v) => v.get(idx).cloned().unwrap_or(Value::Irrelevant),
                other => panic!("ArrayGet: expected Array, got {:?}", other),
            }
        }
        PrimOp::ArraySet => {
            let a = skip_erased_prefix(args);
            assert!(a.len() >= 3, "ArraySet: expected >=3 args after skip, got {}", a.len());
            let idx = a[1].as_u64() as usize;
            let val = a[2].clone();
            match &a[0] {
                Value::Array(v) => {
                    let mut new_arr = v.clone();
                    if idx < new_arr.len() {
                        new_arr[idx] = val;
                    }
                    Value::Array(new_arr)
                }
                other => panic!("ArraySet: expected Array, got {:?}", other),
            }
        }
        PrimOp::ArrayPush => {
            let a = skip_erased_prefix(args);
            assert!(a.len() >= 2, "ArrayPush: expected >=2 args after skip, got {}", a.len());
            let val = a[1].clone();
            match &a[0] {
                Value::Array(v) => {
                    let mut new_arr = v.clone();
                    new_arr.push(val);
                    Value::Array(new_arr)
                }
                other => panic!("ArrayPush: expected Array, got {:?}", other),
            }
        }

        // ByteArray operations
        PrimOp::ByteArrayMkEmpty | PrimOp::ByteArrayEmptyWithCapacity => {
            Value::ByteArray(Vec::new())
        }
        PrimOp::ByteArraySize => {
            let ba = &args[0];
            match ba {
                Value::ByteArray(v) => Value::Scalar(v.len() as u64),
                _ => Value::Scalar(0),
            }
        }
        PrimOp::ByteArrayGet => {
            let ba = &args[0];
            let idx = args[1].as_u64() as usize;
            match ba {
                Value::ByteArray(v) => Value::Scalar(*v.get(idx).unwrap_or(&0) as u64),
                _ => Value::Scalar(0),
            }
        }
        PrimOp::ByteArraySet => {
            let ba = &args[0];
            let idx = args[1].as_u64() as usize;
            let val = args[2].as_u64() as u8;
            match ba {
                Value::ByteArray(v) => {
                    let mut new_ba = v.clone();
                    if idx < new_ba.len() {
                        new_ba[idx] = val;
                    }
                    Value::ByteArray(new_ba)
                }
                _ => Value::ByteArray(vec![]),
            }
        }
        PrimOp::ByteArrayPush => {
            let ba = &args[0];
            let val = args[1].as_u64() as u8;
            match ba {
                Value::ByteArray(v) => {
                    let mut new_ba = v.clone();
                    new_ba.push(val);
                    Value::ByteArray(new_ba)
                }
                _ => Value::ByteArray(vec![val]),
            }
        }
        PrimOp::ByteArrayAppend => {
            let ba1 = &args[0];
            let ba2 = &args[1];
            match (ba1, ba2) {
                (Value::ByteArray(v1), Value::ByteArray(v2)) => {
                    let mut result = v1.clone();
                    result.extend_from_slice(v2);
                    Value::ByteArray(result)
                }
                _ => Value::ByteArray(vec![]),
            }
        }
        PrimOp::ByteArrayCopySlice => {
            // copySlice(src, srcOff, dest, destOff, len)
            let src = &args[0];
            let src_off = args[1].as_u64() as usize;
            let dest = &args[2];
            let dest_off = args[3].as_u64() as usize;
            let len = args[4].as_u64() as usize;
            match (src, dest) {
                (Value::ByteArray(s), Value::ByteArray(d)) => {
                    let mut result = d.clone();
                    let needed = dest_off + len;
                    if result.len() < needed {
                        result.resize(needed, 0);
                    }
                    for i in 0..len {
                        if src_off + i < s.len() {
                            result[dest_off + i] = s[src_off + i];
                        }
                    }
                    Value::ByteArray(result)
                }
                _ => Value::ByteArray(vec![]),
            }
        }

        // String operations
        PrimOp::StringLength => match &args[0] {
            Value::Str(s) => Value::Scalar(s.len() as u64),
            _ => Value::Scalar(0),
        },
        PrimOp::StringAppend => match (&args[0], &args[1]) {
            (Value::Str(a), Value::Str(b)) => Value::Str(format!("{}{}", a, b)),
            _ => Value::Str(String::new()),
        },
        PrimOp::StringDecEq => match (&args[0], &args[1]) {
            (Value::Str(a), Value::Str(b)) => bool_to_value(a == b),
            _ => bool_to_value(false),
        },
        PrimOp::StringMk => {
            // String.mk from List Char
            Value::Str(String::new()) // simplified
        }

        // Conversions
        PrimOp::UInt32ToNat | PrimOp::UInt64ToNat | PrimOp::UInt8ToNat | PrimOp::UInt16ToNat => {
            Value::Scalar(args[0].as_u64())
        }
        PrimOp::NatToUInt32 => Value::Scalar(args[0].as_u64() & 0xFFFFFFFF),
        PrimOp::NatToUInt64 => Value::Scalar(args[0].as_u64()),
        PrimOp::NatToUInt8 => Value::Scalar(args[0].as_u64() & 0xFF),
        PrimOp::NatToUInt16 => Value::Scalar(args[0].as_u64() & 0xFFFF),
        PrimOp::StringToNat => {
            // Parse decimal string to nat
            match &args[0] {
                Value::Str(s) => {
                    let n: u64 = s.parse().unwrap_or(0);
                    Value::Scalar(n)
                }
                _ => Value::Scalar(0),
            }
        }

        // Bool
        PrimOp::BoolNot => {
            let b = args[0].as_bool();
            bool_to_value(!b)
        }

        // Other
        PrimOp::Panic => panic!("Lean panic! called in IR trace"),
        PrimOp::DbgTrace => {
            // dbg_trace returns its second argument (the continuation thunk result)
            args.last().cloned().unwrap_or(Value::Irrelevant)
        }
    }
}

fn nat_binop(args: &[Value], f: impl Fn(u64, u64) -> u64) -> Value {
    let a = args[0].as_u64();
    let b = args[1].as_u64();
    Value::Scalar(f(a, b))
}

fn nat_cmp(args: &[Value], f: impl Fn(u64, u64) -> bool) -> Value {
    let a = args[0].as_u64();
    let b = args[1].as_u64();
    bool_to_value(f(a, b))
}

trait UIntOps: Copy + From<u8> + Into<u64> + TryFrom<u64> {
    fn from_u64(v: u64) -> Self;
}

impl UIntOps for u8 {
    fn from_u64(v: u64) -> Self {
        v as u8
    }
}
impl UIntOps for u16 {
    fn from_u64(v: u64) -> Self {
        v as u16
    }
}
impl UIntOps for u32 {
    fn from_u64(v: u64) -> Self {
        v as u32
    }
}
impl UIntOps for u64 {
    fn from_u64(v: u64) -> Self {
        v
    }
}

fn uint_binop<T: UIntOps>(args: &[Value], f: impl Fn(T, T) -> T) -> Value {
    let a = T::from_u64(args[0].as_u64());
    let b = T::from_u64(args[1].as_u64());
    Value::Scalar(f(a, b).into())
}

fn uint_cmp<T: UIntOps + PartialOrd>(args: &[Value], f: impl Fn(T, T) -> bool) -> Value {
    let a = T::from_u64(args[0].as_u64());
    let b = T::from_u64(args[1].as_u64());
    bool_to_value(f(a, b))
}

fn bool_to_value(b: bool) -> Value {
    // Lean Bool: false = Object{tag:0}, true = Object{tag:1}
    Value::Object {
        tag: if b { 1 } else { 0 },
        fields: vec![],
        scalars: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arr(vals: Vec<u64>) -> Value {
        Value::Array(vals.into_iter().map(Value::Scalar).collect())
    }

    #[test]
    fn test_array_size_with_irr() {
        let result = eval_primitive(&PrimOp::ArraySize, &[Value::Irrelevant, arr(vec![1, 2, 3])]);
        assert_eq!(result.as_u64(), 3);
    }

    #[test]
    fn test_array_size_without_irr() {
        let result = eval_primitive(&PrimOp::ArraySize, &[arr(vec![1, 2, 3])]);
        assert_eq!(result.as_u64(), 3);
    }

    #[test]
    fn test_array_get_3args() {
        let result = eval_primitive(&PrimOp::ArrayGet, &[Value::Irrelevant, arr(vec![10, 20, 30]), Value::Scalar(1)]);
        assert_eq!(result.as_u64(), 20);
    }

    #[test]
    fn test_array_get_4args() {
        let result = eval_primitive(&PrimOp::ArrayGet, &[Value::Irrelevant, arr(vec![10, 20, 30]), Value::Scalar(1), Value::Irrelevant]);
        assert_eq!(result.as_u64(), 20);
    }

    #[test]
    fn test_array_get_out_of_bounds() {
        let result = eval_primitive(&PrimOp::ArrayGet, &[Value::Irrelevant, arr(vec![10, 20]), Value::Scalar(5)]);
        assert!(matches!(result, Value::Irrelevant));
    }

    #[test]
    fn test_array_set() {
        let result = eval_primitive(&PrimOp::ArraySet, &[Value::Irrelevant, arr(vec![10, 20, 30]), Value::Scalar(0), Value::Scalar(99)]);
        match result {
            Value::Array(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0].as_u64(), 99);
                assert_eq!(v[1].as_u64(), 20);
            }
            other => panic!("expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_array_push() {
        let result = eval_primitive(&PrimOp::ArrayPush, &[Value::Irrelevant, arr(vec![10, 20]), Value::Scalar(30)]);
        match result {
            Value::Array(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[2].as_u64(), 30);
            }
            other => panic!("expected Array, got {:?}", other),
        }
    }
}
