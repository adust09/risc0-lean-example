use std::collections::HashMap;

use crate::ir_types::{FnBody, JoinPointId, Param, VarId};

use super::value::Value;

/// Tracks that a variable was obtained via `proj(parent_var, field_idx)`.
/// When the variable is modified via `set`, the change must propagate back
/// to the parent to preserve Lean IR's reference semantics.
#[derive(Debug, Clone)]
pub struct ProjProvenance {
    pub parent_var: VarId,
    pub field_idx: usize,
}

#[derive(Debug)]
pub struct CallFrame {
    pub fn_name: String,
    pub locals: HashMap<VarId, Value>,
    pub join_points: HashMap<JoinPointId, JoinPointDef>,
    pub proj_provenance: HashMap<VarId, ProjProvenance>,
}

#[derive(Debug, Clone)]
pub struct JoinPointDef {
    pub params: Vec<Param>,
    pub body: FnBody,
}

impl CallFrame {
    pub fn new(fn_name: String) -> Self {
        CallFrame {
            fn_name,
            locals: HashMap::new(),
            join_points: HashMap::new(),
            proj_provenance: HashMap::new(),
        }
    }

    pub fn get_var(&self, var: VarId) -> &Value {
        self.locals
            .get(&var)
            .unwrap_or_else(|| panic!("Variable x_{} not found in frame {}", var, self.fn_name))
    }

    pub fn set_var(&mut self, var: VarId, val: Value) {
        self.locals.insert(var, val);
    }

    pub fn record_proj(&mut self, result_var: VarId, parent_var: VarId, field_idx: usize) {
        self.proj_provenance.insert(result_var, ProjProvenance { parent_var, field_idx });
    }

    /// After modifying a variable via `set`, propagate the change back through
    /// the projection provenance chain so parent objects reflect the mutation.
    pub fn propagate_set(&mut self, var: VarId) {
        let mut current = var;
        while let Some(prov) = self.proj_provenance.get(&current).cloned() {
            let child_val = self.locals.get(&current).cloned();
            if let Some(child_val) = child_val {
                if let Some(parent) = self.locals.get_mut(&prov.parent_var) {
                    parent.set_field(prov.field_idx, child_val);
                }
            }
            current = prov.parent_var;
        }
    }

    pub fn register_jp(&mut self, jp: JoinPointId, params: Vec<Param>, body: FnBody) {
        self.join_points.insert(jp, JoinPointDef { params, body });
    }

    pub fn get_jp(&self, jp: JoinPointId) -> &JoinPointDef {
        self.join_points
            .get(&jp)
            .unwrap_or_else(|| panic!("Join point j_{} not found in frame {}", jp, self.fn_name))
    }
}
