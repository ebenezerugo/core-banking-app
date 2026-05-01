use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use serde::{Deserialize, Serialize};

use domain::loan::RepaymentScheduleType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub installment_number: i32,
    pub due_date: NaiveDate,
    pub principal_due: Decimal,
    pub interest_due: Decimal,
    pub total_due: Decimal,
}

pub struct AmortizationEngine;

impl AmortizationEngine {
    /// Generate a repayment schedule.
    pub fn generate_schedule(
        principal: Decimal,
        annual_rate: Decimal,
        term_months: i32,
        first_payment_date: NaiveDate,
        schedule_type: RepaymentScheduleType,
    ) -> Vec<ScheduleEntry> {
        match schedule_type {
            RepaymentScheduleType::Annuity => {
                Self::annuity_schedule(principal, annual_rate, term_months, first_payment_date)
            }
            RepaymentScheduleType::DecliningBalance => {
                Self::declining_balance_schedule(principal, annual_rate, term_months, first_payment_date)
            }
            RepaymentScheduleType::Flat => {
                Self::flat_rate_schedule(principal, annual_rate, term_months, first_payment_date)
            }
        }
    }

    fn annuity_schedule(
        principal: Decimal,
        annual_rate: Decimal,
        term_months: i32,
        first_payment_date: NaiveDate,
    ) -> Vec<ScheduleEntry> {
        let monthly_rate = annual_rate / Decimal::from(100) / Decimal::from(12);
        let n = Decimal::from(term_months);

        // EMI = P * r * (1+r)^n / ((1+r)^n - 1)
        let emi = if monthly_rate == Decimal::ZERO {
            principal / n
        } else {
            let factor = (Decimal::ONE + monthly_rate).powd(n);
            principal * monthly_rate * factor / (factor - Decimal::ONE)
        };
        let emi = emi.round_dp(2);

        let mut schedule = Vec::with_capacity(term_months as usize);
        let mut outstanding = principal;
        let mut current_date = first_payment_date;

        for i in 1..=term_months {
            let interest = (outstanding * monthly_rate).round_dp(2);
            let principal_payment = if i == term_months {
                outstanding
            } else {
                (emi - interest).round_dp(2)
            };
            let total = (principal_payment + interest).round_dp(2);
            outstanding = (outstanding - principal_payment).max(Decimal::ZERO);

            schedule.push(ScheduleEntry {
                installment_number: i,
                due_date: current_date,
                principal_due: principal_payment,
                interest_due: interest,
                total_due: total,
            });

            current_date = advance_month(current_date);
        }
        schedule
    }

    fn declining_balance_schedule(
        principal: Decimal,
        annual_rate: Decimal,
        term_months: i32,
        first_payment_date: NaiveDate,
    ) -> Vec<ScheduleEntry> {
        let monthly_rate = annual_rate / Decimal::from(100) / Decimal::from(12);
        let base_principal = (principal / Decimal::from(term_months)).round_dp(2);

        let mut schedule = Vec::with_capacity(term_months as usize);
        let mut outstanding = principal;
        let mut current_date = first_payment_date;

        for i in 1..=term_months {
            let interest = (outstanding * monthly_rate).round_dp(2);
            let p = if i == term_months { outstanding } else { base_principal };
            outstanding = (outstanding - p).max(Decimal::ZERO);

            schedule.push(ScheduleEntry {
                installment_number: i,
                due_date: current_date,
                principal_due: p,
                interest_due: interest,
                total_due: (p + interest).round_dp(2),
            });
            current_date = advance_month(current_date);
        }
        schedule
    }

    fn flat_rate_schedule(
        principal: Decimal,
        annual_rate: Decimal,
        term_months: i32,
        first_payment_date: NaiveDate,
    ) -> Vec<ScheduleEntry> {
        let total_interest = principal
            * annual_rate
            / Decimal::from(100)
            * Decimal::from(term_months)
            / Decimal::from(12);
        let monthly_principal = (principal / Decimal::from(term_months)).round_dp(2);
        let monthly_interest = (total_interest / Decimal::from(term_months)).round_dp(2);

        let mut schedule = Vec::with_capacity(term_months as usize);
        let mut current_date = first_payment_date;

        for i in 1..=term_months {
            schedule.push(ScheduleEntry {
                installment_number: i,
                due_date: current_date,
                principal_due: monthly_principal,
                interest_due: monthly_interest,
                total_due: (monthly_principal + monthly_interest).round_dp(2),
            });
            current_date = advance_month(current_date);
        }
        schedule
    }
}

fn advance_month(date: NaiveDate) -> NaiveDate {
    let year = date.year();
    let month = date.month();
    let day = date.day();
    let (new_year, new_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let cap = days_in_month(new_year, new_month);
    NaiveDate::from_ymd_opt(new_year, new_month, day.min(cap)).unwrap()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (y, m) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(y, m, 1)
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as u32
}

use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_annuity_schedule_sum_equals_principal_plus_interest() {
        let principal = Decimal::from(10000);
        let annual_rate = Decimal::from(12);
        let term_months = 12;
        let first_date = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();

        let schedule = AmortizationEngine::generate_schedule(
            principal,
            annual_rate,
            term_months,
            first_date,
            RepaymentScheduleType::Annuity,
        );

        assert_eq!(schedule.len(), term_months as usize);

        let total_principal: Decimal = schedule.iter().map(|e| e.principal_due).sum();
        assert!((total_principal - principal).abs() <= Decimal::new(1, 2));
    }

    #[test]
    fn test_flat_rate_schedule_equal_instalments() {
        let principal = Decimal::from(12000);
        let annual_rate = Decimal::from(10);
        let term_months = 12;
        let first_date = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();

        let schedule = AmortizationEngine::generate_schedule(
            principal,
            annual_rate,
            term_months,
            first_date,
            RepaymentScheduleType::Flat,
        );

        let first_principal = schedule[0].principal_due;
        for entry in &schedule {
            assert_eq!(entry.principal_due, first_principal);
        }
    }

    proptest! {
        #[test]
        fn prop_total_principal_repaid_equals_loan_amount(
            principal_cents in 100_000i64..10_000_000i64,
            rate_bps in 100i64..3000i64,
            term in 6i32..60i32,
        ) {
            let principal = Decimal::from(principal_cents) / Decimal::from(100);
            let rate = Decimal::from(rate_bps) / Decimal::from(100);
            let first_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

            let schedule = AmortizationEngine::generate_schedule(
                principal, rate, term, first_date, RepaymentScheduleType::Annuity,
            );

            let total_principal: Decimal = schedule.iter().map(|e| e.principal_due).sum();
            let tolerance = Decimal::from(term) / Decimal::from(100);
            prop_assert!((total_principal - principal).abs() <= tolerance);
        }
    }
}
