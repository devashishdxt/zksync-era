use zksync_prover_fri_types::circuit_definitions::{
    boojum::{
        field::goldilocks::{GoldilocksExt2, GoldilocksField},
        gadgets::recursion::recursive_tree_hasher::CircuitGoldilocksPoseidon2Sponge,
    },
    circuit_definitions::recursion_layer::ZkSyncRecursionLayerVerificationKey,
    zkevm_circuits::recursion::recursion_tip::input::RecursionTipInstanceWitness,
};

pub struct RecursionTipInput {
    pub recursion_tip_witness: RecursionTipInstanceWitness<
        GoldilocksField,
        CircuitGoldilocksPoseidon2Sponge,
        GoldilocksExt2,
    >,
    pub node_vk: ZkSyncRecursionLayerVerificationKey,
}
