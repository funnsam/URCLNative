use crate::{urclrs::ast::{Program, Operand}, PC, SP};
use std::{collections::HashMap, path::Path};
use inkwell::{builder::Builder, module::*, context::Context, values::*, types::*, targets::*, basic_block::*, debug_info::*, *};
pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    regs: HashMap<u64, PointerValue<'ctx>>,
    pc: Vec<BasicBlock<'ctx>>,
    reg_t: IntType<'ctx>
}

impl<'ctx> Codegen<'_> {
    pub fn build(prog: &Program, out: &str, debug: bool, file: &str) {
        let context = Context::create();
        let module  = context.create_module("URCL Program");
        let reg_t = context.i16_type();
        let mut codegen = Codegen {
            context: &context,
            module,
            builder: context.create_builder(),
            regs: HashMap::new(),
            pc: Vec::new(),
            reg_t
        };

        codegen.compile(prog, debug, file);
        if debug {
            codegen.module.print_to_stderr();
        }

        Target::initialize_all(&InitializationConfig::default());
        codegen.write(&FileType::Object, &Path::new(out), debug);
    }

    fn write(&mut self, filetype: &FileType, path: &Path, debug: bool) {
        let triple   = TargetMachine::get_default_triple();
        let target   = Target::from_triple(&triple).unwrap();
        let cpu      = TargetMachine::get_host_cpu_name();
        let features = TargetMachine::get_host_cpu_features();
        let reloc    = RelocMode::Default;
        let model    = CodeModel::Default;
        let opt      = match debug {
            true  => OptimizationLevel::None,
            false => OptimizationLevel::Default,
        };
        let target_machine = target.create_target_machine(
            &triple, 
            cpu.to_str().unwrap(), 
            features.to_str().unwrap(), 
            opt, 
            reloc,
            model
        ).unwrap();
        
        target_machine.write_to_file(&self.module, *filetype, path).unwrap();
    }

    fn compile(&mut self, prog: &Program, debug: bool, file: &str) {
        let reg_t = self.reg_t;
//      let reg_w = reg_t.get_bit_width();
        let pin   = self.module.add_function("urcl_pin", reg_t.fn_type(&[reg_t.try_into().unwrap()], false), Some(Linkage::External));
        let pout  = self.module.add_function("urcl_pout", self.context.void_type().fn_type(&[reg_t.try_into().unwrap(), reg_t.try_into().unwrap()], false), Some(Linkage::External));

        let main  = self.module.add_function("urcl_main", self.context.void_type().fn_type(&[], false), None);

        let debug_metadata_version = self.context.i32_type().const_int(3, false);
        self.module.add_basic_value_flag(
            "Debug Info Version",
            inkwell::module::FlagBehavior::Warning,
            debug_metadata_version,
        );

        let (dibuilder, compile_unit) = self.module.create_debug_info_builder(
            true,
            /* language */ inkwell::debug_info::DWARFSourceLanguage::C,
            /* filename */ file,
            /* directory */ ".",
            /* producer */ "urclnative",
            /* is_optimized */ false,
            /* compiler command line flags */ "",
            /* runtime_ver */ 0,
            /* split_name */ "",
            /* kind */ inkwell::debug_info::DWARFEmissionKind::Full,
            /* dwo_id */ 0,
            /* split_debug_inling */ false,
            /* debug_info_for_profiling */ false,
            "", ""
        );

        let main_dt = dibuilder.create_subroutine_type(
            compile_unit.get_file(), None, &[],
            inkwell::debug_info::DIFlags::PUBLIC,
        );
        let main_scope: DISubprogram<'_> = dibuilder.create_function(
            /* scope */ compile_unit.as_debug_info_scope(),
            /* func name */ "urcl_main",
            /* linkage_name */ None,
            /* file */ compile_unit.get_file(),
            /* line_no */ 0,
            /* DIType */ main_dt,
            /* is_local_to_unit */ true,
            /* is_definition */ true,
            /* scope_line */ 0,
            /* flags */ inkwell::debug_info::DIFlags::PUBLIC,
            /* is_optimized */ false,
        );
        main.set_subprogram(main_scope);

        let lexical_block = dibuilder.create_lexical_block(
            main_scope.as_debug_info_scope(),
            compile_unit.get_file(),
            0, 0
        );

        let alloc = self.context.append_basic_block(main, "alloc");
        let init_v = self.context.append_basic_block(main, "init_v");
        self.builder.position_at_end(alloc);

        for i in 1..=prog.headers.minreg {
            let name = format!("R{i}");
            let reg = self.builder.build_alloca(reg_t, &name);
            self.builder.build_store(reg, reg_t.const_zero());
            self.regs.insert(i, reg);
        }

        let pc = self.builder.build_alloca(reg_t, "reg_pc");
        let sp = self.builder.build_alloca(reg_t, "reg_sp");
        self.regs.insert(PC, pc);
        self.regs.insert(SP, sp);

        let totmem = prog.headers.minstack + prog.headers.minheap + prog.memory.len() as u64;
        let mem = self.builder.build_array_alloca(reg_t, reg_t.const_int(totmem << 1, false), "memory");
        let align = reg_t.get_alignment();

        self.builder.build_unconditional_branch(init_v);

        self.builder.position_at_end(init_v);
        self.builder.build_store(sp, reg_t.const_int(totmem, false));

        for (i, e) in prog.memory.iter().enumerate() {
            self.builder.build_store(self.get_mem_loc(&mem, &reg_t.const_int(i as u64, false), &align), reg_t.const_int(*e, false));
        }

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


            let loc = dibuilder.create_debug_location(
                self.context, prog.debug.pc_to_line_start[i] as u32, 0,
                lexical_block.as_debug_info_scope(),
                None
            );
            self.builder.set_current_debug_location(loc);

            self.builder.build_store(pc, reg_t.const_int(i as u64, false));

            use crate::Inst::*;
            match instr {
                ADD(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let r = self.builder.build_int_add(b, c, "add");
                    self.set_val(a, &r);
                },
                SUB(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let r = self.builder.build_int_sub(b, c, "sub");
                    self.set_val(a, &r);
                },
                MLT(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let r = self.builder.build_int_mul(b, c, "mlt");
                    self.set_val(a, &r);
                },
                DIV(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let r = self.builder.build_int_unsigned_div(b, c, "div");
                    self.set_val(a, &r);
                },
                MOD(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let r = self.builder.build_int_unsigned_rem(b, c, "mod");
                    self.set_val(a, &r);
                },
                INC(a, b) => {
                    self.set_val(a, &self.builder.build_int_add(self.get_val(b), reg_t.const_int(1, false), "inc"));
                },
                DEC(a, b) => {
                    self.set_val(a, &self.builder.build_int_sub(self.get_val(b), reg_t.const_int(1, false), "dec"));
                },
                IN(a, b) => {
                    let b = self.get_val(b).try_into().unwrap();
                    let ret = self.builder.build_call(pin, &[b], "pin_ret").try_as_basic_value().unwrap_left();
                    self.set_val(a, &ret.try_into().unwrap());
                },
                OUT(a, b) => {
                    let a = self.get_val(a).try_into().unwrap();
                    let b = self.get_val(b).try_into().unwrap();
                    self.builder.build_call(pout, &[a, b], "pout_ret");
                },
                LSH(a, b) => {
                    let b  = self.get_val(b);
                    let sh = self.builder.build_left_shift(b, self.reg_t.const_int(1, false), "lsh");
                    self.set_val(a, &sh);
                },
                RSH(a, b) => {
                    let b  = self.get_val(b);
                    let sh = self.builder.build_right_shift(b, self.reg_t.const_int(1, false), false, "rsh");
                    self.set_val(a, &sh);
                },
                SRS(a, b) => {
                    let b  = self.get_val(b);
                    let sh = self.builder.build_right_shift(b, self.reg_t.const_int(1, false), true, "srs");
                    self.set_val(a, &sh);
                },
                BSL(a, b, c) => {
                    let b  = self.get_val(b);
                    let c  = self.get_val(c);
                    let sh = self.builder.build_left_shift(b, c, "bsl");
                    self.set_val(a, &sh);
                },
                BSR(a, b, c) => {
                    let b  = self.get_val(b);
                    let c  = self.get_val(c);
                    let sh = self.builder.build_right_shift(b, c, false, "bsr");
                    self.set_val(a, &sh);
                },
                BSS(a, b, c) => {
                    let b  = self.get_val(b);
                    let c  = self.get_val(c);
                    let sh = self.builder.build_right_shift(b, c, true, "bss");
                    self.set_val(a, &sh);
                },
                MOV(a, b) => self.set_val(a, &self.get_val(b)),
                AND(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let and = self.builder.build_and(b, c, "and");
                    self.set_val(a, &and);
                },
                NAND(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let and = self.builder.build_and(b, c, "nand_and");
                    let nand = self.builder.build_not(and, "nand_not");
                    self.set_val(a, &nand);
                },
                OR(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let or = self.builder.build_or(b, c, "or");
                    self.set_val(a, &or);
                },
                NOR(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let or = self.builder.build_or(b, c, "nor_or");
                    let nor = self.builder.build_not(or, "nor_not");
                    self.set_val(a, &nor);
                },
                XOR(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let xor = self.builder.build_xor(b, c, "xor");
                    self.set_val(a, &xor);
                },
                XNOR(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let xor = self.builder.build_xor(b, c, "xnor_xor");
                    let xnor = self.builder.build_not(xor, "xnor_not");
                    self.set_val(a, &xnor);
                },
                NOT(a, b) => {
                    let b = self.get_val(b);
                    let not = self.builder.build_not(b, "not");
                    self.set_val(a, &not);
                },
                PSH(a) => {
                    self.push(&self.get_val(a), &align, &sp, &mem);
                },
                POP(a) => {
                    self.set_val(a, &self.pop(&align, &sp, &mem));
                },
                CAL(d) => {
                    self.push(&reg_t.const_int(i as u64 + 1, false), &align, &sp, &mem);
    
                    match d {
                        Operand::Reg(_) => self.build_pc_jmp(&reg_t, &self.get_val(d)),
                        Operand::Imm(v) => { self.builder.build_unconditional_branch(self.pc[*v as usize]); },
                        _ => panic!()
                    };
                },
                RET => {
                    let npc = self.pop(&align, &sp, &mem);
                    self.build_pc_jmp(&reg_t, &npc);
                },
                SETE(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::EQ, b, c, "sete_cmp");
                    self.set_val(a, &cmp);
                },
                SETNE(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::NE, b, c, "setne_cmp");
                    self.set_val(a, &cmp);
                },
                SETL(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::ULT, b, c, "setl_cmp");
                    self.set_val(a, &cmp);
                },
                SETG(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::UGT, b, c, "setg_cmp");
                    self.set_val(a, &cmp);
                },
                SETLE(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::ULE, b, c, "setle_cmp");
                    self.set_val(a, &cmp);
                },
                SETGE(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::UGE, b, c, "setge_cmp");
                    self.set_val(a, &cmp);
                },
                SETC(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let add = self.builder.build_int_add(b, c, "setc_test");
                    let cmp1 = self.builder.build_int_compare(IntPredicate::ULT, add, b, "setc_cmp1");
                    let cmp2 = self.builder.build_int_compare(IntPredicate::ULT, add, c, "setc_cmp2");
                    let finl = self.builder.build_or(cmp1, cmp2, "setc_f");
                    self.set_val(a, &finl);
                },
                SETNC(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let add = self.builder.build_int_add(b, c, "setnc_test");
                    let cmp1 = self.builder.build_int_compare(IntPredicate::UGE, add, b, "setnc_cmp1");
                    let cmp2 = self.builder.build_int_compare(IntPredicate::UGE, add, c, "setnc_cmp2");
                    let finl = self.builder.build_or(cmp1, cmp2, "setnc_f");
                    self.set_val(a, &finl);
                },
                BRC(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let add = self.builder.build_int_add(b, c, "brc_test");
                    let cmp1 = self.builder.build_int_compare(IntPredicate::ULT, add, b, "brc_cmp1");
                    let cmp2 = self.builder.build_int_compare(IntPredicate::ULT, add, c, "brc_cmp2");
                    let finl = self.builder.build_or(cmp1, cmp2, "brc_f");
                    self.builder.build_conditional_branch(finl, dest, self.pc[i+1]);
                },
                BNC(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let add = self.builder.build_int_add(b, c, "bnc_test");
                    let cmp1 = self.builder.build_int_compare(IntPredicate::UGE, add, b, "bnc_cmp1");
                    let cmp2 = self.builder.build_int_compare(IntPredicate::UGE, add, c, "bnc_cmp2");
                    let finl = self.builder.build_or(cmp1, cmp2, "bnc_f");
                    self.builder.build_conditional_branch(finl, dest, self.pc[i+1]);
                },
                BRP(a, b) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let cmp = self.builder.build_int_compare(IntPredicate::ULT, b, reg_t.const_int(0x8000_0000, false), "brp_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BRN(a, b) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let cmp = self.builder.build_int_compare(IntPredicate::UGE, b, reg_t.const_int(0x8000_0000, false), "brn_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BRZ(a, b) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let cmp = self.builder.build_int_compare(IntPredicate::EQ, b, reg_t.const_int(0, false), "brz_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BNZ(a, b) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let cmp = self.builder.build_int_compare(IntPredicate::NE, b, reg_t.const_int(0, false), "bnz_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BEV(a, b) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let evn = self.builder.build_and(b, reg_t.const_int(1, false), "even");
                    self.builder.build_conditional_branch(evn, dest, self.pc[i+1]);
                },
                BOD(a, b) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.builder.build_not(self.get_val(b), "bod_invert");
                    let odd = self.builder.build_and(b, reg_t.const_int(1, false), "odd");
                    self.builder.build_conditional_branch(odd, dest, self.pc[i+1]);
                },
                BRE(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::EQ, b, c, "bre_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BNE(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::NE, b, c, "bne_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BRL(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::ULT, b, c, "brl_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BRG(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::UGT, b, c, "brg_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BLE(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::ULE, b, c, "ble_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                BGE(a, b, c) => {
                    let dest = self.pc[unwrap_imm(a) as usize];
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let cmp = self.builder.build_int_compare(IntPredicate::UGE, b, c, "bge_cmp");
                    self.builder.build_conditional_branch(cmp, dest, self.pc[i+1]);
                },
                JMP(d) => {
                    match d {
                        Operand::Reg(_) => self.build_pc_jmp(&reg_t, &self.get_val(d)),
                        Operand::Imm(v) => { self.builder.build_unconditional_branch(self.pc[*v as usize]); },
                        _ => panic!()
                    };
                },
                LOD(a, b) => {
                    let b = self.get_val(b);
                    self.set_val(a, &self.builder.build_load(reg_t, self.get_mem_loc(&mem, &b, &align), "mem_load").try_into().unwrap());
                },
                STR(a, b) => {
                    let a = self.get_val(a);
                    let b = self.get_val(b);
                    self.builder.build_store(self.get_mem_loc(&mem, &a, &align), b);
                },
                LLOD(a, b, c) => {
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let sum = self.builder.build_int_add(b, c, "llod_sum");
                    self.set_val(a, &self.builder.build_load(reg_t, self.get_mem_loc(&mem, &sum, &align), "mem_load").try_into().unwrap());
                },
                LSTR(a, b, c) => {
                    let a = self.get_val(a);
                    let b = self.get_val(b);
                    let c = self.get_val(c);
                    let sum = self.builder.build_int_add(a, b, "lstr_sum");
                    self.builder.build_store(self.get_mem_loc(&mem, &sum, &align), c);
                },
                CPY(a, b) => {
                    let a = self.get_val(a);
                    let b = self.get_val(b);
                    self.builder.build_store(
                        self.get_mem_loc(&mem, &a, &align),
                        self.builder.build_load(
                            reg_t,
                            self.get_mem_loc(&mem, &b, &align),
                            "cpy_load"
                        )
                    );
                },
                HLT => {
                    self.builder.build_return(None);
                },
                NOP => (),
                _ => todo!("unimpl {instr:?}")
            }
        }

        if matches!(self.builder.get_insert_block().unwrap().get_terminator(), None) {
            self.builder.build_unconditional_branch(end);
        }
        self.builder.position_at_end(end);
        self.builder.build_return(None);
        dibuilder.finalize();
    }

    fn push(&'ctx self, a: &IntValue<'ctx>, align: &IntValue<'ctx>, sp: &PointerValue<'ctx>, mem: &PointerValue<'ctx>) {
        let csp = self.builder.build_load(self.reg_t, *sp, "cur_sp").try_into().unwrap();
        let nsp = self.builder.build_int_sub(csp, self.reg_t.const_int(1, false), "sp_sub");
        self.builder.build_store(*sp, nsp);

        self.builder.build_store(self.get_mem_loc(mem, &nsp, align), *a);
    }

    fn pop(&'ctx self, align: &IntValue<'ctx>, sp: &PointerValue<'ctx>, mem: &PointerValue<'ctx>) -> IntValue<'ctx> {
        let csp = self.builder.build_load(self.reg_t, *sp, "cur_sp").try_into().unwrap();

        let ret = self.builder.build_load(self.reg_t, self.get_mem_loc(mem, &csp, align), "mem_load").try_into().unwrap();

        let nsp = self.builder.build_int_add(csp, self.reg_t.const_int(1, false), "sp_add");
        self.builder.build_store(*sp, nsp);

        ret
    }

    fn get_mem_loc(&'ctx self, mem: &PointerValue<'ctx>, indx: &IntValue<'ctx>, al: &IntValue<'ctx>) -> PointerValue<'ctx> {
        let ofs = self.builder.build_int_mul(*indx, *al, "mem_ofs");
        let ofs = self.builder.build_cast(InstructionOpcode::IntToPtr, ofs, mem.get_type(), "mem_ofs_cast").try_into().unwrap();
        self.builder.build_int_add(*mem, ofs, "mem_elem")
    }

    fn build_pc_jmp(&'ctx self, rt: &IntType<'ctx>, val: &IntValue<'ctx>) {
        let mut jmpt = Vec::with_capacity(self.pc.len());
        for (i, j) in self.pc.iter().enumerate() {
            jmpt.push((rt.const_int(i as u64, false), *j))
        }
        self.builder.build_switch(*val, *self.pc.last().unwrap(), jmpt.as_slice());
    }

    fn get_val(&self, oper: &Operand) -> IntValue {
        match oper {
            Operand::Reg(v) => {
                if *v == 0 {
                    self.reg_t.const_zero()
                } else {
                    self.builder.build_load(self.reg_t, self.regs[v], "reg_load").try_into().unwrap()
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
                    self.builder.build_store(self.regs[v], *val);
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
