//! A session's budget, counted by the proxy (`docs/plans/api-builder.md`).
//! A request is admitted only if what is spent, plus the worst case of
//! every request still in flight, plus this request's worst case
//! (`api_prices.rs` `Price::worst`), stays within the budget; it is then
//! settled at what its response counted, or at its worst case when the
//! response did not end whole (review of #476-#478, M9). So the spend
//! never passes the budget while no response costs more than its worst
//! case; one that does stops the session.
//!
//! The worst case holds because of what the proxy forwards
//! (`api_request_policy.rs`, review of #476-#478, N1): input is
//! bounded by the body's bytes because every content block forwarded is
//! text, a tool call or result, or thinking, where a token stands for at
//! least one byte; images and documents, whose tokens their bytes do
//! not bound, are refused; the tool-use system prompt, which is not in
//! the body, is added as a constant. Output is bounded by `max_tokens`. The CLI's `--max-budget-usd` counts too; this count holds
//! whatever the CLI does.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::api_prices::Price;
use crate::api_request_policy::Demand;
use crate::api_usage::Counted;

/// The budget and what is spent and reserved.
pub struct Budget {
    limit: f64,
    prices: BTreeMap<String, Price>,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    spent: f64,
    /// The worst cases of the requests in flight.
    reserved: f64,
    /// Why no more requests may go out, once that is so.
    stopped: Option<String>,
}

/// An admitted request's reservation, settled once by [`Budget::settle`].
#[must_use]
#[derive(Debug)]
pub struct Ticket {
    worst: f64,
}

impl Budget {
    /// `limit` dollars at `prices`.
    ///
    /// # Errors
    ///
    /// A limit that is not a finite number above zero (review of
    /// #476-#478, L1).
    pub fn new(limit: f64, prices: BTreeMap<String, Price>) -> Result<Self, String> {
        if !limit.is_finite() || limit <= 0.0 {
            return Err(format!(
                "the budget {limit} is not a dollar amount above zero; fix: give one, as 2.50"
            ));
        }
        Ok(Self {
            limit,
            prices,
            state: Mutex::new(State::default()),
        })
    }

    /// `model`'s price.
    ///
    /// # Errors
    ///
    /// The model has no price.
    pub fn price(&self, model: &str) -> Result<Price, String> {
        self.prices.get(model).copied().ok_or_else(|| {
            format!(
                "model {model:?} has no price in [api_build.prices] (.config/factory.toml), so \
                 its spend cannot be counted; fix: add its price, or run another model"
            )
        })
    }

    /// `demand` admitted, its worst case reserved.
    ///
    /// # Errors
    ///
    /// No price for its model, no more requests allowed, or its worst
    /// case would take the spend past the budget, each named.
    pub fn admit(&self, demand: &Demand) -> Result<Ticket, String> {
        let worst = self.price(&demand.model)?.worst(demand);
        let mut state = self
            .state
            .lock()
            .map_err(|_| "the budget's lock was poisoned".to_owned())?;
        if let Some(why) = &state.stopped {
            return Err(why.clone());
        }
        if state.spent + state.reserved + worst > self.limit {
            return Err(format!(
                "this request could take the spend past the budget of ${:.4}: ${:.4} spent, \
                 ${:.4} the worst case of those in flight, ${worst:.4} its own",
                self.limit, state.spent, state.reserved
            ));
        }
        state.reserved += worst;
        Ok(Ticket { worst })
    }

    /// `ticket` settled at what its response counted, priced by the model
    /// the response says billed it (review of #476-#478, L2); at its worst
    /// case when the response did not end whole or reported what the
    /// prices cannot express, which also stops the session.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "taking the ticket settles it once: it cannot be settled again"
    )]
    pub fn settle(&self, ticket: Ticket, counted: &Counted) {
        let Ticket { worst } = ticket;
        let (cost, stop) = match counted {
            Counted::Whole(usage, model) => match self.price(model) {
                Ok(price) if price.of(*usage) <= worst => (price.of(*usage), None),
                Ok(price) => (
                    price.of(*usage),
                    Some(format!(
                        "a response cost ${:.4}, more than its worst case",
                        price.of(*usage)
                    )),
                ),
                Err(why) => (worst, Some(why)),
            },
            // An error is charged here as cut: only the proxy, which saw
            // the status, settles one the API answered whole at zero.
            Counted::Cut(_) | Counted::Error(_) => (worst, None),
            Counted::Unpriced(why) => (worst, Some(why.clone())),
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.reserved -= worst;
        state.spent += cost;
        if let Some(why) = stop {
            state
                .stopped
                .get_or_insert(format!("the session is stopped: {why}"));
        }
    }

    /// No more requests admitted: the session has ended.
    pub fn close(&self, why: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.stopped.get_or_insert_with(|| why.to_owned());
        }
    }

    /// Why no request may go out now, if none may.
    pub fn stopped(&self) -> Option<String> {
        let Ok(state) = self.state.lock() else {
            return Some("the budget's lock was poisoned".to_owned());
        };
        state.stopped.clone()
    }

    /// Dollars spent, and the worst case of the requests still in flight.
    pub fn spent(&self) -> (f64, f64) {
        self.state
            .lock()
            .map_or((f64::NAN, f64::NAN), |state| (state.spent, state.reserved))
    }
}

#[cfg(test)]
#[path = "api_budget_tests.rs"]
mod tests;
