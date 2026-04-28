use ark_ff::UniformRand;
use eyre::Context;
use groth16_material::circom::{CircomGroth16Material, Proof};
use rand::{CryptoRng, Rng};
use std::{array, collections::HashMap};

pub struct Transfer {
    amount: ark_bn254::Fr,
    amount_r: ark_bn254::Fr,
    encrypt_sk: ark_babyjubjub::Fr,
    mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
    share_amount: [ark_bn254::Fr; 2],
    share_amount_r: [ark_bn254::Fr; 2],
}

impl Transfer {
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
        }
    }

    pub fn generate_proof<R: Rng + CryptoRng>(
        &self,
        groth16: &CircomGroth16Material,
        rng: &mut R,
    ) -> eyre::Result<(Proof<ark_bn254::Bn254>, Vec<ark_bn254::Fr>)> {
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
        groth16
            .generate_proof(&inputs, rng)
            .context("while computing proof")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circom::config::CircomConfig;
    use rand::thread_rng;

    #[test]
    #[ignore = "Compression is active, this test will fail"]
    fn test_transfer_proof_generation() {
        let mut rng = thread_rng();

        let groth16 = CircomConfig::get_transfer_key_material(&mut rng).unwrap();

        let transfer = Transfer::random(&mut rng);

        let (proof, public_inputs) = transfer.generate_proof(&groth16, &mut rng).unwrap();

        groth16
            .verify_proof(&proof, &public_inputs)
            .expect("Proof verification failed");

        // Check the public inputs are correct
        let encrypt_pk = crate::cryptography::get_pk(transfer.encrypt_sk);
        let amount_c = crate::cryptography::commit1(transfer.amount, transfer.amount_r);

        let amount_shares = [
            transfer.share_amount[0],
            transfer.share_amount[1],
            ark_bn254::Fr::from(transfer.amount)
                - transfer.share_amount[0]
                - transfer.share_amount[1],
        ];
        let amount_r_shares = [
            transfer.share_amount_r[0],
            transfer.share_amount_r[1],
            transfer.amount_r - transfer.share_amount_r[0] - transfer.share_amount_r[1],
        ];

        let mut ciphertexts = Vec::with_capacity(3);
        for i in 0..3 {
            let sk =
                crate::cryptography::dh_key_derivation(&transfer.encrypt_sk, transfer.mpc_pks[i]);
            let ctxt = crate::cryptography::sym_encrypt2(
                sk,
                [amount_shares[i], amount_r_shares[i]],
                Default::default(),
            );
            ciphertexts.push(ctxt);
        }

        let public_inputs_expected = [
            encrypt_pk.x,
            encrypt_pk.y,
            amount_c,
            ciphertexts[0][0],
            ciphertexts[0][1],
            ciphertexts[1][0],
            ciphertexts[1][1],
            ciphertexts[2][0],
            ciphertexts[2][1],
            transfer.mpc_pks[0].x,
            transfer.mpc_pks[0].y,
            transfer.mpc_pks[1].x,
            transfer.mpc_pks[1].y,
            transfer.mpc_pks[2].x,
            transfer.mpc_pks[2].y,
        ];

        for i in 0..public_inputs_expected.len() {
            assert_eq!(
                public_inputs[i], public_inputs_expected[i],
                "Mismatch at index {}",
                i
            );
        }
        assert_eq!(public_inputs, public_inputs_expected);
    }
}
