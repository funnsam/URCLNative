use crate::urclrs::ast::{Program, Operand};
use std::{collections::HashMap, path::Path};
use inkwell::{builder::Builder, module::*, context::Context, values::*, types::*, targets::*, basic_block::*, *};

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    regs: HashMap<usize, PointerValue<'ctx>>,
    pc: Vec<BasicBlock<'ctx>>,
    reg_t: IntType<'ctx>
}

impl Codegen<'_> {
    pub fn build(prog: &Program) {
        let context = Context::create();
        let module  = context.create_module("URCL_App");
        let reg_t = context.i32_type();
        let mut codegen = Codegen {
            context: &context,
            module,
            builder: context.create_builder(),
            regs: HashMap::new(),
            pc: Vec::new(),
            reg_t
        };
        codegen.compile(prog);
        codegen.module.print_to_stderr();

        Target::initialize_all(&InitializationConfig::default());
        codegen.write(&FileType::Object, &Path::new("a.o"));
    }

    fn write(&mut self, filetype: &FileType, path: &Path) {
        let triple  = TargetMachine::get_default_triple();
        let target  = Target::from_triple(&triple).unwrap();
        let cpu     = TargetMachine::get_host_cpu_name();
        let features= TargetMachine::get_host_cpu_features();
        let reloc   = RelocMode::Default;
        let model   = CodeModel::Default;
        let opt     = OptimizationLevel::Default;
        let target_machine = target
            .create_target_machine(
                &triple, 
                cpu.to_str().unwrap(), 
                features.to_str().unwrap(), 
                opt, 
                reloc,
                model
            )
            .unwrap();
        
        target_machine.write_to_file(&self.module, *filetype, path).unwrap();
    }

    fn compile(&mut self, prog: &Program) {
        let reg_t = self.reg_t;
        let pout  = self.module.add_function("urcl_pout", self.context.void_type().fn_type(&[reg_t.try_into().unwrap(), reg_t.try_into().unwrap()], false), Some(Linkage::External));

        let main  = self.module.add_function("urcl_main", self.context.void_type().fn_type(&[], false), None);

        let alloc = self.context.append_basic_block(main, "alloc");
        let dw_s  = self.context.append_basic_block(main, "dw_set");
        self.builder.position_at_end(alloc);

        for i in 1..=prog.headers.minreg {
            let reg = self.builder.build_alloca(reg_t, &format!("reg_{i}"));
            self.regs.insert(i as usize, reg);
        }

        let mem = self.builder.build_array_alloca(reg_t, reg_t.const_int(prog.headers.minheap + prog.headers.minstack + prog.memory.len() as u64, false), "memory");
        let align = reg_t.get_alignment();

        self.builder.build_unconditional_branch(dw_s);

        self.builder.position_at_end(dw_s);

        for (i, _) in prog.instructions.iter().enumerate() {
            let this = self.context.append_basic_block(main, &format!("pc_{i}"));
            self.pc.push(this);
        }

        let end = self.context.append_basic_block(main, "endblk");
        self.pc.push(end);

        for (i, instr) in prog.instructions.iter().enumerate() {
            let this = self.pc[i];

            if matches!(self.builder.get_insert_block().unwrap().get_terminator(), None) {
                self.builder.build_unconditional_branch(this);
            }

            self.builder.position_at_end(this);

            use crate::Inst::*;
            match instr {
                ADD(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let add = self.builder.build_int_add(b, c, "add");
                    self.set_val(a, &add);
                },
                OUT(a, b) => {
                    let a = self.get_val(a).try_into().unwrap();
                    let b = self.get_val(b).try_into().unwrap();
                    self.builder.build_call(pout, &[a, b], "pout_ret");
                },
                RSH(a, b) => {
                    let b = self.get_val(b);
                    let sh = self.builder.build_right_shift(b, self.reg_t.const_int(1, false), false, "rsh");
                    self.set_val(a, &sh);
                },
                MOV(a, b) => self.set_val(a, &self.get_val(b)),
                NOR(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let nor = self.builder.build_or(b, c, "nor_or");
                    let nor = self.builder.build_not(nor, "nor_not");
                    self.set_val(a, &nor);
                },
                BGE(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::UGE, b, c, "bge_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                JMP(d) => {
                    self.builder.build_unconditional_branch(self.pc[unwrap_imm(d) as usize]);
                },
                _ => todo!("unimpl {instr:?}")
            }
        }

        if matches!(self.builder.get_insert_block().unwrap().get_terminator(), None) {
            self.builder.build_unconditional_branch(end);
        }
        self.builder.position_at_end(end);
        self.builder.build_return(None);
    }

    fn get_val(&self, oper: &Operand) -> IntValue {
        match oper {
            Operand::Reg(v) => {
                if *v == 0 {
                    self.reg_t.const_zero()
                } else {
                    self.builder.build_load(self.reg_t, self.regs[&(*v as usize)], "reg_load").try_into().unwrap()
                }
            },
            Operand::Imm(v) => self.reg_t.const_int(*v as u64, false),
            _ => panic!()
        }
    }

    fn set_val(&self, oper: &Operand, val: &IntValue) {
        match oper {
            Operand::Reg(v) => {
                if *v != 0 {
                    self.builder.build_store(self.regs[&(*v as usize)], *val);
                }
            }
            _ => ()
        }
    }
}

fn unwrap_imm(oper: &Operand) -> u64 {
    match oper {
        Operand::Imm(v) => *v as u64,
        _ => panic!()
    }
}
