use std::{fmt, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PtrIdx(pub u32);

// === ValueLattice === //

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum ValueLattice {
    Dead,
    KnownConst(u64),
    KnownLoad(LoadHypothesis),
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LoadHypothesis {
    pub read_src: PtrIdx,
    pub branches: KnownPointeeMap,
}

impl ValueLattice {
    pub fn promote_load_non_monotonic(&self) -> ValueLattice {
        if let ValueLattice::KnownLoad(hypothesis) = self
            && let Some(constant) = hypothesis.branches.get(hypothesis.read_src)
        {
            ValueLattice::KnownConst(constant)
        } else {
            self.clone()
        }
    }

    pub fn map_monotonic(&mut self, mut f: impl FnMut(u64) -> u64) {
        match self {
            ValueLattice::KnownConst(value) => {
                *value = f(*value);
            }
            ValueLattice::KnownLoad(load_hypothesis) => {
                load_hypothesis.branches.map(f);
            }
            ValueLattice::Unknown | ValueLattice::Dead => {
                // (nothing to update)
            }
        }
    }

    pub fn map_pair_monotonic(
        lhs: &ValueLattice,
        rhs: &ValueLattice,
        mut f: impl FnMut(u64, u64) -> u64,
    ) -> ValueLattice {
        match (lhs, rhs) {
            (ValueLattice::Dead, _) | (_, ValueLattice::Dead) => ValueLattice::Dead,
            (ValueLattice::KnownConst(lhs), ValueLattice::KnownConst(rhs)) => {
                ValueLattice::KnownConst(f(*lhs, *rhs))
            }
            (ValueLattice::KnownConst(lhs), ValueLattice::KnownLoad(rhs)) => {
                let mut hypothesis = rhs.clone();
                hypothesis.branches.map(|rhs| f(*lhs, rhs));
                ValueLattice::KnownLoad(hypothesis)
            }
            (ValueLattice::KnownLoad(lhs), ValueLattice::KnownConst(rhs)) => {
                let mut hypothesis = lhs.clone();
                hypothesis.branches.map(|lhs| f(lhs, *rhs));
                ValueLattice::KnownLoad(hypothesis)
            }
            (ValueLattice::KnownLoad(_), ValueLattice::KnownLoad(_))
            | (ValueLattice::Unknown, _)
            | (_, ValueLattice::Unknown) => ValueLattice::Unknown,
        }
    }

    pub fn join_monotonic(&mut self, other: &ValueLattice) {
        match (&mut *self, other) {
            (ValueLattice::KnownConst(lhs), ValueLattice::KnownConst(rhs)) if lhs == rhs => {
                // (no-op)
            }
            (ValueLattice::KnownLoad(lhs), ValueLattice::KnownLoad(rhs))
                if lhs.read_src == rhs.read_src =>
            {
                lhs.branches.join(&rhs.branches);
            }
            (ValueLattice::Dead, other) => {
                *self = other.clone();
            }
            (_, ValueLattice::Dead) => {
                // (no-op)
            }
            (ValueLattice::Unknown, _)
            | (_, ValueLattice::Unknown)
            | (ValueLattice::KnownConst(_), ValueLattice::KnownLoad(_))
            | (ValueLattice::KnownLoad(_), ValueLattice::KnownConst(_))
            | (ValueLattice::KnownConst(_), ValueLattice::KnownConst(_))
            | (ValueLattice::KnownLoad(_), ValueLattice::KnownLoad(_)) => {
                *self = ValueLattice::Unknown;
            }
        }
    }
}

// === EffectLattice === //

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EffectLattice {
    Dead,
    Alive {
        no_alias: NoAliasSet,
        known_pointees: KnownPointeeMap,
    },
}

impl Default for EffectLattice {
    fn default() -> Self {
        Self::Alive {
            no_alias: NoAliasSet::default(),
            known_pointees: KnownPointeeMap::default(),
        }
    }
}

impl EffectLattice {
    pub fn read_monotonic(&self, ptr: PtrIdx) -> ValueLattice {
        match self {
            EffectLattice::Dead => ValueLattice::Dead,
            EffectLattice::Alive {
                no_alias: _,
                known_pointees,
            } => ValueLattice::KnownLoad(LoadHypothesis {
                read_src: ptr,
                branches: known_pointees.clone(),
            }),
        }
    }

    pub fn write_monotonic(&mut self, ptr: PtrIdx, value: &ValueLattice) {
        let EffectLattice::Alive {
            no_alias,
            known_pointees,
        } = self
        else {
            return;
        };

        let value = match value {
            ValueLattice::Dead => {
                return;
            }
            ValueLattice::KnownConst(value) => Some(*value),
            ValueLattice::Unknown | ValueLattice::KnownLoad(_) => None,
        };

        let map = Rc::make_mut(&mut known_pointees.raw);

        map.retain(|&other_ptr, &mut other_value| {
            if no_alias.has(ptr, other_ptr) {
                return true;
            }

            Some(other_value) == value
        });

        if let Some(value) = value {
            map.insert(ptr, value);
        }
    }

    pub fn push_no_alias_monotonic(&mut self, lhs: PtrIdx, rhs: PtrIdx) {
        let EffectLattice::Alive {
            no_alias,
            known_pointees: _,
        } = self
        else {
            return;
        };

        no_alias.add(lhs, rhs);
    }

    pub fn arbitrary_write_monotonic(&mut self) {
        let EffectLattice::Alive {
            no_alias: _,
            known_pointees,
        } = self
        else {
            return;
        };

        known_pointees.clear();
    }

    pub fn join_monotonic(&mut self, other: &Self) {
        match (&mut *self, other) {
            (EffectLattice::Dead, EffectLattice::Dead) => {
                // (no-op)
            }
            (EffectLattice::Dead, alive @ EffectLattice::Alive { .. }) => {
                *self = alive.clone();
            }
            (EffectLattice::Alive { .. }, EffectLattice::Dead) => {
                // (no-op)
            }
            (
                EffectLattice::Alive {
                    no_alias: lhs_no_alias,
                    known_pointees: lhs_known_pointees,
                },
                EffectLattice::Alive {
                    no_alias: rhs_no_alias,
                    known_pointees: rhs_known_pointees,
                },
            ) => {
                lhs_no_alias.join(rhs_no_alias);
                lhs_known_pointees.join(rhs_known_pointees);
            }
        }
    }
}

// === Misc Lattices === //

#[derive(Clone, Eq, PartialEq, Default)]
pub struct NoAliasSet {
    pub pairs: FxHashSet<[PtrIdx; 2]>,
}

impl fmt::Debug for NoAliasSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pairs.fmt(f)
    }
}

impl NoAliasSet {
    pub fn add(&mut self, lhs: PtrIdx, rhs: PtrIdx) {
        let mut pair = [lhs, rhs];
        pair.sort();
        self.pairs.insert(pair);
    }

    pub fn has(&self, lhs: PtrIdx, rhs: PtrIdx) -> bool {
        let mut pair = [lhs, rhs];
        pair.sort();
        self.pairs.contains(&pair)
    }

    pub fn join(&mut self, other: &Self) {
        self.pairs.retain(|pair| other.pairs.contains(pair));
    }
}

#[derive(Clone, Eq, PartialEq, Default)]
pub struct KnownPointeeMap {
    pub raw: Rc<FxHashMap<PtrIdx, u64>>,
}

impl fmt::Debug for KnownPointeeMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.raw.fmt(f)
    }
}

impl KnownPointeeMap {
    pub fn get(&self, idx: PtrIdx) -> Option<u64> {
        self.raw.get(&idx).copied()
    }

    pub fn map(&mut self, mut f: impl FnMut(u64) -> u64) {
        for value in Rc::make_mut(&mut self.raw).values_mut() {
            *value = f(*value);
        }
    }

    pub fn clear(&mut self) {
        if let Some(map) = Rc::get_mut(&mut self.raw) {
            map.clear();
        } else {
            self.raw = Rc::new(FxHashMap::default());
        }
    }

    pub fn join(&mut self, other: &Self) {
        Rc::make_mut(&mut self.raw)
            .retain(|key, value| other.raw.get(key).is_some_and(|other| value == other));
    }
}
