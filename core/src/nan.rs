//! Canonical NaNs (docs/module-abi.md, section 3.2): a module is rewritten so that every float result that can be a NaN
//! is replaced by the canonical NaN when it is one, whatever the runtime's hardware produced.
use wasm_encoder::reencode::{Error, Reencode};
use wasm_encoder::{CodeSection, Instruction as I, ValType};
use wasmparser::{Parser, Payload};

struct Canon {
    type_params: Vec<u32>,
    func_types: Vec<u32>,
    next_body: usize,
}

#[derive(Debug)]
pub struct Unsupported;

impl Reencode for Canon {
    type Error = Unsupported;

    fn parse_function_body(&mut self, code: &mut CodeSection, func: wasmparser::FunctionBody<'_>) -> Result<(), Error<Unsupported>> {
        let ty = self.func_types[self.next_body] as usize;
        self.next_body += 1;
        let mut declared = 0u32;
        let mut locals = Vec::new();
        for pair in func.get_locals_reader()? {
            let (cnt, t) = pair?;
            declared = declared.checked_add(cnt).ok_or(Error::UserError(Unsupported))?;
            locals.push((cnt, self.val_type(t)?));
        }
        let first = self.type_params[ty] + declared;
        locals.push((1, ValType::F32));
        locals.push((1, ValType::F64));
        let (t32, t64) = (first, first + 1);
        let mut f = wasm_encoder::Function::new(locals);
        let mut reader = func.get_operators_reader()?;
        while !reader.eof() {
            let ins = self.parse_instruction(&mut reader)?;
            f.instruction(&ins);
            match ins {
                I::F32Add | I::F32Sub | I::F32Mul | I::F32Div | I::F32Sqrt | I::F32Min | I::F32Max | I::F32Ceil | I::F32Floor
                | I::F32Trunc | I::F32Nearest | I::F32Abs | I::F32Neg | I::F32DemoteF64 => {
                    f.instruction(&I::LocalTee(t32));
                    f.instruction(&I::F32Const(f32::from_bits(0x7fc00000).into()));
                    f.instruction(&I::LocalGet(t32));
                    f.instruction(&I::LocalGet(t32));
                    f.instruction(&I::F32Eq);
                    f.instruction(&I::Select);
                }
                I::F64Add | I::F64Sub | I::F64Mul | I::F64Div | I::F64Sqrt | I::F64Min | I::F64Max | I::F64Ceil | I::F64Floor
                | I::F64Trunc | I::F64Nearest | I::F64Abs | I::F64Neg | I::F64PromoteF32 => {
                    f.instruction(&I::LocalTee(t64));
                    f.instruction(&I::F64Const(f64::from_bits(0x7ff8000000000000).into()));
                    f.instruction(&I::LocalGet(t64));
                    f.instruction(&I::LocalGet(t64));
                    f.instruction(&I::F64Eq);
                    f.instruction(&I::Select);
                }
                _ => {}
            }
        }
        code.function(&f);
        Ok(())
    }
}

/// Returns the module with canonicalised float results, or None when it cannot be read.
pub fn canonicalize(wasm: &[u8]) -> Option<Vec<u8>> {
    let mut c = Canon { type_params: Vec::new(), func_types: Vec::new(), next_body: 0 };
    for p in Parser::new(0).parse_all(wasm) {
        match p.ok()? {
            Payload::TypeSection(r) => {
                for g in r {
                    for st in g.ok()?.types() {
                        match &st.composite_type.inner {
                            wasmparser::CompositeInnerType::Func(f) => c.type_params.push(f.params().len() as u32),
                            _ => c.type_params.push(0),
                        }
                    }
                }
            }
            Payload::FunctionSection(r) => {
                for t in r {
                    c.func_types.push(t.ok()?);
                }
            }
            _ => {}
        }
    }
    let mut module = wasm_encoder::Module::new();
    wasm_encoder::reencode::utils::parse_core_module(&mut c, &mut module, Parser::new(0), wasm).ok()?;
    Some(module.finish())
}
