use ir_trace_common::primitives::eval_primitive;
use ir_trace_common::trace_types::PrimOp;

use super::value::Value;

pub fn lookup_primitive(name: &str) -> Option<PrimOp> {
    match name {
        // Nat
        "Nat.add" => Some(PrimOp::NatAdd),
        "Nat.sub" => Some(PrimOp::NatSub),
        "Nat.mul" => Some(PrimOp::NatMul),
        "Nat.div" => Some(PrimOp::NatDiv),
        "Nat.mod" => Some(PrimOp::NatMod),
        "Nat.beq" | "Nat.decEq" => Some(PrimOp::NatBeq),
        "Nat.blt" => Some(PrimOp::NatBlt),
        "Nat.ble" => Some(PrimOp::NatBle),
        "Nat.land" => Some(PrimOp::NatLand),
        "Nat.lor" => Some(PrimOp::NatLor),
        "Nat.xor" => Some(PrimOp::NatXor),
        "Nat.shiftLeft" => Some(PrimOp::NatShiftLeft),
        "Nat.shiftRight" => Some(PrimOp::NatShiftRight),

        // UInt8
        "UInt8.add" => Some(PrimOp::UInt8Add),
        "UInt8.sub" => Some(PrimOp::UInt8Sub),
        "UInt8.mul" => Some(PrimOp::UInt8Mul),
        "UInt8.div" => Some(PrimOp::UInt8Div),
        "UInt8.mod" => Some(PrimOp::UInt8Mod),
        "UInt8.land" => Some(PrimOp::UInt8Land),
        "UInt8.lor" => Some(PrimOp::UInt8Lor),
        "UInt8.xor" => Some(PrimOp::UInt8Xor),
        "UInt8.shiftLeft" => Some(PrimOp::UInt8ShiftLeft),
        "UInt8.shiftRight" => Some(PrimOp::UInt8ShiftRight),
        "UInt8.decEq" => Some(PrimOp::UInt8DecEq),
        "UInt8.beq" => Some(PrimOp::UInt8Beq),
        "UInt8.blt" => Some(PrimOp::UInt8Blt),
        "UInt8.ble" => Some(PrimOp::UInt8Ble),

        // UInt16
        "UInt16.add" => Some(PrimOp::UInt16Add),
        "UInt16.sub" => Some(PrimOp::UInt16Sub),
        "UInt16.mul" => Some(PrimOp::UInt16Mul),
        "UInt16.div" => Some(PrimOp::UInt16Div),
        "UInt16.mod" => Some(PrimOp::UInt16Mod),
        "UInt16.land" => Some(PrimOp::UInt16Land),
        "UInt16.lor" => Some(PrimOp::UInt16Lor),
        "UInt16.xor" => Some(PrimOp::UInt16Xor),
        "UInt16.shiftLeft" => Some(PrimOp::UInt16ShiftLeft),
        "UInt16.shiftRight" => Some(PrimOp::UInt16ShiftRight),
        "UInt16.decEq" => Some(PrimOp::UInt16DecEq),
        "UInt16.beq" => Some(PrimOp::UInt16Beq),
        "UInt16.blt" => Some(PrimOp::UInt16Blt),
        "UInt16.ble" => Some(PrimOp::UInt16Ble),

        // UInt32
        "UInt32.add" => Some(PrimOp::UInt32Add),
        "UInt32.sub" => Some(PrimOp::UInt32Sub),
        "UInt32.mul" => Some(PrimOp::UInt32Mul),
        "UInt32.div" => Some(PrimOp::UInt32Div),
        "UInt32.mod" => Some(PrimOp::UInt32Mod),
        "UInt32.land" => Some(PrimOp::UInt32Land),
        "UInt32.lor" => Some(PrimOp::UInt32Lor),
        "UInt32.xor" => Some(PrimOp::UInt32Xor),
        "UInt32.shiftLeft" => Some(PrimOp::UInt32ShiftLeft),
        "UInt32.shiftRight" => Some(PrimOp::UInt32ShiftRight),
        "UInt32.decEq" => Some(PrimOp::UInt32DecEq),
        "UInt32.beq" => Some(PrimOp::UInt32Beq),
        "UInt32.blt" => Some(PrimOp::UInt32Blt),
        "UInt32.ble" => Some(PrimOp::UInt32Ble),

        // UInt64
        "UInt64.add" => Some(PrimOp::UInt64Add),
        "UInt64.sub" => Some(PrimOp::UInt64Sub),
        "UInt64.mul" => Some(PrimOp::UInt64Mul),
        "UInt64.div" => Some(PrimOp::UInt64Div),
        "UInt64.mod" => Some(PrimOp::UInt64Mod),
        "UInt64.land" => Some(PrimOp::UInt64Land),
        "UInt64.lor" => Some(PrimOp::UInt64Lor),
        "UInt64.xor" => Some(PrimOp::UInt64Xor),
        "UInt64.shiftLeft" => Some(PrimOp::UInt64ShiftLeft),
        "UInt64.shiftRight" => Some(PrimOp::UInt64ShiftRight),
        "UInt64.decEq" => Some(PrimOp::UInt64DecEq),
        "UInt64.beq" => Some(PrimOp::UInt64Beq),
        "UInt64.blt" => Some(PrimOp::UInt64Blt),
        "UInt64.ble" => Some(PrimOp::UInt64Ble),

        // Array
        "Array.mkEmpty" => Some(PrimOp::ArrayMkEmpty),
        "Array.size" => Some(PrimOp::ArraySize),
        "Array.get!" | "Array.getD" => Some(PrimOp::ArrayGet),
        "Array.set!" | "Array.setD" => Some(PrimOp::ArraySet),
        "Array.push" => Some(PrimOp::ArrayPush),

        // ByteArray
        "ByteArray.mkEmpty" => Some(PrimOp::ByteArrayMkEmpty),
        "ByteArray.size" => Some(PrimOp::ByteArraySize),
        "ByteArray.get!" | "ByteArray.getD" => Some(PrimOp::ByteArrayGet),
        "ByteArray.set!" | "ByteArray.setD" => Some(PrimOp::ByteArraySet),
        "ByteArray.push" => Some(PrimOp::ByteArrayPush),
        "ByteArray.append" => Some(PrimOp::ByteArrayAppend),
        "ByteArray.copySlice" => Some(PrimOp::ByteArrayCopySlice),
        "ByteArray.emptyWithCapacity" => Some(PrimOp::ByteArrayEmptyWithCapacity),

        // String
        "String.length" => Some(PrimOp::StringLength),
        "String.append" => Some(PrimOp::StringAppend),
        "String.decEq" => Some(PrimOp::StringDecEq),
        "String.mk" => Some(PrimOp::StringMk),

        // Conversion
        "UInt32.toNat" => Some(PrimOp::UInt32ToNat),
        "UInt64.toNat" => Some(PrimOp::UInt64ToNat),
        "UInt8.toNat" => Some(PrimOp::UInt8ToNat),
        "UInt16.toNat" => Some(PrimOp::UInt16ToNat),
        "UInt32.ofNat" | "UInt32.ofNat'" => Some(PrimOp::NatToUInt32),
        "UInt64.ofNat" | "UInt64.ofNat'" => Some(PrimOp::NatToUInt64),
        "UInt8.ofNat" | "UInt8.ofNat'" => Some(PrimOp::NatToUInt8),
        "UInt16.ofNat" | "UInt16.ofNat'" => Some(PrimOp::NatToUInt16),
        "String.toNat" => Some(PrimOp::StringToNat),

        // Bool
        "Bool.not" => Some(PrimOp::BoolNot),

        // Other
        "panic" | "panicWithPosWithDecl" => Some(PrimOp::Panic),
        "dbg_trace" | "dbgTrace" => Some(PrimOp::DbgTrace),

        _ => None,
    }
}

pub fn call_primitive(op: &PrimOp, args: Vec<Value>) -> Value {
    eval_primitive(op, &args)
}
