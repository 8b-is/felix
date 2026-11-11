#![allow(non_snake_case)]

/// Opcode for the Felix language virtual machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    /// No-op (used for alignment / padding).
    Noop,

    /// Root-weight operations.
    Op_RootWeight,

    /// Multiplication root.
    Op_MulRoot,

    /// Addition root.
    Op_AddRoot,

    /// Subtraction root.
    Op_SubRoot,

    /// Division root.
    Op_DivRoot,

    /// Incremental module swap (M3 linker).
    Op_SwapModule,
}