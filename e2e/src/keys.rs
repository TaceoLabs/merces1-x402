use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::UniformRand;
use rand::{CryptoRng, Rng};

pub struct Keys {
    mpc1: ark_babyjubjub::Fr,
    mpc2: ark_babyjubjub::Fr,
    mpc3: ark_babyjubjub::Fr,
}

pub struct PublicKeys {
    pub mpc_pk1: ark_babyjubjub::EdwardsAffine,
    pub mpc_pk2: ark_babyjubjub::EdwardsAffine,
    pub mpc_pk3: ark_babyjubjub::EdwardsAffine,
}

impl Keys {
    pub fn random<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        Self {
            mpc1: ark_babyjubjub::Fr::rand(rng),
            mpc2: ark_babyjubjub::Fr::rand(rng),
            mpc3: ark_babyjubjub::Fr::rand(rng),
        }
    }

    pub fn pk(sk: ark_babyjubjub::Fr) -> ark_babyjubjub::EdwardsAffine {
        (ark_babyjubjub::EdwardsAffine::generator() * sk).into_affine()
    }

    pub fn mpc_pk1(&self) -> ark_babyjubjub::EdwardsAffine {
        Self::pk(self.mpc1)
    }

    pub fn mpc_pk2(&self) -> ark_babyjubjub::EdwardsAffine {
        Self::pk(self.mpc2)
    }

    pub fn mpc_pk3(&self) -> ark_babyjubjub::EdwardsAffine {
        Self::pk(self.mpc3)
    }

    pub fn public_keys(&self) -> PublicKeys {
        PublicKeys {
            mpc_pk1: self.mpc_pk1(),
            mpc_pk2: self.mpc_pk2(),
            mpc_pk3: self.mpc_pk3(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ark_babyjubjub::Fr> {
        [&self.mpc1, &self.mpc2, &self.mpc3].into_iter()
    }
}

impl From<Keys> for PublicKeys {
    fn from(val: Keys) -> Self {
        val.public_keys()
    }
}

impl From<PublicKeys> for [ark_babyjubjub::EdwardsAffine; 3] {
    fn from(val: PublicKeys) -> Self {
        [val.mpc_pk1, val.mpc_pk2, val.mpc_pk3]
    }
}
