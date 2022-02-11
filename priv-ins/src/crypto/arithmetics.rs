use crate::crypto::{Comm, Id, sub_id};
use crate::crypto::shares::{Share};
use std::ops::{Mul};
use ff::{PrimeField, Field};


/// Multiply share s1 by constant c
pub fn mul_by_const(s1: &Share, c: &Share) -> Share {
    s1.mul(c)
}

/// todod
pub async fn mul<C: Comm>(g_id: &Id, s1: Share, s2: Share, comm: &mut C) -> Share {
    let (a, b, c) = comm.beaver_for(&g_id).await;

    let e = comm.open(&sub_id(&g_id, &"e".to_string()), s1 - a).await;
    let d = comm.open(&sub_id(&g_id, &"e".to_string()), s2 - b).await;

    c + mul_by_const(&b, &e) + mul_by_const(&a, &d) + e*d
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
}