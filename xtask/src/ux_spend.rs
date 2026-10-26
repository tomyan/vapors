//! The money side of one billed review request (vapors #69, review-v69
//! H1 and M1). Under the ledger's lock: the day's spend, the estimate's
//! worst case, the exact count (charged at the input price), the
//! counted worst case, and a `sent` line at it, written before curl
//! starts. The lock is released before the request; the `sent` line then
//! holds the day's ceiling until the response's usage settles it. A
//! request that is cut off, dropped or answered with no usage stays
//! counted at its worst case.

use std::path::{Path, PathBuf};

use crate::ux_answer::Usage;
use crate::ux_config::UxConfig;
use crate::ux_estimate::dollars;
use crate::ux_ledger;
use crate::ux_ledger_write::{self as write, Entry, LOCK_WAIT, hold};
use crate::ux_review::Prepared;
use crate::ux_secret::Secret;

/// A request with its worst case booked as `sent`.
pub struct Booked {
    ledger: PathBuf,
    time: String,
    sha: String,
    model: &'static str,
    id: String,
    /// The input tokens the API counted.
    pub counted: u64,
}

impl Booked {
    fn entry(&self) -> Entry<'_> {
        Entry {
            time: &self.time,
            sha: &self.sha,
            model: self.model,
            id: &self.id,
        }
    }
}

/// Count `request` and book it, all under the ledger's lock.
///
/// # Errors
/// The lock, the ledger, the count, or a cap or the day's ceiling
/// refusing; nothing is booked then.
pub fn book(
    ux: &UxConfig,
    prepared: &Prepared,
    secret: &Secret,
    request: &Path,
    sha: &str,
) -> Result<Booked, String> {
    let held = hold(&ux.report_dir, LOCK_WAIT)?;
    let time = ux_ledger::utc(std::time::SystemTime::now());
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    let spent = ux_ledger::spent_on(&ledger, &time[..10])?;
    ux_ledger::within_day(
        spent,
        prepared.estimate.worst_cost(&ux.caps, &ux.prices),
        ux.caps.daily_usd,
    )?;
    let counted = crate::ux_count::exact(ux, secret, request)?;
    let id = format!("{time}-{}", std::process::id());
    let booked = Booked {
        ledger,
        time,
        sha: sha.to_owned(),
        model: ux.model.name,
        id,
        counted,
    };
    let count_cost = dollars(counted, ux.prices.input);
    write::count(&booked.ledger, &booked.entry(), counted, count_cost)?;
    let worst =
        crate::ux_count::within(ux, prepared.estimate.input(), counted, spent + count_cost)?;
    write::sent(&booked.ledger, &booked.entry(), worst)?;
    drop(held);
    Ok(booked)
}

/// Settle `booked` with its response's `usage`; refuse billed input
/// over the count. Returns the cost.
///
/// # Errors
/// The ledger cannot be written, or the bill is over the count.
pub fn settle(ux: &UxConfig, booked: &Booked, usage: Usage) -> Result<f64, String> {
    let cost = usage.cost(&ux.prices);
    write::settled(&booked.ledger, &booked.entry(), usage, cost)?;
    if usage.input > booked.counted {
        return Err(format!(
            "the request was billed {} input tokens, over the {} the API counted before it was sent; the count is not what is billed; fix: check work/count-response.json",
            usage.input, booked.counted
        ));
    }
    Ok(cost)
}
