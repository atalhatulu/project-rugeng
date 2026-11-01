//! Temel tipler: Entity kimlikleme ve bit maskeleri.

pub const ENTITY_INDEX_BITS: u32 = 20;
pub const ENTITY_GEN_BITS: u32 = 12;

pub const ENTITY_INDEX_MASK: u32 = (1 << ENTITY_INDEX_BITS) - 1; // 0x000F_FFFF (1.048.575)
pub const ENTITY_GEN_MASK: u32 = (1 << ENTITY_GEN_BITS) - 1; // 0x0000_0FFF (4.095)

/// Maksimum geçerli Entity indeksi. `0x000F_FFFF` indeksi `Entity::NULL` için ayrılmıştır.
pub const MAX_ENTITY_INDEX: u32 = ENTITY_INDEX_MASK - 1; // 0x000F_FFFE

use serde::{Deserialize, Serialize};

/// 32-bit sıkıştırılmış Entity kimliği:
/// [31..20] Generation (12-bit) - ABA problemini önler
/// [19..0]  Index (20-bit)      - 1 milyona kadar benzersiz slot
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Entity(pub u32);

impl Entity {
    pub const NULL: Entity = Entity(u32::MAX);

    #[inline(always)]
    pub const fn new(index: u32, generation: u32) -> Self {
        assert!(
            index <= MAX_ENTITY_INDEX,
            "Entity index sinir disi (0x000F_FFFF Entity::NULL icin ayrilmistir)"
        );
        assert!(
            generation <= ENTITY_GEN_MASK,
            "Entity generation sinir disi"
        );
        Entity(((generation & ENTITY_GEN_MASK) << ENTITY_INDEX_BITS) | (index & ENTITY_INDEX_MASK))
    }

    #[inline(always)]
    pub const fn index(self) -> u32 {
        self.0 & ENTITY_INDEX_MASK
    }

    #[inline(always)]
    pub const fn generation(self) -> u32 {
        (self.0 >> ENTITY_INDEX_BITS) & ENTITY_GEN_MASK
    }

    #[inline(always)]
    pub const fn to_bits(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub const fn from_bits(bits: u32) -> Self {
        Entity(bits)
    }

    #[inline(always)]
    pub const fn is_null(self) -> bool {
        self.0 == Self::NULL.0
    }
}

impl std::fmt::Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "Entity(NULL)")
        } else {
            write!(f, "Entity({}:{})", self.index(), self.generation())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_encoding() {
        let e = Entity::new(42, 5);
        assert_eq!(e.index(), 42);
        assert_eq!(e.generation(), 5);
        assert!(!e.is_null());

        let max_e = Entity::new(MAX_ENTITY_INDEX, ENTITY_GEN_MASK);
        assert_eq!(max_e.index(), MAX_ENTITY_INDEX);
        assert_eq!(max_e.generation(), ENTITY_GEN_MASK);
        assert!(!max_e.is_null());
        assert!(Entity::NULL.is_null());
    }
}
