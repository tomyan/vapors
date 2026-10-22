//! A `calc()` expression back to CSS text.

use super::Calc;
use super::typed::{length, number};

/// Write `c` as `calc()` text.
pub(super) fn write(c: &Calc, out: &mut String) {
    match c {
        Calc::Number(n) => number(*n, out),
        Calc::Length(l) => length(*l, out),
        Calc::Negate(a) => {
            out.push('-');
            write(a, out);
        }
        Calc::Invert(a) => {
            out.push_str("1 / ");
            write(a, out);
        }
        Calc::Sum(terms) => sum(terms, out),
        Calc::Product(factors) => product(factors, out),
        Calc::Min(args) | Calc::Max(args) => {
            out.push_str(if matches!(c, Calc::Min(_)) {
                "min("
            } else {
                "max("
            });
            args_text(args, out);
            out.push(')');
        }
        Calc::Clamp(args) => {
            out.push_str("clamp(");
            args_text(args.as_ref(), out);
            out.push(')');
        }
    }
}

fn args_text(args: &[Calc], out: &mut String) {
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write(a, out);
    }
}

/// `calc(a + b - c)`: a negated term prints as subtraction.
fn sum(terms: &[Calc], out: &mut String) {
    out.push_str("calc(");
    for (i, t) in terms.iter().enumerate() {
        match t {
            Calc::Negate(inner) if i > 0 => {
                out.push_str(" - ");
                write(inner, out);
            }
            _ => {
                if i > 0 {
                    out.push_str(" + ");
                }
                write(t, out);
            }
        }
    }
    out.push(')');
}

/// `calc(a * b)`.
fn product(factors: &[Calc], out: &mut String) {
    out.push_str("calc(");
    for (i, f) in factors.iter().enumerate() {
        if i > 0 {
            out.push_str(" * ");
        }
        write(f, out);
    }
    out.push(')');
}
