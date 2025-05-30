use winnow::combinator::cut_err;
use winnow::combinator::delimited;
use winnow::combinator::opt;
use winnow::combinator::separated1;
use crate::parser::trivia::ws_comment_newline;
use crate::parser::value::value;
use crate::{Array, Item, RawString, Value};
use crate::parser::prelude::*;
pub(crate) fn array(
    check: RecursionCheck,
) -> impl FnMut(Input<'_>) -> IResult<Input<'_>, Array, ParserError<'_>> {
    move |input| {
        delimited(
                ARRAY_OPEN,
                cut_err(array_values(check)),
                cut_err(ARRAY_CLOSE)
                    .context(Context::Expression("array"))
                    .context(Context::Expected(ParserValue::CharLiteral(']'))),
            )
            .parse_next(input)
    }
}
pub(crate) const ARRAY_OPEN: u8 = b'[';
const ARRAY_CLOSE: u8 = b']';
const ARRAY_SEP: u8 = b',';
pub(crate) fn array_values(
    check: RecursionCheck,
) -> impl FnMut(Input<'_>) -> IResult<Input<'_>, Array, ParserError<'_>> {
    move |input| {
        let check = check.recursing(input)?;
        (
            opt(
                (separated1(array_value(check), ARRAY_SEP), opt(ARRAY_SEP))
                    .map(|(v, trailing): (Vec<Value>, Option<u8>)| {
                        (
                            Array::with_vec(v.into_iter().map(Item::Value).collect()),
                            trailing.is_some(),
                        )
                    }),
            ),
            ws_comment_newline.span(),
        )
            .try_map::<
                _,
                _,
                std::str::Utf8Error,
            >(|(array, trailing)| {
                let (mut array, comma) = array.unwrap_or_default();
                array.set_trailing_comma(comma);
                array.set_trailing(RawString::with_span(trailing));
                Ok(array)
            })
            .parse_next(input)
    }
}
pub(crate) fn array_value(
    check: RecursionCheck,
) -> impl FnMut(Input<'_>) -> IResult<Input<'_>, Value, ParserError<'_>> {
    move |input| {
        (ws_comment_newline.span(), value(check), ws_comment_newline.span())
            .map(|(ws1, v, ws2)| {
                v.decorated(RawString::with_span(ws1), RawString::with_span(ws2))
            })
            .parse_next(input)
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn arrays() {
        let inputs = [
            r#"[]"#,
            r#"[   ]"#,
            r#"[
  1, 2, 3
]"#,
            r#"[
  1,
  2, # this is ok
]"#,
            r#"[# comment
# comment2


   ]"#,
            r#"[# comment
# comment2
      1

#sd
,
# comment3

   ]"#,
            r#"[1]"#,
            r#"[1,]"#,
            r#"[ "all", 'strings', """are the same""", '''type''']"#,
            r#"[ 100, -2,]"#,
            r#"[1, 2, 3]"#,
            r#"[1.1, 2.1, 3.1]"#,
            r#"["a", "b", "c"]"#,
            r#"[ [ 1, 2 ], [3, 4, 5] ]"#,
            r#"[ [ 1, 2 ], ["a", "b", "c"] ]"#,
            r#"[ { x = 1, a = "2" }, {a = "a",b = "b",     c =    "c"} ]"#,
        ];
        for input in inputs {
            dbg!(input);
            let mut parsed = array(Default::default()).parse(new_input(input));
            if let Ok(parsed) = &mut parsed {
                parsed.despan(input);
            }
            assert_eq!(parsed.map(| a | a.to_string()), Ok(input.to_owned()));
        }
    }
    #[test]
    fn invalid_arrays() {
        let invalid_inputs = [r#"["#, r#"[,]"#, r#"[,2]"#, r#"[1e165,,]"#];
        for input in invalid_inputs {
            dbg!(input);
            let mut parsed = array(Default::default()).parse(new_input(input));
            if let Ok(parsed) = &mut parsed {
                parsed.despan(input);
            }
            assert!(parsed.is_err());
        }
    }
}
#[cfg(test)]
mod tests_rug_604 {
    use super::*;
    use crate::parser::prelude::RecursionCheck;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_604_rrrruuuugggg_test_rug = 0;
        let mut p0 = RecursionCheck::default();
        crate::parser::array::array(p0);
        let _rug_ed_tests_rug_604_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_605 {
    use super::*;
    use crate::parser::prelude::RecursionCheck;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_605_rrrruuuugggg_test_rug = 0;
        let mut p0: RecursionCheck = RecursionCheck::default();
        crate::parser::array::array_values(p0);
        let _rug_ed_tests_rug_605_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_606 {
    use super::*;
    use crate::parser::array::array_value;
    use crate::parser::prelude::RecursionCheck;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_606_rrrruuuugggg_test_rug = 0;
        let mut p0: RecursionCheck = RecursionCheck::default();
        array_value(p0);
        let _rug_ed_tests_rug_606_rrrruuuugggg_test_rug = 0;
    }
}
