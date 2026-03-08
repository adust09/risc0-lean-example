use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::ir_types::*;

pub fn load_ir_program(json_str: &str) -> Result<HashMap<String, Decl>, String> {
    // Use serde_json with increased recursion limit via the `unbounded_depth` feature
    // or parse manually. The FnBody cont chains create deep nesting.
    let json: JsonValue = {
        let mut de = serde_json::Deserializer::from_str(json_str);
        de.disable_recursion_limit();
        // Safety: we control the input format and handle stack overflow via Box
        let val = JsonValue::deserialize(&mut de)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        val
    };

    let arr = json.as_array().ok_or("Expected JSON array at top level")?;

    let mut map = HashMap::new();
    for item in arr {
        let decl = parse_decl(item)?;
        map.insert(decl.name().to_string(), decl);
    }
    Ok(map)
}

fn parse_decl(v: &JsonValue) -> Result<Decl, String> {
    let kind = get_str(v, "kind")?;
    match kind {
        "fdecl" => {
            let name = get_str(v, "name")?.to_string();
            let params = parse_params(v.get("params").ok_or("missing params")?)?;
            let ret_type = parse_ir_type(v.get("ret_type").ok_or("missing ret_type")?)?;
            let body = parse_fn_body(v.get("body").ok_or("missing body")?)?;
            Ok(Decl::FnDecl {
                name,
                params,
                ret_type,
                body,
            })
        }
        "extern" => {
            let name = get_str(v, "name")?.to_string();
            let params = parse_params(v.get("params").ok_or("missing params")?)?;
            let ret_type = parse_ir_type(v.get("ret_type").ok_or("missing ret_type")?)?;
            Ok(Decl::ExternDecl {
                name,
                params,
                ret_type,
            })
        }
        _ => Err(format!("Unknown decl kind: {}", kind)),
    }
}

fn parse_params(v: &JsonValue) -> Result<Vec<Param>, String> {
    let arr = v.as_array().ok_or("Expected array for params")?;
    arr.iter().map(parse_param).collect()
}

fn parse_param(v: &JsonValue) -> Result<Param, String> {
    Ok(Param {
        var: get_u32(v, "var")?,
        ty: parse_ir_type(v.get("ty").ok_or("missing ty")?)?,
        borrowed: v
            .get("borrowed")
            .and_then(|b| b.as_bool())
            .unwrap_or(false),
    })
}

fn parse_ir_type(v: &JsonValue) -> Result<IRType, String> {
    let s = v.as_str().ok_or_else(|| format!("Expected string for IRType, got {:?}", v))?;
    match s {
        "Float" => Ok(IRType::Float),
        "UInt8" => Ok(IRType::UInt8),
        "UInt16" => Ok(IRType::UInt16),
        "UInt32" => Ok(IRType::UInt32),
        "UInt64" => Ok(IRType::UInt64),
        "USize" => Ok(IRType::USize),
        "Object" => Ok(IRType::Object),
        "TObject" => Ok(IRType::TObject),
        "Irrelevant" => Ok(IRType::Irrelevant),
        _ => Err(format!("Unknown IRType: {}", s)),
    }
}

fn parse_fn_body(v: &JsonValue) -> Result<FnBody, String> {
    let kind = get_str(v, "kind")?;
    match kind {
        "vdecl" => {
            let var = get_u32(v, "var")?;
            let ty = parse_ir_type(v.get("ty").ok_or("missing ty")?)?;
            let expr = parse_expr(v.get("expr").ok_or("missing expr")?)?;
            let cont = parse_fn_body(v.get("cont").ok_or("missing cont")?)?;
            Ok(FnBody::VDecl {
                var,
                ty,
                expr,
                cont: Box::new(cont),
            })
        }
        "jdecl" => {
            let jp = get_u32(v, "jp")?;
            let params = parse_params(v.get("params").ok_or("missing params")?)?;
            let body = parse_fn_body(v.get("body").ok_or("missing body")?)?;
            let cont = parse_fn_body(v.get("cont").ok_or("missing cont")?)?;
            Ok(FnBody::JDecl {
                jp,
                params,
                body: Box::new(body),
                cont: Box::new(cont),
            })
        }
        "set" => {
            let var = get_u32(v, "var")?;
            let idx = get_u32(v, "idx")?;
            let val = parse_arg(v.get("val").ok_or("missing val")?)?;
            let cont = parse_fn_body(v.get("cont").ok_or("missing cont")?)?;
            Ok(FnBody::Set {
                var,
                idx,
                val,
                cont: Box::new(cont),
            })
        }
        "uset" => {
            let var = get_u32(v, "var")?;
            let idx = get_u32(v, "idx")?;
            let val = get_u32(v, "val")?;
            let cont = parse_fn_body(v.get("cont").ok_or("missing cont")?)?;
            Ok(FnBody::USet {
                var,
                idx,
                val,
                cont: Box::new(cont),
            })
        }
        "sset" => {
            let var = get_u32(v, "var")?;
            let n = get_u32(v, "n")?;
            let offset = get_u32(v, "offset")?;
            let val = get_u32(v, "val")?;
            let ty = parse_ir_type(v.get("ty").ok_or("missing ty")?)?;
            let cont = parse_fn_body(v.get("cont").ok_or("missing cont")?)?;
            Ok(FnBody::SSet {
                var,
                n,
                offset,
                val,
                ty,
                cont: Box::new(cont),
            })
        }
        "setTag" => {
            let var = get_u32(v, "var")?;
            let cidx = get_u32(v, "cidx")? as u16;
            let cont = parse_fn_body(v.get("cont").ok_or("missing cont")?)?;
            Ok(FnBody::SetTag {
                var,
                cidx,
                cont: Box::new(cont),
            })
        }
        "case" => {
            let tid = get_str(v, "tid")?.to_string();
            let scrutinee = get_u32(v, "scrutinee")?;
            let alts_json = v
                .get("alts")
                .and_then(|a| a.as_array())
                .ok_or("missing alts")?;
            let alts: Result<Vec<Alt>, String> = alts_json.iter().map(parse_alt).collect();
            Ok(FnBody::Case {
                tid,
                scrutinee,
                alts: alts?,
            })
        }
        "ret" => {
            let arg = parse_arg(v.get("arg").ok_or("missing arg")?)?;
            Ok(FnBody::Ret { arg })
        }
        "jmp" => {
            let jp = get_u32(v, "jp")?;
            let args = parse_args(v.get("args").ok_or("missing args")?)?;
            Ok(FnBody::Jmp { jp, args })
        }
        "unreachable" => Ok(FnBody::Unreachable),
        _ => Err(format!("Unknown FnBody kind: {}", kind)),
    }
}

fn parse_expr(v: &JsonValue) -> Result<Expr, String> {
    let kind = get_str(v, "kind")?;
    match kind {
        "ctor" => {
            let info = parse_ctor_info(v.get("info").ok_or("missing info")?)?;
            let args = parse_args(v.get("args").ok_or("missing args")?)?;
            Ok(Expr::Ctor { info, args })
        }
        "proj" => {
            let idx = get_u32(v, "idx")?;
            let var = get_u32(v, "var")?;
            Ok(Expr::Proj { idx, var })
        }
        "uproj" => {
            let idx = get_u32(v, "idx")?;
            let var = get_u32(v, "var")?;
            Ok(Expr::UProj { idx, var })
        }
        "sproj" => {
            let n = get_u32(v, "n")?;
            let offset = get_u32(v, "offset")?;
            let var = get_u32(v, "var")?;
            Ok(Expr::SProj { n, offset, var })
        }
        "fap" => {
            let fun = get_str(v, "fun")?.to_string();
            let args = parse_args(v.get("args").ok_or("missing args")?)?;
            Ok(Expr::FAp { fun, args })
        }
        "pap" => {
            let fun = get_str(v, "fun")?.to_string();
            let args = parse_args(v.get("args").ok_or("missing args")?)?;
            Ok(Expr::PAp { fun, args })
        }
        "ap" => {
            let fun = get_u32(v, "fun")?;
            let args = parse_args(v.get("args").ok_or("missing args")?)?;
            Ok(Expr::Ap { fun, args })
        }
        "box" => {
            let ty = parse_ir_type(v.get("ty").ok_or("missing ty")?)?;
            let var = get_u32(v, "var")?;
            Ok(Expr::Box { ty, var })
        }
        "unbox" => {
            let var = get_u32(v, "var")?;
            Ok(Expr::Unbox { var })
        }
        "lit" => {
            let val_obj = v.get("val").ok_or("missing val")?;
            let val = parse_lit_val(val_obj)?;
            Ok(Expr::Lit { val })
        }
        "isShared" => {
            let var = get_u32(v, "var")?;
            Ok(Expr::IsShared { var })
        }
        "reset" => {
            let n = get_u32(v, "n")?;
            let var = get_u32(v, "var")?;
            Ok(Expr::Reset { n, var })
        }
        "reuse" => {
            let var = get_u32(v, "var")?;
            let info = parse_ctor_info(v.get("info").ok_or("missing info")?)?;
            let upd_header = v
                .get("upd_header")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let args = parse_args(v.get("args").ok_or("missing args")?)?;
            Ok(Expr::Reuse {
                var,
                info,
                upd_header,
                args,
            })
        }
        _ => Err(format!("Unknown Expr kind: {}", kind)),
    }
}

fn parse_alt(v: &JsonValue) -> Result<Alt, String> {
    let kind = get_str(v, "kind")?;
    match kind {
        "ctor" => {
            let info = parse_ctor_info(v.get("info").ok_or("missing info")?)?;
            let body = parse_fn_body(v.get("body").ok_or("missing body")?)?;
            Ok(Alt::Ctor { info, body })
        }
        "default" => {
            let body = parse_fn_body(v.get("body").ok_or("missing body")?)?;
            Ok(Alt::Default { body })
        }
        _ => Err(format!("Unknown Alt kind: {}", kind)),
    }
}

fn parse_ctor_info(v: &JsonValue) -> Result<CtorInfo, String> {
    Ok(CtorInfo {
        name: get_str(v, "name")?.to_string(),
        cidx: get_u32(v, "cidx")? as u16,
        size: get_u32(v, "size")?,
        usize_fields: get_u32(v, "usize_fields")?,
        scalar_size: get_u32(v, "scalar_size")?,
    })
}

fn parse_arg(v: &JsonValue) -> Result<Arg, String> {
    let kind = get_str(v, "kind")?;
    match kind {
        "var" => {
            let var = get_u32(v, "var")?;
            Ok(Arg::Var(var))
        }
        "irrelevant" => Ok(Arg::Irrelevant),
        _ => Err(format!("Unknown Arg kind: {}", kind)),
    }
}

fn parse_args(v: &JsonValue) -> Result<Vec<Arg>, String> {
    let arr = v.as_array().ok_or("Expected array for args")?;
    arr.iter().map(parse_arg).collect()
}

fn parse_lit_val(v: &JsonValue) -> Result<LitValue, String> {
    let kind = get_str(v, "kind")?;
    match kind {
        "num" => {
            let val = v
                .get("val")
                .and_then(|n| n.as_u64())
                .ok_or("missing num val")?;
            Ok(LitValue::Num(val))
        }
        "str" => {
            let val = get_str(v, "val")?.to_string();
            Ok(LitValue::Str(val))
        }
        _ => Err(format!("Unknown LitVal kind: {}", kind)),
    }
}

// Helper functions

fn get_str<'a>(v: &'a JsonValue, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("Missing or non-string field: {}", key))
}

fn get_u32(v: &JsonValue, key: &str) -> Result<u32, String> {
    v.get(key)
        .and_then(|n| n.as_u64())
        .map(|n| n as u32)
        .ok_or_else(|| format!("Missing or non-number field: {}", key))
}
