use crate::crypto::Fp;
use ff::{PrimeField, Field};
use std::ops::Sub;
use rand;

pub type Share = Fp;
pub type Shares = Vec<Share>;
pub type BeaverShare = (Share, Share, Share);
pub type Beaver = (Shares, Shares, Shares);


pub fn shares_from_secret(secret: &Fp, n_parties: u8) -> Shares {
    let mut shares = random_shares(n_parties - 1);
    let sum = sum_shares(&shares);

    shares.push(secret.sub(sum));
    shares
}

pub fn random_shares(n_parties: u8) -> Shares {
    let mut shares = vec![];

    for _ in 0..n_parties {
        let si = Share::random(rand::thread_rng());

        shares.push(si);
    }

    shares
}

pub fn random_beaver(n_parties: u8) -> Beaver {
    let a = Share::random(rand::thread_rng());
    let b = Share::random(rand::thread_rng());

    let c = a * b;

    (
        shares_from_secret(&a, n_parties),
        shares_from_secret(&b, n_parties),
        shares_from_secret(&c, n_parties),
    )
}

pub fn sum_shares(shares: &Shares) -> Fp {
    shares.iter().fold(Fp::zero(), |a, &b| a + b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_number(n: u64) -> Share {
        let mut x = Share::zero();

        for _ in 0..n {
            x += Share::one();
        }

        x
    }

    #[test]
    fn generate_shares_correctly() {
        let shares = shares_from_secret(&from_number(10), 100);

        assert_eq!(100, shares.len());
        assert_eq!(from_number(10), sum_shares(&shares))
    }

    #[test]
    fn generate_beaver_correctly() {
        let (a, b, c) = random_beaver(100);

        assert_eq!(sum_shares(&a) * sum_shares(&b), sum_shares(&c));
    }
}


