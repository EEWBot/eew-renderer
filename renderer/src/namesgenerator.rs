use rand::{Rng, seq::IndexedRandom};

pub fn generate<R: Rng>(rng: &mut R) -> String {
    let left = justnames::LEFT.choose(rng).unwrap();
    let right = justnames::RIGHT.choose(rng).unwrap();
    format!("{left}-{right}")
}
