use ark_bn254::Bn254;
use ark_ff::UniformRand;
use eyre::Context;
use groth16_material::circom::{CircomGroth16Material, Proof};
use rand::{CryptoRng, Rng};
use std::{array, collections::HashMap};

pub struct TransferCompressed {
    amount: ark_bn254::Fr,
    amount_r: ark_bn254::Fr,
    encrypt_sk: ark_babyjubjub::Fr,
    mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
    share_amount: [ark_bn254::Fr; 2],
    share_amount_r: [ark_bn254::Fr; 2],
    alpha: ark_bn254::Fr,
}

impl TransferCompressed {
    #[cfg(test)]
    fn random<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        let amount = ark_bn254::Fr::from(rng.r#gen::<u64>());
        let mpc_pks = array::from_fn(|_| ark_babyjubjub::EdwardsAffine::rand(rng));

        Self::new(amount, mpc_pks, rng)
    }

    pub fn new<R: Rng + CryptoRng>(
        amount: ark_bn254::Fr,
        mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
        rng: &mut R,
    ) -> Self {
        let amount_r = ark_bn254::Fr::rand(rng);
        let encrypt_sk = ark_babyjubjub::Fr::rand(rng);
        let share_amount = array::from_fn(|_| ark_bn254::Fr::rand(rng));
        let share_amount_r = array::from_fn(|_| ark_bn254::Fr::rand(rng));

        Self {
            amount,
            amount_r,
            encrypt_sk,
            mpc_pks,
            share_amount,
            share_amount_r,
            alpha: ark_bn254::Fr::default(),
        }
    }

    // Computes and sets alpha and returns the ciphertexts
    pub fn compute_alpha(&mut self) -> [[ark_bn254::Fr; 2]; 3] {
        let (hash_inputs, ciphertexts) = self.get_sha_inputs();
        self.alpha = crate::compute_alpha(hash_inputs);
        ciphertexts
    }

    pub fn generate_proof<R: Rng + CryptoRng>(
        &self,
        groth16: &CircomGroth16Material,
        rng: &mut R,
    ) -> eyre::Result<(Proof<Bn254>, Vec<ark_bn254::Fr>)> {
        let mut inputs = HashMap::new();
        inputs.insert("amount".to_string(), vec![self.amount.into()]);
        inputs.insert("amount_r".to_string(), vec![self.amount_r.into()]);
        inputs.insert("encrypt_sk".to_string(), vec![self.encrypt_sk.into()]);
        let mpc_pks = self
            .mpc_pks
            .iter()
            .flat_map(|pk| vec![pk.x.into(), pk.y.into()])
            .collect();
        inputs.insert("mpc_pks".to_string(), mpc_pks);
        inputs.insert(
            "share_amount".to_string(),
            vec![self.share_amount[0].into(), self.share_amount[1].into()],
        );
        inputs.insert(
            "share_amount_r".to_string(),
            vec![self.share_amount_r[0].into(), self.share_amount_r[1].into()],
        );
        inputs.insert("alpha".to_string(), vec![self.alpha.into()]);

        groth16
            .generate_proof(&inputs, rng)
            .context("while computing proof")
    }

    pub fn get_amount_commitment(&self) -> ark_bn254::Fr {
        crate::cryptography::commit1(self.amount, self.amount_r)
    }

    pub fn get_encrypt_pk(&self) -> ark_babyjubjub::EdwardsAffine {
        crate::cryptography::get_pk(self.encrypt_sk)
    }

    pub fn get_ciphertextexts(&self) -> [[ark_bn254::Fr; 2]; 3] {
        let amount_shares = [
            self.share_amount[0],
            self.share_amount[1],
            ark_bn254::Fr::from(self.amount) - self.share_amount[0] - self.share_amount[1],
        ];
        let amount_r_shares = [
            self.share_amount_r[0],
            self.share_amount_r[1],
            self.amount_r - self.share_amount_r[0] - self.share_amount_r[1],
        ];

        array::from_fn(|i| {
            let sk = crate::cryptography::dh_key_derivation(&self.encrypt_sk, self.mpc_pks[i]);
            crate::cryptography::sym_encrypt2(
                sk,
                [amount_shares[i], amount_r_shares[i]],
                Default::default(),
            )
        })
    }

    pub fn get_sha_inputs(&self) -> (Vec<ark_bn254::Fr>, [[ark_bn254::Fr; 2]; 3]) {
        let encrypt_pk = self.get_encrypt_pk();
        let amount_commitment = self.get_amount_commitment();
        let ciphertexts = self.get_ciphertextexts();

        let mut hash_inputs = Vec::with_capacity(15);
        hash_inputs.push(encrypt_pk.x);
        hash_inputs.push(encrypt_pk.y);
        hash_inputs.push(amount_commitment);
        for ctxt in ciphertexts.iter() {
            hash_inputs.push(ctxt[0]);
            hash_inputs.push(ctxt[1]);
        }
        for pk in self.mpc_pks.iter() {
            hash_inputs.push(pk.x);
            hash_inputs.push(pk.y);
        }
        (hash_inputs, ciphertexts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circom::config::CircomConfig;
    use rand::thread_rng;

    #[test]
    fn test_transfer_proof_generation() {
        let mut rng = thread_rng();

        let groth16 = CircomConfig::get_transfer_key_material(&mut rng).unwrap();

        let mut transfer = TransferCompressed::random(&mut rng);
        transfer.compute_alpha();

        let (proof, public_inputs) = transfer.generate_proof(&groth16, &mut rng).unwrap();

        groth16
            .verify_proof(&proof, &public_inputs)
            .expect("Proof verification failed");
    }
}
