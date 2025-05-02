/*!
Provides routines for extracting literal prefixes and suffixes from an `Hir`.
*/
use std::cmp;
use std::fmt;
use std::iter;
use std::mem;
use std::ops;
use hir::{self, Hir, HirKind};
/// A set of literal byte strings extracted from a regular expression.
///
/// Every member of the set is a `Literal`, which is represented by a
/// `Vec<u8>`. (Notably, it may contain invalid UTF-8.) Every member is
/// said to be either *complete* or *cut*. A complete literal means that
/// it extends until the beginning (or end) of the regular expression. In
/// some circumstances, this can be used to indicate a match in the regular
/// expression.
///
/// A key aspect of literal extraction is knowing when to stop. It is not
/// feasible to blindly extract all literals from a regular expression, even if
/// there are finitely many. For example, the regular expression `[0-9]{10}`
/// has `10^10` distinct literals. For this reason, literal extraction is
/// bounded to some low number by default using heuristics, but the limits can
/// be tweaked.
///
/// **WARNING**: Literal extraction uses stack space proportional to the size
/// of the `Hir` expression. At some point, this drawback will be eliminated.
/// To protect yourself, set a reasonable
/// [`nest_limit` on your `Parser`](../../struct.ParserBuilder.html#method.nest_limit).
/// This is done for you by default.
#[derive(Clone, Eq, PartialEq)]
pub struct Literals {
    lits: Vec<Literal>,
    limit_size: usize,
    limit_class: usize,
}
/// A single member of a set of literals extracted from a regular expression.
///
/// This type has `Deref` and `DerefMut` impls to `Vec<u8>` so that all slice
/// and `Vec` operations are available.
#[derive(Clone, Eq, Ord)]
pub struct Literal {
    v: Vec<u8>,
    cut: bool,
}
impl Literals {
    /// Returns a new empty set of literals using default limits.
    pub fn empty() -> Literals {
        Literals {
            lits: vec![],
            limit_size: 250,
            limit_class: 10,
        }
    }
    /// Returns a set of literal prefixes extracted from the given `Hir`.
    pub fn prefixes(expr: &Hir) -> Literals {
        let mut lits = Literals::empty();
        lits.union_prefixes(expr);
        lits
    }
    /// Returns a set of literal suffixes extracted from the given `Hir`.
    pub fn suffixes(expr: &Hir) -> Literals {
        let mut lits = Literals::empty();
        lits.union_suffixes(expr);
        lits
    }
    /// Get the approximate size limit (in bytes) of this set.
    pub fn limit_size(&self) -> usize {
        self.limit_size
    }
    /// Set the approximate size limit (in bytes) of this set.
    ///
    /// If extracting a literal would put the set over this limit, then
    /// extraction stops.
    ///
    /// The new limits will only apply to additions to this set. Existing
    /// members remain unchanged, even if the set exceeds the new limit.
    pub fn set_limit_size(&mut self, size: usize) -> &mut Literals {
        self.limit_size = size;
        self
    }
    /// Get the character class size limit for this set.
    pub fn limit_class(&self) -> usize {
        self.limit_class
    }
    /// Limits the size of character(or byte) classes considered.
    ///
    /// A value of `0` prevents all character classes from being considered.
    ///
    /// This limit also applies to case insensitive literals, since each
    /// character in the case insensitive literal is converted to a class, and
    /// then case folded.
    ///
    /// The new limits will only apply to additions to this set. Existing
    /// members remain unchanged, even if the set exceeds the new limit.
    pub fn set_limit_class(&mut self, size: usize) -> &mut Literals {
        self.limit_class = size;
        self
    }
    /// Returns the set of literals as a slice. Its order is unspecified.
    pub fn literals(&self) -> &[Literal] {
        &self.lits
    }
    /// Returns the length of the smallest literal.
    ///
    /// Returns None is there are no literals in the set.
    pub fn min_len(&self) -> Option<usize> {
        let mut min = None;
        for lit in &self.lits {
            match min {
                None => min = Some(lit.len()),
                Some(m) if lit.len() < m => min = Some(lit.len()),
                _ => {}
            }
        }
        min
    }
    /// Returns true if all members in this set are complete.
    pub fn all_complete(&self) -> bool {
        !self.lits.is_empty() && self.lits.iter().all(|l| !l.is_cut())
    }
    /// Returns true if any member in this set is complete.
    pub fn any_complete(&self) -> bool {
        self.lits.iter().any(|lit| !lit.is_cut())
    }
    /// Returns true if this set contains an empty literal.
    pub fn contains_empty(&self) -> bool {
        self.lits.iter().any(|lit| lit.is_empty())
    }
    /// Returns true if this set is empty or if all of its members is empty.
    pub fn is_empty(&self) -> bool {
        self.lits.is_empty() || self.lits.iter().all(|lit| lit.is_empty())
    }
    /// Returns a new empty set of literals using this set's limits.
    pub fn to_empty(&self) -> Literals {
        let mut lits = Literals::empty();
        lits.set_limit_size(self.limit_size).set_limit_class(self.limit_class);
        lits
    }
    /// Returns the longest common prefix of all members in this set.
    pub fn longest_common_prefix(&self) -> &[u8] {
        if self.is_empty() {
            return &[];
        }
        let lit0 = &*self.lits[0];
        let mut len = lit0.len();
        for lit in &self.lits[1..] {
            len = cmp::min(
                len,
                lit.iter().zip(lit0).take_while(|&(a, b)| a == b).count(),
            );
        }
        &self.lits[0][..len]
    }
    /// Returns the longest common suffix of all members in this set.
    pub fn longest_common_suffix(&self) -> &[u8] {
        if self.is_empty() {
            return &[];
        }
        let lit0 = &*self.lits[0];
        let mut len = lit0.len();
        for lit in &self.lits[1..] {
            len = cmp::min(
                len,
                lit
                    .iter()
                    .rev()
                    .zip(lit0.iter().rev())
                    .take_while(|&(a, b)| a == b)
                    .count(),
            );
        }
        &self.lits[0][self.lits[0].len() - len..]
    }
    /// Returns a new set of literals with the given number of bytes trimmed
    /// from the suffix of each literal.
    ///
    /// If any literal would be cut out completely by trimming, then None is
    /// returned.
    ///
    /// Any duplicates that are created as a result of this transformation are
    /// removed.
    pub fn trim_suffix(&self, num_bytes: usize) -> Option<Literals> {
        if self.min_len().map(|len| len <= num_bytes).unwrap_or(true) {
            return None;
        }
        let mut new = self.to_empty();
        for mut lit in self.lits.iter().cloned() {
            let new_len = lit.len() - num_bytes;
            lit.truncate(new_len);
            lit.cut();
            new.lits.push(lit);
        }
        new.lits.sort();
        new.lits.dedup();
        Some(new)
    }
    /// Returns a new set of prefixes of this set of literals that are
    /// guaranteed to be unambiguous.
    ///
    /// Any substring match with a member of the set is returned is guaranteed
    /// to never overlap with a substring match of another member of the set
    /// at the same starting position.
    ///
    /// Given any two members of the returned set, neither is a substring of
    /// the other.
    pub fn unambiguous_prefixes(&self) -> Literals {
        if self.lits.is_empty() {
            return self.to_empty();
        }
        let mut old: Vec<Literal> = self.lits.iter().cloned().collect();
        let mut new = self.to_empty();
        'OUTER: while let Some(mut candidate) = old.pop() {
            if candidate.is_empty() {
                continue;
            }
            if new.lits.is_empty() {
                new.lits.push(candidate);
                continue;
            }
            for lit2 in &mut new.lits {
                if lit2.is_empty() {
                    continue;
                }
                if &candidate == lit2 {
                    candidate.cut = candidate.cut || lit2.cut;
                    lit2.cut = candidate.cut;
                    continue 'OUTER;
                }
                if candidate.len() < lit2.len() {
                    if let Some(i) = position(&candidate, &lit2) {
                        candidate.cut();
                        let mut lit3 = lit2.clone();
                        lit3.truncate(i);
                        lit3.cut();
                        old.push(lit3);
                        lit2.clear();
                    }
                } else {
                    if let Some(i) = position(&lit2, &candidate) {
                        lit2.cut();
                        let mut new_candidate = candidate.clone();
                        new_candidate.truncate(i);
                        new_candidate.cut();
                        old.push(new_candidate);
                        candidate.clear();
                    }
                }
                if candidate.is_empty() {
                    continue 'OUTER;
                }
            }
            new.lits.push(candidate);
        }
        new.lits.retain(|lit| !lit.is_empty());
        new.lits.sort();
        new.lits.dedup();
        new
    }
    /// Returns a new set of suffixes of this set of literals that are
    /// guaranteed to be unambiguous.
    ///
    /// Any substring match with a member of the set is returned is guaranteed
    /// to never overlap with a substring match of another member of the set
    /// at the same ending position.
    ///
    /// Given any two members of the returned set, neither is a substring of
    /// the other.
    pub fn unambiguous_suffixes(&self) -> Literals {
        let mut lits = self.clone();
        lits.reverse();
        let mut unamb = lits.unambiguous_prefixes();
        unamb.reverse();
        unamb
    }
    /// Unions the prefixes from the given expression to this set.
    ///
    /// If prefixes could not be added (for example, this set would exceed its
    /// size limits or the set of prefixes from `expr` includes the empty
    /// string), then false is returned.
    ///
    /// Note that prefix literals extracted from `expr` are said to be complete
    /// if and only if the literal extends from the beginning of `expr` to the
    /// end of `expr`.
    pub fn union_prefixes(&mut self, expr: &Hir) -> bool {
        let mut lits = self.to_empty();
        prefixes(expr, &mut lits);
        !lits.is_empty() && !lits.contains_empty() && self.union(lits)
    }
    /// Unions the suffixes from the given expression to this set.
    ///
    /// If suffixes could not be added (for example, this set would exceed its
    /// size limits or the set of suffixes from `expr` includes the empty
    /// string), then false is returned.
    ///
    /// Note that prefix literals extracted from `expr` are said to be complete
    /// if and only if the literal extends from the end of `expr` to the
    /// beginning of `expr`.
    pub fn union_suffixes(&mut self, expr: &Hir) -> bool {
        let mut lits = self.to_empty();
        suffixes(expr, &mut lits);
        lits.reverse();
        !lits.is_empty() && !lits.contains_empty() && self.union(lits)
    }
    /// Unions this set with another set.
    ///
    /// If the union would cause the set to exceed its limits, then the union
    /// is skipped and it returns false. Otherwise, if the union succeeds, it
    /// returns true.
    pub fn union(&mut self, lits: Literals) -> bool {
        if self.num_bytes() + lits.num_bytes() > self.limit_size {
            return false;
        }
        if lits.is_empty() {
            self.lits.push(Literal::empty());
        } else {
            self.lits.extend(lits.lits);
        }
        true
    }
    /// Extends this set with another set.
    ///
    /// The set of literals is extended via a cross product.
    ///
    /// If a cross product would cause this set to exceed its limits, then the
    /// cross product is skipped and it returns false. Otherwise, if the cross
    /// product succeeds, it returns true.
    pub fn cross_product(&mut self, lits: &Literals) -> bool {
        if lits.is_empty() {
            return true;
        }
        let mut size_after;
        if self.is_empty() || !self.any_complete() {
            size_after = self.num_bytes();
            for lits_lit in lits.literals() {
                size_after += lits_lit.len();
            }
        } else {
            size_after = self
                .lits
                .iter()
                .fold(
                    0,
                    |accum, lit| { accum + if lit.is_cut() { lit.len() } else { 0 } },
                );
            for lits_lit in lits.literals() {
                for self_lit in self.literals() {
                    if !self_lit.is_cut() {
                        size_after += self_lit.len() + lits_lit.len();
                    }
                }
            }
        }
        if size_after > self.limit_size {
            return false;
        }
        let mut base = self.remove_complete();
        if base.is_empty() {
            base = vec![Literal::empty()];
        }
        for lits_lit in lits.literals() {
            for mut self_lit in base.clone() {
                self_lit.extend(&**lits_lit);
                self_lit.cut = lits_lit.cut;
                self.lits.push(self_lit);
            }
        }
        true
    }
    /// Extends each literal in this set with the bytes given.
    ///
    /// If the set is empty, then the given literal is added to the set.
    ///
    /// If adding any number of bytes to all members of this set causes a limit
    /// to be exceeded, then no bytes are added and false is returned. If a
    /// prefix of `bytes` can be fit into this set, then it is used and all
    /// resulting literals are cut.
    pub fn cross_add(&mut self, bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return true;
        }
        if self.lits.is_empty() {
            let i = cmp::min(self.limit_size, bytes.len());
            self.lits.push(Literal::new(bytes[..i].to_owned()));
            self.lits[0].cut = i < bytes.len();
            return !self.lits[0].is_cut();
        }
        let size = self.num_bytes();
        if size + self.lits.len() >= self.limit_size {
            return false;
        }
        let mut i = 1;
        while size + (i * self.lits.len()) <= self.limit_size && i < bytes.len() {
            i += 1;
        }
        for lit in &mut self.lits {
            if !lit.is_cut() {
                lit.extend(&bytes[..i]);
                if i < bytes.len() {
                    lit.cut();
                }
            }
        }
        true
    }
    /// Adds the given literal to this set.
    ///
    /// Returns false if adding this literal would cause the class to be too
    /// big.
    pub fn add(&mut self, lit: Literal) -> bool {
        if self.num_bytes() + lit.len() > self.limit_size {
            return false;
        }
        self.lits.push(lit);
        true
    }
    /// Extends each literal in this set with the character class given.
    ///
    /// Returns false if the character class was too big to add.
    pub fn add_char_class(&mut self, cls: &hir::ClassUnicode) -> bool {
        self._add_char_class(cls, false)
    }
    /// Extends each literal in this set with the character class given,
    /// writing the bytes of each character in reverse.
    ///
    /// Returns false if the character class was too big to add.
    fn add_char_class_reverse(&mut self, cls: &hir::ClassUnicode) -> bool {
        self._add_char_class(cls, true)
    }
    fn _add_char_class(&mut self, cls: &hir::ClassUnicode, reverse: bool) -> bool {
        use std::char;
        if self.class_exceeds_limits(cls_char_count(cls)) {
            return false;
        }
        let mut base = self.remove_complete();
        if base.is_empty() {
            base = vec![Literal::empty()];
        }
        for r in cls.iter() {
            let (s, e) = (r.start as u32, r.end as u32 + 1);
            for c in (s..e).filter_map(char::from_u32) {
                for mut lit in base.clone() {
                    let mut bytes = c.to_string().into_bytes();
                    if reverse {
                        bytes.reverse();
                    }
                    lit.extend(&bytes);
                    self.lits.push(lit);
                }
            }
        }
        true
    }
    /// Extends each literal in this set with the byte class given.
    ///
    /// Returns false if the byte class was too big to add.
    pub fn add_byte_class(&mut self, cls: &hir::ClassBytes) -> bool {
        if self.class_exceeds_limits(cls_byte_count(cls)) {
            return false;
        }
        let mut base = self.remove_complete();
        if base.is_empty() {
            base = vec![Literal::empty()];
        }
        for r in cls.iter() {
            let (s, e) = (r.start as u32, r.end as u32 + 1);
            for b in (s..e).map(|b| b as u8) {
                for mut lit in base.clone() {
                    lit.push(b);
                    self.lits.push(lit);
                }
            }
        }
        true
    }
    /// Cuts every member of this set. When a member is cut, it can never
    /// be extended.
    pub fn cut(&mut self) {
        for lit in &mut self.lits {
            lit.cut();
        }
    }
    /// Reverses all members in place.
    pub fn reverse(&mut self) {
        for lit in &mut self.lits {
            lit.reverse();
        }
    }
    /// Clears this set of all members.
    pub fn clear(&mut self) {
        self.lits.clear();
    }
    /// Pops all complete literals out of this set.
    fn remove_complete(&mut self) -> Vec<Literal> {
        let mut base = vec![];
        for lit in mem::replace(&mut self.lits, vec![]) {
            if lit.is_cut() {
                self.lits.push(lit);
            } else {
                base.push(lit);
            }
        }
        base
    }
    /// Returns the total number of bytes in this set.
    fn num_bytes(&self) -> usize {
        self.lits.iter().fold(0, |accum, lit| accum + lit.len())
    }
    /// Returns true if a character class with the given size would cause this
    /// set to exceed its limits.
    ///
    /// The size given should correspond to the number of items in the class.
    fn class_exceeds_limits(&self, size: usize) -> bool {
        if size > self.limit_class {
            return true;
        }
        let new_byte_count = if self.lits.is_empty() {
            size
        } else {
            self.lits
                .iter()
                .fold(
                    0,
                    |accum, lit| {
                        accum + if lit.is_cut() { 0 } else { (lit.len() + 1) * size }
                    },
                )
        };
        new_byte_count > self.limit_size
    }
}
fn prefixes(expr: &Hir, lits: &mut Literals) {
    match *expr.kind() {
        HirKind::Literal(hir::Literal::Unicode(c)) => {
            let mut buf = [0; 4];
            lits.cross_add(c.encode_utf8(&mut buf).as_bytes());
        }
        HirKind::Literal(hir::Literal::Byte(b)) => {
            lits.cross_add(&[b]);
        }
        HirKind::Class(hir::Class::Unicode(ref cls)) => {
            if !lits.add_char_class(cls) {
                lits.cut();
            }
        }
        HirKind::Class(hir::Class::Bytes(ref cls)) => {
            if !lits.add_byte_class(cls) {
                lits.cut();
            }
        }
        HirKind::Group(hir::Group { ref hir, .. }) => {
            prefixes(&**hir, lits);
        }
        HirKind::Repetition(ref x) => {
            match x.kind {
                hir::RepetitionKind::ZeroOrOne => {
                    repeat_zero_or_one_literals(&x.hir, lits, prefixes);
                }
                hir::RepetitionKind::ZeroOrMore => {
                    repeat_zero_or_more_literals(&x.hir, lits, prefixes);
                }
                hir::RepetitionKind::OneOrMore => {
                    repeat_one_or_more_literals(&x.hir, lits, prefixes);
                }
                hir::RepetitionKind::Range(ref rng) => {
                    let (min, max) = match *rng {
                        hir::RepetitionRange::Exactly(m) => (m, Some(m)),
                        hir::RepetitionRange::AtLeast(m) => (m, None),
                        hir::RepetitionRange::Bounded(m, n) => (m, Some(n)),
                    };
                    repeat_range_literals(&x.hir, min, max, x.greedy, lits, prefixes)
                }
            }
        }
        HirKind::Concat(ref es) if es.is_empty() => {}
        HirKind::Concat(ref es) if es.len() == 1 => prefixes(&es[0], lits),
        HirKind::Concat(ref es) => {
            for e in es {
                if let HirKind::Anchor(hir::Anchor::StartText) = *e.kind() {
                    if !lits.is_empty() {
                        lits.cut();
                        break;
                    }
                    lits.add(Literal::empty());
                    continue;
                }
                let mut lits2 = lits.to_empty();
                prefixes(e, &mut lits2);
                if !lits.cross_product(&lits2) || !lits2.any_complete() {
                    lits.cut();
                    break;
                }
            }
        }
        HirKind::Alternation(ref es) => {
            alternate_literals(es, lits, prefixes);
        }
        _ => lits.cut(),
    }
}
fn suffixes(expr: &Hir, lits: &mut Literals) {
    match *expr.kind() {
        HirKind::Literal(hir::Literal::Unicode(c)) => {
            let mut buf = [0u8; 4];
            let i = c.encode_utf8(&mut buf).len();
            let buf = &mut buf[..i];
            buf.reverse();
            lits.cross_add(buf);
        }
        HirKind::Literal(hir::Literal::Byte(b)) => {
            lits.cross_add(&[b]);
        }
        HirKind::Class(hir::Class::Unicode(ref cls)) => {
            if !lits.add_char_class_reverse(cls) {
                lits.cut();
            }
        }
        HirKind::Class(hir::Class::Bytes(ref cls)) => {
            if !lits.add_byte_class(cls) {
                lits.cut();
            }
        }
        HirKind::Group(hir::Group { ref hir, .. }) => {
            suffixes(&**hir, lits);
        }
        HirKind::Repetition(ref x) => {
            match x.kind {
                hir::RepetitionKind::ZeroOrOne => {
                    repeat_zero_or_one_literals(&x.hir, lits, suffixes);
                }
                hir::RepetitionKind::ZeroOrMore => {
                    repeat_zero_or_more_literals(&x.hir, lits, suffixes);
                }
                hir::RepetitionKind::OneOrMore => {
                    repeat_one_or_more_literals(&x.hir, lits, suffixes);
                }
                hir::RepetitionKind::Range(ref rng) => {
                    let (min, max) = match *rng {
                        hir::RepetitionRange::Exactly(m) => (m, Some(m)),
                        hir::RepetitionRange::AtLeast(m) => (m, None),
                        hir::RepetitionRange::Bounded(m, n) => (m, Some(n)),
                    };
                    repeat_range_literals(&x.hir, min, max, x.greedy, lits, suffixes)
                }
            }
        }
        HirKind::Concat(ref es) if es.is_empty() => {}
        HirKind::Concat(ref es) if es.len() == 1 => suffixes(&es[0], lits),
        HirKind::Concat(ref es) => {
            for e in es.iter().rev() {
                if let HirKind::Anchor(hir::Anchor::EndText) = *e.kind() {
                    if !lits.is_empty() {
                        lits.cut();
                        break;
                    }
                    lits.add(Literal::empty());
                    continue;
                }
                let mut lits2 = lits.to_empty();
                suffixes(e, &mut lits2);
                if !lits.cross_product(&lits2) || !lits2.any_complete() {
                    lits.cut();
                    break;
                }
            }
        }
        HirKind::Alternation(ref es) => {
            alternate_literals(es, lits, suffixes);
        }
        _ => lits.cut(),
    }
}
fn repeat_zero_or_one_literals<F: FnMut(&Hir, &mut Literals)>(
    e: &Hir,
    lits: &mut Literals,
    mut f: F,
) {
    let (mut lits2, mut lits3) = (lits.clone(), lits.to_empty());
    lits3.set_limit_size(lits.limit_size() / 2);
    f(e, &mut lits3);
    if lits3.is_empty() || !lits2.cross_product(&lits3) {
        lits.cut();
        return;
    }
    lits2.add(Literal::empty());
    if !lits.union(lits2) {
        lits.cut();
    }
}
fn repeat_zero_or_more_literals<F: FnMut(&Hir, &mut Literals)>(
    e: &Hir,
    lits: &mut Literals,
    mut f: F,
) {
    let (mut lits2, mut lits3) = (lits.clone(), lits.to_empty());
    lits3.set_limit_size(lits.limit_size() / 2);
    f(e, &mut lits3);
    if lits3.is_empty() || !lits2.cross_product(&lits3) {
        lits.cut();
        return;
    }
    lits2.cut();
    lits2.add(Literal::empty());
    if !lits.union(lits2) {
        lits.cut();
    }
}
fn repeat_one_or_more_literals<F: FnMut(&Hir, &mut Literals)>(
    e: &Hir,
    lits: &mut Literals,
    mut f: F,
) {
    f(e, lits);
    lits.cut();
}
fn repeat_range_literals<F: FnMut(&Hir, &mut Literals)>(
    e: &Hir,
    min: u32,
    max: Option<u32>,
    greedy: bool,
    lits: &mut Literals,
    mut f: F,
) {
    if min == 0 {
        f(
            &Hir::repetition(hir::Repetition {
                kind: hir::RepetitionKind::ZeroOrMore,
                greedy: greedy,
                hir: Box::new(e.clone()),
            }),
            lits,
        );
    } else {
        if min > 0 {
            let n = cmp::min(lits.limit_size, min as usize);
            let es = iter::repeat(e.clone()).take(n).collect();
            f(&Hir::concat(es), lits);
            if n < min as usize || lits.contains_empty() {
                lits.cut();
            }
        }
        if max.map_or(true, |max| min < max) {
            lits.cut();
        }
    }
}
fn alternate_literals<F: FnMut(&Hir, &mut Literals)>(
    es: &[Hir],
    lits: &mut Literals,
    mut f: F,
) {
    let mut lits2 = lits.to_empty();
    for e in es {
        let mut lits3 = lits.to_empty();
        lits3.set_limit_size(lits.limit_size() / 5);
        f(e, &mut lits3);
        if lits3.is_empty() || !lits2.union(lits3) {
            lits.cut();
            return;
        }
    }
    if !lits.cross_product(&lits2) {
        lits.cut();
    }
}
impl fmt::Debug for Literals {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Literals")
            .field("lits", &self.lits)
            .field("limit_size", &self.limit_size)
            .field("limit_class", &self.limit_class)
            .finish()
    }
}
impl Literal {
    /// Returns a new complete literal with the bytes given.
    pub fn new(bytes: Vec<u8>) -> Literal {
        Literal { v: bytes, cut: false }
    }
    /// Returns a new complete empty literal.
    pub fn empty() -> Literal {
        Literal { v: vec![], cut: false }
    }
    /// Returns true if this literal was "cut."
    pub fn is_cut(&self) -> bool {
        self.cut
    }
    /// Cuts this literal.
    pub fn cut(&mut self) {
        self.cut = true;
    }
}
impl PartialEq for Literal {
    fn eq(&self, other: &Literal) -> bool {
        self.v == other.v
    }
}
impl PartialOrd for Literal {
    fn partial_cmp(&self, other: &Literal) -> Option<cmp::Ordering> {
        self.v.partial_cmp(&other.v)
    }
}
impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_cut() {
            write!(f, "Cut({})", escape_unicode(& self.v))
        } else {
            write!(f, "Complete({})", escape_unicode(& self.v))
        }
    }
}
impl AsRef<[u8]> for Literal {
    fn as_ref(&self) -> &[u8] {
        &self.v
    }
}
impl ops::Deref for Literal {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> {
        &self.v
    }
}
impl ops::DerefMut for Literal {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        &mut self.v
    }
}
fn position(needle: &[u8], mut haystack: &[u8]) -> Option<usize> {
    let mut i = 0;
    while haystack.len() >= needle.len() {
        if needle == &haystack[..needle.len()] {
            return Some(i);
        }
        i += 1;
        haystack = &haystack[1..];
    }
    None
}
fn escape_unicode(bytes: &[u8]) -> String {
    let show = match ::std::str::from_utf8(bytes) {
        Ok(v) => v.to_string(),
        Err(_) => escape_bytes(bytes),
    };
    let mut space_escaped = String::new();
    for c in show.chars() {
        if c.is_whitespace() {
            let escaped = if c as u32 <= 0x7F {
                escape_byte(c as u8)
            } else {
                if c as u32 <= 0xFFFF {
                    format!(r"\u{{{:04x}}}", c as u32)
                } else {
                    format!(r"\U{{{:08x}}}", c as u32)
                }
            };
            space_escaped.push_str(&escaped);
        } else {
            space_escaped.push(c);
        }
    }
    space_escaped
}
fn escape_bytes(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        s.push_str(&escape_byte(b));
    }
    s
}
fn escape_byte(byte: u8) -> String {
    use std::ascii::escape_default;
    let escaped: Vec<u8> = escape_default(byte).collect();
    String::from_utf8_lossy(&escaped).into_owned()
}
fn cls_char_count(cls: &hir::ClassUnicode) -> usize {
    cls.iter().map(|&r| 1 + (r.end as u32) - (r.start as u32)).sum::<u32>() as usize
}
fn cls_byte_count(cls: &hir::ClassBytes) -> usize {
    cls.iter().map(|&r| 1 + (r.end as u32) - (r.start as u32)).sum::<u32>() as usize
}
#[cfg(test)]
mod tests {
    use std::fmt;
    use super::{escape_bytes, Literal, Literals};
    use hir::Hir;
    use ParserBuilder;
    #[derive(Debug, Eq, PartialEq)]
    struct Bytes(Vec<ULiteral>);
    #[derive(Debug, Eq, PartialEq)]
    struct Unicode(Vec<ULiteral>);
    fn escape_lits(blits: &[Literal]) -> Vec<ULiteral> {
        let mut ulits = vec![];
        for blit in blits {
            ulits
                .push(ULiteral {
                    v: escape_bytes(&blit),
                    cut: blit.is_cut(),
                });
        }
        ulits
    }
    fn create_lits<I: IntoIterator<Item = Literal>>(it: I) -> Literals {
        Literals {
            lits: it.into_iter().collect(),
            limit_size: 0,
            limit_class: 0,
        }
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct ULiteral {
        v: String,
        cut: bool,
    }
    impl ULiteral {
        fn is_cut(&self) -> bool {
            self.cut
        }
    }
    impl fmt::Debug for ULiteral {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if self.is_cut() {
                write!(f, "Cut({})", self.v)
            } else {
                write!(f, "Complete({})", self.v)
            }
        }
    }
    impl PartialEq<Literal> for ULiteral {
        fn eq(&self, other: &Literal) -> bool {
            self.v.as_bytes() == &*other.v && self.is_cut() == other.is_cut()
        }
    }
    impl PartialEq<ULiteral> for Literal {
        fn eq(&self, other: &ULiteral) -> bool {
            &*self.v == other.v.as_bytes() && self.is_cut() == other.is_cut()
        }
    }
    #[allow(non_snake_case)]
    fn C(s: &'static str) -> ULiteral {
        ULiteral {
            v: s.to_owned(),
            cut: true,
        }
    }
    #[allow(non_snake_case)]
    fn M(s: &'static str) -> ULiteral {
        ULiteral {
            v: s.to_owned(),
            cut: false,
        }
    }
    fn prefixes(lits: &mut Literals, expr: &Hir) {
        lits.union_prefixes(expr);
    }
    fn suffixes(lits: &mut Literals, expr: &Hir) {
        lits.union_suffixes(expr);
    }
    macro_rules! assert_lit_eq {
        ($which:ident, $got_lits:expr, $($expected_lit:expr),*) => {
            { let expected : Vec < ULiteral > = vec![$($expected_lit),*]; let lits =
            $got_lits; assert_eq!($which (expected.clone()), $which (escape_lits(lits
            .literals()))); assert_eq!(! expected.is_empty() && expected.iter().all(| l |
            ! l.is_cut()), lits.all_complete()); assert_eq!(expected.iter().any(| l | ! l
            .is_cut()), lits.any_complete()); }
        };
    }
    macro_rules! test_lit {
        ($name:ident, $which:ident, $re:expr) => {
            test_lit!($name, $which, $re,);
        };
        ($name:ident, $which:ident, $re:expr, $($lit:expr),*) => {
            #[test] fn $name () { let expr = ParserBuilder::new().build().parse($re)
            .unwrap(); let lits = Literals::$which (& expr); assert_lit_eq!(Unicode,
            lits, $($lit),*); let expr = ParserBuilder::new().allow_invalid_utf8(true)
            .unicode(false).build().parse($re).unwrap(); let lits = Literals::$which (&
            expr); assert_lit_eq!(Bytes, lits, $($lit),*); }
        };
    }
    test_lit!(pfx_one_lit1, prefixes, "a", M("a"));
    test_lit!(pfx_one_lit2, prefixes, "abc", M("abc"));
    test_lit!(pfx_one_lit3, prefixes, "(?u)☃", M("\\xe2\\x98\\x83"));
    #[cfg(feature = "unicode-case")]
    test_lit!(pfx_one_lit4, prefixes, "(?ui)☃", M("\\xe2\\x98\\x83"));
    test_lit!(pfx_class1, prefixes, "[1-4]", M("1"), M("2"), M("3"), M("4"));
    test_lit!(
        pfx_class2, prefixes, "(?u)[☃Ⅰ]", M("\\xe2\\x85\\xa0"), M("\\xe2\\x98\\x83")
    );
    #[cfg(feature = "unicode-case")]
    test_lit!(
        pfx_class3, prefixes, "(?ui)[☃Ⅰ]", M("\\xe2\\x85\\xa0"),
        M("\\xe2\\x85\\xb0"), M("\\xe2\\x98\\x83")
    );
    test_lit!(pfx_one_lit_casei1, prefixes, "(?i-u)a", M("A"), M("a"));
    test_lit!(
        pfx_one_lit_casei2, prefixes, "(?i-u)abc", M("ABC"), M("aBC"), M("AbC"),
        M("abC"), M("ABc"), M("aBc"), M("Abc"), M("abc")
    );
    test_lit!(pfx_group1, prefixes, "(a)", M("a"));
    test_lit!(pfx_rep_zero_or_one1, prefixes, "a?");
    test_lit!(pfx_rep_zero_or_one2, prefixes, "(?:abc)?");
    test_lit!(pfx_rep_zero_or_more1, prefixes, "a*");
    test_lit!(pfx_rep_zero_or_more2, prefixes, "(?:abc)*");
    test_lit!(pfx_rep_one_or_more1, prefixes, "a+", C("a"));
    test_lit!(pfx_rep_one_or_more2, prefixes, "(?:abc)+", C("abc"));
    test_lit!(pfx_rep_nested_one_or_more, prefixes, "(?:a+)+", C("a"));
    test_lit!(pfx_rep_range1, prefixes, "a{0}");
    test_lit!(pfx_rep_range2, prefixes, "a{0,}");
    test_lit!(pfx_rep_range3, prefixes, "a{0,1}");
    test_lit!(pfx_rep_range4, prefixes, "a{1}", M("a"));
    test_lit!(pfx_rep_range5, prefixes, "a{2}", M("aa"));
    test_lit!(pfx_rep_range6, prefixes, "a{1,2}", C("a"));
    test_lit!(pfx_rep_range7, prefixes, "a{2,3}", C("aa"));
    test_lit!(pfx_cat1, prefixes, "(?:a)(?:b)", M("ab"));
    test_lit!(pfx_cat2, prefixes, "[ab]z", M("az"), M("bz"));
    test_lit!(
        pfx_cat3, prefixes, "(?i-u)[ab]z", M("AZ"), M("BZ"), M("aZ"), M("bZ"), M("Az"),
        M("Bz"), M("az"), M("bz")
    );
    test_lit!(pfx_cat4, prefixes, "[ab][yz]", M("ay"), M("by"), M("az"), M("bz"));
    test_lit!(pfx_cat5, prefixes, "a*b", C("a"), M("b"));
    test_lit!(pfx_cat6, prefixes, "a*b*c", C("a"), C("b"), M("c"));
    test_lit!(pfx_cat7, prefixes, "a*b*c+", C("a"), C("b"), C("c"));
    test_lit!(pfx_cat8, prefixes, "a*b+c", C("a"), C("b"));
    test_lit!(pfx_cat9, prefixes, "a*b+c*", C("a"), C("b"));
    test_lit!(pfx_cat10, prefixes, "ab*", C("ab"), M("a"));
    test_lit!(pfx_cat11, prefixes, "ab*c", C("ab"), M("ac"));
    test_lit!(pfx_cat12, prefixes, "ab+", C("ab"));
    test_lit!(pfx_cat13, prefixes, "ab+c", C("ab"));
    test_lit!(pfx_cat14, prefixes, "a^", C("a"));
    test_lit!(pfx_cat15, prefixes, "$a");
    test_lit!(pfx_cat16, prefixes, r"ab*c", C("ab"), M("ac"));
    test_lit!(pfx_cat17, prefixes, r"ab+c", C("ab"));
    test_lit!(pfx_cat18, prefixes, r"z*azb", C("z"), M("azb"));
    test_lit!(pfx_cat19, prefixes, "a.z", C("a"));
    test_lit!(pfx_alt1, prefixes, "a|b", M("a"), M("b"));
    test_lit!(pfx_alt2, prefixes, "[1-3]|b", M("1"), M("2"), M("3"), M("b"));
    test_lit!(pfx_alt3, prefixes, "y(?:a|b)z", M("yaz"), M("ybz"));
    test_lit!(pfx_alt4, prefixes, "a|b*");
    test_lit!(pfx_alt5, prefixes, "a|b+", M("a"), C("b"));
    test_lit!(pfx_alt6, prefixes, "a|(?:b|c*)");
    test_lit!(
        pfx_alt7, prefixes, "(a|b)*c|(a|ab)*c", C("a"), C("b"), M("c"), C("a"), C("ab"),
        M("c")
    );
    test_lit!(pfx_alt8, prefixes, "a*b|c", C("a"), M("b"), M("c"));
    test_lit!(pfx_empty1, prefixes, "^a", M("a"));
    test_lit!(pfx_empty2, prefixes, "a${2}", C("a"));
    test_lit!(pfx_empty3, prefixes, "^abc", M("abc"));
    test_lit!(pfx_empty4, prefixes, "(?:^abc)|(?:^z)", M("abc"), M("z"));
    test_lit!(pfx_nothing1, prefixes, ".");
    test_lit!(pfx_nothing2, prefixes, "(?s).");
    test_lit!(pfx_nothing3, prefixes, "^");
    test_lit!(pfx_nothing4, prefixes, "$");
    test_lit!(pfx_nothing6, prefixes, "(?m)$");
    test_lit!(pfx_nothing7, prefixes, r"\b");
    test_lit!(pfx_nothing8, prefixes, r"\B");
    test_lit!(pfx_defeated1, prefixes, ".a");
    test_lit!(pfx_defeated2, prefixes, "(?s).a");
    test_lit!(pfx_defeated3, prefixes, "a*b*c*");
    test_lit!(pfx_defeated4, prefixes, "a|.");
    test_lit!(pfx_defeated5, prefixes, ".|a");
    test_lit!(pfx_defeated6, prefixes, "a|^");
    test_lit!(pfx_defeated7, prefixes, ".(?:a(?:b)(?:c))");
    test_lit!(pfx_defeated8, prefixes, "$a");
    test_lit!(pfx_defeated9, prefixes, "(?m)$a");
    test_lit!(pfx_defeated10, prefixes, r"\ba");
    test_lit!(pfx_defeated11, prefixes, r"\Ba");
    test_lit!(pfx_defeated12, prefixes, "^*a");
    test_lit!(pfx_defeated13, prefixes, "^+a");
    test_lit!(
        pfx_crazy1, prefixes,
        r"M[ou]'?am+[ae]r .*([AEae]l[- ])?[GKQ]h?[aeu]+([dtz][dhz]?)+af[iy]",
        C("Mo\\'am"), C("Mu\\'am"), C("Moam"), C("Muam")
    );
    macro_rules! test_exhausted {
        ($name:ident, $which:ident, $re:expr) => {
            test_exhausted!($name, $which, $re,);
        };
        ($name:ident, $which:ident, $re:expr, $($lit:expr),*) => {
            #[test] fn $name () { let expr = ParserBuilder::new().build().parse($re)
            .unwrap(); let mut lits = Literals::empty(); lits.set_limit_size(20)
            .set_limit_class(10); $which (& mut lits, & expr); assert_lit_eq!(Unicode,
            lits, $($lit),*); let expr = ParserBuilder::new().allow_invalid_utf8(true)
            .unicode(false).build().parse($re).unwrap(); let mut lits =
            Literals::empty(); lits.set_limit_size(20).set_limit_class(10); $which (& mut
            lits, & expr); assert_lit_eq!(Bytes, lits, $($lit),*); }
        };
    }
    test_exhausted!(pfx_exhausted1, prefixes, "[a-z]");
    test_exhausted!(pfx_exhausted2, prefixes, "[a-z]*A");
    test_exhausted!(pfx_exhausted3, prefixes, "A[a-z]Z", C("A"));
    test_exhausted!(
        pfx_exhausted4, prefixes, "(?i-u)foobar", C("FO"), C("fO"), C("Fo"), C("fo")
    );
    test_exhausted!(pfx_exhausted5, prefixes, "(?:ab){100}", C("abababababababababab"));
    test_exhausted!(
        pfx_exhausted6, prefixes, "(?:(?:ab){100})*cd", C("ababababab"), M("cd")
    );
    test_exhausted!(
        pfx_exhausted7, prefixes, "z(?:(?:ab){100})*cd", C("zababababab"), M("zcd")
    );
    test_exhausted!(
        pfx_exhausted8, prefixes, "aaaaaaaaaaaaaaaaaaaaz", C("aaaaaaaaaaaaaaaaaaaa")
    );
    test_lit!(sfx_one_lit1, suffixes, "a", M("a"));
    test_lit!(sfx_one_lit2, suffixes, "abc", M("abc"));
    test_lit!(sfx_one_lit3, suffixes, "(?u)☃", M("\\xe2\\x98\\x83"));
    #[cfg(feature = "unicode-case")]
    test_lit!(sfx_one_lit4, suffixes, "(?ui)☃", M("\\xe2\\x98\\x83"));
    test_lit!(sfx_class1, suffixes, "[1-4]", M("1"), M("2"), M("3"), M("4"));
    test_lit!(
        sfx_class2, suffixes, "(?u)[☃Ⅰ]", M("\\xe2\\x85\\xa0"), M("\\xe2\\x98\\x83")
    );
    #[cfg(feature = "unicode-case")]
    test_lit!(
        sfx_class3, suffixes, "(?ui)[☃Ⅰ]", M("\\xe2\\x85\\xa0"),
        M("\\xe2\\x85\\xb0"), M("\\xe2\\x98\\x83")
    );
    test_lit!(sfx_one_lit_casei1, suffixes, "(?i-u)a", M("A"), M("a"));
    test_lit!(
        sfx_one_lit_casei2, suffixes, "(?i-u)abc", M("ABC"), M("ABc"), M("AbC"),
        M("Abc"), M("aBC"), M("aBc"), M("abC"), M("abc")
    );
    test_lit!(sfx_group1, suffixes, "(a)", M("a"));
    test_lit!(sfx_rep_zero_or_one1, suffixes, "a?");
    test_lit!(sfx_rep_zero_or_one2, suffixes, "(?:abc)?");
    test_lit!(sfx_rep_zero_or_more1, suffixes, "a*");
    test_lit!(sfx_rep_zero_or_more2, suffixes, "(?:abc)*");
    test_lit!(sfx_rep_one_or_more1, suffixes, "a+", C("a"));
    test_lit!(sfx_rep_one_or_more2, suffixes, "(?:abc)+", C("abc"));
    test_lit!(sfx_rep_nested_one_or_more, suffixes, "(?:a+)+", C("a"));
    test_lit!(sfx_rep_range1, suffixes, "a{0}");
    test_lit!(sfx_rep_range2, suffixes, "a{0,}");
    test_lit!(sfx_rep_range3, suffixes, "a{0,1}");
    test_lit!(sfx_rep_range4, suffixes, "a{1}", M("a"));
    test_lit!(sfx_rep_range5, suffixes, "a{2}", M("aa"));
    test_lit!(sfx_rep_range6, suffixes, "a{1,2}", C("a"));
    test_lit!(sfx_rep_range7, suffixes, "a{2,3}", C("aa"));
    test_lit!(sfx_cat1, suffixes, "(?:a)(?:b)", M("ab"));
    test_lit!(sfx_cat2, suffixes, "[ab]z", M("az"), M("bz"));
    test_lit!(
        sfx_cat3, suffixes, "(?i-u)[ab]z", M("AZ"), M("Az"), M("BZ"), M("Bz"), M("aZ"),
        M("az"), M("bZ"), M("bz")
    );
    test_lit!(sfx_cat4, suffixes, "[ab][yz]", M("ay"), M("az"), M("by"), M("bz"));
    test_lit!(sfx_cat5, suffixes, "a*b", C("ab"), M("b"));
    test_lit!(sfx_cat6, suffixes, "a*b*c", C("bc"), C("ac"), M("c"));
    test_lit!(sfx_cat7, suffixes, "a*b*c+", C("c"));
    test_lit!(sfx_cat8, suffixes, "a*b+c", C("bc"));
    test_lit!(sfx_cat9, suffixes, "a*b+c*", C("c"), C("b"));
    test_lit!(sfx_cat10, suffixes, "ab*", C("b"), M("a"));
    test_lit!(sfx_cat11, suffixes, "ab*c", C("bc"), M("ac"));
    test_lit!(sfx_cat12, suffixes, "ab+", C("b"));
    test_lit!(sfx_cat13, suffixes, "ab+c", C("bc"));
    test_lit!(sfx_cat14, suffixes, "a^");
    test_lit!(sfx_cat15, suffixes, "$a", C("a"));
    test_lit!(sfx_cat16, suffixes, r"ab*c", C("bc"), M("ac"));
    test_lit!(sfx_cat17, suffixes, r"ab+c", C("bc"));
    test_lit!(sfx_cat18, suffixes, r"z*azb", C("zazb"), M("azb"));
    test_lit!(sfx_cat19, suffixes, "a.z", C("z"));
    test_lit!(sfx_alt1, suffixes, "a|b", M("a"), M("b"));
    test_lit!(sfx_alt2, suffixes, "[1-3]|b", M("1"), M("2"), M("3"), M("b"));
    test_lit!(sfx_alt3, suffixes, "y(?:a|b)z", M("yaz"), M("ybz"));
    test_lit!(sfx_alt4, suffixes, "a|b*");
    test_lit!(sfx_alt5, suffixes, "a|b+", M("a"), C("b"));
    test_lit!(sfx_alt6, suffixes, "a|(?:b|c*)");
    test_lit!(
        sfx_alt7, suffixes, "(a|b)*c|(a|ab)*c", C("ac"), C("bc"), M("c"), C("ac"),
        C("abc"), M("c")
    );
    test_lit!(sfx_alt8, suffixes, "a*b|c", C("ab"), M("b"), M("c"));
    test_lit!(sfx_empty1, suffixes, "a$", M("a"));
    test_lit!(sfx_empty2, suffixes, "${2}a", C("a"));
    test_lit!(sfx_nothing1, suffixes, ".");
    test_lit!(sfx_nothing2, suffixes, "(?s).");
    test_lit!(sfx_nothing3, suffixes, "^");
    test_lit!(sfx_nothing4, suffixes, "$");
    test_lit!(sfx_nothing6, suffixes, "(?m)$");
    test_lit!(sfx_nothing7, suffixes, r"\b");
    test_lit!(sfx_nothing8, suffixes, r"\B");
    test_lit!(sfx_defeated1, suffixes, "a.");
    test_lit!(sfx_defeated2, suffixes, "(?s)a.");
    test_lit!(sfx_defeated3, suffixes, "a*b*c*");
    test_lit!(sfx_defeated4, suffixes, "a|.");
    test_lit!(sfx_defeated5, suffixes, ".|a");
    test_lit!(sfx_defeated6, suffixes, "a|^");
    test_lit!(sfx_defeated7, suffixes, "(?:a(?:b)(?:c)).");
    test_lit!(sfx_defeated8, suffixes, "a^");
    test_lit!(sfx_defeated9, suffixes, "(?m)a$");
    test_lit!(sfx_defeated10, suffixes, r"a\b");
    test_lit!(sfx_defeated11, suffixes, r"a\B");
    test_lit!(sfx_defeated12, suffixes, "a^*");
    test_lit!(sfx_defeated13, suffixes, "a^+");
    test_exhausted!(sfx_exhausted1, suffixes, "[a-z]");
    test_exhausted!(sfx_exhausted2, suffixes, "A[a-z]*");
    test_exhausted!(sfx_exhausted3, suffixes, "A[a-z]Z", C("Z"));
    test_exhausted!(
        sfx_exhausted4, suffixes, "(?i-u)foobar", C("AR"), C("Ar"), C("aR"), C("ar")
    );
    test_exhausted!(sfx_exhausted5, suffixes, "(?:ab){100}", C("abababababababababab"));
    test_exhausted!(
        sfx_exhausted6, suffixes, "cd(?:(?:ab){100})*", C("ababababab"), M("cd")
    );
    test_exhausted!(
        sfx_exhausted7, suffixes, "cd(?:(?:ab){100})*z", C("abababababz"), M("cdz")
    );
    test_exhausted!(
        sfx_exhausted8, suffixes, "zaaaaaaaaaaaaaaaaaaaa", C("aaaaaaaaaaaaaaaaaaaa")
    );
    macro_rules! test_unamb {
        ($name:ident, $given:expr, $expected:expr) => {
            #[test] fn $name () { let given : Vec < Literal > = $given .into_iter().map(|
            ul | { let cut = ul.is_cut(); Literal { v : ul.v.into_bytes(), cut : cut } })
            .collect(); let lits = create_lits(given); let got = lits
            .unambiguous_prefixes(); assert_eq!($expected, escape_lits(got.literals()));
            }
        };
    }
    test_unamb!(unambiguous1, vec![M("z"), M("azb")], vec![C("a"), C("z")]);
    test_unamb!(unambiguous2, vec![M("zaaaaaa"), M("aa")], vec![C("aa"), C("z")]);
    test_unamb!(
        unambiguous3, vec![M("Sherlock"), M("Watson")], vec![M("Sherlock"), M("Watson")]
    );
    test_unamb!(unambiguous4, vec![M("abc"), M("bc")], vec![C("a"), C("bc")]);
    test_unamb!(unambiguous5, vec![M("bc"), M("abc")], vec![C("a"), C("bc")]);
    test_unamb!(unambiguous6, vec![M("a"), M("aa")], vec![C("a")]);
    test_unamb!(unambiguous7, vec![M("aa"), M("a")], vec![C("a")]);
    test_unamb!(unambiguous8, vec![M("ab"), M("a")], vec![C("a")]);
    test_unamb!(
        unambiguous9, vec![M("ac"), M("bc"), M("c"), M("ac"), M("abc"), M("c")],
        vec![C("a"), C("b"), C("c")]
    );
    test_unamb!(
        unambiguous10, vec![M("Mo'"), M("Mu'"), M("Mo"), M("Mu")], vec![C("Mo"), C("Mu")]
    );
    test_unamb!(unambiguous11, vec![M("zazb"), M("azb")], vec![C("a"), C("z")]);
    test_unamb!(unambiguous12, vec![M("foo"), C("foo")], vec![C("foo")]);
    test_unamb!(
        unambiguous13, vec![M("ABCX"), M("CDAX"), M("BCX")], vec![C("A"), C("BCX"),
        C("CD")]
    );
    test_unamb!(
        unambiguous14, vec![M("IMGX"), M("MVIX"), M("MGX"), M("DSX")], vec![M("DSX"),
        C("I"), C("MGX"), C("MV")]
    );
    test_unamb!(
        unambiguous15, vec![M("IMG_"), M("MG_"), M("CIMG")], vec![C("C"), C("I"),
        C("MG_")]
    );
    macro_rules! test_trim {
        ($name:ident, $trim:expr, $given:expr, $expected:expr) => {
            #[test] fn $name () { let given : Vec < Literal > = $given .into_iter().map(|
            ul | { let cut = ul.is_cut(); Literal { v : ul.v.into_bytes(), cut : cut } })
            .collect(); let lits = create_lits(given); let got = lits.trim_suffix($trim)
            .unwrap(); assert_eq!($expected, escape_lits(got.literals())); }
        };
    }
    test_trim!(trim1, 1, vec![M("ab"), M("yz")], vec![C("a"), C("y")]);
    test_trim!(trim2, 1, vec![M("abc"), M("abd")], vec![C("ab")]);
    test_trim!(trim3, 2, vec![M("abc"), M("abd")], vec![C("a")]);
    test_trim!(trim4, 2, vec![M("abc"), M("ghij")], vec![C("a"), C("gh")]);
    macro_rules! test_lcp {
        ($name:ident, $given:expr, $expected:expr) => {
            #[test] fn $name () { let given : Vec < Literal > = $given .into_iter().map(|
            s : & str | Literal { v : s.to_owned().into_bytes(), cut : false, })
            .collect(); let lits = create_lits(given); let got = lits
            .longest_common_prefix(); assert_eq!($expected, escape_bytes(got)); }
        };
    }
    test_lcp!(lcp1, vec!["a"], "a");
    test_lcp!(lcp2, vec![], "");
    test_lcp!(lcp3, vec!["a", "b"], "");
    test_lcp!(lcp4, vec!["ab", "ab"], "ab");
    test_lcp!(lcp5, vec!["ab", "a"], "a");
    test_lcp!(lcp6, vec!["a", "ab"], "a");
    test_lcp!(lcp7, vec!["ab", "b"], "");
    test_lcp!(lcp8, vec!["b", "ab"], "");
    test_lcp!(lcp9, vec!["foobar", "foobaz"], "fooba");
    test_lcp!(lcp10, vec!["foobar", "foobaz", "a"], "");
    test_lcp!(lcp11, vec!["a", "foobar", "foobaz"], "");
    test_lcp!(lcp12, vec!["foo", "flub", "flab", "floo"], "f");
    macro_rules! test_lcs {
        ($name:ident, $given:expr, $expected:expr) => {
            #[test] fn $name () { let given : Vec < Literal > = $given .into_iter().map(|
            s : & str | Literal { v : s.to_owned().into_bytes(), cut : false, })
            .collect(); let lits = create_lits(given); let got = lits
            .longest_common_suffix(); assert_eq!($expected, escape_bytes(got)); }
        };
    }
    test_lcs!(lcs1, vec!["a"], "a");
    test_lcs!(lcs2, vec![], "");
    test_lcs!(lcs3, vec!["a", "b"], "");
    test_lcs!(lcs4, vec!["ab", "ab"], "ab");
    test_lcs!(lcs5, vec!["ab", "a"], "");
    test_lcs!(lcs6, vec!["a", "ab"], "");
    test_lcs!(lcs7, vec!["ab", "b"], "b");
    test_lcs!(lcs8, vec!["b", "ab"], "b");
    test_lcs!(lcs9, vec!["barfoo", "bazfoo"], "foo");
    test_lcs!(lcs10, vec!["barfoo", "bazfoo", "a"], "");
    test_lcs!(lcs11, vec!["a", "barfoo", "bazfoo"], "");
    test_lcs!(lcs12, vec!["flub", "bub", "boob", "dub"], "b");
}
#[cfg(test)]
mod tests_llm_16_69 {
    use crate::hir::literal::Literal;
    use std::cmp::PartialEq;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_69_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 1;
        let literal1 = Literal::new(vec![rug_fuzz_0, 2, 3]);
        let literal2 = Literal::new(vec![rug_fuzz_1, 2, 3]);
        let literal3 = Literal::new(vec![rug_fuzz_2, 2, 4]);
        debug_assert_eq!(literal1.eq(& literal2), true);
        debug_assert_eq!(literal1.eq(& literal3), false);
        let _rug_ed_tests_llm_16_69_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_70 {
    use super::*;
    use crate::*;
    use std::cmp::Ordering;
    #[test]
    fn test_partial_cmp() {
        let _rug_st_tests_llm_16_70_rrrruuuugggg_test_partial_cmp = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 97;
        let rug_fuzz_2 = 97;
        let literal1 = Literal::new(vec![rug_fuzz_0, 98, 99]);
        let literal2 = Literal::new(vec![rug_fuzz_1, 98, 99]);
        let literal3 = Literal::new(vec![rug_fuzz_2, 98, 99, 100]);
        let result1 = literal1.partial_cmp(&literal2);
        let result2 = literal2.partial_cmp(&literal3);
        let result3 = literal3.partial_cmp(&literal1);
        debug_assert_eq!(result1, Some(Ordering::Equal));
        debug_assert_eq!(result2, Some(Ordering::Less));
        debug_assert_eq!(result3, Some(Ordering::Greater));
        let _rug_ed_tests_llm_16_70_rrrruuuugggg_test_partial_cmp = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_71 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_ref() {
        let _rug_st_tests_llm_16_71_rrrruuuugggg_test_as_ref = 0;
        let rug_fuzz_0 = 97;
        let literal = Literal::new(vec![rug_fuzz_0, 98, 99]);
        let result: &[u8] = literal.as_ref();
        debug_assert_eq!(result, & [97, 98, 99]);
        let _rug_ed_tests_llm_16_71_rrrruuuugggg_test_as_ref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_72 {
    use super::*;
    use crate::*;
    #[test]
    fn test_deref() {
        let _rug_st_tests_llm_16_72_rrrruuuugggg_test_deref = 0;
        let rug_fuzz_0 = b"hello";
        let mut literal = Literal::new(rug_fuzz_0.to_vec());
        let deref_result: &[u8] = &*literal;
        debug_assert_eq!(deref_result, & b"hello"[..]);
        let _rug_ed_tests_llm_16_72_rrrruuuugggg_test_deref = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_451 {
    use crate::hir::literal::Literal;
    #[test]
    fn test_cut() {
        let _rug_st_tests_llm_16_451_rrrruuuugggg_test_cut = 0;
        let rug_fuzz_0 = 97;
        let mut literal = Literal::new(vec![rug_fuzz_0, 98, 99]);
        literal.cut();
        debug_assert_eq!(literal.is_cut(), true);
        let _rug_ed_tests_llm_16_451_rrrruuuugggg_test_cut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_452 {
    use super::*;
    use crate::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_452_rrrruuuugggg_test_empty = 0;
        let literal = Literal::empty();
        debug_assert_eq!(literal.v, vec![]);
        debug_assert_eq!(literal.cut, false);
        let _rug_ed_tests_llm_16_452_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_453 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_cut() {
        let _rug_st_tests_llm_16_453_rrrruuuugggg_test_is_cut = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = false;
        let literal = Literal::new(vec![rug_fuzz_0, 98, 99]);
        debug_assert_eq!(rug_fuzz_1, literal.is_cut());
        let _rug_ed_tests_llm_16_453_rrrruuuugggg_test_is_cut = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_454 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_454_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 97;
        let bytes = vec![rug_fuzz_0, 98, 99];
        let literal = Literal::new(bytes);
        debug_assert_eq!(* literal, vec![97, 98, 99]);
        debug_assert_eq!(literal.cut, false);
        let _rug_ed_tests_llm_16_454_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_456 {
    use super::*;
    use crate::*;
    use hir::literal::Literal;
    #[test]
    fn test_add_returns_true_when_capacity_sufficient() {
        let _rug_st_tests_llm_16_456_rrrruuuugggg_test_add_returns_true_when_capacity_sufficient = 0;
        let rug_fuzz_0 = b"test";
        let mut literals = Literals::empty();
        let lit = Literal::new(rug_fuzz_0.to_vec());
        let result = literals.add(lit);
        debug_assert_eq!(result, true);
        let _rug_ed_tests_llm_16_456_rrrruuuugggg_test_add_returns_true_when_capacity_sufficient = 0;
    }
    #[test]
    fn test_add_returns_false_when_capacity_exceeded() {
        let _rug_st_tests_llm_16_456_rrrruuuugggg_test_add_returns_false_when_capacity_exceeded = 0;
        let rug_fuzz_0 = 200;
        let rug_fuzz_1 = 10;
        let mut literals = Literals {
            lits: vec![Literal::new(vec![0; 200])],
            limit_size: rug_fuzz_0,
            limit_class: rug_fuzz_1,
        };
        let lit = Literal::new(vec![0; 100]);
        let result = literals.add(lit);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_456_rrrruuuugggg_test_add_returns_false_when_capacity_exceeded = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_460 {
    use super::*;
    use crate::*;
    use crate::hir::literal::Literal;
    use crate::hir::ClassUnicode;
    use crate::hir::ClassUnicodeRange;
    #[test]
    fn test_add_char_class() {
        let _rug_st_tests_llm_16_460_rrrruuuugggg_test_add_char_class = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'z';
        let mut set = Literals::empty();
        let class_unicode = ClassUnicode::new(
            vec![ClassUnicodeRange { start : rug_fuzz_0, end : rug_fuzz_1, }],
        );
        let result = set.add_char_class(&class_unicode);
        debug_assert!(result);
        let _rug_ed_tests_llm_16_460_rrrruuuugggg_test_add_char_class = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_463 {
    use super::*;
    use crate::*;
    #[test]
    fn test_all_complete_true() {
        let _rug_st_tests_llm_16_463_rrrruuuugggg_test_all_complete_true = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"def";
        let rug_fuzz_2 = b"ghi";
        let mut literals = Literals::empty();
        literals.add(Literal::new(rug_fuzz_0.to_vec()));
        literals.add(Literal::new(rug_fuzz_1.to_vec()));
        literals.add(Literal::new(rug_fuzz_2.to_vec()));
        debug_assert!(literals.all_complete());
        let _rug_ed_tests_llm_16_463_rrrruuuugggg_test_all_complete_true = 0;
    }
    #[test]
    fn test_all_complete_false() {
        let _rug_st_tests_llm_16_463_rrrruuuugggg_test_all_complete_false = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"def";
        let rug_fuzz_2 = b"ghi";
        let rug_fuzz_3 = 1;
        let mut literals = Literals::empty();
        literals.add(Literal::new(rug_fuzz_0.to_vec()));
        literals.add(Literal::new(rug_fuzz_1.to_vec()));
        literals.add(Literal::new(rug_fuzz_2.to_vec()));
        literals.lits[rug_fuzz_3].cut();
        debug_assert!(! literals.all_complete());
        let _rug_ed_tests_llm_16_463_rrrruuuugggg_test_all_complete_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_469 {
    use super::*;
    use crate::*;
    use crate::hir::literal::Literal;
    use crate::hir::literal::Literals;
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_469_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let mut literals = Literals {
            lits: vec![
                Literal::new(vec![rug_fuzz_0, 98, 99]), Literal::new(vec![100, 101, 102])
            ],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        literals.clear();
        debug_assert_eq!(literals.lits.len(), 0);
        let _rug_ed_tests_llm_16_469_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_470 {
    use super::*;
    use crate::*;
    #[test]
    fn test_contains_empty_true() {
        let _rug_st_tests_llm_16_470_rrrruuuugggg_test_contains_empty_true = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 10;
        let literals = Literals {
            lits: vec![Literal::empty()],
            limit_size: rug_fuzz_0,
            limit_class: rug_fuzz_1,
        };
        debug_assert_eq!(literals.contains_empty(), true);
        let _rug_ed_tests_llm_16_470_rrrruuuugggg_test_contains_empty_true = 0;
    }
    #[test]
    fn test_contains_empty_false() {
        let _rug_st_tests_llm_16_470_rrrruuuugggg_test_contains_empty_false = 0;
        let rug_fuzz_0 = b"hello";
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 10;
        let literals = Literals {
            lits: vec![Literal::new(rug_fuzz_0.to_vec())],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        debug_assert_eq!(literals.contains_empty(), false);
        let _rug_ed_tests_llm_16_470_rrrruuuugggg_test_contains_empty_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_473 {
    use super::*;
    use crate::*;
    use std::cmp;
    #[test]
    fn test_cross_product() {
        let _rug_st_tests_llm_16_473_rrrruuuugggg_test_cross_product = 0;
        let mut literals = Literals::empty();
        let literals2 = Literals::empty();
        literals.cross_product(&literals2);
        let _rug_ed_tests_llm_16_473_rrrruuuugggg_test_cross_product = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_477 {
    use super::*;
    use crate::*;
    #[test]
    fn test_empty() {
        let _rug_st_tests_llm_16_477_rrrruuuugggg_test_empty = 0;
        let result: Literals = Literals::empty();
        debug_assert_eq!(result.lits, Vec:: < Literal > ::new());
        debug_assert_eq!(result.lits.len(), 0);
        debug_assert_eq!(result.limit_size, 250);
        debug_assert_eq!(result.limit_class, 10);
        let _rug_ed_tests_llm_16_477_rrrruuuugggg_test_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_478 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_empty_empty_set() {
        let _rug_st_tests_llm_16_478_rrrruuuugggg_test_is_empty_empty_set = 0;
        let literals = Literals::empty();
        debug_assert!(literals.is_empty());
        let _rug_ed_tests_llm_16_478_rrrruuuugggg_test_is_empty_empty_set = 0;
    }
    #[test]
    fn test_is_empty_non_empty_set() {
        let _rug_st_tests_llm_16_478_rrrruuuugggg_test_is_empty_non_empty_set = 0;
        let rug_fuzz_0 = b"test";
        let mut literals = Literals::empty();
        let lit = Literal::new(rug_fuzz_0.to_vec());
        literals.add(lit);
        debug_assert!(! literals.is_empty());
        let _rug_ed_tests_llm_16_478_rrrruuuugggg_test_is_empty_non_empty_set = 0;
    }
    #[test]
    fn test_is_empty_all_complete() {
        let _rug_st_tests_llm_16_478_rrrruuuugggg_test_is_empty_all_complete = 0;
        let rug_fuzz_0 = b"test1";
        let rug_fuzz_1 = b"test2";
        let mut literals = Literals::empty();
        let lit1 = Literal::new(rug_fuzz_0.to_vec());
        let lit2 = Literal::new(rug_fuzz_1.to_vec());
        literals.add(lit1);
        literals.add(lit2);
        literals.cut();
        debug_assert!(literals.is_empty());
        let _rug_ed_tests_llm_16_478_rrrruuuugggg_test_is_empty_all_complete = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_479 {
    use super::*;
    use crate::*;
    #[test]
    fn test_limit_class() {
        let _rug_st_tests_llm_16_479_rrrruuuugggg_test_limit_class = 0;
        let literals = Literals::empty();
        let result = literals.limit_class();
        debug_assert_eq!(result, 10);
        let _rug_ed_tests_llm_16_479_rrrruuuugggg_test_limit_class = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_480 {
    use super::*;
    use crate::*;
    #[test]
    fn test_limit_size() {
        let _rug_st_tests_llm_16_480_rrrruuuugggg_test_limit_size = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 5;
        let lits = Literals {
            lits: vec![],
            limit_size: rug_fuzz_0,
            limit_class: rug_fuzz_1,
        };
        debug_assert_eq!(lits.limit_size(), 100);
        let _rug_ed_tests_llm_16_480_rrrruuuugggg_test_limit_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_481 {
    use super::*;
    use crate::*;
    #[test]
    fn test_literals() {
        let _rug_st_tests_llm_16_481_rrrruuuugggg_test_literals = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 65;
        let rug_fuzz_4 = 49;
        let literals = Literals {
            lits: vec![
                Literal::new(vec![rug_fuzz_0, 66, 67]), Literal::new(vec![49, 50, 51])
            ],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        let expected = &[
            Literal::new(vec![rug_fuzz_3, 66, 67]),
            Literal::new(vec![rug_fuzz_4, 50, 51]),
        ];
        debug_assert_eq!(literals.literals(), expected);
        let _rug_ed_tests_llm_16_481_rrrruuuugggg_test_literals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_486 {
    use super::*;
    use crate::*;
    #[test]
    fn test_min_len_empty_set() {
        let _rug_st_tests_llm_16_486_rrrruuuugggg_test_min_len_empty_set = 0;
        let literals = Literals::empty();
        debug_assert_eq!(literals.min_len(), None);
        let _rug_ed_tests_llm_16_486_rrrruuuugggg_test_min_len_empty_set = 0;
    }
    #[test]
    fn test_min_len_single_literal() {
        let _rug_st_tests_llm_16_486_rrrruuuugggg_test_min_len_single_literal = 0;
        let rug_fuzz_0 = b'a';
        let mut literals = Literals::empty();
        literals.add(Literal::new(vec![rug_fuzz_0, b'b', b'c']));
        debug_assert_eq!(literals.min_len(), Some(3));
        let _rug_ed_tests_llm_16_486_rrrruuuugggg_test_min_len_single_literal = 0;
    }
    #[test]
    fn test_min_len_multiple_literals() {
        let _rug_st_tests_llm_16_486_rrrruuuugggg_test_min_len_multiple_literals = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'x';
        let rug_fuzz_2 = b'1';
        let mut literals = Literals::empty();
        literals.add(Literal::new(vec![rug_fuzz_0, b'b', b'c']));
        literals.add(Literal::new(vec![rug_fuzz_1, b'y']));
        literals.add(Literal::new(vec![rug_fuzz_2, b'2', b'3', b'4', b'5']));
        debug_assert_eq!(literals.min_len(), Some(2));
        let _rug_ed_tests_llm_16_486_rrrruuuugggg_test_min_len_multiple_literals = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_488 {
    use super::*;
    use crate::*;
    use crate::hir::literal::Literal;
    #[test]
    fn test_num_bytes() {
        let _rug_st_tests_llm_16_488_rrrruuuugggg_test_num_bytes = 0;
        let mut literals = Literals::empty();
        let lit1 = Literal::empty();
        let lit2 = Literal::empty();
        literals.add(lit1);
        literals.add(lit2);
        debug_assert_eq!(literals.num_bytes(), 0);
        let _rug_ed_tests_llm_16_488_rrrruuuugggg_test_num_bytes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_489 {
    use super::*;
    use crate::*;
    #[test]
    fn test_remove_complete() {
        let _rug_st_tests_llm_16_489_rrrruuuugggg_test_remove_complete = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 1;
        let mut literals = Literals {
            lits: vec![
                Literal::new(vec![rug_fuzz_0, 2, 3]), Literal::new(vec![4, 5, 6]),
                Literal::new(vec![7, 8, 9]), Literal::new(vec![10, 11, 12])
            ],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        let result = literals.remove_complete();
        let expected_result = vec![
            Literal::new(vec![rug_fuzz_3, 2, 3]), Literal::new(vec![4, 5, 6])
        ];
        debug_assert_eq!(result, expected_result);
        let _rug_ed_tests_llm_16_489_rrrruuuugggg_test_remove_complete = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_490 {
    use super::*;
    use crate::*;
    #[test]
    fn test_reverse() {
        let _rug_st_tests_llm_16_490_rrrruuuugggg_test_reverse = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"def";
        let rug_fuzz_2 = b"ghi";
        let rug_fuzz_3 = b"cba";
        let mut literals = Literals::empty();
        literals.add(Literal::new(rug_fuzz_0.to_vec()));
        literals.add(Literal::new(rug_fuzz_1.to_vec()));
        literals.add(Literal::new(rug_fuzz_2.to_vec()));
        literals.reverse();
        let expected_literals = vec![
            Literal::new(rug_fuzz_3.to_vec()), Literal::new(b"fed".to_vec()),
            Literal::new(b"ihg".to_vec())
        ];
        debug_assert_eq!(literals.literals(), expected_literals);
        let _rug_ed_tests_llm_16_490_rrrruuuugggg_test_reverse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_491 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_limit_class() {
        let _rug_st_tests_llm_16_491_rrrruuuugggg_test_set_limit_class = 0;
        let rug_fuzz_0 = 5;
        let mut literals = Literals::empty();
        literals.set_limit_class(rug_fuzz_0);
        debug_assert_eq!(literals.limit_class, 5);
        let _rug_ed_tests_llm_16_491_rrrruuuugggg_test_set_limit_class = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_492 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_limit_size() {
        let _rug_st_tests_llm_16_492_rrrruuuugggg_test_set_limit_size = 0;
        let rug_fuzz_0 = 500;
        let mut literals = Literals::empty();
        literals.set_limit_size(rug_fuzz_0);
        debug_assert_eq!(literals.limit_size(), 500);
        let _rug_ed_tests_llm_16_492_rrrruuuugggg_test_set_limit_size = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_495 {
    use super::*;
    use crate::*;
    #[test]
    fn test_trim_suffix_none() {
        let _rug_st_tests_llm_16_495_rrrruuuugggg_test_trim_suffix_none = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 4;
        let literals = Literals {
            lits: vec![Literal::new(rug_fuzz_0.to_vec())],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        debug_assert_eq!(literals.trim_suffix(rug_fuzz_3), None);
        let _rug_ed_tests_llm_16_495_rrrruuuugggg_test_trim_suffix_none = 0;
    }
    #[test]
    fn test_trim_suffix_some() {
        let _rug_st_tests_llm_16_495_rrrruuuugggg_test_trim_suffix_some = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = b"ab";
        let rug_fuzz_4 = 100;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 1;
        let literals = Literals {
            lits: vec![Literal::new(rug_fuzz_0.to_vec())],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        let expected = Literals {
            lits: vec![Literal::new(rug_fuzz_3.to_vec())],
            limit_size: rug_fuzz_4,
            limit_class: rug_fuzz_5,
        };
        debug_assert_eq!(literals.trim_suffix(rug_fuzz_6), Some(expected));
        let _rug_ed_tests_llm_16_495_rrrruuuugggg_test_trim_suffix_some = 0;
    }
    #[test]
    fn test_trim_suffix_empty() {
        let _rug_st_tests_llm_16_495_rrrruuuugggg_test_trim_suffix_empty = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 100;
        let rug_fuzz_3 = 10;
        let rug_fuzz_4 = 4;
        let literals = Literals {
            lits: vec![Literal::empty()],
            limit_size: rug_fuzz_0,
            limit_class: rug_fuzz_1,
        };
        let expected = Literals {
            lits: vec![Literal::empty()],
            limit_size: rug_fuzz_2,
            limit_class: rug_fuzz_3,
        };
        debug_assert_eq!(literals.trim_suffix(rug_fuzz_4), Some(expected));
        let _rug_ed_tests_llm_16_495_rrrruuuugggg_test_trim_suffix_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_498 {
    use crate::hir::literal::{Literal, Literals};
    #[test]
    fn test_unambiguous_suffixes() {
        let _rug_st_tests_llm_16_498_rrrruuuugggg_test_unambiguous_suffixes = 0;
        let rug_fuzz_0 = 97;
        let rug_fuzz_1 = 100;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = 99;
        let rug_fuzz_4 = 100;
        let rug_fuzz_5 = 10;
        let literals = Literals {
            lits: vec![
                Literal::new(vec![rug_fuzz_0, 98, 99]), Literal::new(vec![98, 99, 100]),
                Literal::new(vec![99, 100, 101])
            ],
            limit_size: rug_fuzz_1,
            limit_class: rug_fuzz_2,
        };
        let expected = Literals {
            lits: vec![
                Literal::new(vec![rug_fuzz_3, 100, 101]), Literal::new(vec![98, 99,
                100]), Literal::new(vec![97, 98, 99])
            ],
            limit_size: rug_fuzz_4,
            limit_class: rug_fuzz_5,
        };
        debug_assert_eq!(literals.unambiguous_suffixes(), expected);
        let _rug_ed_tests_llm_16_498_rrrruuuugggg_test_unambiguous_suffixes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_504 {
    use super::*;
    use crate::*;
    #[test]
    fn test_escape_byte() {
        let _rug_st_tests_llm_16_504_rrrruuuugggg_test_escape_byte = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 7;
        let rug_fuzz_2 = 8;
        let rug_fuzz_3 = 9;
        let rug_fuzz_4 = 10;
        let rug_fuzz_5 = 11;
        let rug_fuzz_6 = 12;
        let rug_fuzz_7 = 13;
        let rug_fuzz_8 = 34;
        let rug_fuzz_9 = 92;
        let rug_fuzz_10 = 127;
        let rug_fuzz_11 = 128;
        let rug_fuzz_12 = 255;
        debug_assert_eq!(escape_byte(rug_fuzz_0), "\\0");
        debug_assert_eq!(escape_byte(rug_fuzz_1), "\\a");
        debug_assert_eq!(escape_byte(rug_fuzz_2), "\\t");
        debug_assert_eq!(escape_byte(rug_fuzz_3), "\\n");
        debug_assert_eq!(escape_byte(rug_fuzz_4), "\\r");
        debug_assert_eq!(escape_byte(rug_fuzz_5), "\\v");
        debug_assert_eq!(escape_byte(rug_fuzz_6), "\\f");
        debug_assert_eq!(escape_byte(rug_fuzz_7), "\\'");
        debug_assert_eq!(escape_byte(rug_fuzz_8), "\\\"");
        debug_assert_eq!(escape_byte(rug_fuzz_9), "\\\\");
        debug_assert_eq!(escape_byte(rug_fuzz_10), "\\x7f");
        debug_assert_eq!(escape_byte(rug_fuzz_11), "\\x80");
        debug_assert_eq!(escape_byte(rug_fuzz_12), "\\xff");
        let _rug_ed_tests_llm_16_504_rrrruuuugggg_test_escape_byte = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_507 {
    use super::*;
    use crate::*;
    #[test]
    fn test_escape_unicode() {
        let _rug_st_tests_llm_16_507_rrrruuuugggg_test_escape_unicode = 0;
        let rug_fuzz_0 = 104;
        let rug_fuzz_1 = 101;
        let rug_fuzz_2 = 108;
        let rug_fuzz_3 = 108;
        let rug_fuzz_4 = 111;
        let rug_fuzz_5 = 240;
        let rug_fuzz_6 = 159;
        let rug_fuzz_7 = 145;
        let rug_fuzz_8 = 135;
        let rug_fuzz_9 = 226;
        let rug_fuzz_10 = 128;
        let rug_fuzz_11 = 166;
        let rug_fuzz_12 = 195;
        let rug_fuzz_13 = 177;
        let rug_fuzz_14 = 230;
        let rug_fuzz_15 = 152;
        let rug_fuzz_16 = 175;
        let rug_fuzz_17 = 240;
        let rug_fuzz_18 = 159;
        let rug_fuzz_19 = 152;
        let rug_fuzz_20 = 128;
        let rug_fuzz_21 = 195;
        let rug_fuzz_22 = 177;
        let rug_fuzz_23 = 226;
        let rug_fuzz_24 = 128;
        let rug_fuzz_25 = 166;
        let rug_fuzz_26 = 195;
        let rug_fuzz_27 = 177;
        let rug_fuzz_28 = 226;
        let rug_fuzz_29 = 128;
        let rug_fuzz_30 = 166;
        let rug_fuzz_31 = 240;
        let rug_fuzz_32 = 159;
        let rug_fuzz_33 = 145;
        let rug_fuzz_34 = 135;
        let input1: &[u8] = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
        ];
        debug_assert_eq!(escape_unicode(input1), "hello");
        let input2: &[u8] = &[rug_fuzz_5, rug_fuzz_6, rug_fuzz_7, rug_fuzz_8];
        debug_assert_eq!(escape_unicode(input2), "\\u{1f497}");
        let input3: &[u8] = &[rug_fuzz_9, rug_fuzz_10, rug_fuzz_11];
        debug_assert_eq!(escape_unicode(input3), "\\u{a6}");
        let input4: &[u8] = &[rug_fuzz_12, rug_fuzz_13];
        debug_assert_eq!(escape_unicode(input4), "\\u{f1}");
        let input5: &[u8] = &[rug_fuzz_14, rug_fuzz_15, rug_fuzz_16];
        debug_assert_eq!(escape_unicode(input5), "\\u{65e5}");
        let input6: &[u8] = &[rug_fuzz_17, rug_fuzz_18, rug_fuzz_19, rug_fuzz_20];
        debug_assert_eq!(escape_unicode(input6), "\\u{1f400}");
        let input7: &[u8] = &[
            rug_fuzz_21,
            rug_fuzz_22,
            rug_fuzz_23,
            rug_fuzz_24,
            rug_fuzz_25,
        ];
        debug_assert_eq!(escape_unicode(input7), "\\u{f1}\\u{a6}");
        let input8: &[u8] = &[
            rug_fuzz_26,
            rug_fuzz_27,
            rug_fuzz_28,
            rug_fuzz_29,
            rug_fuzz_30,
            rug_fuzz_31,
            rug_fuzz_32,
            rug_fuzz_33,
            rug_fuzz_34,
        ];
        debug_assert_eq!(escape_unicode(input8), "\\u{f1}\\u{a6}\\u{1f497}");
        let _rug_ed_tests_llm_16_507_rrrruuuugggg_test_escape_unicode = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_508 {
    use super::*;
    use crate::*;
    #[test]
    fn test_position_match() {
        let _rug_st_tests_llm_16_508_rrrruuuugggg_test_position_match = 0;
        let rug_fuzz_0 = b"def";
        let rug_fuzz_1 = b"abcdef";
        let needle = rug_fuzz_0;
        let haystack = rug_fuzz_1;
        debug_assert_eq!(position(needle, haystack), Some(3));
        let _rug_ed_tests_llm_16_508_rrrruuuugggg_test_position_match = 0;
    }
    #[test]
    fn test_position_no_match() {
        let _rug_st_tests_llm_16_508_rrrruuuugggg_test_position_no_match = 0;
        let rug_fuzz_0 = b"xyz";
        let rug_fuzz_1 = b"abcdef";
        let needle = rug_fuzz_0;
        let haystack = rug_fuzz_1;
        debug_assert_eq!(position(needle, haystack), None);
        let _rug_ed_tests_llm_16_508_rrrruuuugggg_test_position_no_match = 0;
    }
    #[test]
    fn test_position_empty_needle() {
        let _rug_st_tests_llm_16_508_rrrruuuugggg_test_position_empty_needle = 0;
        let rug_fuzz_0 = b"";
        let rug_fuzz_1 = b"abcdef";
        let needle = rug_fuzz_0;
        let haystack = rug_fuzz_1;
        debug_assert_eq!(position(needle, haystack), Some(0));
        let _rug_ed_tests_llm_16_508_rrrruuuugggg_test_position_empty_needle = 0;
    }
    #[test]
    fn test_position_empty_haystack() {
        let _rug_st_tests_llm_16_508_rrrruuuugggg_test_position_empty_haystack = 0;
        let rug_fuzz_0 = b"abc";
        let rug_fuzz_1 = b"";
        let needle = rug_fuzz_0;
        let haystack = rug_fuzz_1;
        debug_assert_eq!(position(needle, haystack), None);
        let _rug_ed_tests_llm_16_508_rrrruuuugggg_test_position_empty_haystack = 0;
    }
    #[test]
    fn test_position_empty_needle_haystack() {
        let _rug_st_tests_llm_16_508_rrrruuuugggg_test_position_empty_needle_haystack = 0;
        let rug_fuzz_0 = b"";
        let rug_fuzz_1 = b"";
        let needle = rug_fuzz_0;
        let haystack = rug_fuzz_1;
        debug_assert_eq!(position(needle, haystack), Some(0));
        let _rug_ed_tests_llm_16_508_rrrruuuugggg_test_position_empty_needle_haystack = 0;
    }
}
#[cfg(test)]
mod tests_rug_110 {
    use super::*;
    use crate::hir::{Hir, Literal, Class, RepetitionKind, RepetitionRange, Anchor};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_110_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        let mut p1 = Literals::empty();
        suffixes(&p0, &mut p1);
        let _rug_ed_tests_rug_110_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_115 {
    use super::*;
    use crate::hir::literal::{Hir, Literals};
    #[test]
    fn test_alternate_literals() {
        let _rug_st_tests_rug_115_rrrruuuugggg_test_alternate_literals = 0;
        let mut v49: Vec<Hir> = Vec::new();
        let mut v42 = Literals::empty();
        let mut f = |hir: &Hir, lits: &mut Literals| {};
        crate::hir::literal::alternate_literals(&v49, &mut v42, f);
        let _rug_ed_tests_rug_115_rrrruuuugggg_test_alternate_literals = 0;
    }
}
#[cfg(test)]
mod tests_rug_116 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_116_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b"Hello, World!";
        let p0 = rug_fuzz_0;
        crate::hir::literal::escape_bytes(p0);
        let _rug_ed_tests_rug_116_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_117 {
    use super::*;
    use crate::hir::{ClassUnicode, ClassUnicodeRange};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_117_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = '\u{0041}';
        let rug_fuzz_1 = '\u{0043}';
        let mut p0 = ClassUnicode::new(
            vec![
                ClassUnicodeRange { start : rug_fuzz_0, end : rug_fuzz_1 },
                ClassUnicodeRange { start : '\u{004D}', end : '\u{004F}' },
                ClassUnicodeRange { start : '\u{0061}', end : '\u{0063}' },
                ClassUnicodeRange { start : '\u{006D}', end : '\u{006F}' }
            ],
        );
        hir::literal::cls_char_count(&p0);
        let _rug_ed_tests_rug_117_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_118 {
    use super::*;
    use crate::hir::{ClassBytes, ClassBytesRange};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_118_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let mut p0 = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'A',
                b'Z')
            ],
        );
        crate::hir::literal::cls_byte_count(&p0);
        let _rug_ed_tests_rug_118_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_120 {
    use super::*;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_120_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        use crate::hir::*;
        let mut p0 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        <hir::literal::Literals>::suffixes(&p0);
        let _rug_ed_tests_rug_120_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_121 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_121_rrrruuuugggg_test_rug = 0;
        let mut p0 = Literals::empty();
        p0.any_complete();
        let _rug_ed_tests_rug_121_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_122 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_122_rrrruuuugggg_test_rug = 0;
        let mut p0 = Literals::empty();
        <hir::literal::Literals>::to_empty(&p0);
        let _rug_ed_tests_rug_122_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_123 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_123_rrrruuuugggg_test_rug = 0;
        let mut p0 = Literals::empty();
        p0.longest_common_suffix();
        let _rug_ed_tests_rug_123_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_124 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_124_rrrruuuugggg_test_rug = 0;
        let mut p0 = Literals::empty();
        Literals::unambiguous_prefixes(&mut p0);
        let _rug_ed_tests_rug_124_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_125 {
    use super::*;
    use crate::hir::literal::Literals;
    use crate::hir::{Hir, Literal};
    #[test]
    fn test_union_prefixes() {
        let _rug_st_tests_rug_125_rrrruuuugggg_test_union_prefixes = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Literals::empty();
        let v41 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        let mut p1 = v41.clone();
        let result = p0.union_prefixes(&v41);
        debug_assert!(result);
        let _rug_ed_tests_rug_125_rrrruuuugggg_test_union_prefixes = 0;
    }
}
#[cfg(test)]
mod tests_rug_126 {
    use super::*;
    use crate::hir::literal::Literals;
    use crate::hir::{Hir, Literal};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_126_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Literals::empty();
        let mut p1 = Hir::literal(Literal::Unicode(rug_fuzz_0));
        Literals::union_suffixes(&mut p0, &p1);
        let _rug_ed_tests_rug_126_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_127 {
    use super::*;
    use crate::hir::literal::{Literal, Literals};
    #[test]
    fn test_union() {
        let _rug_st_tests_rug_127_rrrruuuugggg_test_union = 0;
        let mut p0 = Literals::empty();
        let mut p1 = Literals::empty();
        p0.union(p1);
        let _rug_ed_tests_rug_127_rrrruuuugggg_test_union = 0;
    }
}
#[cfg(test)]
mod tests_rug_128 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_cross_add() {
        let _rug_st_tests_rug_128_rrrruuuugggg_test_cross_add = 0;
        let rug_fuzz_0 = b"sample_data";
        let mut p0 = Literals::empty();
        let p1: &[u8] = rug_fuzz_0;
        p0.cross_add(p1);
        let _rug_ed_tests_rug_128_rrrruuuugggg_test_cross_add = 0;
    }
}
#[cfg(test)]
mod tests_rug_129 {
    use super::*;
    use crate::hir::literal::Literals;
    use crate::hir::ClassUnicode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_129_rrrruuuugggg_test_rug = 0;
        let mut p0 = Literals::empty();
        let p1 = hir::ClassUnicode::empty();
        <hir::literal::Literals>::add_char_class_reverse(&mut p0, &p1);
        let _rug_ed_tests_rug_129_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_130 {
    use super::*;
    use crate::hir::literal::Literals;
    use crate::hir::ClassUnicode;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_130_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = false;
        let mut p0 = Literals::empty();
        let p1 = ClassUnicode::empty();
        let p2 = rug_fuzz_0;
        crate::hir::literal::Literals::_add_char_class(&mut p0, &p1, p2);
        let _rug_ed_tests_rug_130_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_131 {
    use super::*;
    use crate::hir::literal::Literals;
    use crate::hir::{ClassBytes, ClassBytesRange};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_131_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = b'a';
        let rug_fuzz_1 = b'z';
        let mut p0 = Literals::empty();
        let mut p1 = ClassBytes::new(
            vec![
                ClassBytesRange::new(rug_fuzz_0, rug_fuzz_1), ClassBytesRange::new(b'A',
                b'Z')
            ],
        );
        Literals::add_byte_class(&mut p0, &p1);
        let _rug_ed_tests_rug_131_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_132 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_132_rrrruuuugggg_test_rug = 0;
        let mut p0 = Literals::empty();
        <hir::literal::Literals>::cut(&mut p0);
        let _rug_ed_tests_rug_132_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_133 {
    use super::*;
    use crate::hir::literal::Literals;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_133_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = Literals::empty();
        let p1: usize = rug_fuzz_0;
        p0.class_exceeds_limits(p1);
        let _rug_ed_tests_rug_133_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_134 {
    use super::*;
    use crate::std::ops::DerefMut;
    use crate::hir::literal::Literal;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_134_rrrruuuugggg_sample = 0;
        #[cfg(test)]
        mod tests_rug_134_prepare {
            #[test]
            fn sample() {
                let _rug_st_tests_rug_134_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = 0;
                let _rug_st_tests_rug_134_rrrruuuugggg_sample = rug_fuzz_0;
                use crate::hir::literal::Literal;
                let mut v53: Literal = Literal::empty();
                let _rug_ed_tests_rug_134_rrrruuuugggg_sample = rug_fuzz_1;
                let _rug_ed_tests_rug_134_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let mut p0: Literal = Literal::empty();
        <hir::literal::Literal as std::ops::DerefMut>::deref_mut(&mut p0);
        let _rug_ed_tests_rug_134_rrrruuuugggg_sample = 0;
    }
}
