//! The money side of review requests (vapors #69: review-v69 H1 and M1,
//! review-v69b N1). Every request that may be billed, an `input_tokens`
//! count as much as a review, is booked as `sent` at its worst case
//! before it is made: under the ledger's lock, against the day's ceiling.
//! The lock is let go before the request, so no request is made under
//! it; the `sent` line then holds the ceiling until a `count` or
//! `settled` line with its id closes it. A request that is cut off,
//! dropped or refused stays counted at its worst case.

use std::path::{Path, PathBuf};

use crate::ux_answer::Usage;
use crate::ux_config::UxConfig;
use crate::ux_estimate::dollars;
use crate::ux_ledger;
use crate::ux_ledger_write::{self as write, Entry, LOCK_WAIT, hold};
use crate::ux_review::Prepared;
use crate::ux_secret::Secret;

/// One booked request's ledger entry.
pub struct Booked {
    ledger: PathBuf,
    time: String,
    sha: String,
    model: &'static str,
    id: String,
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

/// Whether the day has room for `amount` more, read under the lock;
/// nothing is booked.
///
/// # Errors
/// The lock, the ledger, or the ceiling.
pub fn room(ux: &UxConfig, amount: f64) -> Result<(), String> {
    let _held = hold(&ux.report_dir, LOCK_WAIT)?;
    let today = ux_ledger::utc(std::time::SystemTime::now());
    let spent = ux_ledger::spent_on(&ux.report_dir.join(ux_ledger::FILE), &today[..10])?;
    ux_ledger::within_day(spent, amount, ux.caps.daily_usd)
}

/// Book a request at `worst`, under the lock, refusing it when the day's
/// spend plus `worst` plus `also` (what the caller will book next) would
/// pass the ceiling.
///
/// # Errors
/// The lock, the ledger, or the ceiling.
pub fn book(ux: &UxConfig, sha: &str, worst: f64, also: f64) -> Result<Booked, String> {
    let held = hold(&ux.report_dir, LOCK_WAIT)?;
    let time = ux_ledger::utc(std::time::SystemTime::now());
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    ux_ledger::within_day(
        ux_ledger::spent_on(&ledger, &time[..10])?,
        worst + also,
        ux.caps.daily_usd,
    )?;
    let id = format!("{time}-{}-{}", std::process::id(), next_id());
    let booked = Booked {
        ledger,
        time,
        sha: sha.to_owned(),
        model: ux.model.name,
        id,
    };
    write::sent(&booked.ledger, &booked.entry(), worst)?;
    drop(held);
    Ok(booked)
}

/// A count of `bound` tokens at most, booked before `count` makes it and
/// closed by its `count` line at what it counted.
///
/// # Errors
/// The booking, or the count.
pub fn count(
    ux: &UxConfig,
    sha: &str,
    bound: u64,
    also: f64,
    count: impl FnOnce() -> Result<u64, String>,
) -> Result<u64, String> {
    let booked = book(ux, sha, dollars(bound, ux.prices.input), also)?;
    let counted = count()?;
    write::count(
        &booked.ledger,
        &booked.entry(),
        counted,
        dollars(counted, ux.prices.input),
    )?;
    Ok(counted)
}

/// Count a review's `request`, then book it at its counted worst case.
/// The count is booked at the estimate's input, and the ceiling is first
/// checked against that plus the estimate's worst case.
///
/// # Errors
/// A booking, the count, or a cap.
pub fn book_review(
    ux: &UxConfig,
    prepared: &Prepared,
    secret: &Secret,
    request: &Path,
    sha: &str,
) -> Result<(Booked, u64), String> {
    let estimate = prepared.estimate.input();
    let also = prepared.estimate.worst_cost(&ux.caps, &ux.prices);
    let counted = count(ux, sha, estimate, also, || {
        crate::ux_count::exact(ux, secret, request)
    })?;
    let worst = crate::ux_count::within(ux, estimate, counted)?;
    Ok((book(ux, sha, worst, 0.0)?, counted))
}

/// Settle `booked` with its response's `usage`; refuse billed input
/// over the `counted` tokens. Returns the cost.
///
/// # Errors
/// The ledger cannot be written, or the bill is over the count.
pub fn settle(ux: &UxConfig, booked: &Booked, usage: Usage, counted: u64) -> Result<f64, String> {
    let cost = usage.cost(&ux.prices);
    write::settled(&booked.ledger, &booked.entry(), usage, cost)?;
    if usage.input > counted {
        return Err(format!(
            "the request was billed {} input tokens, over the {counted} the API counted before it was sent; the count is not what is billed; fix: check work/count-response.json",
            usage.input
        ));
    }
    Ok(cost)
}

/// A number for each booking in this process, so two bookings in the
/// same second have different ids.
fn next_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
