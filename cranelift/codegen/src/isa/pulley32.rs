pub use super::pulley_shared::isa_builder;

use super::pulley_shared::PulleyTargetKind;
use crate::isa::pulley_shared::PointerWidth;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Pulley32;

impl PulleyTargetKind for Pulley32 {
    fn pointer_width() -> PointerWidth {
        PointerWidth::PointerWidth32
    }
}

/// The no-OS Pulley profile used as PolyASM's compiler interchange.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct BarePulley32;

impl PulleyTargetKind for BarePulley32 {
    fn pointer_width() -> PointerWidth {
        PointerWidth::PointerWidth32
    }

    fn compact_integer_registers() -> bool {
        true
    }
}
