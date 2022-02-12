use crate::crypto::Fp;
use ff::{Field, PrimeField};
use rand;
use std::ops::Sub;

pub type Elem = Fp;
pub type Elems = Vec<Elem>;
pub type Share = (Elem, Elem);
pub type Shares = Vec<Share>;
pub type BeaverShare = (Share, Share, Share);
pub type Beaver = (Elems, Elems, Elems);

pub fn elems_from_secret(secret: &Elem, n_parties: u8) -> Elems {
    let mut shares = random_shares(n_parties - 1);
    let sum = sum_elems(&shares);

    shares.push(secret.sub(sum));
    shares
}

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

pub fn random_shares(n_parties: u8) -> Elems {
    let mut shares = vec![];

    for _ in 0..n_parties {
        let si = Elem::random(rand::thread_rng());

        shares.push(si);
    }

    shares
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn generate_shares_correctly() {
    //     let shares = shares_from_secret(&Elem::from(10), 100);
    //     assert_eq!(100, shares.len());
    //     assert_eq!(Elem::from(10), sum_shares(&shares))
    // }

    // #[test]
    // fn generate_beaver_correctly() {
    //     let (a, b, c) = random_beaver(100);

    //     assert_eq!(sum_shares(&a) * sum_shares(&b), sum_shares(&c));
    // }
}
