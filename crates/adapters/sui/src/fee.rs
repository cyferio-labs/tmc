use sov_rollup_interface::services::da::Fee;


#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub struct SuiFee(u64);

impl SuiFee {
    /// Creates a new [`MockFee`] with the given rate.
    pub const fn new(rate: u64) -> Self {
        Self(rate)
    }

    /// Creates a new [`MockFee`] with the zero rate.
    pub const fn zero() -> Self {
        Self(0)
    }
}

impl Fee for SuiFee {
    type FeeRate = u64;

    fn fee_rate(&self) -> Self::FeeRate {
        self.0
    }

    fn set_fee_rate(&mut self, rate: Self::FeeRate) {
        self.0 = rate;
    }

    fn gas_estimate(&self) -> u64 {
        10_000_000u64
    }
}
