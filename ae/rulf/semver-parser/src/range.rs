use crate::*;
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Range {
    pub comparator_set: Vec<Comparator>,
    pub compat: range_set::Compat,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Comparator {
    pub op: Op,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<Identifier>,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Op {
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Identifier {
    Numeric(u64),
    AlphaNumeric(String),
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Partial {
    major: Option<u64>,
    minor: Option<u64>,
    patch: Option<u64>,
    pre: Vec<Identifier>,
    kind: PartialKind,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PartialKind {
    XRangeOnly,
    MajorOnly,
    MajorMinor,
    MajorMinorPatch,
}
impl Partial {
    pub fn new() -> Self {
        Self {
            major: None,
            minor: None,
            patch: None,
            pre: Vec::new(),
            kind: PartialKind::XRangeOnly,
        }
    }
    pub fn as_comparator(&self, op: Op) -> Comparator {
        Comparator {
            op,
            major: self.major.unwrap_or(0),
            minor: self.minor.unwrap_or(0),
            patch: self.patch.unwrap_or(0),
            pre: self.pre.clone(),
        }
    }
    pub fn inc_major(&mut self) -> &mut Self {
        self.major = Some(self.major.unwrap_or(0) + 1);
        self
    }
    pub fn inc_minor(&mut self) -> &mut Self {
        self.minor = Some(self.minor.unwrap_or(0) + 1);
        self
    }
    pub fn inc_patch(&mut self) -> &mut Self {
        self.patch = Some(self.patch.unwrap_or(0) + 1);
        self
    }
    pub fn zero_missing(&mut self) -> &mut Self {
        self.major = Some(self.major.unwrap_or(0));
        self.minor = Some(self.minor.unwrap_or(0));
        self.patch = Some(self.patch.unwrap_or(0));
        self
    }
    pub fn zero_minor(&mut self) -> &mut Self {
        self.minor = Some(0);
        self
    }
    pub fn zero_patch(&mut self) -> &mut Self {
        self.patch = Some(0);
        self
    }
    pub fn no_pre(&mut self) -> &mut Self {
        self.pre = Vec::new();
        self
    }
}
pub fn from_pair_iterator(
    parsed_range: pest::iterators::Pair<'_, Rule>,
    compat: range_set::Compat,
) -> Result<Range, String> {
    if parsed_range.as_rule() != Rule::range {
        return Err(String::from("Error parsing range"));
    }
    let mut comparator_set = Vec::new();
    for record in parsed_range.into_inner() {
        match record.as_rule() {
            Rule::hyphen => {
                let mut hyphen_set = simple::from_hyphen_range(record)?;
                comparator_set.append(&mut hyphen_set);
            }
            Rule::simple => {
                let mut comparators = simple::from_pair_iterator(record, compat)?;
                comparator_set.append(&mut comparators);
            }
            Rule::empty => {
                comparator_set
                    .push(Partial::new().zero_missing().as_comparator(Op::Gte));
            }
            _ => unreachable!(),
        }
    }
    Ok(Range { comparator_set, compat })
}
pub mod simple {
    use super::*;
    pub fn from_pair_iterator(
        parsed_simple: pest::iterators::Pair<'_, Rule>,
        compat: range_set::Compat,
    ) -> Result<Vec<Comparator>, String> {
        if parsed_simple.as_rule() != Rule::simple {
            return Err(String::from("Error parsing comparator set"));
        }
        let mut comparators = Vec::new();
        for record in parsed_simple.into_inner() {
            match record.as_rule() {
                Rule::partial => {
                    let components: Vec<_> = record.into_inner().collect();
                    let mut partial = parse_partial(components);
                    match partial.kind {
                        PartialKind::XRangeOnly => {
                            comparators
                                .push(partial.zero_missing().as_comparator(Op::Gte));
                        }
                        PartialKind::MajorOnly => {
                            comparators
                                .push(
                                    partial.clone().zero_missing().as_comparator(Op::Gte),
                                );
                            comparators
                                .push(
                                    partial.inc_major().zero_missing().as_comparator(Op::Lt),
                                );
                        }
                        PartialKind::MajorMinor => {
                            comparators
                                .push(partial.clone().zero_patch().as_comparator(Op::Gte));
                            comparators
                                .push(
                                    partial.inc_minor().zero_patch().as_comparator(Op::Lt),
                                );
                        }
                        PartialKind::MajorMinorPatch => {
                            match compat {
                                range_set::Compat::Npm => {
                                    comparators.push(partial.as_comparator(Op::Eq));
                                }
                                range_set::Compat::Cargo => {
                                    handle_caret_range(partial, &mut comparators);
                                }
                            }
                        }
                    }
                }
                Rule::primitive => {
                    let mut components: Vec<_> = record.into_inner().collect();
                    let op_component = components.remove(0);
                    let op = match op_component.as_str() {
                        "=" => Op::Eq,
                        "<" => Op::Lt,
                        "<=" => Op::Lte,
                        ">" => Op::Gt,
                        ">=" => Op::Gte,
                        _ => unreachable!(),
                    };
                    let partial_component = components.remove(0);
                    let components: Vec<_> = partial_component.into_inner().collect();
                    let mut partial = parse_partial(components);
                    if op == Op::Eq {
                        match partial.kind {
                            PartialKind::XRangeOnly => {
                                comparators
                                    .push(partial.zero_missing().as_comparator(Op::Gte));
                            }
                            PartialKind::MajorOnly => {
                                comparators
                                    .push(
                                        partial.clone().zero_missing().as_comparator(Op::Gte),
                                    );
                                comparators
                                    .push(
                                        partial.inc_major().zero_missing().as_comparator(Op::Lt),
                                    );
                            }
                            PartialKind::MajorMinor => {
                                comparators
                                    .push(partial.clone().zero_patch().as_comparator(Op::Gte));
                                comparators
                                    .push(
                                        partial.inc_minor().zero_patch().as_comparator(Op::Lt),
                                    );
                            }
                            PartialKind::MajorMinorPatch => {
                                comparators.push(partial.as_comparator(Op::Eq));
                            }
                        }
                    } else {
                        match partial.kind {
                            PartialKind::XRangeOnly => {
                                match op {
                                    Op::Eq => {
                                        comparators
                                            .push(partial.zero_missing().as_comparator(Op::Gte))
                                    }
                                    Op::Lt => {
                                        comparators
                                            .push(partial.zero_missing().as_comparator(Op::Lt))
                                    }
                                    Op::Lte => {
                                        comparators
                                            .push(partial.zero_missing().as_comparator(Op::Gte))
                                    }
                                    Op::Gt => {
                                        comparators
                                            .push(partial.zero_missing().as_comparator(Op::Lt))
                                    }
                                    Op::Gte => {
                                        comparators
                                            .push(partial.zero_missing().as_comparator(Op::Gte))
                                    }
                                }
                            }
                            PartialKind::MajorOnly => {
                                match op {
                                    Op::Lte => {
                                        comparators
                                            .push(
                                                partial
                                                    .inc_major()
                                                    .zero_minor()
                                                    .zero_patch()
                                                    .as_comparator(Op::Lt),
                                            )
                                    }
                                    _ => {
                                        comparators.push(partial.zero_missing().as_comparator(op))
                                    }
                                }
                            }
                            PartialKind::MajorMinor => {
                                match op {
                                    Op::Lte => {
                                        comparators
                                            .push(
                                                partial.inc_minor().zero_patch().as_comparator(Op::Lt),
                                            )
                                    }
                                    _ => {
                                        comparators.push(partial.zero_patch().as_comparator(op))
                                    }
                                }
                            }
                            PartialKind::MajorMinorPatch => {
                                comparators.push(partial.as_comparator(op));
                            }
                        }
                    }
                }
                Rule::caret => {
                    let mut components: Vec<_> = record.into_inner().collect();
                    let partial_component = components.remove(0);
                    let components: Vec<_> = partial_component.into_inner().collect();
                    let partial = parse_partial(components);
                    handle_caret_range(partial, &mut comparators);
                }
                Rule::tilde => {
                    let mut components: Vec<_> = record.into_inner().collect();
                    let partial_component = components.remove(0);
                    let components: Vec<_> = partial_component.into_inner().collect();
                    let mut partial = parse_partial(components);
                    comparators
                        .push(partial.clone().zero_missing().as_comparator(Op::Gte));
                    match partial.kind {
                        PartialKind::XRangeOnly => {}
                        PartialKind::MajorOnly => {
                            comparators
                                .push(
                                    partial
                                        .inc_major()
                                        .zero_missing()
                                        .no_pre()
                                        .as_comparator(Op::Lt),
                                );
                        }
                        PartialKind::MajorMinor | PartialKind::MajorMinorPatch => {
                            comparators
                                .push(
                                    partial
                                        .inc_minor()
                                        .zero_patch()
                                        .no_pre()
                                        .as_comparator(Op::Lt),
                                );
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(comparators)
    }
    fn handle_caret_range(mut partial: Partial, comparators: &mut Vec<Comparator>) {
        if partial.major == Some(0) {
            match partial.kind {
                PartialKind::XRangeOnly => unreachable!(),
                PartialKind::MajorOnly => {
                    comparators
                        .push(partial.clone().zero_missing().as_comparator(Op::Gte));
                    comparators
                        .push(
                            partial
                                .inc_major()
                                .zero_missing()
                                .no_pre()
                                .as_comparator(Op::Lt),
                        );
                }
                PartialKind::MajorMinor => {
                    comparators
                        .push(partial.clone().zero_missing().as_comparator(Op::Gte));
                    comparators
                        .push(
                            partial
                                .inc_minor()
                                .zero_patch()
                                .no_pre()
                                .as_comparator(Op::Lt),
                        );
                }
                PartialKind::MajorMinorPatch => {
                    if partial.minor == Some(0) {
                        comparators.push(partial.as_comparator(Op::Gte));
                        comparators
                            .push(partial.inc_patch().no_pre().as_comparator(Op::Lt));
                    } else {
                        comparators.push(partial.as_comparator(Op::Gte));
                        comparators
                            .push(
                                partial
                                    .inc_minor()
                                    .zero_patch()
                                    .no_pre()
                                    .as_comparator(Op::Lt),
                            );
                    }
                }
            }
        } else {
            match partial.kind {
                PartialKind::XRangeOnly => {
                    comparators.push(partial.zero_missing().as_comparator(Op::Gte));
                }
                _ => {
                    comparators
                        .push(partial.clone().zero_missing().as_comparator(Op::Gte));
                    comparators
                        .push(
                            partial
                                .inc_major()
                                .zero_minor()
                                .zero_patch()
                                .no_pre()
                                .as_comparator(Op::Lt),
                        );
                }
            }
        }
    }
    pub fn from_hyphen_range(
        parsed_simple: pest::iterators::Pair<'_, Rule>,
    ) -> Result<Vec<Comparator>, String> {
        if parsed_simple.as_rule() != Rule::hyphen {
            return Err(String::from("Error parsing comparator set"));
        }
        let mut comparators = Vec::new();
        let mut records = parsed_simple.into_inner();
        let components1: Vec<_> = records.next().unwrap().into_inner().collect();
        let mut partial1 = parse_partial(components1);
        match partial1.kind {
            PartialKind::XRangeOnly => {}
            _ => comparators.push(partial1.zero_missing().as_comparator(Op::Gte)),
        }
        let components2: Vec<_> = records.next().unwrap().into_inner().collect();
        let mut partial2 = parse_partial(components2);
        match partial2.kind {
            PartialKind::XRangeOnly => {
                if partial1.kind == PartialKind::XRangeOnly {
                    comparators.push(partial2.zero_missing().as_comparator(Op::Gte));
                }
            }
            PartialKind::MajorOnly => {
                comparators
                    .push(
                        partial2
                            .inc_major()
                            .zero_minor()
                            .zero_patch()
                            .as_comparator(Op::Lt),
                    );
            }
            PartialKind::MajorMinor => {
                comparators
                    .push(partial2.inc_minor().zero_patch().as_comparator(Op::Lt));
            }
            PartialKind::MajorMinorPatch => {
                comparators.push(partial2.as_comparator(Op::Lte));
            }
        }
        Ok(comparators)
    }
    fn parse_partial(mut components: Vec<pest::iterators::Pair<'_, Rule>>) -> Partial {
        let mut partial = Partial::new();
        let one = components.remove(0);
        match one.as_rule() {
            Rule::xr => {
                let inner = one.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::xr_op => {
                        partial.major = None;
                        partial.kind = PartialKind::XRangeOnly;
                        return partial;
                    }
                    Rule::nr => {
                        partial.major = Some(inner.as_str().parse::<u64>().unwrap());
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
        if components.is_empty() {
            partial.kind = PartialKind::MajorOnly;
            return partial;
        } else {
            let two = components.remove(0);
            match two.as_rule() {
                Rule::xr => {
                    let inner = two.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::xr_op => {
                            partial.minor = None;
                            partial.kind = PartialKind::MajorOnly;
                            return partial;
                        }
                        Rule::nr => {
                            partial.minor = Some(inner.as_str().parse::<u64>().unwrap());
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        if components.is_empty() {
            partial.kind = PartialKind::MajorMinor;
            return partial;
        } else {
            let three = components.remove(0);
            match three.as_rule() {
                Rule::xr => {
                    let inner = three.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::xr_op => {
                            partial.patch = None;
                            partial.kind = PartialKind::MajorMinor;
                            return partial;
                        }
                        Rule::nr => {
                            partial.patch = Some(inner.as_str().parse::<u64>().unwrap());
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        partial.kind = PartialKind::MajorMinorPatch;
        if !components.is_empty() {
            let pre = components.remove(0);
            let mut pre: Vec<_> = pre.into_inner().collect();
            let pre = pre.remove(0);
            let pre = pre.as_str();
            for bit in pre.split('.') {
                let identifier = match bit.parse::<u64>() {
                    Ok(num) => Identifier::Numeric(num),
                    Err(_) => Identifier::AlphaNumeric(bit.to_string()),
                };
                partial.pre.push(identifier);
            }
        }
        partial
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    fn parse_range(input: &str) -> pest::iterators::Pair<'_, Rule> {
        match SemverParser::parse(Rule::range, input) {
            Ok(mut parsed) => {
                match parsed.next() {
                    Some(parsed) => parsed,
                    None => panic!("Could not parse {}", input),
                }
            }
            Err(e) => panic!("Parse error:\n{}", e),
        }
    }
    macro_rules! range_tests {
        ($($name:ident : $value:expr,)*) => {
            $(#[test] fn $name () { let (input, expected_range) = $value; let
            parsed_range = parse_range(input); let range =
            from_pair_iterator(parsed_range, range_set::Compat::Cargo)
            .expect("parsing failed"); let num_comparators = range.comparator_set.len();
            let expected_comparators = expected_range.comparator_set.len();
            assert_eq!(expected_comparators, num_comparators,
            "expected number of comparators: {}, got: {}", expected_comparators,
            num_comparators); assert_eq!(range, expected_range); })*
        };
    }
    macro_rules! range_tests_nodecompat {
        ($($name:ident : $value:expr,)*) => {
            $(#[test] fn $name () { let (input, expected_range) = $value; let
            parsed_range = parse_range(input); let range =
            from_pair_iterator(parsed_range, range_set::Compat::Npm)
            .expect("parsing failed"); let num_comparators = range.comparator_set.len();
            let expected_comparators = expected_range.comparator_set.len();
            assert_eq!(expected_comparators, num_comparators,
            "expected number of comparators: {}, got: {}", expected_comparators,
            num_comparators); assert_eq!(range, expected_range); })*
        };
    }
    macro_rules! comp_sets {
        ($([$op:expr, $major:expr, $minor:expr, $patch:expr]),*) => {
            Range { comparator_set : vec![$(Comparator { op : $op, major : $major, minor
            : $minor, patch : $patch, pre : pre!(None), },)*], compat :
            range_set::Compat::Cargo }
        };
        ($([$op:expr, $major:expr, $minor:expr, $patch:expr, $pre:expr]),*) => {
            Range { comparator_set : vec![$(Comparator { op : $op, major : $major, minor
            : $minor, patch : $patch, pre : $pre, },)*], compat :
            range_set::Compat::Cargo }
        };
    }
    macro_rules! comp_sets_node {
        ($([$op:expr, $major:expr, $minor:expr, $patch:expr]),*) => {
            Range { comparator_set : vec![$(Comparator { op : $op, major : $major, minor
            : $minor, patch : $patch, pre : pre!(None), },)*], compat :
            range_set::Compat::Npm }
        };
    }
    macro_rules! id_num {
        ($num:expr) => {
            Identifier::Numeric($num)
        };
    }
    macro_rules! id_alpha {
        ($alpha:expr) => {
            Identifier::AlphaNumeric(String::from($alpha))
        };
    }
    macro_rules! pre {
        (None) => {
            Vec::new()
        };
        ($($e:expr),*) => {
            vec![$($e,)*]
        };
    }
    macro_rules! op {
        ("=") => {
            Op::Eq
        };
        ("<") => {
            Op::Lt
        };
        ("<=") => {
            Op::Lte
        };
        (">") => {
            Op::Gt
        };
        (">=") => {
            Op::Gte
        };
    }
    range_tests! {
        major : ("1", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])), major_minor
        : ("1.2", comp_sets!([op!(">="), 1, 2, 0], [op!("<"), 1, 3, 0])),
        major_minor_patch : ("1.2.3", comp_sets!([op!(">="), 1, 2, 3], [op!("<"), 2, 0,
        0])), major_0_minor_patch : ("0.2.3", comp_sets!([op!(">="), 0, 2, 3], [op!("<"),
        0, 3, 0])), major_0_minor_0_patch : ("0.0.1", comp_sets!([op!(">="), 0, 0, 1],
        [op!("<"), 0, 0, 2])), eq_major : ("=1", comp_sets!([op!(">="), 1, 0, 0],
        [op!("<"), 2, 0, 0])), eq_major_minor : ("=1.2", comp_sets!([op!(">="), 1, 2, 0],
        [op!("<"), 1, 3, 0])), eq_major_minor_patch : ("=1.2.3", comp_sets!([op!("="), 1,
        2, 3])), eq_all : ("=*", comp_sets!([op!(">="), 0, 0, 0])), eq_major_star :
        ("=1.*", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])),
        eq_major_minor_star : ("=1.2.*", comp_sets!([op!(">="), 1, 2, 0], [op!("<"), 1,
        3, 0])), lt_major : ("<1", comp_sets!([op!("<"), 1, 0, 0])), lt_major_minor :
        ("<1.2", comp_sets!([op!("<"), 1, 2, 0])), lt_major_minor_patch : ("<1.2.3",
        comp_sets!([op!("<"), 1, 2, 3])), lt_all : ("<*", comp_sets!([op!("<"), 0, 0,
        0])), lt_major_star : ("<1.*", comp_sets!([op!("<"), 1, 0, 0])),
        lt_major_minor_star : ("<1.2.*", comp_sets!([op!("<"), 1, 2, 0])), lte_major :
        ("<=1", comp_sets!([op!("<"), 2, 0, 0])), lte_major_minor : ("<=1.2",
        comp_sets!([op!("<"), 1, 3, 0])), lte_major_minor_patch : ("<=1.2.3",
        comp_sets!([op!("<="), 1, 2, 3])), lte_all : ("<=*", comp_sets!([op!(">="), 0, 0,
        0])), lte_major_star : ("<=1.*", comp_sets!([op!("<"), 2, 0, 0])),
        lte_major_minor_star : ("<=1.2.*", comp_sets!([op!("<"), 1, 3, 0])), gt_major :
        (">1", comp_sets!([op!(">"), 1, 0, 0])), gt_major_minor : (">1.2",
        comp_sets!([op!(">"), 1, 2, 0])), gt_major_minor_patch : (">1.2.3",
        comp_sets!([op!(">"), 1, 2, 3])), gt_all : (">*", comp_sets!([op!("<"), 0, 0,
        0])), gt_major_star : (">1.*", comp_sets!([op!(">"), 1, 0, 0])),
        gt_major_minor_star : (">1.2.*", comp_sets!([op!(">"), 1, 2, 0])), gte_major :
        (">=1", comp_sets!([op!(">="), 1, 0, 0])), gte_major_minor : (">=1.2",
        comp_sets!([op!(">="), 1, 2, 0])), gte_major_minor_patch : (">=1.2.3",
        comp_sets!([op!(">="), 1, 2, 3])), gte_all : (">=*", comp_sets!([op!(">="), 0, 0,
        0])), gte_major_star : (">=1.*", comp_sets!([op!(">="), 1, 0, 0])),
        gte_major_minor_star : (">=1.2.*", comp_sets!([op!(">="), 1, 2, 0])), tilde_major
        : ("~1", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])), tilde_major_0 :
        ("~0", comp_sets!([op!(">="), 0, 0, 0], [op!("<"), 1, 0, 0])), tilde_major_xrange
        : ("~1.x", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])), tilde_major_2
        : ("~>1", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])),
        tilde_major_minor : ("~1.2", comp_sets!([op!(">="), 1, 2, 0], [op!("<"), 1, 3,
        0])), tilde_major_minor_xrange : ("~1.2.x", comp_sets!([op!(">="), 1, 2, 0],
        [op!("<"), 1, 3, 0])), tilde_major_minor_2 : ("~>1.2", comp_sets!([op!(">="), 1,
        2, 0], [op!("<"), 1, 3, 0])), tilde_major_minor_patch : ("~1.2.3",
        comp_sets!([op!(">="), 1, 2, 3], [op!("<"), 1, 3, 0])),
        tilde_major_minor_patch_pre : ("~1.2.3-beta", comp_sets!([op!(">="), 1, 2, 3,
        pre!(id_alpha!("beta"))], [op!("<"), 1, 3, 0, pre!()])),
        tilde_major_minor_patch_2 : ("~>1.2.3", comp_sets!([op!(">="), 1, 2, 3],
        [op!("<"), 1, 3, 0])), tilde_major_0_minor_patch : ("~0.2.3",
        comp_sets!([op!(">="), 0, 2, 3], [op!("<"), 0, 3, 0])), tilde_all : ("~*",
        comp_sets!([op!(">="), 0, 0, 0])), caret_major : ("^1", comp_sets!([op!(">="), 1,
        0, 0], [op!("<"), 2, 0, 0])), caret_major_xrange : ("^1.x",
        comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])), caret_major_minor :
        ("^1.2", comp_sets!([op!(">="), 1, 2, 0], [op!("<"), 2, 0, 0])),
        caret_major_minor_xrange : ("^1.2.x", comp_sets!([op!(">="), 1, 2, 0], [op!("<"),
        2, 0, 0])), caret_major_minor_patch : ("^1.2.3", comp_sets!([op!(">="), 1, 2, 3],
        [op!("<"), 2, 0, 0])), caret_major_minor_patch_pre : ("^1.2.3-beta.4",
        comp_sets!([op!(">="), 1, 2, 3, pre!(id_alpha!("beta"), id_num!(4))], [op!("<"),
        2, 0, 0, pre!()])), caret_major_0 : ("^0", comp_sets!([op!(">="), 0, 0, 0],
        [op!("<"), 1, 0, 0])), caret_major_0_xrange : ("^0.x", comp_sets!([op!(">="), 0,
        0, 0], [op!("<"), 1, 0, 0])), caret_major_0_minor_0 : ("^0.0",
        comp_sets!([op!(">="), 0, 0, 0], [op!("<"), 0, 1, 0])),
        caret_major_0_minor_0_xrange : ("^0.0.x", comp_sets!([op!(">="), 0, 0, 0],
        [op!("<"), 0, 1, 0])), caret_major_0_minor : ("^0.1", comp_sets!([op!(">="), 0,
        1, 0], [op!("<"), 0, 2, 0])), caret_major_0_minor_xrange : ("^0.1.x",
        comp_sets!([op!(">="), 0, 1, 0], [op!("<"), 0, 2, 0])), caret_major_0_minor_patch
        : ("^0.1.2", comp_sets!([op!(">="), 0, 1, 2], [op!("<"), 0, 2, 0])),
        caret_major_0_minor_0_patch : ("^0.0.1", comp_sets!([op!(">="), 0, 0, 1],
        [op!("<"), 0, 0, 2])), caret_major_0_minor_0_pre : ("^0.0.1-beta",
        comp_sets!([op!(">="), 0, 0, 1, pre!(id_alpha!("beta"))], [op!("<"), 0, 0, 2,
        pre!()])), caret_all : ("^*", comp_sets!([op!(">="), 0, 0, 0])),
        two_comparators_1 : (">1.2.3 <4.5.6", comp_sets!([op!(">"), 1, 2, 3], [op!("<"),
        4, 5, 6])), two_comparators_2 : ("^1.2 ^1", comp_sets!([op!(">="), 1, 2, 0],
        [op!("<"), 2, 0, 0], [op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])),
        comparator_with_pre : ("=1.2.3-rc.1", comp_sets!([op!("="), 1, 2, 3,
        pre!(id_alpha!("rc"), id_num!(1))])), hyphen_major : ("1 - 4",
        comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 5, 0, 0])), hyphen_major_x :
        ("1.* - 4.*", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 5, 0, 0])),
        hyphen_major_minor_x : ("1.2.x - 4.5.x", comp_sets!([op!(">="), 1, 2, 0],
        [op!("<"), 4, 6, 0])), hyphen_major_minor_patch : ("1.2.3 - 4.5.6",
        comp_sets!([op!(">="), 1, 2, 3], [op!("<="), 4, 5, 6])), hyphen_with_pre :
        ("1.2.3-rc1 - 4.5.6", comp_sets!([op!(">="), 1, 2, 3, pre!(id_alpha!("rc1"))],
        [op!("<="), 4, 5, 6, pre!()])), hyphen_xrange_minor_only1 : ("1.*.3 - 3.4.5",
        comp_sets!([op!(">="), 1, 0, 0], [op!("<="), 3, 4, 5])),
        hyphen_xrange_minor_only2 : ("1.2.3 - 3.*.5", comp_sets!([op!(">="), 1, 2, 3],
        [op!("<"), 4, 0, 0])), hyphen_all_to_something : ("* - 3.4.5",
        comp_sets!([op!("<="), 3, 4, 5])), hyphen_to_all : ("1.2.3 - *",
        comp_sets!([op!(">="), 1, 2, 3])), hyphen_all_to_all : ("* - *",
        comp_sets!([op!(">="), 0, 0, 0])), gte_space : (">= 1.2.3",
        comp_sets!([op!(">="), 1, 2, 3])), gte_tab : (">=\t1.2.3", comp_sets!([op!(">="),
        1, 2, 3])), gte_two_spaces : (">=  1.2.3", comp_sets!([op!(">="), 1, 2, 3])),
        gt_space : ("> 1.2.3", comp_sets!([op!(">"), 1, 2, 3])), gt_two_spaces :
        (">  1.2.3", comp_sets!([op!(">"), 1, 2, 3])), lte_space : ("<= 1.2.3",
        comp_sets!([op!("<="), 1, 2, 3])), lte_two_spaces : ("<=  1.2.3",
        comp_sets!([op!("<="), 1, 2, 3])), lt_space : ("< 1.2.3", comp_sets!([op!("<"),
        1, 2, 3])), lt_two_spaces : ("<  1.2.3", comp_sets!([op!("<"), 1, 2, 3])),
        eq_space : ("= 1.2.3", comp_sets!([op!("="), 1, 2, 3])), eq_two_spaces :
        ("=  1.2.3", comp_sets!([op!("="), 1, 2, 3])), caret_space : ("^ 1.2.3",
        comp_sets!([op!(">="), 1, 2, 3], [op!("<"), 2, 0, 0])), tilde_space : ("~ 1.2.3",
        comp_sets!([op!(">="), 1, 2, 3], [op!("<"), 1, 3, 0])), hyphen_spacing :
        ("1.2.3 -  4.5.6", comp_sets!([op!(">="), 1, 2, 3], [op!("<="), 4, 5, 6])),
        digits : ("=0.2.3", comp_sets!([op!("="), 0, 2, 3])), digits_2 : ("=11.2.3",
        comp_sets!([op!("="), 11, 2, 3])), digits_3 : ("=1.12.3", comp_sets!([op!("="),
        1, 12, 3])), digits_4 : ("=1.2.13", comp_sets!([op!("="), 1, 2, 13])), digits_5 :
        ("=1.2.5678", comp_sets!([op!("="), 1, 2, 5678])), xrange_major_x : ("1.x",
        comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])), xrange_major_x_x :
        ("1.x.x", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])),
        xrange_major_minor_x : ("1.2.x", comp_sets!([op!(">="), 1, 2, 0], [op!("<"), 1,
        3, 0])), xrange_major_xx : ("1.X", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2,
        0, 0])), xrange_major_xx_xx : ("1.X.X", comp_sets!([op!(">="), 1, 0, 0],
        [op!("<"), 2, 0, 0])), xrange_major_minor_xx : ("1.2.X", comp_sets!([op!(">="),
        1, 2, 0], [op!("<"), 1, 3, 0])), xrange_star : ("*", comp_sets!([op!(">="), 0, 0,
        0])), xrange_x : ("x", comp_sets!([op!(">="), 0, 0, 0])), xrange_xx : ("X",
        comp_sets!([op!(">="), 0, 0, 0])), xrange_major_star : ("1.*",
        comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])), xrange_major_star_star :
        ("1.*.*", comp_sets!([op!(">="), 1, 0, 0], [op!("<"), 2, 0, 0])),
        xrange_major_minor_star : ("1.2.*", comp_sets!([op!(">="), 1, 2, 0], [op!("<"),
        1, 3, 0])), xrange_with_pre : ("1.*.*-beta", comp_sets!([op!(">="), 1, 0, 0],
        [op!("<"), 2, 0, 0])), xrange_minor_only : ("1.*.3", comp_sets!([op!(">="), 1, 0,
        0], [op!("<"), 2, 0, 0])), gte_star : (">=*", comp_sets!([op!(">="), 0, 0, 0])),
        empty : ("", comp_sets!([op!(">="), 0, 0, 0])),
    }
    range_tests_nodecompat! {
        node_major_minor_patch : ("1.2.3", comp_sets_node!([op!("="), 1, 2, 3])),
    }
}
#[cfg(test)]
mod tests_llm_16_84 {
    use crate::range::Partial;
    #[test]
    fn test_inc_major() {
        let _rug_st_tests_llm_16_84_rrrruuuugggg_test_inc_major = 0;
        let mut partial = Partial::new();
        partial.inc_major();
        debug_assert_eq!(partial.major, Some(1));
        let _rug_ed_tests_llm_16_84_rrrruuuugggg_test_inc_major = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_85 {
    use super::*;
    use crate::*;
    #[test]
    fn test_inc_minor() {
        let _rug_st_tests_llm_16_85_rrrruuuugggg_test_inc_minor = 0;
        let rug_fuzz_0 = 4;
        let rug_fuzz_1 = 100;
        let mut partial = Partial::new();
        partial.inc_minor();
        debug_assert_eq!(partial.minor, Some(1));
        partial.minor = Some(rug_fuzz_0);
        partial.inc_minor();
        debug_assert_eq!(partial.minor, Some(5));
        partial.minor = Some(rug_fuzz_1);
        partial.inc_minor();
        debug_assert_eq!(partial.minor, Some(101));
        let _rug_ed_tests_llm_16_85_rrrruuuugggg_test_inc_minor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_86 {
    use super::*;
    use crate::*;
    #[test]
    fn test_inc_patch() {
        let _rug_st_tests_llm_16_86_rrrruuuugggg_test_inc_patch = 0;
        let mut range = range::Partial::new();
        range.inc_patch();
        debug_assert_eq!(range.patch, Some(1));
        let _rug_ed_tests_llm_16_86_rrrruuuugggg_test_inc_patch = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_87 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_87_rrrruuuugggg_test_new = 0;
        let partial = Partial::new();
        debug_assert_eq!(partial.major, None);
        debug_assert_eq!(partial.minor, None);
        debug_assert_eq!(partial.patch, None);
        debug_assert_eq!(partial.pre, Vec::new());
        debug_assert_eq!(partial.kind, PartialKind::XRangeOnly);
        let _rug_ed_tests_llm_16_87_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_88 {
    use super::*;
    use crate::*;
    #[test]
    fn test_no_pre() {
        let _rug_st_tests_llm_16_88_rrrruuuugggg_test_no_pre = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = "alpha";
        let rug_fuzz_2 = 2;
        let mut range = Partial::new();
        range.pre.push(Identifier::Numeric(rug_fuzz_0));
        range.pre.push(Identifier::AlphaNumeric(rug_fuzz_1.to_string()));
        range.pre.push(Identifier::Numeric(rug_fuzz_2));
        range.no_pre();
        debug_assert!(range.pre.is_empty());
        let _rug_ed_tests_llm_16_88_rrrruuuugggg_test_no_pre = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_89 {
    use crate::range::Partial;
    #[test]
    fn test_zero_minor() {
        let _rug_st_tests_llm_16_89_rrrruuuugggg_test_zero_minor = 0;
        let rug_fuzz_0 = 0;
        let mut partial = Partial::new();
        partial.zero_minor();
        debug_assert_eq!(rug_fuzz_0, partial.minor.unwrap());
        let _rug_ed_tests_llm_16_89_rrrruuuugggg_test_zero_minor = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_91_llm_16_90 {
    use super::*;
    use crate::*;
    use crate::range::{Partial, PartialKind};
    #[test]
    fn test_zero_missing() {
        let _rug_st_tests_llm_16_91_llm_16_90_rrrruuuugggg_test_zero_missing = 0;
        let rug_fuzz_0 = 1;
        let mut partial = Partial::new();
        partial.major = Some(rug_fuzz_0);
        partial.zero_missing();
        debug_assert_eq!(partial.major, Some(1));
        debug_assert_eq!(partial.minor, Some(0));
        debug_assert_eq!(partial.patch, Some(0));
        let _rug_ed_tests_llm_16_91_llm_16_90_rrrruuuugggg_test_zero_missing = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_92 {
    use super::*;
    use crate::*;
    use crate::range::{Partial, PartialKind};
    #[test]
    fn test_zero_patch() {
        let _rug_st_tests_llm_16_92_rrrruuuugggg_test_zero_patch = 0;
        let mut partial = Partial::new();
        partial.zero_patch();
        debug_assert_eq!(partial.patch, Some(0));
        let _rug_ed_tests_llm_16_92_rrrruuuugggg_test_zero_patch = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_94_llm_16_93 {
    use super::*;
    use crate::*;
    use crate::*;
    use pest::Parser;
    #[test]
    fn test_from_pair_iterator_hyphen() {
        let _rug_st_tests_llm_16_94_llm_16_93_rrrruuuugggg_test_from_pair_iterator_hyphen = 0;
        let rug_fuzz_0 = "1.0.0 - 2.0.0";
        let pair = SemverParser::parse(Rule::range, rug_fuzz_0).unwrap().next().unwrap();
        let result = from_pair_iterator(pair, range_set::Compat::Cargo);
        debug_assert!(result.is_ok());
        let range = result.unwrap();
        debug_assert_eq!(range.compat, range_set::Compat::Cargo);
        debug_assert_eq!(range.comparator_set.len(), 1);
        let _rug_ed_tests_llm_16_94_llm_16_93_rrrruuuugggg_test_from_pair_iterator_hyphen = 0;
    }
    #[test]
    fn test_from_pair_iterator_simple() {
        let _rug_st_tests_llm_16_94_llm_16_93_rrrruuuugggg_test_from_pair_iterator_simple = 0;
        let rug_fuzz_0 = ">=1.0.0";
        let pair = SemverParser::parse(Rule::range, rug_fuzz_0).unwrap().next().unwrap();
        let result = from_pair_iterator(pair, range_set::Compat::Cargo);
        debug_assert!(result.is_ok());
        let range = result.unwrap();
        debug_assert_eq!(range.compat, range_set::Compat::Cargo);
        debug_assert_eq!(range.comparator_set.len(), 1);
        let _rug_ed_tests_llm_16_94_llm_16_93_rrrruuuugggg_test_from_pair_iterator_simple = 0;
    }
    #[test]
    fn test_from_pair_iterator_empty() {
        let _rug_st_tests_llm_16_94_llm_16_93_rrrruuuugggg_test_from_pair_iterator_empty = 0;
        let rug_fuzz_0 = "";
        let pair = SemverParser::parse(Rule::range, rug_fuzz_0).unwrap().next().unwrap();
        let result = from_pair_iterator(pair, range_set::Compat::Cargo);
        debug_assert!(result.is_ok());
        let range = result.unwrap();
        debug_assert_eq!(range.compat, range_set::Compat::Cargo);
        debug_assert_eq!(range.comparator_set.len(), 1);
        let _rug_ed_tests_llm_16_94_llm_16_93_rrrruuuugggg_test_from_pair_iterator_empty = 0;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::range::Partial;
    use crate::range::Op;
    use crate::range::Comparator;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_26_rrrruuuugggg_test_rug = 0;
        let mut p0 = Partial::new();
        let mut p1: Op = Op::Lt;
        p0.as_comparator(p1);
        let _rug_ed_tests_rug_26_rrrruuuugggg_test_rug = 0;
    }
}
