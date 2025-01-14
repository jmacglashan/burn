use crate::stream::{store::OptimizationId, TensorOpsDescription};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

/// Index used to search optimizations.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct OptimizationIndex {
    /// We can't use `HashMap<TensorOpsDescription, Vec<OptimizationId>>` since `TensorOpsDescription`
    /// doesn't implement [`Eq`](core::cmp::Eq).
    ///
    /// `TensorOpsDescription` can't implement `Eq` since float types don't implement it.
    ///
    /// We rely instead on [`PartialEq`](core::cmp::PartialEq) to manually handle hash collisions.
    /// This is OK because we use `relative` streams where any scalar values are set to zeros,
    /// see [`RelativeStreamConverter`](crate::stream::RelativeStreamConverter).
    mapping: HashMap<u64, Vec<(TensorOpsDescription, usize)>>,
    starters: Vec<Vec<OptimizationId>>,
}

pub enum SearchQuery<'a> {
    OptimizationsStartingWith(&'a TensorOpsDescription),
}

pub enum InsertQuery<'a> {
    NewOptimization {
        stream: &'a [TensorOpsDescription],
        id: OptimizationId,
    },
}

impl OptimizationIndex {
    /// Search optimizations with the given [query](SearchQuery).
    pub fn find(&self, query: SearchQuery<'_>) -> Vec<OptimizationId> {
        match query {
            SearchQuery::OptimizationsStartingWith(ops) => self.find_starting_with(ops),
        }
    }

    /// Register a new optimization with the given [query](InsertQuery).
    pub fn insert(&mut self, query: InsertQuery<'_>) {
        match query {
            InsertQuery::NewOptimization { stream, id } => self.insert_new_ops(
                stream
                    .first()
                    .expect("An optimization should never have an empty stream."),
                id,
            ),
        }
    }

    fn find_starting_with(&self, ops: &TensorOpsDescription) -> Vec<OptimizationId> {
        let key = self.stream_key(ops);
        let values = match self.mapping.get(&key) {
            Some(val) => val,
            None => return Vec::new(),
        };

        if values.is_empty() {
            return Vec::new();
        }

        let (_, index) = match values.iter().find(|value| &value.0 == ops) {
            Some(val) => val,
            None => return Vec::new(),
        };

        let val = match self.starters.get(*index) {
            Some(value) => value.clone(),
            None => Vec::new(),
        };

        val
    }

    fn insert_new_ops(&mut self, ops: &TensorOpsDescription, new_id: OptimizationId) {
        let key = self.stream_key(ops);
        let values = match self.mapping.get_mut(&key) {
            Some(val) => val,
            None => {
                // New starter ops.
                let index = self.starters.len();
                self.starters.push(vec![new_id]);
                self.mapping.insert(key, vec![(ops.clone(), index)]);

                return;
            }
        };
        let (_, index) = match values.iter_mut().find(|value| &value.0 == ops) {
            Some(val) => val,
            None => {
                // New with hash collision.
                let index = self.starters.len();
                self.starters.push(vec![new_id]);
                values.push((ops.clone(), index));
                return;
            }
        };

        // New optimization for an existing starter.
        self.starters
            .get_mut(*index)
            .expect("Should exist")
            .push(new_id);
    }

    // Hash the value of the first operation in a stream.
    fn stream_key(&self, ops: &TensorOpsDescription) -> u64 {
        let mut hasher = DefaultHasher::new();
        ops.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stream::{BinaryOpsDescription, NumericOpsDescription, ScalarOpsDescription},
        TensorDescription, TensorId, TensorStatus,
    };

    #[test]
    fn should_find_optimization_id_based_on_tensor_ops() {
        let mut index = OptimizationIndex::default();
        let stream_1 = [ops_1()];
        let optimization_id_1 = 0;

        index.insert(InsertQuery::NewOptimization {
            stream: &stream_1,
            id: optimization_id_1,
        });

        let found = index.find(SearchQuery::OptimizationsStartingWith(&stream_1[0]));

        assert_eq!(found, vec![optimization_id_1]);
    }

    #[test]
    fn should_support_multiple_optimization_ids_with_same_starting_ops() {
        let mut index = OptimizationIndex::default();
        let stream_1 = [ops_1(), ops_2(), ops_1()];
        let stream_2 = [ops_1(), ops_1(), ops_2()];
        let optimization_id_1 = 0;
        let optimization_id_2 = 1;

        index.insert(InsertQuery::NewOptimization {
            stream: &stream_1,
            id: optimization_id_1,
        });
        index.insert(InsertQuery::NewOptimization {
            stream: &stream_2,
            id: optimization_id_2,
        });

        let found = index.find(SearchQuery::OptimizationsStartingWith(&stream_1[0]));

        assert_eq!(found, vec![optimization_id_1, optimization_id_2]);
    }

    #[test]
    fn should_only_find_optimization_with_correct_starting_ops() {
        let mut index = OptimizationIndex::default();
        let stream_1 = [ops_1(), ops_1()];
        let stream_2 = [ops_2(), ops_1()];
        let optimization_id_1 = 0;
        let optimization_id_2 = 1;

        index.insert(InsertQuery::NewOptimization {
            stream: &stream_1,
            id: optimization_id_1,
        });
        index.insert(InsertQuery::NewOptimization {
            stream: &stream_2,
            id: optimization_id_2,
        });

        let found = index.find(SearchQuery::OptimizationsStartingWith(&stream_1[0]));

        assert_eq!(found, vec![optimization_id_1]);
    }

    #[test]
    fn should_handle_hash_collisions() {
        let mut index = OptimizationIndex::default();
        let stream_1 = [ops_1(), ops_1()];
        let stream_2 = [ops_3(), ops_1()];
        let optimization_id_1 = 0;
        let optimization_id_2 = 1;

        let stream_1_key = index.stream_key(&stream_1[0]);
        let stream_2_key = index.stream_key(&stream_2[0]);

        assert_eq!(
            stream_1_key, stream_2_key,
            "Ops 1 and Ops 3 have the same hash"
        );
        assert_ne!(stream_1[0], stream_2[0], "Ops 1 and Ops 3 are different.");

        index.insert(InsertQuery::NewOptimization {
            stream: &stream_1,
            id: optimization_id_1,
        });
        index.insert(InsertQuery::NewOptimization {
            stream: &stream_2,
            id: optimization_id_2,
        });

        let found = index.find(SearchQuery::OptimizationsStartingWith(&stream_1[0]));

        assert_eq!(found, vec![optimization_id_1]);
    }

    fn ops_1() -> TensorOpsDescription {
        TensorOpsDescription::NumericOpsFloat(NumericOpsDescription::Add(BinaryOpsDescription {
            lhs: TensorDescription {
                id: TensorId::new(0),
                shape: vec![32, 32],
                status: TensorStatus::ReadOnly,
            },
            rhs: TensorDescription {
                id: TensorId::new(1),
                shape: vec![32, 32],
                status: TensorStatus::ReadOnly,
            },
            out: TensorDescription {
                id: TensorId::new(2),
                shape: vec![32, 32],
                status: TensorStatus::NotInit,
            },
        }))
    }

    fn ops_2() -> TensorOpsDescription {
        TensorOpsDescription::NumericOpsFloat(NumericOpsDescription::AddScalar(
            ScalarOpsDescription {
                lhs: TensorDescription {
                    id: TensorId::new(0),
                    shape: vec![32, 32],
                    status: TensorStatus::ReadOnly,
                },
                rhs: 5.0,
                out: TensorDescription {
                    id: TensorId::new(2),
                    shape: vec![32, 32],
                    status: TensorStatus::NotInit,
                },
            },
        ))
    }

    fn ops_3() -> TensorOpsDescription {
        TensorOpsDescription::NumericOpsFloat(NumericOpsDescription::Sub(BinaryOpsDescription {
            lhs: TensorDescription {
                id: TensorId::new(0),
                shape: vec![32, 32],
                status: TensorStatus::ReadOnly,
            },
            rhs: TensorDescription {
                id: TensorId::new(1),
                shape: vec![32, 32],
                status: TensorStatus::ReadOnly,
            },
            out: TensorDescription {
                id: TensorId::new(2),
                shape: vec![32, 32],
                status: TensorStatus::NotInit,
            },
        }))
    }
}
