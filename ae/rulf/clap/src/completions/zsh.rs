#[allow(deprecated, unused_imports)]
use std::ascii::AsciiExt;
use std::io::Write;
use app::parser::Parser;
use app::App;
use args::{AnyArg, ArgSettings};
use completions;
use INTERNAL_ERROR_MSG;
pub struct ZshGen<'a, 'b>
where
    'a: 'b,
{
    p: &'b Parser<'a, 'b>,
}
impl<'a, 'b> ZshGen<'a, 'b> {
    pub fn new(p: &'b Parser<'a, 'b>) -> Self {
        debugln!("ZshGen::new;");
        ZshGen { p: p }
    }
    pub fn generate_to<W: Write>(&self, buf: &mut W) {
        debugln!("ZshGen::generate_to;");
        w!(
            buf,
            format!("\
#compdef {name}

autoload -U is-at-least

_{name}() {{
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext=\"$curcontext\" state line
    {initial_args}
    {subcommands}
}}

{subcommand_details}

_{name} \"$@\"",
            name = self.p.meta.bin_name.as_ref().unwrap(), initial_args =
            get_args_of(self.p), subcommands = get_subcommands_of(self.p),
            subcommand_details = subcommand_details(self.p)) .as_bytes()
        );
    }
}
fn subcommand_details(p: &Parser) -> String {
    debugln!("ZshGen::subcommand_details;");
    let mut ret = vec![
        format!("\
(( $+functions[_{bin_name_underscore}_commands] )) ||
_{bin_name_underscore}_commands() {{
    local commands; commands=(
        {subcommands_and_args}
    )
    _describe -t commands '{bin_name} commands' commands \"$@\"
}}",
        bin_name_underscore = p.meta.bin_name.as_ref().unwrap().replace(" ", "__"),
        bin_name = p.meta.bin_name.as_ref().unwrap(), subcommands_and_args =
        subcommands_of(p))
    ];
    let mut all_subcommands = completions::all_subcommands(p);
    all_subcommands.sort();
    all_subcommands.dedup();
    for &(_, ref bin_name) in &all_subcommands {
        debugln!("ZshGen::subcommand_details:iter: bin_name={}", bin_name);
        ret.push(
            format!(
                "\
(( $+functions[_{bin_name_underscore}_commands] )) ||
_{bin_name_underscore}_commands() {{
    local commands; commands=(
        {subcommands_and_args}
    )
    _describe -t commands '{bin_name} commands' commands \"$@\"
}}",
                bin_name_underscore = bin_name.replace(" ", "__"), bin_name = bin_name,
                subcommands_and_args = subcommands_of(parser_of(p, bin_name))
            ),
        );
    }
    ret.join("\n")
}
fn subcommands_of(p: &Parser) -> String {
    debugln!("ZshGen::subcommands_of;");
    let mut ret = vec![];
    fn add_sc(sc: &App, n: &str, ret: &mut Vec<String>) {
        debugln!("ZshGen::add_sc;");
        let s = format!(
            "\"{name}:{help}\" \\", name = n, help = sc.p.meta.about.unwrap_or("")
            .replace("[", "\\[").replace("]", "\\]")
        );
        if !s.is_empty() {
            ret.push(s);
        }
    }
    for sc in p.subcommands() {
        debugln!("ZshGen::subcommands_of:iter: subcommand={}", sc.p.meta.name);
        add_sc(sc, &sc.p.meta.name, &mut ret);
        if let Some(ref v) = sc.p.meta.aliases {
            for alias in v.iter().filter(|&&(_, vis)| vis).map(|&(n, _)| n) {
                add_sc(sc, alias, &mut ret);
            }
        }
    }
    ret.join("\n")
}
fn get_subcommands_of(p: &Parser) -> String {
    debugln!("get_subcommands_of;");
    debugln!("get_subcommands_of: Has subcommands...{:?}", p.has_subcommands());
    if !p.has_subcommands() {
        return String::new();
    }
    let sc_names = completions::subcommands_of(p);
    let mut subcmds = vec![];
    for &(ref name, ref bin_name) in &sc_names {
        let mut v = vec![format!("({})", name)];
        let subcommand_args = get_args_of(parser_of(p, &*bin_name));
        if !subcommand_args.is_empty() {
            v.push(subcommand_args);
        }
        let subcommands = get_subcommands_of(parser_of(p, &*bin_name));
        if !subcommands.is_empty() {
            v.push(subcommands);
        }
        v.push(String::from(";;"));
        subcmds.push(v.join("\n"));
    }
    format!(
        "case $state in
    ({name})
        words=($line[{pos}] \"${{words[@]}}\")
        (( CURRENT += 1 ))
        curcontext=\"${{curcontext%:*:*}}:{name_hyphen}-command-$line[{pos}]:\"
        case $line[{pos}] in
            {subcommands}
        esac
    ;;
esac",
        name = p.meta.name, name_hyphen = p.meta.bin_name.as_ref().unwrap().replace(" ",
        "-"), subcommands = subcmds.join("\n"), pos = p.positionals().len() + 1
    )
}
fn parser_of<'a, 'b>(p: &'b Parser<'a, 'b>, sc: &str) -> &'b Parser<'a, 'b> {
    debugln!("parser_of: sc={}", sc);
    if sc == p.meta.bin_name.as_ref().unwrap_or(&String::new()) {
        return p;
    }
    &p.find_subcommand(sc).expect(INTERNAL_ERROR_MSG).p
}
fn get_args_of(p: &Parser) -> String {
    debugln!("get_args_of;");
    let mut ret = vec![String::from("_arguments \"${_arguments_options[@]}\" \\")];
    let opts = write_opts_of(p);
    let flags = write_flags_of(p);
    let positionals = write_positionals_of(p);
    let sc_or_a = if p.has_subcommands() {
        format!(
            "\":: :_{name}_commands\" \\", name = p.meta.bin_name.as_ref().unwrap()
            .replace(" ", "__")
        )
    } else {
        String::new()
    };
    let sc = if p.has_subcommands() {
        format!("\"*::: :->{name}\" \\", name = p.meta.name)
    } else {
        String::new()
    };
    if !opts.is_empty() {
        ret.push(opts);
    }
    if !flags.is_empty() {
        ret.push(flags);
    }
    if !positionals.is_empty() {
        ret.push(positionals);
    }
    if !sc_or_a.is_empty() {
        ret.push(sc_or_a);
    }
    if !sc.is_empty() {
        ret.push(sc);
    }
    ret.push(String::from("&& ret=0"));
    ret.join("\n")
}
fn escape_help(string: &str) -> String {
    string
        .replace("\\", "\\\\")
        .replace("'", "'\\''")
        .replace("[", "\\[")
        .replace("]", "\\]")
}
fn escape_value(string: &str) -> String {
    string
        .replace("\\", "\\\\")
        .replace("'", "'\\''")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace(" ", "\\ ")
}
fn write_opts_of(p: &Parser) -> String {
    debugln!("write_opts_of;");
    let mut ret = vec![];
    for o in p.opts() {
        debugln!("write_opts_of:iter: o={}", o.name());
        let help = o.help().map_or(String::new(), escape_help);
        let mut conflicts = get_zsh_arg_conflicts!(p, o, INTERNAL_ERROR_MSG);
        conflicts = if conflicts.is_empty() {
            String::new()
        } else {
            format!("({})", conflicts)
        };
        let multiple = if o.is_set(ArgSettings::Multiple) { "*" } else { "" };
        let pv = if let Some(pv_vec) = o.possible_vals() {
            format!(
                ": :({})", pv_vec.iter().map(| v | escape_value(* v)).collect::< Vec <
                String >> ().join(" ")
            )
        } else {
            String::new()
        };
        if let Some(short) = o.short() {
            let s = format!(
                "'{conflicts}{multiple}-{arg}+[{help}]{possible_values}' \\", conflicts =
                conflicts, multiple = multiple, arg = short, possible_values = pv, help =
                help
            );
            debugln!("write_opts_of:iter: Wrote...{}", &* s);
            ret.push(s);
        }
        if let Some(long) = o.long() {
            let l = format!(
                "'{conflicts}{multiple}--{arg}=[{help}]{possible_values}' \\", conflicts
                = conflicts, multiple = multiple, arg = long, possible_values = pv, help
                = help
            );
            debugln!("write_opts_of:iter: Wrote...{}", &* l);
            ret.push(l);
        }
    }
    ret.join("\n")
}
fn write_flags_of(p: &Parser) -> String {
    debugln!("write_flags_of;");
    let mut ret = vec![];
    for f in p.flags() {
        debugln!("write_flags_of:iter: f={}", f.name());
        let help = f.help().map_or(String::new(), escape_help);
        let mut conflicts = get_zsh_arg_conflicts!(p, f, INTERNAL_ERROR_MSG);
        conflicts = if conflicts.is_empty() {
            String::new()
        } else {
            format!("({})", conflicts)
        };
        let multiple = if f.is_set(ArgSettings::Multiple) { "*" } else { "" };
        if let Some(short) = f.short() {
            let s = format!(
                "'{conflicts}{multiple}-{arg}[{help}]' \\", multiple = multiple,
                conflicts = conflicts, arg = short, help = help
            );
            debugln!("write_flags_of:iter: Wrote...{}", &* s);
            ret.push(s);
        }
        if let Some(long) = f.long() {
            let l = format!(
                "'{conflicts}{multiple}--{arg}[{help}]' \\", conflicts = conflicts,
                multiple = multiple, arg = long, help = help
            );
            debugln!("write_flags_of:iter: Wrote...{}", &* l);
            ret.push(l);
        }
    }
    ret.join("\n")
}
fn write_positionals_of(p: &Parser) -> String {
    debugln!("write_positionals_of;");
    let mut ret = vec![];
    for arg in p.positionals() {
        debugln!("write_positionals_of:iter: arg={}", arg.b.name);
        let a = format!(
            "'{optional}:{name}{help}:{action}' \\", optional = if ! arg.b
            .is_set(ArgSettings::Required) { ":" } else { "" }, name = arg.b.name, help =
            arg.b.help.map_or("".to_owned(), | v | " -- ".to_owned() + v).replace("[",
            "\\[").replace("]", "\\]"), action = arg.possible_vals().map_or("_files"
            .to_owned(), | values | { format!("({})", values.iter().map(| v |
            escape_value(* v)).collect::< Vec < String >> ().join(" ")) })
        );
        debugln!("write_positionals_of:iter: Wrote...{}", a);
        ret.push(a);
    }
    ret.join("\n")
}
#[cfg(test)]
mod tests_llm_16_260 {
    use super::*;
    use crate::*;
    #[test]
    fn test_escape_value() {
        let _rug_st_tests_llm_16_260_rrrruuuugggg_test_escape_value = 0;
        let rug_fuzz_0 = r"foo\bar";
        let rug_fuzz_1 = r"''";
        let rug_fuzz_2 = r"foo(bar)";
        let rug_fuzz_3 = r"foo bar";
        debug_assert_eq!(escape_value(rug_fuzz_0), r"foo\\bar");
        debug_assert_eq!(escape_value(rug_fuzz_1), r"''\\''");
        debug_assert_eq!(escape_value(rug_fuzz_2), r"foo\\(bar\\)");
        debug_assert_eq!(escape_value(rug_fuzz_3), r"foo\\ bar");
        let _rug_ed_tests_llm_16_260_rrrruuuugggg_test_escape_value = 0;
    }
}
#[cfg(test)]
mod tests_rug_165 {
    use super::*;
    use crate::app::parser::Parser;
    use completions::subcommands_of;
    use completions::zsh::{get_args_of, parser_of};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_165_rrrruuuugggg_test_rug = 0;
        let mut p0: Parser<'_, '_> = Parser::default();
        let result = get_subcommands_of(&p0);
        let _rug_ed_tests_rug_165_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_166 {
    use super::*;
    use crate::app::parser::Parser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_166_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let mut p0: Parser = Parser::<'static, 'static>::default();
        let p1: &str = rug_fuzz_0;
        crate::completions::zsh::parser_of(&p0, p1);
        let _rug_ed_tests_rug_166_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_168 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_168_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "Sample data";
        let mut p0: &str = rug_fuzz_0;
        crate::completions::zsh::escape_help(&p0);
        let _rug_ed_tests_rug_168_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_169 {
    use super::*;
    use crate::app::parser::Parser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_169_rrrruuuugggg_test_rug = 0;
        let mut p0: Parser<'_, '_> = unimplemented!();
        completions::zsh::write_opts_of(&p0);
        let _rug_ed_tests_rug_169_rrrruuuugggg_test_rug = 0;
    }
}
