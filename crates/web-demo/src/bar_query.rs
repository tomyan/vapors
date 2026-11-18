//! `?bar=0`: the page without the demo bar, for a page that embeds one
//! demo in a frame of its own, where the bar's links would switch the
//! demo inside that frame. The canvas then takes the window's height
//! (index.html sizes it below the bar only while the bar is shown).
//! Read once, in `start`.

/// Whether the page shows the demo bar, from every `bar` parameter
/// given: yes with none, no with `bar=0`.
///
/// # Errors
///
/// Another value, or the parameter given more than once, naming the fix.
pub fn bar_shown(values: &[String]) -> Result<bool, String> {
    match values {
        [] => Ok(true),
        [zero] if zero == "0" => Ok(false),
        [other] => Err(format!(
            "bar={other}: give bar=0 to leave the demo bar out, or no bar= to show it"
        )),
        _ => Err(format!(
            "bar given {} times ({}): give ?bar=0 once",
            values.len(),
            values.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::bar_shown;

    fn given(values: &[&str]) -> Result<bool, String> {
        bar_shown(&values.iter().map(ToString::to_string).collect::<Vec<_>>())
    }

    #[test]
    fn the_bar_shows_unless_bar_0_leaves_it_out() {
        assert_eq!(given(&[]), Ok(true));
        assert_eq!(given(&["0"]), Ok(false));
    }

    #[test]
    fn another_value_is_refused_naming_it() {
        let refused = given(&["no"]).expect_err("refused");
        assert!(refused.starts_with("bar=no: give bar=0"), "{refused}");
        assert!(given(&["1"]).is_err(), "only 0 is a value");
    }

    #[test]
    fn bar_given_twice_is_refused_naming_both() {
        assert_eq!(
            given(&["0", "0"]),
            Err("bar given 2 times (0, 0): give ?bar=0 once".to_owned())
        );
    }
}
