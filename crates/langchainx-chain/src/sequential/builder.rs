//! Builder and convenience macro for sequential chains.
use std::collections::HashSet;

use crate::chain::Chain;

use super::SequentialChain;

/// Collects chains in the order they should execute.
pub struct SequentialChainBuilder {
    chains: Vec<Box<dyn Chain>>,
}

#[allow(clippy::new_without_default)] // Builder pattern
impl SequentialChainBuilder {
    /// Creates an empty sequence.
    pub fn new() -> Self {
        Self { chains: Vec::new() }
    }

    /// Appends a chain to the execution sequence.
    pub fn add_chain<C: Chain + 'static>(mut self, chain: C) -> Self {
        self.chains.push(Box::new(chain));
        self
    }

    /// Builds a sequence and derives its aggregate input and output keys.
    pub fn build(self) -> SequentialChain {
        let outputs: HashSet<String> = self
            .chains
            .iter()
            .flat_map(|c| c.get_output_keys())
            .collect();

        let input_keys: HashSet<String> = self
            .chains
            .iter()
            .flat_map(|c| c.get_input_keys())
            .collect();

        SequentialChain {
            chains: self.chains,
            input_keys,
            outputs,
        }
    }
}

#[macro_export]
/// Builds a [`SequentialChain`] from chains listed in execution order.
macro_rules! sequential_chain {
    ( $( $chain:expr ),* $(,)? ) => {
        {
            let mut builder = $crate::chain::SequentialChainBuilder::new();
            $(
                builder = builder.add_chain($chain);
            )*
            builder.build()
        }
    };
}
