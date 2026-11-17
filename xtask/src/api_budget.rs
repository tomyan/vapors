//! A session's budget, counted by the proxy (`docs/plans/api-builder.md`):
//! each response's usage at its model's price, summed. A request is
//! refused once the sum reaches the budget, and so is every request after
//! a successful response whose usage could not be read, as the sum is no
//! longer known. The CLI's `--max-budget-usd` counts too; this count holds
//! whatever the CLI does.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::api_prices::{Price, Usage};

/// The budget and what is spent.
pub struct Budget {
    limit: f64,
    prices: BTreeMap<String, Price>,
    state: Mutex<Spent>,
}

#[derive(Default)]
struct Spent {
    dollars: f64,
    /// Why the sum is no longer known, once it is not.
    unknown: Option<String>,
}

impl Budget {
    /// `limit` dollars at `prices`.
    pub fn new(limit: f64, prices: BTreeMap<String, Price>) -> Self {
        Self {
            limit,
            prices,
            state: Mutex::new(Spent::default()),
        }
    }

    /// `model`'s price, when a request for it may go out.
    ///
    /// # Errors
    ///
    /// The model has no price, the budget is spent, or the sum is not
    /// known; each names the fix.
    pub fn admit(&self, model: &str) -> Result<Price, String> {
        let price = self.prices.get(model).copied().ok_or_else(|| {
            format!(
                "model {model:?} has no price in [api_build.prices] (.config/factory.toml), so \
                 its spend cannot be counted; fix: add its price, or run another model"
            )
        })?;
        self.stopped().map_or(Ok(price), Err)
    }

    /// Why no request may go out now, if none may: the budget spent, or
    /// the sum no longer known.
    pub fn stopped(&self) -> Option<String> {
        let Ok(spent) = self.state.lock() else {
            return Some("the budget's lock was poisoned".to_owned());
        };
        if let Some(why) = &spent.unknown {
            return Some(format!(
                "the spend is no longer known ({why}); the session is stopped"
            ));
        }
        (spent.dollars >= self.limit).then(|| {
            format!(
                "the budget of ${:.4} is spent (${:.4}); the session is stopped",
                self.limit, spent.dollars
            )
        })
    }

    /// Adds a response's `usage` at `price`; `None` (a successful response
    /// with no usage) makes the sum unknown, with `why`.
    pub fn charge(&self, price: Price, usage: Option<Usage>, why: &str) {
        let Ok(mut spent) = self.state.lock() else {
            return;
        };
        match usage {
            Some(usage) => spent.dollars += price.of(usage),
            None => spent.unknown = Some(why.to_owned()),
        }
    }

    /// Dollars spent, and why the sum is unknown if it is.
    pub fn spent(&self) -> (f64, Option<String>) {
        self.state
            .lock()
            .map_or((f64::NAN, Some("lock poisoned".into())), |spent| {
                (spent.dollars, spent.unknown.clone())
            })
    }
}

#[cfg(test)]
#[path = "api_budget_tests.rs"]
mod tests;
