use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;

pub struct InterestEngine;

impl InterestEngine {
    /// Simple interest: I = P * r/100 * d/365
    pub fn calculate_simple_interest(
        principal: Decimal,
        rate: Decimal,
        days: i64,
    ) -> Decimal {
        (principal * rate / Decimal::from(100) * Decimal::from(days) / Decimal::from(365))
            .round_dp(2)
    }

    /// Compound interest: I = P * (1 + r/n)^(n*t) - P
    pub fn calculate_compound_interest(
        principal: Decimal,
        annual_rate: Decimal,
        periods_per_year: u32,
        years: Decimal,
    ) -> Decimal {
        let r = annual_rate / Decimal::from(100) / Decimal::from(periods_per_year);
        let n = years * Decimal::from(periods_per_year);
        let amount = principal * (Decimal::ONE + r).powd(n);
        (amount - principal).round_dp(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interest_1_year() {
        let interest = InterestEngine::calculate_simple_interest(
            Decimal::from(10000),
            Decimal::from(10),
            365,
        );
        assert_eq!(interest, Decimal::from(1000));
    }

    #[test]
    fn test_zero_rate() {
        let interest = InterestEngine::calculate_simple_interest(
            Decimal::from(5000),
            Decimal::ZERO,
            180,
        );
        assert_eq!(interest, Decimal::ZERO);
    }
}
