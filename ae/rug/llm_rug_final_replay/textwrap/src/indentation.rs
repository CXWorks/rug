//! Functions related to adding and removing indentation from lines of
//! text.
//!
//! The functions here can be used to uniformly indent or dedent
//! (unindent) word wrapped lines of text.
/// Indent each line by the given prefix.
///
/// # Examples
///
/// ```
/// use textwrap::indent;
///
/// assert_eq!(indent("First line.\nSecond line.\n", "  "),
///            "  First line.\n  Second line.\n");
/// ```
///
/// When indenting, trailing whitespace is stripped from the prefix.
/// This means that empty lines remain empty afterwards:
///
/// ```
/// use textwrap::indent;
///
/// assert_eq!(indent("First line.\n\n\nSecond line.\n", "  "),
///            "  First line.\n\n\n  Second line.\n");
/// ```
///
/// Notice how `"\n\n\n"` remained as `"\n\n\n"`.
///
/// This feature is useful when you want to indent text and have a
/// space between your prefix and the text. In this case, you _don't_
/// want a trailing space on empty lines:
///
/// ```
/// use textwrap::indent;
///
/// assert_eq!(indent("foo = 123\n\nprint(foo)\n", "# "),
///            "# foo = 123\n#\n# print(foo)\n");
/// ```
///
/// Notice how `"\n\n"` became `"\n#\n"` instead of `"\n# \n"` which
/// would have trailing whitespace.
///
/// Leading and trailing whitespace coming from the text itself is
/// kept unchanged:
///
/// ```
/// use textwrap::indent;
///
/// assert_eq!(indent(" \t  Foo   ", "->"), "-> \t  Foo   ");
/// ```
pub fn indent(s: &str, prefix: &str) -> String {
    let mut result = String::with_capacity(2 * s.len());
    let trimmed_prefix = prefix.trim_end();
    for (idx, line) in s.split_terminator('\n').enumerate() {
        if idx > 0 {
            result.push('\n');
        }
        if line.trim().is_empty() {
            result.push_str(trimmed_prefix);
        } else {
            result.push_str(prefix);
        }
        result.push_str(line);
    }
    if s.ends_with('\n') {
        result.push('\n');
    }
    result
}
/// Removes common leading whitespace from each line.
///
/// This function will look at each non-empty line and determine the
/// maximum amount of whitespace that can be removed from all lines:
///
/// ```
/// use textwrap::dedent;
///
/// assert_eq!(dedent("
///     1st line
///       2nd line
///     3rd line
/// "), "
/// 1st line
///   2nd line
/// 3rd line
/// ");
/// ```
pub fn dedent(s: &str) -> String {
    let mut prefix = "";
    let mut lines = s.lines();
    for line in &mut lines {
        let mut whitespace_idx = line.len();
        for (idx, ch) in line.char_indices() {
            if !ch.is_whitespace() {
                whitespace_idx = idx;
                break;
            }
        }
        if whitespace_idx < line.len() {
            prefix = &line[..whitespace_idx];
            break;
        }
    }
    for line in &mut lines {
        let mut whitespace_idx = line.len();
        for ((idx, a), b) in line.char_indices().zip(prefix.chars()) {
            if a != b {
                whitespace_idx = idx;
                break;
            }
        }
        if whitespace_idx < line.len() && whitespace_idx < prefix.len() {
            prefix = &line[..whitespace_idx];
        }
    }
    let mut result = String::new();
    for line in s.lines() {
        if line.starts_with(prefix) && line.chars().any(|c| !c.is_whitespace()) {
            let (_, tail) = line.split_at(prefix.len());
            result.push_str(tail);
        }
        result.push('\n');
    }
    if result.ends_with('\n') && !s.ends_with('\n') {
        let new_len = result.len() - 1;
        result.truncate(new_len);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indent_empty() {
        assert_eq!(indent("\n", "  "), "\n");
    }
    #[test]
    #[rustfmt::skip]
    fn indent_nonempty() {
        let text = ["  foo\n", "bar\n", "  baz\n"].join("");
        let expected = ["//   foo\n", "// bar\n", "//   baz\n"].join("");
        assert_eq!(indent(& text, "// "), expected);
    }
    #[test]
    #[rustfmt::skip]
    fn indent_empty_line() {
        let text = ["  foo", "bar", "", "  baz"].join("\n");
        let expected = ["//   foo", "// bar", "//", "//   baz"].join("\n");
        assert_eq!(indent(& text, "// "), expected);
    }
    #[test]
    fn dedent_empty() {
        assert_eq!(dedent(""), "");
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_multi_line() {
        let x = ["    foo", "  bar", "    baz"].join("\n");
        let y = ["  foo", "bar", "  baz"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_empty_line() {
        let x = ["    foo", "  bar", "   ", "    baz"].join("\n");
        let y = ["  foo", "bar", "", "  baz"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_blank_line() {
        let x = [
            "      foo",
            "",
            "        bar",
            "          foo",
            "          bar",
            "          baz",
        ]
            .join("\n");
        let y = ["foo", "", "  bar", "    foo", "    bar", "    baz"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_whitespace_line() {
        let x = [
            "      foo",
            " ",
            "        bar",
            "          foo",
            "          bar",
            "          baz",
        ]
            .join("\n");
        let y = ["foo", "", "  bar", "    foo", "    bar", "    baz"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_mixed_whitespace() {
        let x = ["\tfoo", "  bar"].join("\n");
        let y = ["\tfoo", "  bar"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_tabbed_whitespace() {
        let x = ["\t\tfoo", "\t\t\tbar"].join("\n");
        let y = ["foo", "\tbar"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_mixed_tabbed_whitespace() {
        let x = ["\t  \tfoo", "\t  \t\tbar"].join("\n");
        let y = ["foo", "\tbar"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_mixed_tabbed_whitespace2() {
        let x = ["\t  \tfoo", "\t    \tbar"].join("\n");
        let y = ["\tfoo", "  \tbar"].join("\n");
        assert_eq!(dedent(& x), y);
    }
    #[test]
    #[rustfmt::skip]
    fn dedent_preserve_no_terminating_newline() {
        let x = ["  foo", "    bar"].join("\n");
        let y = ["foo", "  bar"].join("\n");
        assert_eq!(dedent(& x), y);
    }
}
#[cfg(test)]
mod tests_rug_29 {
    use super::*;
    use crate::indentation::indent;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(&str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0 = rug_fuzz_0;
        let p1 = rug_fuzz_1;
        indent(&p0, &p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    #[test]
    fn test_dedent() {
        let _rug_st_tests_rug_30_rrrruuuugggg_test_dedent = 0;
        let rug_fuzz_0 = "
            1st line
              2nd line
            3rd line
        ";
        let p0: &str = rug_fuzz_0;
        crate::indentation::dedent(&p0);
        let _rug_ed_tests_rug_30_rrrruuuugggg_test_dedent = 0;
    }
}
