use polars::prelude::*;
use rhai::plugin::*;

#[export_module]
pub mod module {

    #[rhai_fn(name = "str_len_bytes", global)]
    pub fn len_bytes(e: Expr) -> Expr {
        e.str().len_bytes()
    }

    #[rhai_fn(name = "str_len_chars", global)]
    pub fn len_chars(e: Expr) -> Expr {
        e.str().len_chars()
    }

    #[rhai_fn(name = "str_to_uppercase", global)]
    pub fn to_uppercase(e: Expr) -> Expr {
        e.str().to_uppercase()
    }

    #[rhai_fn(name = "str_to_lowercase", global)]
    pub fn to_lowercase(e: Expr) -> Expr {
        e.str().to_lowercase()
    }

    #[rhai_fn(name = "str_strip_chars", global)]
    pub fn strip_chars(e: Expr, chars: &str) -> Expr {
        e.str().strip_chars(lit(chars))
    }

    #[rhai_fn(name = "str_strip_chars_start", global)]
    pub fn strip_chars_start(e: Expr, chars: &str) -> Expr {
        e.str().strip_chars_start(lit(chars))
    }

    #[rhai_fn(name = "str_strip_chars_end", global)]
    pub fn strip_chars_end(e: Expr, chars: &str) -> Expr {
        e.str().strip_chars_end(lit(chars))
    }

    #[rhai_fn(name = "str_strip_prefix", global)]
    pub fn strip_prefix(e: Expr, prefix: &str) -> Expr {
        e.str().strip_prefix(lit(prefix))
    }

    #[rhai_fn(name = "str_strip_suffix", global)]
    pub fn strip_suffix(e: Expr, suffix: &str) -> Expr {
        e.str().strip_suffix(lit(suffix))
    }

    #[rhai_fn(name = "str_contains", global)]
    pub fn contains(e: Expr, pat: &str, literal: bool) -> Expr {
        e.str().contains(lit(pat), literal)
    }

    #[rhai_fn(name = "str_starts_with", global)]
    pub fn starts_with(e: Expr, prefix: &str) -> Expr {
        e.str().starts_with(lit(prefix))
    }

    #[rhai_fn(name = "str_ends_with", global)]
    pub fn ends_with(e: Expr, suffix: &str) -> Expr {
        e.str().ends_with(lit(suffix))
    }

    #[rhai_fn(name = "str_replace", global)]
    pub fn replace(e: Expr, pat: &str, val: &str, literal: bool) -> Expr {
        e.str().replace(lit(pat), lit(val), literal)
    }

    #[rhai_fn(name = "str_replace_all", global)]
    pub fn replace_all(e: Expr, pat: &str, val: &str, literal: bool) -> Expr {
        e.str().replace_all(lit(pat), lit(val), literal)
    }

    #[rhai_fn(name = "str_slice", global)]
    pub fn slice(e: Expr, offset: i64, length: i64) -> Expr {
        e.str().slice(lit(offset), lit(length))
    }

    #[rhai_fn(name = "str_head", global)]
    pub fn head(e: Expr, n: i64) -> Expr {
        e.str().head(lit(n))
    }

    #[rhai_fn(name = "str_tail", global)]
    pub fn tail(e: Expr, n: i64) -> Expr {
        e.str().tail(lit(n))
    }

    #[rhai_fn(name = "str_reverse", global)]
    pub fn reverse(e: Expr) -> Expr {
        e.str().reverse()
    }

    #[rhai_fn(name = "str_count_matches", global)]
    pub fn count_matches(e: Expr, pat: &str, literal: bool) -> Expr {
        e.str().count_matches(lit(pat), literal)
    }

    #[rhai_fn(name = "str_split", global)]
    pub fn split(e: Expr, by: &str) -> Expr {
        e.str().split(lit(by))
    }

    #[rhai_fn(name = "str_split_exact", global)]
    pub fn split_exact(e: Expr, by: &str, n: i64) -> Expr {
        e.str().split_exact(lit(by), n as usize)
    }

    #[rhai_fn(name = "str_splitn", global)]
    pub fn splitn(e: Expr, by: &str, n: i64) -> Expr {
        e.str().splitn(lit(by), n as usize)
    }

    #[rhai_fn(name = "str_to_integer", global)]
    pub fn to_integer(e: Expr, base: i64) -> Expr {
        e.str().to_integer(lit(base), None, false)
    }

    #[rhai_fn(name = "str_extract", global)]
    pub fn extract(e: Expr, pat: &str, group_index: i64) -> Expr {
        e.str().extract(lit(pat), group_index as usize)
    }

    #[rhai_fn(name = "str_extract_all", global)]
    pub fn extract_all(e: Expr, pat: &str) -> Expr {
        e.str().extract_all(lit(pat))
    }

    #[rhai_fn(name = "str_find", global)]
    pub fn find(e: Expr, pat: &str, literal: bool) -> Expr {
        e.str().find(lit(pat), literal)
    }
}
