use serde::{Deserialize, Serialize};

pub type FunId = String;
pub type VarId = u32;
pub type JoinPointId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRType {
    Float,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    USize,
    Object,
    TObject,
    Irrelevant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub var: VarId,
    pub ty: IRType,
    pub borrowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decl {
    FnDecl {
        name: FunId,
        params: Vec<Param>,
        ret_type: IRType,
        body: FnBody,
    },
    ExternDecl {
        name: FunId,
        params: Vec<Param>,
        ret_type: IRType,
    },
}

impl Decl {
    pub fn name(&self) -> &str {
        match self {
            Decl::FnDecl { name, .. } | Decl::ExternDecl { name, .. } => name,
        }
    }

    pub fn params(&self) -> &[Param] {
        match self {
            Decl::FnDecl { params, .. } | Decl::ExternDecl { params, .. } => params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtorInfo {
    pub name: String,
    pub cidx: u16,
    pub size: u32,
    pub usize_fields: u32,
    pub scalar_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LitValue {
    Num(u64),
    Str(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Arg {
    Var(VarId),
    Irrelevant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FnBody {
    VDecl {
        var: VarId,
        ty: IRType,
        expr: Expr,
        cont: Box<FnBody>,
    },
    JDecl {
        jp: JoinPointId,
        params: Vec<Param>,
        body: Box<FnBody>,
        cont: Box<FnBody>,
    },
    Set {
        var: VarId,
        idx: u32,
        val: Arg,
        cont: Box<FnBody>,
    },
    USet {
        var: VarId,
        idx: u32,
        val: VarId,
        cont: Box<FnBody>,
    },
    SSet {
        var: VarId,
        n: u32,
        offset: u32,
        val: VarId,
        ty: IRType,
        cont: Box<FnBody>,
    },
    SetTag {
        var: VarId,
        cidx: u16,
        cont: Box<FnBody>,
    },
    Case {
        tid: String,
        scrutinee: VarId,
        alts: Vec<Alt>,
    },
    Ret {
        arg: Arg,
    },
    Jmp {
        jp: JoinPointId,
        args: Vec<Arg>,
    },
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Ctor {
        info: CtorInfo,
        args: Vec<Arg>,
    },
    Proj {
        idx: u32,
        var: VarId,
    },
    UProj {
        idx: u32,
        var: VarId,
    },
    SProj {
        n: u32,
        offset: u32,
        var: VarId,
    },
    FAp {
        fun: FunId,
        args: Vec<Arg>,
    },
    PAp {
        fun: FunId,
        args: Vec<Arg>,
    },
    Ap {
        fun: VarId,
        args: Vec<Arg>,
    },
    Box {
        ty: IRType,
        var: VarId,
    },
    Unbox {
        var: VarId,
    },
    Lit {
        val: LitValue,
    },
    IsShared {
        var: VarId,
    },
    Reset {
        n: u32,
        var: VarId,
    },
    Reuse {
        var: VarId,
        info: CtorInfo,
        upd_header: bool,
        args: Vec<Arg>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Alt {
    Ctor {
        info: CtorInfo,
        body: FnBody,
    },
    Default {
        body: FnBody,
    },
}

impl Alt {
    pub fn body(&self) -> &FnBody {
        match self {
            Alt::Ctor { body, .. } | Alt::Default { body } => body,
        }
    }
}
