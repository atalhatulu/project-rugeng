//! Sistem erişim kümesi (Access / SystemAccess) ve çakışma tespiti.

use rugeng_core::AccessSet;
use std::any::TypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Access {
    Read(TypeId),
    Write(TypeId),
}

#[derive(Debug, Default, Clone)]
pub struct SystemAccess {
    set: AccessSet,
}

impl SystemAccess {
    pub fn new() -> Self {
        Self {
            set: AccessSet::new(),
        }
    }

    pub fn with_read<T: 'static>(mut self) -> Self {
        self.set.add_read::<T>();
        self
    }

    pub fn with_write<T: 'static>(mut self) -> Self {
        self.set.add_write::<T>();
        self
    }

    pub fn read<T: 'static>(&mut self) {
        self.set.add_read::<T>();
    }

    pub fn write<T: 'static>(&mut self) {
        self.set.add_write::<T>();
    }

    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        self.set.conflicts_with(&other.set)
    }

    pub fn access_set(&self) -> &AccessSet {
        &self.set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CompA;
    struct CompB;
    struct CompC;

    #[test]
    fn test_system_access_conflicts() {
        let sys1 = SystemAccess::new()
            .with_write::<CompA>()
            .with_read::<CompB>();
        let sys2 = SystemAccess::new().with_read::<CompA>();
        let sys3 = SystemAccess::new()
            .with_read::<CompB>()
            .with_write::<CompC>();
        let sys4 = SystemAccess::new().with_read::<CompC>();

        // sys1 CompA'ya yazıyor, sys2 CompA'yı okuyor -> çakışmalı
        assert!(sys1.conflicts_with(&sys2));

        // sys1 CompB'yi okuyor, sys3 CompB'yi okuyor -> çakışmaz!
        assert!(!sys1.conflicts_with(&sys3));

        // sys3 CompC'ye yazıyor, sys4 CompC'yi okuyor -> çakışmalı
        assert!(sys3.conflicts_with(&sys4));

        // sys1 ve sys4 tamamen bağımsız -> çakışmaz
        assert!(!sys1.conflicts_with(&sys4));
    }
}
