pub mod eval_expr;
pub mod exec_body;
pub mod primitives;
pub mod stack;
pub mod value;

use std::collections::HashMap;

use ir_trace_common::trace_types::{TraceStep, ValueId};

use crate::ir_types::{Decl, FnBody};

use exec_body::{BodyExecutor, ExecResult};
use primitives::{call_primitive, lookup_primitive};
use stack::CallFrame;
use value::Value;

const MAX_CALL_DEPTH: usize = 10000;

pub struct Interpreter {
    pub decls: HashMap<String, Decl>,
    pub trace_steps: Vec<TraceStep>,
    pub value_table: Vec<Value>,
    pub fn_name_table: Vec<String>,
    fn_name_index: HashMap<String, u32>,
    call_depth: usize,
    pub extern_stubs: HashMap<String, Box<dyn Fn(&[Value]) -> Value>>,
}

impl Interpreter {
    pub fn new(decls: HashMap<String, Decl>) -> Self {
        Interpreter {
            decls,
            trace_steps: Vec::new(),
            value_table: Vec::new(),
            fn_name_table: Vec::new(),
            fn_name_index: HashMap::new(),
            call_depth: 0,
            extern_stubs: HashMap::new(),
        }
    }

    pub fn register_extern_stub(
        &mut self,
        name: &str,
        f: Box<dyn Fn(&[Value]) -> Value>,
    ) {
        self.extern_stubs.insert(name.to_string(), f);
    }

    fn get_fn_id(&mut self, name: &str) -> u32 {
        if let Some(id) = self.fn_name_index.get(name) {
            return *id;
        }
        let id = self.fn_name_table.len() as u32;
        self.fn_name_table.push(name.to_string());
        self.fn_name_index.insert(name.to_string(), id);
        id
    }

    fn register_value(&mut self, val: &Value) -> ValueId {
        let id = self.value_table.len() as ValueId;
        self.value_table.push(val.clone());
        id
    }

    pub fn call_function(&mut self, name: &str, args: Vec<Value>) -> Value {
        self.call_depth += 1;
        if self.call_depth > MAX_CALL_DEPTH {
            panic!(
                "Maximum call depth ({}) exceeded at function: {}",
                MAX_CALL_DEPTH, name
            );
        }
        let result = self.call_function_inner(name, args);
        self.call_depth -= 1;
        result
    }

    fn call_function_inner(&mut self, name: &str, args: Vec<Value>) -> Value {
        // Check for primitive operations first
        if let Some(prim_op) = lookup_primitive(name) {
            let result = call_primitive(&prim_op, args.clone());
            let arg_ids: Vec<ValueId> = args.iter().map(|a| self.register_value(a)).collect();
            let result_id = self.register_value(&result);
            self.trace_steps.push(TraceStep::PrimResult {
                op: prim_op,
                args: arg_ids,
                result: result_id,
            });
            return result;
        }

        // Check for extern stubs
        if let Some(stub) = self.extern_stubs.get(name) {
            let result = stub(&args);
            let fn_id = self.get_fn_id(name);
            let arg_ids: Vec<ValueId> = args.iter().map(|a| self.register_value(a)).collect();
            let result_id = self.register_value(&result);
            self.trace_steps.push(TraceStep::Call {
                fn_id,
                args: arg_ids,
                result: result_id,
            });
            return result;
        }

        // Look up declaration
        let decl = self
            .decls
            .get(name)
            .unwrap_or_else(|| panic!("Function not found: {}", name))
            .clone();

        match &decl {
            Decl::ExternDecl {
                name,
                params,
                ret_type,
            } => {
                // Unknown extern - check if it's a _boxed variant
                let base_name = name.strip_suffix("._boxed").unwrap_or(name);
                if base_name != name {
                    // Try calling the non-boxed version
                    if self.decls.contains_key(base_name) {
                        return self.call_function(base_name, args);
                    }
                    if let Some(prim) = lookup_primitive(base_name) {
                        let result = call_primitive(&prim, args.clone());
                        let arg_ids: Vec<ValueId> =
                            args.iter().map(|a| self.register_value(a)).collect();
                        let result_id = self.register_value(&result);
                        self.trace_steps.push(TraceStep::PrimResult {
                            op: prim,
                            args: arg_ids,
                            result: result_id,
                        });
                        return result;
                    }
                    if let Some(stub) = self.extern_stubs.get(base_name) {
                        let result = stub(&args);
                        return result;
                    }
                }
                eprintln!(
                    "Warning: unhandled extern function: {} (params: {}, ret: {:?})",
                    name,
                    params.len(),
                    ret_type
                );
                Value::Irrelevant
            }
            Decl::FnDecl {
                name: fn_name,
                params,
                body,
                ..
            } => {
                let fn_id = self.get_fn_id(fn_name);
                let arg_ids: Vec<ValueId> =
                    args.iter().map(|a| self.register_value(a)).collect();

                // Create a new call frame
                let mut frame = CallFrame::new(fn_name.clone());
                for (param, val) in params.iter().zip(args.iter()) {
                    frame.set_var(param.var, val.clone());
                }

                // Execute the body with continuation-based loop
                let result = self.exec_with_continuations(&mut frame, body);

                let result_id = self.register_value(&result);
                self.trace_steps.push(TraceStep::Call {
                    fn_id,
                    args: arg_ids,
                    result: result_id,
                });
                result
            }
        }
    }

    fn exec_with_continuations(&mut self, frame: &mut CallFrame, body: &FnBody) -> Value {
        let mut current_body = body.clone();

        loop {
            let exec_result = {
                let mut executor = BodyExecutor {
                    frame,
                    trace_steps: &mut self.trace_steps,
                    value_table: &mut self.value_table,
                };
                executor.exec(&current_body)
            };

            match exec_result {
                ExecResult::Return(val) => return val,
                ExecResult::Call {
                    target,
                    args,
                    result_var,
                    cont,
                } => {
                    let result = self.call_function(&target, args);
                    frame.set_var(result_var, result);
                    current_body = cont;
                }
            }
        }
    }
}
