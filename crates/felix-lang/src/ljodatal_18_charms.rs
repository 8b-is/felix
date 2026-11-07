//! `ljodatal_18_charms.rs`
//! Defines the 18 charms of Hávamál (Ljóðatal) and Capability Effect Masks,
//! specifically Charm XVIII (`CAP_ROOT_SEAL = 0x8000_0000`).

use crate::felix_phoenix::CapEffectMask;

/// Charm XVIII: The Eighteenth Charm (Hávamál Stanza 163)
/// "That eighteenth I know, which to none I will ever tell,
/// girl nor man's wife, all is better that one alone knows..."
/// Forbidden from ever crossing an external, unsealed network or host bus.
pub const CAP_ROOT_SEAL: CapEffectMask = 0x8000_0000;

pub const CHARM_NAMES: [&str; 18] = [
    "Hjálp (Help)",
    "Lækning (Healing)",
    "Fjöturr (Fetters)",
    "Lausn (Release)",
    "Stöðvun (Arresting Spear)",
    "Bann (Hate Turning)",
    "Eldsslökkvun (Quenching Fire)",
    "Sátt (Reconciliation)",
    "Bylgjustöðvun (Stilling Waves)",
    "Vittfarir (Witch Banishment)",
    "Hlíf (Shield Defense)",
    "Draugatal (Hanged Man's Song)",
    "Vatnsausa (Baptism / Invulnerability)",
    "Ásatöl (Genealogy of the Gods)",
    "Dvalins Leika (Sun / Dwarf Wisdom)",
    "Hugsvinns (Mind Striving)",
    "Meyjarrúnar (Maiden's Love)",
    "Ókunnr (The Unspoken Eighteenth)",
];
