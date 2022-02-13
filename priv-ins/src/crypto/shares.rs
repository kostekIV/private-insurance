use crate::crypto::Fp;
use ff::{Field, PrimeField};
use rand;
use rand::Rng;
use sha3::{Digest, Sha3_256};
use std::ops::Sub;

pub type Elem = Fp;
pub type Elems = Vec<Elem>;
pub type Share = (Elem, Elem);
pub type Shares = Vec<Share>;
pub type BeaverShare = (Share, Share, Share);

pub type Hash = [u8; 32];
pub type Salt = Vec<u8>;
pub type Commitment = Hash;
pub type CommitmentProof = (Commitment, Elem, Salt);

/// Generates a vector Elems for `secret` for `n_parties`
pub fn elems_from_secret(secret: &Elem, n_parties: u8) -> Elems {
    let mut shares = random_shares(n_parties - 1);
    let sum = sum_elems(&shares);

    shares.push(secret.sub(sum));
    shares
}

/// Generates a vector of shares for `secret` with given shares of alpha for `n_parties`
pub fn shares_from_secret(secret: &Elem, alpha: &Elems, n_parties: u8) -> Shares {
    let mut shares = random_shares(n_parties - 1);
    let sum = sum_elems(&shares);

    shares.push(secret.sub(sum));
    shares
        .iter()
        .zip(alpha.iter())
        // Im terrible sorry for this syntax
        .map(|(s, a)| (*s, *a * *secret))
        .collect()
}

/// Generates a random vector of shares for `n_parties`
pub fn random_shares(n_parties: u8) -> Elems {
    let mut shares = vec![];

    for _ in 0..n_parties {
        let si = Elem::random(rand::thread_rng());

        shares.push(si);
    }

    shares
}

/// Generates a random vector of BeaverShare with given shares of alpha for `n_parties`
pub fn random_beaver(alpha: &Elems, n_parties: u8) -> Vec<BeaverShare> {
    let a = Elem::random(rand::thread_rng());
    let b = Elem::random(rand::thread_rng());

    let c = a * b;

    shares_from_secret(&a, alpha, n_parties)
        .iter()
        .zip(shares_from_secret(&b, alpha, n_parties).iter())
        .zip(shares_from_secret(&c, alpha, n_parties).iter())
        .map(|((a, b), c)| (*a, *b, *c))
        .collect()
}

pub fn sum_elems(elems: &Elems) -> Elem {
    elems.iter().fold(Elem::zero(), |a, &b| a + b)
}

/// Computes commitment Hash which is H(elem || salt)
pub fn compute_commitment(elem: &Elem, salt: &Salt) -> Hash {
    hash(&[&elem.to_repr().0[..], salt].concat())
}

pub fn random_salt() -> Salt {
    rand::thread_rng().gen::<[u8; 32]>().to_vec()
}

/// Hashes a &[u8]
pub fn hash(x: &[u8]) -> Hash {
    Sha3_256::digest(x).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_elems_correctly() {
        let n_parties = 100;
        let alpha = elems_from_secret(&Elem::from(420), n_parties);

        assert_eq!(n_parties as usize, alpha.len());
        assert_eq!(Elem::from(420), sum_elems(&alpha));
    }

    #[test]
    fn generate_shares_correctly() {
        let n_parties = 100;
        let alpha = elems_from_secret(&Elem::from(420), n_parties);
        let shares = shares_from_secret(&Elem::from(42), &alpha, n_parties);

        assert_eq!(n_parties as usize, shares.len());
        assert_eq!(
            Elem::from(42),
            sum_elems(&shares.iter().map(|s| s.0).collect())
        );
        assert_eq!(
            Elem::from(42 * 420),
            sum_elems(&shares.iter().map(|s| s.1).collect())
        );
    }

    #[test]
    fn generate_beaver_correctly() {
        let n_parties = 100;
        let alpha = elems_from_secret(&Elem::from(420), n_parties);
        let beaver_shares = random_beaver(&alpha, 100);

        assert_eq!(n_parties as usize, beaver_shares.len());
        assert_eq!(
            sum_elems(&beaver_shares.iter().map(|(a, _, _)| a.0).collect())
                * sum_elems(&beaver_shares.iter().map(|(_, b, _)| b.0).collect()),
            sum_elems(&beaver_shares.iter().map(|(_, _, c)| c.0).collect()),
        );
        assert_eq!(
            sum_elems(&beaver_shares.iter().map(|(a, _, _)| a.1).collect())
                * sum_elems(&beaver_shares.iter().map(|(_, b, _)| b.1).collect()),
            Elem::from(420) * sum_elems(&beaver_shares.iter().map(|(_, _, c)| c.1).collect()),
        );
    }
}
