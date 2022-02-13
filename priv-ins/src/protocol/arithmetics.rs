use crate::{
    crypto::shares::{
        self, compute_commitment, random_salt, BeaverShare, CommitmentProof, Elem, Hash, Salt,
        Share,
    },
    ff::Field,
    protocol::NodeId,
};

pub struct Calculator {
    id: NodeId,
    alpha_share: Elem,
}

impl Calculator {
    /// Returns a `Calculator` for Node with `id` and alpha_i = `alpha`
    pub fn new(id: NodeId, alpha: Elem) -> Self {
        Self {
            id,
            alpha_share: alpha,
        }
    }

    /// Returns a sum of two shares [x+y]
    pub fn add(&self, share_1: Share, share_2: Share) -> Share {
        (share_1.0 + share_2.0, share_1.1 + share_2.1)
    }

    /// Returns a substraction of two shares [x-y]
    pub fn sub(&self, share_1: Share, share_2: Share) -> Share {
        (share_1.0 - share_2.0, share_1.1 - share_2.1)
    }

    /// Returns shares [x-a] and [y-b] that need to be opened for multiplication
    pub fn mul_prepare(
        &self,
        share_1: Share,
        share_2: Share,
        beaver: BeaverShare,
    ) -> (Share, Share) {
        (self.sub(share_1, beaver.0), self.sub(share_2, beaver.1))
    }

    /// Multiplies beaver share with opened [x-a] and [y-b]
    pub fn mul(&self, beaver: BeaverShare, opened_1: Elem, opened_2: Elem) -> Share {
        let mac_1 = self.mul_by_const(beaver.0, opened_2);
        let mac_2 = self.mul_by_const(beaver.1, opened_1);
        let opened = opened_1 * opened_2;
        let result = self.add(mac_1, mac_2);
        let result = self.add(result, beaver.2);
        self.add_const(result, opened)
    }

    /// Adds opened a element to share [x]
    pub fn add_const(&self, share: Share, constant: Elem) -> Share {
        (
            if self.id == 0 {
                share.0 + constant
            } else {
                share.0
            },
            self.alpha_share * constant + share.1,
        )
    }

    /// Multiplies share [x] by opened element a
    pub fn mul_by_const(&self, share: Share, constant: Elem) -> Share {
        (share.0 * constant, share.1 * constant)
    }

    /// Generates commitment (hash, salt) pair for `elem` such that hash = H(elem || salt)
    pub fn generate_commitment(elem: &Elem) -> (Hash, Salt) {
        let salt = random_salt();
        (compute_commitment(elem, &salt), salt)
    }

    /// Generates element that we want to commit for partial opening d_i = a_i * x' - m(x)_i
    pub fn generate_commitment_share(&self, opened: Elem, share: Share) -> Elem {
        self.alpha_share * opened - share.1
    }
}

/// Verifies whether vector of CommitmentProof is correct and whether Elem's sum to 0. Returns true if correct
fn verify_commitments(commitments: &Vec<CommitmentProof>) -> bool {
    for (hash, elem, salt) in commitments.iter() {
        if *hash != shares::compute_commitment(elem, salt) {
            return false;
        }
    }
    shares::sum_elems(&commitments.iter().map(|c| c.1).collect()) == Elem::zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::shares::{self, Elems, Shares};
    use ff::Field;

    #[test]
    fn test_add() {
        let alpha = Elem::from(69);
        let a = Elem::from(2137);
        let b = Elem::from(420);
        let n_parties = 5;
        let alpha_shares = shares::elems_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, &alpha_shares, n_parties);
        let b_shares = shares::shares_from_secret(&b, &alpha_shares, n_parties);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = a_shares
            .iter()
            .zip(b_shares.iter())
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
        let alpha_shares = shares::elems_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, &alpha_shares, n_parties);
        let b_shares = shares::shares_from_secret(&b, &alpha_shares, n_parties);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = a_shares
            .iter()
            .zip(b_shares.iter())
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
        let alpha_shares = shares::elems_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, &alpha_shares, n_parties);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = a_shares
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
        let alpha_shares = shares::elems_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, &alpha_shares, n_parties);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();

        let macs: Vec<_> = a_shares
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
        let alpha_shares = shares::elems_from_secret(&alpha, n_parties);
        let a_shares = shares::shares_from_secret(&a, &alpha_shares, n_parties);
        let b_shares = shares::shares_from_secret(&b, &alpha_shares, n_parties);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();
        let beaver_shares = shares::random_beaver(&alpha_shares, n_parties);

        let macs: Vec<_> = a_shares
            .iter()
            .zip(b_shares.iter())
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

        let beaver_a = shares::sum_elems(&beaver_shares.iter().map(|b| b.0 .0).collect());
        let beaver_b = shares::sum_elems(&beaver_shares.iter().map(|b| b.1 .0).collect());

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
        let alpha_shares = shares::elems_from_secret(&alpha, n_parties);

        let calculators: Vec<_> = (0..n_parties)
            .map(|id| Calculator {
                id: id as u64,
                alpha_share: alpha_shares[id as usize],
            })
            .collect();
        let beaver_shares = shares::random_beaver(&alpha_shares, n_parties);

        let beaver_a = shares::sum_elems(&beaver_shares.iter().map(|b| b.0 .0).collect());
        let beaver_b = shares::sum_elems(&beaver_shares.iter().map(|b| b.1 .0).collect());

        let macs: Vec<_> = calculators
            .iter()
            .zip(beaver_shares.iter())
            .map(|(c, beaver)| c.mul(*beaver, a - beaver_a, b - beaver_b))
            .collect();

        let shared = macs.iter().fold(Elem::zero(), |a, &b| a + b.0);
        let shared_alpha = macs.iter().fold(Elem::zero(), |a, &b| a + b.1);

        assert_eq!(shared, a * b);
        assert_eq!(shared_alpha, alpha * a * b);
    }
}
