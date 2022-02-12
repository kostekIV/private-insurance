use crate::{
    crypto::shares::{BeaverShare, Elem, Share},
    protocol::NodeId,
};

pub struct Calculator {
    id: NodeId,
    alpha_share: Elem,
}

impl Calculator {
    pub fn new(id: NodeId, alpha: Elem) -> Self {
        Self {
            id,
            alpha_share: alpha,
        }
    }

    pub fn add(&self, mac_1: Share, mac_2: Share) -> Share {
        (mac_1.0 + mac_2.0, mac_1.1 + mac_2.1)
    }

    pub fn sub(&self, mac_1: Share, mac_2: Share) -> Share {
        (mac_1.0 - mac_2.0, mac_1.1 - mac_2.1)
    }

    pub fn mul_prepare(&self, mac_1: Share, mac_2: Share, beaver: BeaverShare) -> (Share, Share) {
        (self.sub(mac_1, beaver.0), self.sub(mac_2, beaver.1))
    }

    pub fn mul(
        &self,
        mac_1: Share,
        mac_2: Share,
        opened_1: Elem,
        opened_2: Elem,
        beaver_c: Share,
    ) -> Share {
        let mac_1 = self.mul_by_const(mac_1, opened_2);
        let mac_2 = self.mul_by_const(mac_2, opened_1);
        let opened = opened_1 * opened_2;
        let result = self.add(mac_1, mac_2);
        let result = self.add(result, beaver_c);
        self.add_const(result, opened)
    }

    pub fn add_const(&self, mac: Share, share: Elem) -> Share {
        (
            if self.id == 0 { mac.0 + share } else { mac.0 },
            self.alpha_share * share + mac.1,
        )
    }

    pub fn mul_by_const(&self, mac: Share, share: Elem) -> Share {
        (mac.0 * share, mac.1 * share)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::shares::{self, Elems, Shares};
    use ff::{Field, PrimeField};

    fn get_shares(shares: &Elems, alpha: &Elems) -> Shares {
        let secret = shares::sum_elems(shares);
        shares
            .iter()
            .zip(alpha.iter())
            .map(|(b_s, alpha_s)| (*b_s, *alpha_s * secret))
            .collect()
    }

    #[test]
    fn test_add() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let b_shares = shares::shares_from_secret(&b, n_parties);
        let macs_a = get_shares(&a_shares, &alpha_shares);
        let macs_b = get_shares(&b_shares, &alpha_shares);
        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = macs_a
            .iter()
            .zip(macs_b.iter())
            .zip(calculators.iter())
            .map(|((m_a, m_b), c)| c.add(*m_a, *m_b))
            .collect();
        let shared = macs.iter().fold(Elem::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Elem::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a + b);
        assert_eq!(shared_alpha, alpha * (a + b));
    }

    #[test]
    fn test_sub() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let b_shares = shares::shares_from_secret(&b, n_parties);
        let macs_a = get_shares(&a_shares, &alpha_shares);
        let macs_b = get_shares(&b_shares, &alpha_shares);
        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = macs_a
            .iter()
            .zip(macs_b.iter())
            .zip(calculators.iter())
            .map(|((m_a, m_b), c)| c.sub(*m_a, *m_b))
            .collect();
        let shared = macs.iter().fold(Elem::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Elem::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a - b);
        assert_eq!(shared_alpha, alpha * (a - b));
    }

    #[test]
    fn test_add_const() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let macs_a = get_shares(&a_shares, &alpha_shares);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = macs_a
            .iter()
            .zip(calculators.iter())
            .map(|(m_a, c)| c.add_const(*m_a, b))
            .collect();
        let shared = macs.iter().fold(Elem::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Elem::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a + b);
        assert_eq!(shared_alpha, alpha * (a + b));
    }

    #[test]
    fn test_mul_by_const() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let macs_a = get_shares(&a_shares, &alpha_shares);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = macs_a
            .iter()
            .zip(calculators.iter())
            .map(|(m_a, c)| c.mul_by_const(*m_a, b))
            .collect();
        let shared = macs.iter().fold(Elem::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Elem::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a * b);
        assert_eq!(shared_alpha, alpha * (a * b));
    }

    #[test]
    fn test_mul_prepare() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let b_shares = shares::shares_from_secret(&b, n_parties);
        let macs_a = get_shares(&a_shares, &alpha_shares);
        let macs_b = get_shares(&b_shares, &alpha_shares);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();
        let beaver = shares::random_beaver(n_parties);

        let beaver_shares: Vec<_> = (get_shares(&beaver.0, &alpha_shares)
            .iter()
            .zip(get_shares(&beaver.1, &alpha_shares).iter())
            .zip(get_shares(&beaver.2, &alpha_shares).iter())
            .map(|((a, b), c)| (*a, *b, *c)))
        .collect();
        let macs: Vec<_> = macs_a
            .iter()
            .zip(macs_b.iter())
            .zip(calculators.iter())
            .zip(beaver_shares.iter())
            .map(|(((m_a, m_b), c), b)| c.mul_prepare(*m_a, *m_b, *b))
            .collect();

        let shared = macs.iter().fold((Elem::zero(), Elem::zero()), |a, &b| {
            (a.0 + b.0 .0, a.1 + b.1 .0)
        });
        let shared_alpha = macs.iter().fold((Elem::zero(), Elem::zero()), |a, &b| {
            (a.0 + b.0 .1, a.1 + b.1 .1)
        });

        let beaver_a = shares::sum_elems(&beaver.0);
        let beaver_b = shares::sum_elems(&beaver.1);
        assert_eq!(shared.0, a - beaver_a);
        assert_eq!(shared.1, b - beaver_b);

        assert_eq!(shared_alpha.0, alpha * (a - beaver_a));
        assert_eq!(shared_alpha.1, alpha * (b - beaver_b));
    }

    #[test]
    fn test_mul() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();
        let beaver = shares::random_beaver(n_parties);

        let beaver_a = shares::sum_elems(&beaver.0);
        let beaver_b = shares::sum_elems(&beaver.1);

        let macs_a = get_shares(&beaver.0, &alpha_shares);
        let macs_b = get_shares(&beaver.1, &alpha_shares);
        let macs_c: Vec<_> = get_shares(&beaver.2, &alpha_shares);
        let macs: Vec<_> = macs_a
            .iter()
            .zip(macs_b.iter())
            .zip(calculators.iter())
            .zip(macs_c.iter())
            .map(|(((m_a, m_b), c), m_c)| c.mul(*m_a, *m_b, a - beaver_a, b - beaver_b, *m_c))
            .collect();

        let shared = macs.iter().fold(Elem::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Elem::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a * b);
        assert_eq!(shared_alpha, alpha * a * b);
    }
}
