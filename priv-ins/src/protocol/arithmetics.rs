use crate::{
    crypto::shares::{BeaverShare, Mac, Share},
    protocol::NodeId,
};

pub struct Calculator {
    id: NodeId,
    alpha_share: Share,
}

impl Calculator {
    pub fn add(&self, mac_1: Mac, mac_2: Mac) -> Mac {
        (mac_1.0 + mac_2.0, mac_1.1 + mac_2.1)
    }

    pub fn sub(&self, mac_1: Mac, mac_2: Mac) -> Mac {
        (mac_1.0 - mac_2.0, mac_1.1 - mac_2.1)
    }

    pub fn mul_prepare(&self, mac_1: Mac, mac_2: Mac, beaver: BeaverShare) -> (Mac, Mac) {
        (self.sub(mac_1, beaver.0), self.sub(mac_2, beaver.1))
    }

    pub fn mul(
        &self,
        mac_1: Mac,
        mac_2: Mac,
        opened_1: Share,
        opened_2: Share,
        beaver_c: Mac,
    ) -> Mac {
        let mac_1 = self.mul_by_const(mac_1, opened_2);
        let mac_2 = self.mul_by_const(mac_2, opened_1);
        let opened = opened_1 * opened_2;
        let result = self.add(mac_1, mac_2);
        let result = self.add(result, beaver_c);
        self.add_const(result, opened)
    }

    pub fn add_const(&self, mac: Mac, share: Share) -> Mac {
        (
            if self.id == 0 { mac.0 + share } else { mac.0 },
            self.alpha_share * share + mac.1,
        )
    }

    pub fn mul_by_const(&self, mac: Mac, share: Share) -> Mac {
        (mac.0 * share, mac.1 * share)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::shares::{self, Share};
    use ff::{Field, PrimeField};

    fn get_mac(shares: &Vec<Share>, alpha: &Vec<Share>) -> Vec<Mac> {
        let secret = shares::sum_shares(shares);
        shares
            .iter()
            .zip(alpha.iter())
            .map(|(b_s, alpha_s)| (*b_s, *alpha_s * secret))
            .collect()
    }

    #[test]
    fn test_add() {
        let alpha = Share::from(69);
        let a = Share::from(2137);
        let b = Share::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let b_shares = shares::shares_from_secret(&b, n_parties);
        let macs_a = get_mac(&a_shares, &alpha_shares);
        let macs_b = get_mac(&b_shares, &alpha_shares);
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
        let shared = macs.iter().fold(Share::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Share::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a + b);
        assert_eq!(shared_alpha, alpha * (a + b));
    }

    #[test]
    fn test_sub() {
        let alpha = Share::from(69);
        let a = Share::from(2137);
        let b = Share::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let b_shares = shares::shares_from_secret(&b, n_parties);
        let macs_a = get_mac(&a_shares, &alpha_shares);
        let macs_b = get_mac(&b_shares, &alpha_shares);
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
        let shared = macs.iter().fold(Share::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Share::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a - b);
        assert_eq!(shared_alpha, alpha * (a - b));
    }

    #[test]
    fn test_add_const() {
        let alpha = Share::from(69);
        let a = Share::from(2137);
        let b = Share::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let macs_a = get_mac(&a_shares, &alpha_shares);

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
        let shared = macs.iter().fold(Share::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Share::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a + b);
        assert_eq!(shared_alpha, alpha * (a + b));
    }

    #[test]
    fn test_mul_by_const() {
        let alpha = Share::from(69);
        let a = Share::from(2137);
        let b = Share::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let macs_a = get_mac(&a_shares, &alpha_shares);

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
        let shared = macs.iter().fold(Share::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Share::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a * b);
        assert_eq!(shared_alpha, alpha * (a * b));
    }

    #[test]
    fn test_mul_prepare() {
        let alpha = Share::from(69);
        let a = Share::from(2137);
        let b = Share::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, n_parties);
        let b_shares = shares::shares_from_secret(&b, n_parties);
        let macs_a = get_mac(&a_shares, &alpha_shares);
        let macs_b = get_mac(&b_shares, &alpha_shares);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();
        let beaver = shares::random_beaver(n_parties);

        let beaver_shares: Vec<_> = (get_mac(&beaver.0, &alpha_shares)
            .iter()
            .zip(get_mac(&beaver.1, &alpha_shares).iter())
            .zip(get_mac(&beaver.2, &alpha_shares).iter())
            .map(|((a, b), c)| (*a, *b, *c)))
        .collect();
        let macs: Vec<_> = macs_a
            .iter()
            .zip(macs_b.iter())
            .zip(calculators.iter())
            .zip(beaver_shares.iter())
            .map(|(((m_a, m_b), c), b)| c.mul_prepare(*m_a, *m_b, *b))
            .collect();

        let shared = macs.iter().fold((Share::zero(), Share::zero()), |a, &b| {
            (a.0 + b.0 .0, a.1 + b.1 .0)
        });
        let shared_alpha = macs.iter().fold((Share::zero(), Share::zero()), |a, &b| {
            (a.0 + b.0 .1, a.1 + b.1 .1)
        });

        let beaver_a = shares::sum_shares(&beaver.0);
        let beaver_b = shares::sum_shares(&beaver.1);
        assert_eq!(shared.0, a - beaver_a);
        assert_eq!(shared.1, b - beaver_b);

        assert_eq!(shared_alpha.0, alpha * (a - beaver_a));
        assert_eq!(shared_alpha.1, alpha * (b - beaver_b));
    }

    #[test]
    fn test_mul() {
        let alpha = Share::from(69);
        let a = Share::from(2137);
        let b = Share::from(420);
        let n_parties = 5;
        let alpha_shares = shares::shares_from_secret(&alpha, n_parties);
        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();
        let beaver = shares::random_beaver(n_parties);

        let beaver_a = shares::sum_shares(&beaver.0);
        let beaver_b = shares::sum_shares(&beaver.1);

        let macs_a = get_mac(&beaver.0, &alpha_shares);
        let macs_b = get_mac(&beaver.1, &alpha_shares);
        let macs_c: Vec<_> = get_mac(&beaver.2, &alpha_shares);
        let macs: Vec<_> = macs_a
            .iter()
            .zip(macs_b.iter())
            .zip(calculators.iter())
            .zip(macs_c.iter())
            .map(|(((m_a, m_b), c), m_c)| c.mul(*m_a, *m_b, a - beaver_a, b - beaver_b, *m_c))
            .collect();

        let shared = macs.iter().fold(Share::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Share::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a * b);
        assert_eq!(shared_alpha, alpha * a * b);
    }
}
