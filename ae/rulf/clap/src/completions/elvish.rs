use std::io::Write;
use app::parser::Parser;
use INTERNAL_ERROR_MSG;
pub struct ElvishGen<'a, 'b>
where
    'a: 'b,
{
    p: &'b Parser<'a, 'b>,
}
impl<'a, 'b> ElvishGen<'a, 'b> {
    pub fn new(p: &'b Parser<'a, 'b>) -> Self {
        ElvishGen { p: p }
    }
    pub fn generate_to<W: Write>(&self, buf: &mut W) {
        let bin_name = self.p.meta.bin_name.as_ref().unwrap();
        let mut names = vec![];
        let subcommands_cases = generate_inner(self.p, "", &mut names);
        let result = format!(
            r#"
edit:completion:arg-completer[{bin_name}] = [@words]{{
    fn spaces [n]{{
        repeat $n ' ' | joins ''
    }}
    fn cand [text desc]{{
        edit:complex-candidate $text &display-suffix=' '(spaces (- 14 (wcswidth $text)))$desc
    }}
    command = '{bin_name}'
    for word $words[1:-1] {{
        if (has-prefix $word '-') {{
            break
        }}
        command = $command';'$word
    }}
    completions = [{subcommands_cases}
    ]
    $completions[$command]
}}
"#,
            bin_name = bin_name, subcommands_cases = subcommands_cases
        );
        w!(buf, result.as_bytes());
    }
}
fn escape_string(string: &str) -> String {
    string.replace("'", "''")
}
fn get_tooltip<T: ToString>(help: Option<&str>, data: T) -> String {
    match help {
        Some(help) => escape_string(help),
        _ => data.to_string(),
    }
}
fn generate_inner<'a, 'b, 'p>(
    p: &'p Parser<'a, 'b>,
    previous_command_name: &str,
    names: &mut Vec<&'p str>,
) -> String {
    debugln!("ElvishGen::generate_inner;");
    let command_name = if previous_command_name.is_empty() {
        p.meta.bin_name.as_ref().expect(INTERNAL_ERROR_MSG).clone()
    } else {
        format!("{};{}", previous_command_name, & p.meta.name)
    };
    let mut completions = String::new();
    let preamble = String::from("\n            cand ");
    for option in p.opts() {
        if let Some(data) = option.s.short {
            let tooltip = get_tooltip(option.b.help, data);
            completions.push_str(&preamble);
            completions.push_str(format!("-{} '{}'", data, tooltip).as_str());
        }
        if let Some(data) = option.s.long {
            let tooltip = get_tooltip(option.b.help, data);
            completions.push_str(&preamble);
            completions.push_str(format!("--{} '{}'", data, tooltip).as_str());
        }
    }
    for flag in p.flags() {
        if let Some(data) = flag.s.short {
            let tooltip = get_tooltip(flag.b.help, data);
            completions.push_str(&preamble);
            completions.push_str(format!("-{} '{}'", data, tooltip).as_str());
        }
        if let Some(data) = flag.s.long {
            let tooltip = get_tooltip(flag.b.help, data);
            completions.push_str(&preamble);
            completions.push_str(format!("--{} '{}'", data, tooltip).as_str());
        }
    }
    for subcommand in &p.subcommands {
        let data = &subcommand.p.meta.name;
        let tooltip = get_tooltip(subcommand.p.meta.about, data);
        completions.push_str(&preamble);
        completions.push_str(format!("{} '{}'", data, tooltip).as_str());
    }
    let mut subcommands_cases = format!(
        r"
        &'{}'= {{{}
        }}", & command_name, completions
    );
    for subcommand in &p.subcommands {
        let subcommand_subcommands_cases = generate_inner(
            &subcommand.p,
            &command_name,
            names,
        );
        subcommands_cases.push_str(&subcommand_subcommands_cases);
    }
    subcommands_cases
}
#[cfg(test)]
mod tests_llm_16_252 {
    use super::*;
    use crate::*;
    #[test]
    fn test_escape_string() {
        let _rug_st_tests_llm_16_252_rrrruuuugggg_test_escape_string = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = "abc";
        let rug_fuzz_2 = "a'b";
        let rug_fuzz_3 = "a'b'c";
        let string = rug_fuzz_0;
        debug_assert_eq!(escape_string(string), "");
        let string = rug_fuzz_1;
        debug_assert_eq!(escape_string(string), "abc");
        let string = rug_fuzz_2;
        debug_assert_eq!(escape_string(string), "a''b");
        let string = rug_fuzz_3;
        debug_assert_eq!(escape_string(string), "a''b''c");
        let _rug_ed_tests_llm_16_252_rrrruuuugggg_test_escape_string = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_253 {
    use super::*;
    use crate::*;
    use crate::crate_name;
    #[test]
    fn test_get_tooltip_with_help() {
        let _rug_st_tests_llm_16_253_rrrruuuugggg_test_get_tooltip_with_help = 0;
        let rug_fuzz_0 = "This is a help message";
        let rug_fuzz_1 = 123;
        let help = Some(rug_fuzz_0);
        let data = rug_fuzz_1;
        let tooltip = get_tooltip(help, data);
        debug_assert_eq!(tooltip, "This is a help message");
        let _rug_ed_tests_llm_16_253_rrrruuuugggg_test_get_tooltip_with_help = 0;
    }
    #[test]
    fn test_get_tooltip_with_data() {
        let _rug_st_tests_llm_16_253_rrrruuuugggg_test_get_tooltip_with_data = 0;
        let rug_fuzz_0 = 123;
        let help = None;
        let data = rug_fuzz_0;
        let tooltip = get_tooltip(help, data);
        debug_assert_eq!(tooltip, "123");
        let _rug_ed_tests_llm_16_253_rrrruuuugggg_test_get_tooltip_with_data = 0;
    }
}
#[cfg(test)]
mod tests_rug_153 {
    use super::*;
    use crate::app::parser::Parser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_153_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "previous_command_name";
        let p0: Parser<'static, 'static> = unimplemented!();
        let p1 = rug_fuzz_0;
        let mut p2: Vec<&'static str> = Vec::new();
        crate::completions::elvish::generate_inner(&p0, p1, &mut p2);
        let _rug_ed_tests_rug_153_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_154 {
    use super::*;
    use crate::app::parser::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_154_rrrruuuugggg_test_rug = 0;
        let mut p0: Parser = Default::default();
        crate::completions::elvish::ElvishGen::new(&p0);
        let _rug_ed_tests_rug_154_rrrruuuugggg_test_rug = 0;
    }
}
