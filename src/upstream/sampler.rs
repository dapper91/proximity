use std::fmt;
use std::iter;
use std::ops;

pub use rand::distributions::WeightedError;
use rand::{distributions::Distribution, SeedableRng};

pub trait Sampler {
    fn sample(&mut self) -> usize;
}

#[derive(Debug)]
pub struct RoundRobinSampler {
    iter: iter::Cycle<ops::Range<usize>>,
}

#[derive(Debug, PartialEq)]
pub enum RoundRobinSamplerError {
    ZeroLength,
}

impl fmt::Display for RoundRobinSamplerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            RoundRobinSamplerError::ZeroLength => "round robin set is of size zero",
        })
    }
}

impl RoundRobinSampler {
    pub fn new(length: usize) -> Result<Self, RoundRobinSamplerError> {
        match length {
            0 => Err(RoundRobinSamplerError::ZeroLength),
            _ => Ok(RoundRobinSampler {
                iter: (0..length).into_iter().cycle(),
            }),
        }
    }
}

impl Sampler for RoundRobinSampler {
    fn sample(&mut self) -> usize {
        self.iter.next().unwrap()
    }
}

#[derive(Debug)]
pub struct WeightedSampler {
    dist: rand::distributions::WeightedIndex<usize>,
    rng: rand::rngs::SmallRng,
}

impl WeightedSampler {
    pub fn new<I>(weights: I) -> Result<Self, rand::distributions::WeightedError>
    where
        I: iter::IntoIterator<Item = usize>,
    {
        let sampler = WeightedSampler {
            dist: rand::distributions::WeightedIndex::new(weights)?,
            rng: rand::rngs::SmallRng::from_entropy(),
        };

        return Ok(sampler);
    }
}

impl Sampler for WeightedSampler {
    fn sample(&mut self) -> usize {
        self.dist.sample(&mut self.rng)
    }
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;

    use super::Sampler;
    use super::{RoundRobinSampler, RoundRobinSamplerError};
    use super::{WeightedError, WeightedSampler};

    #[test]
    fn test_round_robin_sampler_error() {
        let mut rrs = RoundRobinSampler::new(0);
        assert_eq!(rrs.unwrap_err(), RoundRobinSamplerError::ZeroLength);
    }

    #[test]
    fn test_round_robin_sampler() {
        let mut rrs = RoundRobinSampler::new(3).unwrap();

        let mut result = vec![];
        for _ in 0..6 {
            result.push(rrs.sample());
        }
        assert_eq!(result, vec![0, 1, 2, 0, 1, 2])
    }

    #[test]
    fn test_weighted_sampler_error() {
        let weights: Vec<usize> = vec![];
        let mut rrs = WeightedSampler::new(weights);
        assert_eq!(rrs.unwrap_err(), WeightedError::NoItem);
    }

    #[test]
    fn test_weighted_sampler() {
        let weights = vec![1, 2, 1];

        let mut rrs = WeightedSampler::new(weights).unwrap();
        rrs.rng = rand::rngs::SmallRng::seed_from_u64(0);

        let mut result = vec![];
        for _ in 0..10 {
            result.push(rrs.sample());
        }
        assert_eq!(result, vec![1, 1, 2, 1, 2, 2, 1, 1, 1, 2])
    }
}
