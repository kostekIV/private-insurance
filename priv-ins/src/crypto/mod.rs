use crate::crypto::shares::{BeaverShare, Share};
use async_trait::async_trait;

#[derive(PrimeField)]
#[PrimeFieldModulus = "52435875175126190479447740508185965837690552500527637822603658699938581184513"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
pub struct Fp([u64; 4]);


mod shares;
mod arithmetics;

pub type Id = String;

pub fn sub_id(id: &Id, name: &Id) -> Id {
    format!("{}-{}", id, name)
}

#[async_trait]
pub trait Comm {
    /// Opens value under `id`, waits for every share to be delivered and returns discovered value
    async fn open(&mut self, id: &Id, value: Share) -> Share;
    /// Retrieve beaver share for `id`.
    async fn beaver_for(&mut self, id: &Id) -> BeaverShare;
}