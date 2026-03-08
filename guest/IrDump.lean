/-
  IR Dump: Extract lambda-RC IR declarations from .olean environment and serialize to JSON.

  Usage: lake env lean --run guest/IrDump.lean
  Output: ir_program.json
-/
import Lean
import Lean.Compiler.IR
import Guest

open Lean
open Lean.IR

/-- Convert VarId to JSON -/
def varToJson (v : IR.VarId) : Json :=
  Json.num v.idx

/-- Convert JoinPointId to JSON -/
def jpToJson (j : IR.JoinPointId) : Json :=
  Json.num j.idx

/-- Convert IRType to JSON -/
def irTypeToJson : IR.IRType → Json
  | .float   => "Float"
  | .float32 => "Float"
  | .uint8   => "UInt8"
  | .uint16  => "UInt16"
  | .uint32  => "UInt32"
  | .uint64  => "UInt64"
  | .usize   => "USize"
  | .object  => "Object"
  | .tobject => "TObject"
  | .irrelevant => "Irrelevant"
  | .struct _ _ => "Object"
  | .union _ _  => "Object"

/-- Convert Arg to JSON -/
def argToJson : IR.Arg → Json
  | .var v => Json.mkObj [("kind", "var"), ("var", varToJson v)]
  | .irrelevant => Json.mkObj [("kind", "irrelevant")]

/-- Convert Param to JSON -/
def paramToJson (p : IR.Param) : Json :=
  Json.mkObj [
    ("var", varToJson p.x),
    ("ty", irTypeToJson p.ty),
    ("borrowed", Json.bool p.borrow)
  ]

/-- Convert CtorInfo to JSON -/
def ctorInfoToJson (i : IR.CtorInfo) : Json :=
  Json.mkObj [
    ("name", Json.str i.name.toString),
    ("cidx", Json.num i.cidx),
    ("size", Json.num i.size),
    ("usize_fields", Json.num i.usize),
    ("scalar_size", Json.num i.ssize)
  ]

/-- Convert LitVal to JSON -/
def litValToJson : IR.LitVal → Json
  | .num n => Json.mkObj [("kind", "num"), ("val", Json.num n)]
  | .str s => Json.mkObj [("kind", "str"), ("val", Json.str s)]

/-- Convert Expr to JSON -/
def exprToJson : IR.Expr → Json
  | .ctor i ys => Json.mkObj [
      ("kind", "ctor"),
      ("info", ctorInfoToJson i),
      ("args", Json.arr (ys.map argToJson))
    ]
  | .proj i x => Json.mkObj [
      ("kind", "proj"),
      ("idx", Json.num i),
      ("var", varToJson x)
    ]
  | .uproj i x => Json.mkObj [
      ("kind", "uproj"),
      ("idx", Json.num i),
      ("var", varToJson x)
    ]
  | .sproj n off x => Json.mkObj [
      ("kind", "sproj"),
      ("n", Json.num n),
      ("offset", Json.num off),
      ("var", varToJson x)
    ]
  | .fap c ys => Json.mkObj [
      ("kind", "fap"),
      ("fun", Json.str c.toString),
      ("args", Json.arr (ys.map argToJson))
    ]
  | .pap c ys => Json.mkObj [
      ("kind", "pap"),
      ("fun", Json.str c.toString),
      ("args", Json.arr (ys.map argToJson))
    ]
  | .ap x ys => Json.mkObj [
      ("kind", "ap"),
      ("fun", varToJson x),
      ("args", Json.arr (ys.map argToJson))
    ]
  | .box ty x => Json.mkObj [
      ("kind", "box"),
      ("ty", irTypeToJson ty),
      ("var", varToJson x)
    ]
  | .unbox x => Json.mkObj [
      ("kind", "unbox"),
      ("var", varToJson x)
    ]
  | .lit v => Json.mkObj [
      ("kind", "lit"),
      ("val", litValToJson v)
    ]
  | .isShared x => Json.mkObj [
      ("kind", "isShared"),
      ("var", varToJson x)
    ]
  | .reset n x => Json.mkObj [
      ("kind", "reset"),
      ("n", Json.num n),
      ("var", varToJson x)
    ]
  | .reuse x i upd ys => Json.mkObj [
      ("kind", "reuse"),
      ("var", varToJson x),
      ("info", ctorInfoToJson i),
      ("upd_header", Json.bool upd),
      ("args", Json.arr (ys.map argToJson))
    ]

mutual

partial def altToJson : IR.Alt → Json
  | .ctor i b => Json.mkObj [
      ("kind", "ctor"),
      ("info", ctorInfoToJson i),
      ("body", fnBodyToJson b)
    ]
  | .default b => Json.mkObj [
      ("kind", "default"),
      ("body", fnBodyToJson b)
    ]

partial def fnBodyToJson : IR.FnBody → Json
  | .vdecl x ty e b => Json.mkObj [
      ("kind", "vdecl"),
      ("var", varToJson x),
      ("ty", irTypeToJson ty),
      ("expr", exprToJson e),
      ("cont", fnBodyToJson b)
    ]
  | .jdecl j xs v b => Json.mkObj [
      ("kind", "jdecl"),
      ("jp", jpToJson j),
      ("params", Json.arr (xs.map paramToJson)),
      ("body", fnBodyToJson v),
      ("cont", fnBodyToJson b)
    ]
  | .set x i y b => Json.mkObj [
      ("kind", "set"),
      ("var", varToJson x),
      ("idx", Json.num i),
      ("val", argToJson y),
      ("cont", fnBodyToJson b)
    ]
  | .uset x i y b => Json.mkObj [
      ("kind", "uset"),
      ("var", varToJson x),
      ("idx", Json.num i),
      ("val", varToJson y),
      ("cont", fnBodyToJson b)
    ]
  | .sset x n off y ty b => Json.mkObj [
      ("kind", "sset"),
      ("var", varToJson x),
      ("n", Json.num n),
      ("offset", Json.num off),
      ("val", varToJson y),
      ("ty", irTypeToJson ty),
      ("cont", fnBodyToJson b)
    ]
  | .setTag x cidx b => Json.mkObj [
      ("kind", "setTag"),
      ("var", varToJson x),
      ("cidx", Json.num cidx),
      ("cont", fnBodyToJson b)
    ]
  | .inc _ _ _ _ b => fnBodyToJson b  -- skip RC++
  | .dec _ _ _ _ b => fnBodyToJson b  -- skip RC--
  | .del _ b => fnBodyToJson b        -- skip free
  | .mdata _ b => fnBodyToJson b      -- skip metadata
  | .case tid x _xty alts => Json.mkObj [
      ("kind", "case"),
      ("tid", Json.str tid.toString),
      ("scrutinee", varToJson x),
      ("alts", Json.arr (alts.map altToJson))
    ]
  | .ret x => Json.mkObj [
      ("kind", "ret"),
      ("arg", argToJson x)
    ]
  | .jmp j ys => Json.mkObj [
      ("kind", "jmp"),
      ("jp", jpToJson j),
      ("args", Json.arr (ys.map argToJson))
    ]
  | .unreachable => Json.mkObj [("kind", "unreachable")]

end

/-- Convert a full Decl to JSON -/
def declToJson : IR.Decl → Json
  | .fdecl (f := f) (xs := xs) (type := ty) (body := body) .. => Json.mkObj [
      ("kind", "fdecl"),
      ("name", Json.str f.toString),
      ("params", Json.arr (xs.map paramToJson)),
      ("ret_type", irTypeToJson ty),
      ("body", fnBodyToJson body)
    ]
  | .extern (f := f) (xs := xs) (type := ty) .. => Json.mkObj [
      ("kind", "extern"),
      ("name", Json.str f.toString),
      ("params", Json.arr (xs.map paramToJson)),
      ("ret_type", irTypeToJson ty)
    ]

/-- Main: import Guest module environment and dump all IR declarations to JSON -/
def main : IO Unit := do
  -- Import the Guest module to get full environment with all dependencies
  let env ← importModules
    #[{ module := `Guest : Import }]
    {}
    (trustLevel := 0)
  -- Iterate over all imported modules' IR declarations
  let mut decls : Array Json := #[]
  let numModules := env.header.moduleNames.size
  for modIdx in [:numModules] do
    let entries := IR.declMapExt.getModuleEntries env modIdx
    for decl in entries do
      decls := decls.push (declToJson decl)
  IO.println s!"Dumped {decls.size} IR declarations"
  let json := Json.arr decls
  IO.FS.writeFile "ir_program.json" (json.pretty 2)
  IO.println "Written to ir_program.json"
