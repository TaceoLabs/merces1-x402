use ark_bn254::Bn254;
use ark_groth16::{Proof, VerifyingKey};
use circom_mpc_vm::{ComponentAcceleratorOutput, Rep3VmType, mpc_vm::Rep3WitnessExtension};
use circom_proof_schema::proof_schema::CircomProofSchema;
use co_circom::{
    CircomReduction, CoCircomCompilerParsed, Rep3CoGroth16, Rep3SharedWitness, VMConfig,
};
use eyre::Context;
use mpc_net::Network;
use std::{collections::BTreeMap, time::Instant};

pub struct Groth16Material {
    pub(crate) proof_schema: CircomProofSchema<Bn254>,
    pub(crate) circuit: CoCircomCompilerParsed<ark_bn254::Fr>,
}

impl Groth16Material {
    /// Creates the Groth16 material by providing the actual values.
    pub fn new(
        proof_schema: CircomProofSchema<Bn254>,
        circuit: CoCircomCompilerParsed<ark_bn254::Fr>,
    ) -> Self {
        Self {
            proof_schema,
            circuit,
        }
    }

    pub fn get_vk(&self) -> VerifyingKey<Bn254> {
        self.proof_schema.pk.vk.to_owned()
    }

    pub fn size(&self) -> (usize, usize) {
        self.proof_schema.size()
    }

    pub fn verify(
        &self,
        proof: &Proof<Bn254>,
        public_inputs: &[ark_bn254::Fr],
    ) -> eyre::Result<bool> {
        self.proof_schema.verify(proof, public_inputs)
    }

    pub fn trace_to_witness<N: Network>(
        &self,
        inputs: BTreeMap<String, Rep3VmType<ark_bn254::Fr>>,
        traces: Vec<ComponentAcceleratorOutput<Rep3VmType<ark_bn254::Fr>>>,
        net0: &N,
        net1: &N,
    ) -> eyre::Result<Rep3SharedWitness<ark_bn254::Fr>> {
        let rep3_vm = Rep3WitnessExtension::new(net0, net1, &self.circuit, VMConfig::default())
            .context("while constructing MPC VM")?;

        let mut traces = Some(traces);
        // execute witness generation in MPC
        let witness = rep3_vm
            .run_with_helper_trace(inputs, self.circuit.public_inputs().len(), &mut traces)
            .context("while running witness generation")?
            .into_shared_witness();
        // debug_assert!(traces.unwrap().is_empty(), "Traces were not fully consumed");
        Ok(witness)
    }

    pub fn prove<N: Network>(
        &self,
        witness: Rep3SharedWitness<ark_bn254::Fr>,
        net0: &N,
        net1: &N,
    ) -> eyre::Result<(Proof<Bn254>, Vec<ark_bn254::Fr>)> {
        let public_input = witness.public_inputs[1..].to_vec(); // Skip the constant 1

        let start = Instant::now();
        let proof = Rep3CoGroth16::prove::<N, CircomReduction>(
            net0,
            net1,
            &self.proof_schema.pk,
            &self.proof_schema.matrices,
            witness,
        )?;

        let duration_ms = start.elapsed().as_micros() as f64 / 1000.;
        tracing::info!("Generate proof took {duration_ms} ms");

        Ok((proof, public_input))
    }

    pub fn trace_to_proof<N: Network>(
        &self,
        inputs: BTreeMap<String, Rep3VmType<ark_bn254::Fr>>,
        traces: Vec<ComponentAcceleratorOutput<Rep3VmType<ark_bn254::Fr>>>,
        net0: &N,
        net1: &N,
    ) -> eyre::Result<(Proof<Bn254>, Vec<ark_bn254::Fr>)> {
        let witness = self.trace_to_witness(inputs, traces, net0, net1)?;
        self.prove(witness, net0, net1)
    }
}
