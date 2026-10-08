//! Integer-register allocation in a permuted physical coordinate system.
//!
//! Instructions retain their original hardware meaning. Only the allocator's
//! ordering changes; operands and clobbers enter the same coordinate system,
//! and every allocation leaves it before the target emits machine code.

use alloc::{vec, vec::Vec};
use regalloc2::{
    Allocation, Block, Edit, Function, Inst, InstRange, MachineEnv, Operand, OperandConstraint,
    Output, PReg, PRegSet, RegAllocError, RegClass, RegallocOptions, VReg,
};

struct Order {
    forward: [PReg; PReg::NUM_INDEX],
    inverse: [PReg; PReg::NUM_INDEX],
}

impl Order {
    fn new(f: &impl Function, env: &MachineEnv, order: u8) -> Self {
        let mut mapping = Self {
            forward: core::array::from_fn(PReg::from_index),
            inverse: core::array::from_fn(PReg::from_index),
        };
        let mut limits = vec![0, PReg::MAX + 1];
        for index in 0..f.num_insts() {
            for operand in f.inst_operands(Inst::new(index)) {
                if operand.class() == RegClass::Int {
                    if let OperandConstraint::Limit(max) = operand.constraint() {
                        limits.push(max);
                    }
                }
            }
        }
        limits.sort_unstable();
        limits.dedup();
        let available = env.preferred_regs_by_class[0] | env.non_preferred_regs_by_class[0];
        // Range-constrained operands keep exactly the same set of candidates.
        for interval in limits.windows(2) {
            let registers = available
                .into_iter()
                .filter(|r| {
                    r.class() == RegClass::Int
                        && r.hw_enc() >= interval[0]
                        && r.hw_enc() < interval[1]
                        && !env.fixed_stack_slots.contains(r)
                })
                .collect::<Vec<_>>();
            if registers.len() < 2 {
                continue;
            }
            let mut permuted = registers.clone();
            if usize::from(order) < permuted.len() {
                permuted.rotate_left(usize::from(order));
            } else {
                let mut state = u64::from(order) | (1u64 << 32);
                for index in (1..permuted.len()).rev() {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    permuted.swap(index, (state % (index as u64 + 1)) as usize);
                }
            }
            for (&from, &to) in registers.iter().zip(&permuted) {
                mapping.forward[from.index()] = to;
                mapping.inverse[to.index()] = from;
            }
        }
        mapping
    }

    fn register(&self, register: PReg) -> PReg {
        self.forward[register.index()]
    }

    fn set(&self, set: PRegSet) -> PRegSet {
        set.into_iter()
            .fold(PRegSet::empty(), |set, r| set.with(self.register(r)))
    }

    fn operand(&self, operand: Operand) -> Operand {
        let constraint = match operand.constraint() {
            OperandConstraint::FixedReg(r) => OperandConstraint::FixedReg(self.register(r)),
            constraint => constraint,
        };
        Operand::new(operand.vreg(), constraint, operand.kind(), operand.pos())
    }

    fn restore(&self, allocation: &mut Allocation) {
        if let Some(register) = allocation.as_reg() {
            *allocation = Allocation::reg(self.inverse[register.index()]);
        }
    }
}

struct Coordinates<'a, F> {
    original: &'a F,
    order: &'a Order,
    operands: Vec<Vec<Operand>>,
}

impl<F: Function> Function for Coordinates<'_, F> {
    fn num_insts(&self) -> usize {
        self.original.num_insts()
    }
    fn num_blocks(&self) -> usize {
        self.original.num_blocks()
    }
    fn entry_block(&self) -> Block {
        self.original.entry_block()
    }
    fn block_insns(&self, block: Block) -> InstRange {
        self.original.block_insns(block)
    }
    fn block_succs(&self, block: Block) -> &[Block] {
        self.original.block_succs(block)
    }
    fn block_preds(&self, block: Block) -> &[Block] {
        self.original.block_preds(block)
    }
    fn block_params(&self, block: Block) -> &[VReg] {
        self.original.block_params(block)
    }
    fn is_ret(&self, inst: Inst) -> bool {
        self.original.is_ret(inst)
    }
    fn is_branch(&self, inst: Inst) -> bool {
        self.original.is_branch(inst)
    }
    fn branch_blockparams(&self, block: Block, inst: Inst, successor: usize) -> &[VReg] {
        self.original.branch_blockparams(block, inst, successor)
    }
    fn inst_operands(&self, inst: Inst) -> &[Operand] {
        &self.operands[inst.index()]
    }
    fn inst_clobbers(&self, inst: Inst) -> PRegSet {
        self.order.set(self.original.inst_clobbers(inst))
    }
    fn num_vregs(&self) -> usize {
        self.original.num_vregs()
    }
    fn debug_value_labels(&self) -> &[(VReg, Inst, Inst, u32)] {
        self.original.debug_value_labels()
    }
    fn spillslot_size(&self, class: RegClass) -> usize {
        self.original.spillslot_size(class)
    }
    fn multi_spillslot_named_by_last_slot(&self) -> bool {
        self.original.multi_spillslot_named_by_last_slot()
    }
    fn allow_multiple_vreg_defs(&self) -> bool {
        self.original.allow_multiple_vreg_defs()
    }
}

pub(super) fn run(
    f: &impl Function,
    environment: &MachineEnv,
    options: &RegallocOptions,
    selection: u8,
) -> Result<Output, RegAllocError> {
    if selection == 0 {
        return regalloc2::run(f, environment, options);
    }
    let order = Order::new(f, environment, selection);
    let coordinates = Coordinates {
        original: f,
        order: &order,
        operands: (0..f.num_insts())
            .map(|index| {
                f.inst_operands(Inst::new(index))
                    .iter()
                    .map(|&o| order.operand(o))
                    .collect()
            })
            .collect(),
    };
    let mut environment = environment.clone();
    for class in 0..3 {
        environment.preferred_regs_by_class[class] =
            order.set(environment.preferred_regs_by_class[class]);
        environment.non_preferred_regs_by_class[class] =
            order.set(environment.non_preferred_regs_by_class[class]);
        environment.scratch_by_class[class] =
            environment.scratch_by_class[class].map(|r| order.register(r));
    }
    for register in &mut environment.fixed_stack_slots {
        *register = order.register(*register);
    }
    let mut result = regalloc2::run(&coordinates, &environment, options)?;
    for allocation in &mut result.allocs {
        order.restore(allocation);
    }
    for (_, edit) in &mut result.edits {
        let Edit::Move { from, to } = edit;
        order.restore(from);
        order.restore(to);
    }
    for (_, _, _, location) in &mut result.debug_locations {
        order.restore(location);
    }
    Ok(result)
}
