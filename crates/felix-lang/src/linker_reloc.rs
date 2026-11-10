//! Incremental relocation engine for Felix modules.
//! Responsible for applying relocation fixes when a module is swapped in.
//! Currently a stub – real relocation logic will be added in M3/M4.

use crate::codegen_v2::Instruction; // placeholder import

/// A relocation entry describes how to fix up an instruction after a module swap.
/// In a full implementation this would contain the original offset, the new target,
/// and the kind of fix (register rename, address adjust, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reloc {
    pub offset: usize,
    pub new_target: usize,
    /// Kind of relocation – e.g. register rename, address adjust, capability update.
    pub kind: RelocKind,
}

/// Kinds of relocations the linker may need to perform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelocKind {
    RegisterRename,
    AddressAdjust,
    CapabilityUpdate,
    /// Add more as needed.
}

/// Apply a slice of relocations to the given machine code buffer.
/// Returns `true` if all relocations were applied successfully, `false` otherwise.
pub fn relocate(code: &mut [u8], relocs: &[Reloc]) -> bool {
    // TODO: real relocation logic.
    // For now, just validate that offsets are within bounds.
    for r in relocs {
        if r.offset >= code.len() {
            eprintln!("Relocation offset {} out of bounds (code len {})", r.offset, code.len());
            return false;
        }
        // Stub: just record the new target (could patch a byte, update a register, etc.).
        // e.g., code[r.offset] = r.new_target as u8;
    }
    true // assume success for now
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reloc_basic() {
        let mut code: [u8; 16] = [0u8; 16];
        let relocs = vec![
            Reloc {
                offset: 0,
                new_target: 42,
                kind: RelocKind::RegisterRename,
            },
            Reloc {
                offset: 4,
                new_target: 99,
                kind: RelocKind::AddressAdjust,
            },
        ];
        let ok = relocate(&mut code, &relocs);
        assert!(ok, "relocate should return true for in‑bounds offsets");
    }

    #[test]
    fn test_reloc_out_of_bounds() {
        let mut code: [u8; 8] = [0u8; 8];
        let relocs = vec![Reloc {
            offset: 100, // out of bounds
            new_target: 0,
            kind: RelocKind::RegisterRename,
        }];
        let ok = relocate(&mut code, &relocs);
        assert!(!ok, "relocate should return false for out‑of‑bounds offsets");
    }
}