//! Functionality for wrapping text into columns.
use crate::core::display_width;
use crate::{wrap, Options};
/// Wrap text into columns with a given total width.
///
/// The `left_gap`, `middle_gap` and `right_gap` arguments specify the
/// strings to insert before, between, and after the columns. The
/// total width of all columns and all gaps is specified using the
/// `total_width_or_options` argument. This argument can simply be an
/// integer if you want to use default settings when wrapping, or it
/// can be a [`Options`] value if you want to customize the wrapping.
///
/// If the columns are narrow, it is recommended to set
/// [`Options::break_words`] to `true` to prevent words from
/// protruding into the margins.
///
/// The per-column width is computed like this:
///
/// ```
/// # let (left_gap, middle_gap, right_gap) = ("", "", "");
/// # let columns = 2;
/// # let options = textwrap::Options::new(80);
/// let inner_width = options.width
///     - textwrap::core::display_width(left_gap)
///     - textwrap::core::display_width(right_gap)
///     - textwrap::core::display_width(middle_gap) * (columns - 1);
/// let column_width = inner_width / columns;
/// ```
///
/// The `text` is wrapped using [`wrap()`] and the given `options`
/// argument, but the width is overwritten to the computed
/// `column_width`.
///
/// # Panics
///
/// Panics if `columns` is zero.
///
/// # Examples
///
/// ```
/// use textwrap::wrap_columns;
///
/// let text = "\
/// This is an example text, which is wrapped into three columns. \
/// Notice how the final column can be shorter than the others.";
///
/// #[cfg(feature = "smawk")]
/// assert_eq!(wrap_columns(text, 3, 50, "| ", " | ", " |"),
///            vec!["| This is       | into three    | column can be  |",
///                 "| an example    | columns.      | shorter than   |",
///                 "| text, which   | Notice how    | the others.    |",
///                 "| is wrapped    | the final     |                |"]);
///
/// // Without the `smawk` feature, the middle column is a little more uneven:
/// #[cfg(not(feature = "smawk"))]
/// assert_eq!(wrap_columns(text, 3, 50, "| ", " | ", " |"),
///            vec!["| This is an    | three         | column can be  |",
///                 "| example text, | columns.      | shorter than   |",
///                 "| which is      | Notice how    | the others.    |",
///                 "| wrapped into  | the final     |                |"]);
pub fn wrap_columns<'a, Opt>(
    text: &str,
    columns: usize,
    total_width_or_options: Opt,
    left_gap: &str,
    middle_gap: &str,
    right_gap: &str,
) -> Vec<String>
where
    Opt: Into<Options<'a>>,
{
    assert!(columns > 0);
    let mut options: Options = total_width_or_options.into();
    let inner_width = options
        .width
        .saturating_sub(display_width(left_gap))
        .saturating_sub(display_width(right_gap))
        .saturating_sub(display_width(middle_gap) * (columns - 1));
    let column_width = std::cmp::max(inner_width / columns, 1);
    options.width = column_width;
    let last_column_padding = " ".repeat(inner_width % column_width);
    let wrapped_lines = wrap(text, options);
    let lines_per_column = wrapped_lines.len() / columns
        + usize::from(wrapped_lines.len() % columns > 0);
    let mut lines = Vec::new();
    for line_no in 0..lines_per_column {
        let mut line = String::from(left_gap);
        for column_no in 0..columns {
            match wrapped_lines.get(line_no + column_no * lines_per_column) {
                Some(column_line) => {
                    line.push_str(column_line);
                    line.push_str(
                        &" ".repeat(column_width - display_width(column_line)),
                    );
                }
                None => {
                    line.push_str(&" ".repeat(column_width));
                }
            }
            if column_no == columns - 1 {
                line.push_str(&last_column_padding);
            } else {
                line.push_str(middle_gap);
            }
        }
        line.push_str(right_gap);
        lines.push(line);
    }
    lines
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrap_columns_empty_text() {
        assert_eq!(wrap_columns("", 1, 10, "| ", "", " |"), vec!["|        |"]);
    }
    #[test]
    fn wrap_columns_single_column() {
        assert_eq!(
            wrap_columns("Foo", 3, 30, "| ", " | ", " |"),
            vec!["| Foo    |        |          |"]
        );
    }
    #[test]
    fn wrap_columns_uneven_columns() {
        assert_eq!(
            wrap_columns("Foo Bar Baz Quux", 4, 21, "|", "|", "|"),
            vec!["|Foo |Bar |Baz |Quux|"]
        );
        assert_eq!(
            wrap_columns("Foo Bar Baz Quux", 4, 24, "|", "|", "|"),
            vec!["|Foo |Bar |Baz |Quux   |"]
        );
        assert_eq!(
            wrap_columns("Foo Bar Baz Quux", 4, 25, "|", "|", "|"),
            vec!["|Foo  |Bar  |Baz  |Quux |"]
        );
    }
    #[test]
    #[cfg(feature = "unicode-width")]
    fn wrap_columns_with_emojis() {
        assert_eq!(
            wrap_columns("Words and a few emojis 😍 wrapped in ⓶ columns", 2, 30,
            "✨ ", " ⚽ ", " 👀"), vec!["✨ Words      ⚽ wrapped in 👀",
            "✨ and a few  ⚽ ⓶ columns  👀",
            "✨ emojis 😍  ⚽            👀"]
        );
    }
    #[test]
    fn wrap_columns_big_gaps() {
        assert_eq!(
            wrap_columns("xyz", 2, 10, "----> ", " !!! ", " <----"),
            vec!["----> x !!! z <----", "----> y !!!   <----"]
        );
    }
    #[test]
    #[should_panic]
    fn wrap_columns_panic_with_zero_columns() {
        wrap_columns("", 0, 10, "", "", "");
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::{wrap, Options};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5)) = <(&str, usize, usize, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: &str = rug_fuzz_0;
        let p1: usize = rug_fuzz_1;
        let p2: Options = Options::new(rug_fuzz_2);
        let p3: &str = rug_fuzz_3;
        let p4: &str = rug_fuzz_4;
        let p5: &str = rug_fuzz_5;
        wrap_columns(p0, p1, p2, p3, p4, p5);
             }
}
}
}    }
}
