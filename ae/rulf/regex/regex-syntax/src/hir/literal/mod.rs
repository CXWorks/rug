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
        Literals { lits: vec![], limit_size: 250, limit_class: 10 }
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
                lit.iter()
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
                    // If the literal is already in the set, then we can
                    // just drop it. But make sure that cut literals are
                    // infectious!
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
                // Oops, the candidate is already represented in the set.
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
        // This is a touch wasteful...
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
        // Check that we make sure we stay in our limits.
        let mut size_after;
        if self.is_empty() || !self.any_complete() {
            size_after = self.num_bytes();
            for lits_lit in lits.literals() {
                size_after += lits_lit.len();
            }
        } else {
            size_after = self.lits.iter().fold(0, |accum, lit| {
                accum + if lit.is_cut() { lit.len() } else { 0 }
            });
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
        // N.B. This could be implemented by simply calling cross_product with
        // a literal set containing just `bytes`, but we can be smarter about
        // taking shorter prefixes of `bytes` if they'll fit.
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
        while size + (i * self.lits.len()) <= self.limit_size
            && i < bytes.len()
        {
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

    fn _add_char_class(
        &mut self,
        cls: &hir::ClassUnicode,
        reverse: bool,
    ) -> bool {
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
        // This is an approximation since codepoints in a char class can encode
        // to 1-4 bytes.
        let new_byte_count = if self.lits.is_empty() {
            size
        } else {
            self.lits.iter().fold(0, |accum, lit| {
                accum
                    + if lit.is_cut() {
                        // If the literal is cut, then we'll never add
                        // anything to it, so don't count it.
                        0
                    } else {
                        (lit.len() + 1) * size
                    }
            })
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
        HirKind::Repetition(ref x) => match x.kind {
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
                repeat_range_literals(
                    &x.hir, min, max, x.greedy, lits, prefixes,
                )
            }
        },
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
                    // If this expression couldn't yield any literal that
                    // could be extended, then we need to quit. Since we're
                    // short-circuiting, we also need to freeze every member.
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
        HirKind::Repetition(ref x) => match x.kind {
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
                repeat_range_literals(
                    &x.hir, min, max, x.greedy, lits, suffixes,
                )
            }
        },
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
                    // If this expression couldn't yield any literal that
                    // could be extended, then we need to quit. Since we're
                    // short-circuiting, we also need to freeze every member.
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
        // This is a bit conservative. If `max` is set, then we could
        // treat this as a finite set of alternations. For now, we
        // just treat it as `e*`.
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
            // If we couldn't find suffixes for *any* of the
            // alternates, then the entire alternation has to be thrown
            // away and any existing members must be frozen. Similarly,
            // if the union couldn't complete, stop and freeze.
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
            write!(f, "Cut({})", escape_unicode(&self.v))
        } else {
            write!(f, "Complete({})", escape_unicode(&self.v))
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
    cls.iter().map(|&r| 1 + (r.end as u32) - (r.start as u32)).sum::<u32>()
        as usize
}

fn cls_byte_count(cls: &hir::ClassBytes) -> usize {
    cls.iter().map(|&r| 1 + (r.end as u32) - (r.start as u32)).sum::<u32>()
        as usize
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::{escape_bytes, Literal, Literals};
    use hir::Hir;
    use ParserBuilder;

    // To make test failures easier to read.
    #[derive(Debug, Eq, PartialEq)]
    struct Bytes(Vec<ULiteral>);
    #[derive(Debug, Eq, PartialEq)]
    struct Unicode(Vec<ULiteral>);

    fn escape_lits(blits: &[Literal]) -> Vec<ULiteral> {
        let mut ulits = vec![];
        for blit in blits {
            ulits
                .push(ULiteral { v: escape_bytes(&blit), cut: blit.is_cut() });
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

    // Needs to be pub for 1.3?
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
        ULiteral { v: s.to_owned(), cut: true }
    }
    #[allow(non_snake_case)]
    fn M(s: &'static str) -> ULiteral {
        ULiteral { v: s.to_owned(), cut: false }
    }

    fn prefixes(lits: &mut Literals, expr: &Hir) {
        lits.union_prefixes(expr);
    }

    fn suffixes(lits: &mut Literals, expr: &Hir) {
        lits.union_suffixes(expr);
    }

    macro_rules! assert_lit_eq {
        ($which:ident, $got_lits:expr, $($expected_lit:expr),*) => {{
            let expected: Vec<ULiteral> = vec![$($expected_lit),*];
            let lits = $got_lits;
            assert_eq!(
                $which(expected.clone()),
                $which(escape_lits(lits.literals())));
            assert_eq!(
                !expected.is_empty() && expected.iter().all(|l| !l.is_cut()),
                lits.all_complete());
            assert_eq!(
                expected.iter().any(|l| !l.is_cut()),
                lits.any_complete());
        }};
    }

    macro_rules! test_lit {
        ($name:ident, $which:ident, $re:expr) => {
            test_lit!($name, $which, $re,);
        };
        ($name:ident, $which:ident, $re:expr, $($lit:expr),*) => {
            #[test]
            fn $name() {
                let expr = ParserBuilder::new()
                    .build()
                    .parse($re)
                    .unwrap();
                let lits = Literals::$which(&expr);
                assert_lit_eq!(Unicode, lits, $($lit),*);

                let expr = ParserBuilder::new()
                    .allow_invalid_utf8(true)
                    .unicode(false)
                    .build()
                    .parse($re)
                    .unwrap();
                let lits = Literals::$which(&expr);
                assert_lit_eq!(Bytes, lits, $($lit),*);
            }
        };
    }

    // ************************************************************************
    // Tests for prefix literal extraction.
    // ************************************************************************

    // Elementary tests.
    test_lit!(pfx_one_lit1, prefixes, "a", M("a"));
    test_lit!(pfx_one_lit2, prefixes, "abc", M("abc"));
    test_lit!(pfx_one_lit3, prefixes, "(?u)☃", M("\\xe2\\x98\\x83"));
    #[cfg(feature = "unicode-case")]
    test_lit!(pfx_one_lit4, prefixes, "(?ui)☃", M("\\xe2\\x98\\x83"));
    test_lit!(pfx_class1, prefixes, "[1-4]", M("1"), M("2"), M("3"), M("4"));
    test_lit!(
        pfx_class2,
        prefixes,
        "(?u)[☃Ⅰ]",
        M("\\xe2\\x85\\xa0"),
        M("\\xe2\\x98\\x83")
    );
    #[cfg(feature = "unicode-case")]
    test_lit!(
        pfx_class3,
        prefixes,
        "(?ui)[☃Ⅰ]",
        M("\\xe2\\x85\\xa0"),
        M("\\xe2\\x85\\xb0"),
        M("\\xe2\\x98\\x83")
    );
    test_lit!(pfx_one_lit_casei1, prefixes, "(?i-u)a", M("A"), M("a"));
    test_lit!(
        pfx_one_lit_casei2,
        prefixes,
        "(?i-u)abc",
        M("ABC"),
        M("aBC"),
        M("AbC"),
        M("abC"),
        M("ABc"),
        M("aBc"),
        M("Abc"),
        M("abc")
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

    // Test regexes with concatenations.
    test_lit!(pfx_cat1, prefixes, "(?:a)(?:b)", M("ab"));
    test_lit!(pfx_cat2, prefixes, "[ab]z", M("az"), M("bz"));
    test_lit!(
        pfx_cat3,
        prefixes,
        "(?i-u)[ab]z",
        M("AZ"),
        M("BZ"),
        M("aZ"),
        M("bZ"),
        M("Az"),
        M("Bz"),
        M("az"),
        M("bz")
    );
    test_lit!(
        pfx_cat4,
        prefixes,
        "[ab][yz]",
        M("ay"),
        M("by"),
        M("az"),
        M("bz")
    );
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

    // Test regexes with alternations.
    test_lit!(pfx_alt1, prefixes, "a|b", M("a"), M("b"));
    test_lit!(pfx_alt2, prefixes, "[1-3]|b", M("1"), M("2"), M("3"), M("b"));
    test_lit!(pfx_alt3, prefixes, "y(?:a|b)z", M("yaz"), M("ybz"));
    test_lit!(pfx_alt4, prefixes, "a|b*");
    test_lit!(pfx_alt5, prefixes, "a|b+", M("a"), C("b"));
    test_lit!(pfx_alt6, prefixes, "a|(?:b|c*)");
    test_lit!(
        pfx_alt7,
        prefixes,
        "(a|b)*c|(a|ab)*c",
        C("a"),
        C("b"),
        M("c"),
        C("a"),
        C("ab"),
        M("c")
    );
    test_lit!(pfx_alt8, prefixes, "a*b|c", C("a"), M("b"), M("c"));

    // Test regexes with empty assertions.
    test_lit!(pfx_empty1, prefixes, "^a", M("a"));
    test_lit!(pfx_empty2, prefixes, "a${2}", C("a"));
    test_lit!(pfx_empty3, prefixes, "^abc", M("abc"));
    test_lit!(pfx_empty4, prefixes, "(?:^abc)|(?:^z)", M("abc"), M("z"));

    // Make sure some curious regexes have no prefixes.
    test_lit!(pfx_nothing1, prefixes, ".");
    test_lit!(pfx_nothing2, prefixes, "(?s).");
    test_lit!(pfx_nothing3, prefixes, "^");
    test_lit!(pfx_nothing4, prefixes, "$");
    test_lit!(pfx_nothing6, prefixes, "(?m)$");
    test_lit!(pfx_nothing7, prefixes, r"\b");
    test_lit!(pfx_nothing8, prefixes, r"\B");

    // Test a few regexes that defeat any prefix literal detection.
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
        pfx_crazy1,
        prefixes,
        r"M[ou]'?am+[ae]r .*([AEae]l[- ])?[GKQ]h?[aeu]+([dtz][dhz]?)+af[iy]",
        C("Mo\\'am"),
        C("Mu\\'am"),
        C("Moam"),
        C("Muam")
    );

    // ************************************************************************
    // Tests for quiting prefix literal search.
    // ************************************************************************

    macro_rules! test_exhausted {
        ($name:ident, $which:ident, $re:expr) => {
            test_exhausted!($name, $which, $re,);
        };
        ($name:ident, $which:ident, $re:expr, $($lit:expr),*) => {
            #[test]
            fn $name() {
                let expr = ParserBuilder::new()
                    .build()
                    .parse($re)
                    .unwrap();
                let mut lits = Literals::empty();
                lits.set_limit_size(20).set_limit_class(10);
                $which(&mut lits, &expr);
                assert_lit_eq!(Unicode, lits, $($lit),*);

                let expr = ParserBuilder::new()
                    .allow_invalid_utf8(true)
                    .unicode(false)
                    .build()
                    .parse($re)
                    .unwrap();
                let mut lits = Literals::empty();
                lits.set_limit_size(20).set_limit_class(10);
                $which(&mut lits, &expr);
                assert_lit_eq!(Bytes, lits, $($lit),*);
            }
        };
    }

    // These test use a much lower limit than the default so that we can
    // write test cases of reasonable size.
    test_exhausted!(pfx_exhausted1, prefixes, "[a-z]");
    test_exhausted!(pfx_exhausted2, prefixes, "[a-z]*A");
    test_exhausted!(pfx_exhausted3, prefixes, "A[a-z]Z", C("A"));
    test_exhausted!(
        pfx_exhausted4,
        prefixes,
        "(?i-u)foobar",
        C("FO"),
        C("fO"),
        C("Fo"),
        C("fo")
    );
    test_exhausted!(
        pfx_exhausted5,
        prefixes,
        "(?:ab){100}",
        C("abababababababababab")
    );
    test_exhausted!(
        pfx_exhausted6,
        prefixes,
        "(?:(?:ab){100})*cd",
        C("ababababab"),
        M("cd")
    );
    test_exhausted!(
        pfx_exhausted7,
        prefixes,
        "z(?:(?:ab){100})*cd",
        C("zababababab"),
        M("zcd")
    );
    test_exhausted!(
        pfx_exhausted8,
        prefixes,
        "aaaaaaaaaaaaaaaaaaaaz",
        C("aaaaaaaaaaaaaaaaaaaa")
    );

    // ************************************************************************
    // Tests for suffix literal extraction.
    // ************************************************************************

    // Elementary tests.
    test_lit!(sfx_one_lit1, suffixes, "a", M("a"));
    test_lit!(sfx_one_lit2, suffixes, "abc", M("abc"));
    test_lit!(sfx_one_lit3, suffixes, "(?u)☃", M("\\xe2\\x98\\x83"));
    #[cfg(feature = "unicode-case")]
    test_lit!(sfx_one_lit4, suffixes, "(?ui)☃", M("\\xe2\\x98\\x83"));
    test_lit!(sfx_class1, suffixes, "[1-4]", M("1"), M("2"), M("3"), M("4"));
    test_lit!(
        sfx_class2,
        suffixes,
        "(?u)[☃Ⅰ]",
        M("\\xe2\\x85\\xa0"),
        M("\\xe2\\x98\\x83")
    );
    #[cfg(feature = "unicode-case")]
    test_lit!(
        sfx_class3,
        suffixes,
        "(?ui)[☃Ⅰ]",
        M("\\xe2\\x85\\xa0"),
        M("\\xe2\\x85\\xb0"),
        M("\\xe2\\x98\\x83")
    );
    test_lit!(sfx_one_lit_casei1, suffixes, "(?i-u)a", M("A"), M("a"));
    test_lit!(
        sfx_one_lit_casei2,
        suffixes,
        "(?i-u)abc",
        M("ABC"),
        M("ABc"),
        M("AbC"),
        M("Abc"),
        M("aBC"),
        M("aBc"),
        M("abC"),
        M("abc")
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

    // Test regexes with concatenations.
    test_lit!(sfx_cat1, suffixes, "(?:a)(?:b)", M("ab"));
    test_lit!(sfx_cat2, suffixes, "[ab]z", M("az"), M("bz"));
    test_lit!(
        sfx_cat3,
        suffixes,
        "(?i-u)[ab]z",
        M("AZ"),
        M("Az"),
        M("BZ"),
        M("Bz"),
        M("aZ"),
        M("az"),
        M("bZ"),
        M("bz")
    );
    test_lit!(
        sfx_cat4,
        suffixes,
        "[ab][yz]",
        M("ay"),
        M("az"),
        M("by"),
        M("bz")
    );
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

    // Test regexes with alternations.
    test_lit!(sfx_alt1, suffixes, "a|b", M("a"), M("b"));
    test_lit!(sfx_alt2, suffixes, "[1-3]|b", M("1"), M("2"), M("3"), M("b"));
    test_lit!(sfx_alt3, suffixes, "y(?:a|b)z", M("yaz"), M("ybz"));
    test_lit!(sfx_alt4, suffixes, "a|b*");
    test_lit!(sfx_alt5, suffixes, "a|b+", M("a"), C("b"));
    test_lit!(sfx_alt6, suffixes, "a|(?:b|c*)");
    test_lit!(
        sfx_alt7,
        suffixes,
        "(a|b)*c|(a|ab)*c",
        C("ac"),
        C("bc"),
        M("c"),
        C("ac"),
        C("abc"),
        M("c")
    );
    test_lit!(sfx_alt8, suffixes, "a*b|c", C("ab"), M("b"), M("c"));

    // Test regexes with empty assertions.
    test_lit!(sfx_empty1, suffixes, "a$", M("a"));
    test_lit!(sfx_empty2, suffixes, "${2}a", C("a"));

    // Make sure some curious regexes have no suffixes.
    test_lit!(sfx_nothing1, suffixes, ".");
    test_lit!(sfx_nothing2, suffixes, "(?s).");
    test_lit!(sfx_nothing3, suffixes, "^");
    test_lit!(sfx_nothing4, suffixes, "$");
    test_lit!(sfx_nothing6, suffixes, "(?m)$");
    test_lit!(sfx_nothing7, suffixes, r"\b");
    test_lit!(sfx_nothing8, suffixes, r"\B");

    // Test a few regexes that defeat any suffix literal detection.
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

    // These test use a much lower limit than the default so that we can
    // write test cases of reasonable size.
    test_exhausted!(sfx_exhausted1, suffixes, "[a-z]");
    test_exhausted!(sfx_exhausted2, suffixes, "A[a-z]*");
    test_exhausted!(sfx_exhausted3, suffixes, "A[a-z]Z", C("Z"));
    test_exhausted!(
        sfx_exhausted4,
        suffixes,
        "(?i-u)foobar",
        C("AR"),
        C("Ar"),
        C("aR"),
        C("ar")
    );
    test_exhausted!(
        sfx_exhausted5,
        suffixes,
        "(?:ab){100}",
        C("abababababababababab")
    );
    test_exhausted!(
        sfx_exhausted6,
        suffixes,
        "cd(?:(?:ab){100})*",
        C("ababababab"),
        M("cd")
    );
    test_exhausted!(
        sfx_exhausted7,
        suffixes,
        "cd(?:(?:ab){100})*z",
        C("abababababz"),
        M("cdz")
    );
    test_exhausted!(
        sfx_exhausted8,
        suffixes,
        "zaaaaaaaaaaaaaaaaaaaa",
        C("aaaaaaaaaaaaaaaaaaaa")
    );

    // ************************************************************************
    // Tests for generating unambiguous literal sets.
    // ************************************************************************

    macro_rules! test_unamb {
        ($name:ident, $given:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let given: Vec<Literal> = $given
                    .into_iter()
                    .map(|ul| {
                        let cut = ul.is_cut();
                        Literal { v: ul.v.into_bytes(), cut: cut }
                    })
                    .collect();
                let lits = create_lits(given);
                let got = lits.unambiguous_prefixes();
                assert_eq!($expected, escape_lits(got.literals()));
            }
        };
    }

    test_unamb!(unambiguous1, vec![M("z"), M("azb")], vec![C("a"), C("z")]);
    test_unamb!(
        unambiguous2,
        vec![M("zaaaaaa"), M("aa")],
        vec![C("aa"), C("z")]
    );
    test_unamb!(
        unambiguous3,
        vec![M("Sherlock"), M("Watson")],
        vec![M("Sherlock"), M("Watson")]
    );
    test_unamb!(unambiguous4, vec![M("abc"), M("bc")], vec![C("a"), C("bc")]);
    test_unamb!(unambiguous5, vec![M("bc"), M("abc")], vec![C("a"), C("bc")]);
    test_unamb!(unambiguous6, vec![M("a"), M("aa")], vec![C("a")]);
    test_unamb!(unambiguous7, vec![M("aa"), M("a")], vec![C("a")]);
    test_unamb!(unambiguous8, vec![M("ab"), M("a")], vec![C("a")]);
    test_unamb!(
        unambiguous9,
        vec![M("ac"), M("bc"), M("c"), M("ac"), M("abc"), M("c")],
        vec![C("a"), C("b"), C("c")]
    );
    test_unamb!(
        unambiguous10,
        vec![M("Mo'"), M("Mu'"), M("Mo"), M("Mu")],
        vec![C("Mo"), C("Mu")]
    );
    test_unamb!(
        unambiguous11,
        vec![M("zazb"), M("azb")],
        vec![C("a"), C("z")]
    );
    test_unamb!(unambiguous12, vec![M("foo"), C("foo")], vec![C("foo")]);
    test_unamb!(
        unambiguous13,
        vec![M("ABCX"), M("CDAX"), M("BCX")],
        vec![C("A"), C("BCX"), C("CD")]
    );
    test_unamb!(
        unambiguous14,
        vec![M("IMGX"), M("MVIX"), M("MGX"), M("DSX")],
        vec![M("DSX"), C("I"), C("MGX"), C("MV")]
    );
    test_unamb!(
        unambiguous15,
        vec![M("IMG_"), M("MG_"), M("CIMG")],
        vec![C("C"), C("I"), C("MG_")]
    );

    // ************************************************************************
    // Tests for suffix trimming.
    // ************************************************************************
    macro_rules! test_trim {
        ($name:ident, $trim:expr, $given:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let given: Vec<Literal> = $given
                    .into_iter()
                    .map(|ul| {
                        let cut = ul.is_cut();
                        Literal { v: ul.v.into_bytes(), cut: cut }
                    })
                    .collect();
                let lits = create_lits(given);
                let got = lits.trim_suffix($trim).unwrap();
                assert_eq!($expected, escape_lits(got.literals()));
            }
        };
    }

    test_trim!(trim1, 1, vec![M("ab"), M("yz")], vec![C("a"), C("y")]);
    test_trim!(trim2, 1, vec![M("abc"), M("abd")], vec![C("ab")]);
    test_trim!(trim3, 2, vec![M("abc"), M("abd")], vec![C("a")]);
    test_trim!(trim4, 2, vec![M("abc"), M("ghij")], vec![C("a"), C("gh")]);

    // ************************************************************************
    // Tests for longest common prefix.
    // ************************************************************************

    macro_rules! test_lcp {
        ($name:ident, $given:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let given: Vec<Literal> = $given
                    .into_iter()
                    .map(|s: &str| Literal {
                        v: s.to_owned().into_bytes(),
                        cut: false,
                    })
                    .collect();
                let lits = create_lits(given);
                let got = lits.longest_common_prefix();
                assert_eq!($expected, escape_bytes(got));
            }
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

    // ************************************************************************
    // Tests for longest common suffix.
    // ************************************************************************

    macro_rules! test_lcs {
        ($name:ident, $given:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let given: Vec<Literal> = $given
                    .into_iter()
                    .map(|s: &str| Literal {
                        v: s.to_owned().into_bytes(),
                        cut: false,
                    })
                    .collect();
                let lits = create_lits(given);
                let got = lits.longest_common_suffix();
                assert_eq!($expected, escape_bytes(got));
            }
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
mod tests_llm_16_72 {
    use super::*;

use crate::*;
    use std::cmp::PartialEq;

    #[test]
    fn test_eq() {
        let literal1 = Literal::new(vec![1, 2, 3]);
        let literal2 = Literal::new(vec![1, 2, 3]);
        let literal3 = Literal::new(vec![1, 2, 4]);

        assert_eq!(literal1.eq(&literal2), true);
        assert_eq!(literal1.eq(&literal3), false);
    }
}#[cfg(test)]
mod tests_llm_16_73 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_partial_cmp() {
        let literal1 = Literal::new(vec![1, 2, 3]);
        let literal2 = Literal::new(vec![1, 2, 3]);
        let literal3 = Literal::new(vec![4, 5, 6]);
        
        let result1 = literal1.partial_cmp(&literal2);
        let result2 = literal1.partial_cmp(&literal3);
        
        assert_eq!(result1, Some(cmp::Ordering::Equal));
        assert_eq!(result2, Some(cmp::Ordering::Less));
    }
}#[cfg(test)]
mod tests_llm_16_74 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_as_ref() {
        let literal = Literal::new(vec![97, 98, 99]);
        let result = literal.as_ref();
        assert_eq!(result, [97, 98, 99].as_ref());
    }
}#[cfg(test)]
mod tests_llm_16_76_llm_16_75 {
    use super::*;

use crate::*;
    use std::ops::Deref;

    #[test]
    fn test_deref() {
        let literal = Literal::new(vec![97, 98, 99]);
        assert_eq!(Deref::deref(&literal), &vec![97, 98, 99]);
    }
}#[cfg(test)]
mod tests_llm_16_78_llm_16_77 {
    use super::*;

use crate::*;
    use std::ops::DerefMut;
    
    #[test]
    fn test_deref_mut() {
        let mut literal = Literal::new(vec![65, 66, 67]);
        assert_eq!(*DerefMut::deref_mut(&mut literal), vec![65, 66, 67]);
        
        let mut literal = Literal::empty();
        assert_eq!(*DerefMut::deref_mut(&mut literal), vec![]);
    }
}#[cfg(test)]
mod tests_llm_16_456 {
    use super::*;

use crate::*;
    use std::cmp::Ordering;
    use std::fmt::Debug;
    use std::ops::{Deref, DerefMut};
    
    #[test]
    fn test_cut() {
        let mut literal: Literal = Literal::new(vec![65, 66, 67]);
        literal.cut();
        
        assert_eq!(literal.is_cut(), true);
    }
}#[cfg(test)]
mod tests_llm_16_457 {
    use super::*;

use crate::*;

    #[test]
    fn test_empty() {
        let literal = Literal::empty();
        assert_eq!(literal.v, vec![]);
        assert_eq!(literal.cut, false);
    }
}#[cfg(test)]
mod tests_llm_16_459 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_cut_true() {
        let mut literal = Literal::new(vec![97, 98, 99]);
        literal.cut();
        assert_eq!(literal.is_cut(), true);
    }

    #[test]
    fn test_is_cut_false() {
        let literal = Literal::new(vec![97, 98, 99]);
        assert_eq!(literal.is_cut(), false);
    }
}#[cfg(test)]
mod tests_llm_16_460 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let bytes = vec![97, 98, 99]; // example bytes
        let literal = Literal::new(bytes);
        assert_eq!(literal.v, vec![97, 98, 99]);
        assert_eq!(literal.cut, false);
    }
}#[cfg(test)]
mod tests_llm_16_465 {
    use super::*;

use crate::*;
    use hir::*;

    #[test]
    fn test_add_char_class() {
        let mut literals = Literals::empty();
        let class_unicode = ClassUnicode::new(vec![ClassUnicodeRange::new('a', 'z')]);
        let result = literals.add_char_class(&class_unicode);
        assert_eq!(result, true);
    }
}#[cfg(test)]
mod tests_llm_16_474 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_contains_empty() {
        let mut literals = Literals::empty();
        literals.add(Literal::new(b"abc".to_vec()));
        assert!(!literals.contains_empty());
        
        literals.add(Literal::new(Vec::new()));
        assert!(literals.contains_empty());
        
        literals.clear();
        assert!(!literals.contains_empty());
    }
}#[cfg(test)]
mod tests_llm_16_475 {
    use super::*;

use crate::*;
    use std::cmp;
    
    #[test]
    fn test_cross_add_with_empty_bytes() {
        let mut literals = Literals::empty();
        assert!(literals.cross_add(&[]));
        assert_eq!(literals.literals().len(), 1);
        assert!(literals.literals()[0].is_empty());
    }
    
    #[test]
    fn test_cross_add_with_empty_set() {
        let mut literals = Literals::empty();
        assert!(literals.cross_add(&[1, 2, 3]));
        assert_eq!(literals.literals().len(), 1);
        assert_eq!(literals.literals()[0].len(), 3);
        assert_eq!(literals.literals()[0].iter().cloned().collect::<Vec<u8>>(), vec![1, 2, 3]);
        assert!(!literals.literals()[0].is_cut());
    }
    
    #[test]
    fn test_cross_add_with_non_empty_set() {
        let mut literals = Literals::empty();
        literals.cross_add(&[1, 2, 3]);
        assert!(literals.cross_add(&[4, 5]));
        assert_eq!(literals.literals().len(), 2);
        assert_eq!(literals.literals()[0].len(), 5);
        assert_eq!(literals.literals()[1].len(), 5);
        assert_eq!(literals.literals()[0].iter().cloned().collect::<Vec<u8>>(), vec![1, 2, 3, 4, 5]);
        assert_eq!(literals.literals()[1].iter().cloned().collect::<Vec<u8>>(), vec![1, 2, 3, 4, 5]);
        assert!(!literals.literals()[0].is_cut());
        assert!(!literals.literals()[1].is_cut());
    }
    
    #[test]
    fn test_cross_add_with_limit_exceeded() {
        let mut literals = Literals::empty();
        literals.set_limit_size(5);
        literals.cross_add(&[1, 2, 3]);
        assert!(!literals.cross_add(&[4, 5, 6]));
        assert_eq!(literals.literals().len(), 1);
    }
    
    #[test]
    fn test_cross_add_with_shorter_prefix() {
        let mut literals = Literals::empty();
        literals.cross_add(&[1, 2, 3]);
        assert!(literals.cross_add(&[4]));
        assert_eq!(literals.literals().len(), 2);
        assert_eq!(literals.literals()[0].len(), 4);
        assert_eq!(literals.literals()[1].len(), 4);
        assert_eq!(literals.literals()[0].iter().cloned().collect::<Vec<u8>>(), vec![1, 2, 3, 4]);
        assert_eq!(literals.literals()[1].iter().cloned().collect::<Vec<u8>>(), vec![1, 2, 3, 4]);
        assert!(!literals.literals()[0].is_cut());
        assert!(!literals.literals()[1].is_cut());
    }
}#[cfg(test)]
mod tests_llm_16_479 {
    use super::*;

use crate::*;

    #[test]
    fn test_empty() {
        let literals = Literals::empty();
        assert_eq!(literals.lits.len(), 0);
        assert_eq!(literals.limit_size, 250);
        assert_eq!(literals.limit_class, 10);
    }
}#[cfg(test)]
mod tests_llm_16_480 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_empty_empty() {
        let literals = Literals::empty();
        assert!(literals.is_empty());
    }

    #[test]
    fn test_is_empty_non_empty() {
        let literal = Literal::new(Vec::from("abc"));
        let literals = Literals {
            lits: vec![literal],
            limit_size: 100,
            limit_class: 10,
        };
        assert!(!literals.is_empty());
    }

    #[test]
    fn test_is_empty_all_empty() {
        let literal1 = Literal::empty();
        let literal2 = Literal::empty();
        let literals = Literals {
            lits: vec![literal1, literal2],
            limit_size: 100,
            limit_class: 10,
        };
        assert!(literals.is_empty());
    }
}#[cfg(test)]
mod tests_llm_16_481 {
    use crate::hir::literal::{Literal, Literals};
    
    #[test]
    fn test_limit_class() {
        let lits = Literals::empty();
        assert_eq!(lits.limit_class(), 10);
    }
}#[cfg(test)]
mod tests_llm_16_482 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_limit_size() {
        let literals = Literals::empty();
        let size = literals.limit_size();
        assert_eq!(size, 250);
    }
}#[cfg(test)]
mod tests_llm_16_485 {
    use super::*;

use crate::*;
    use std::cmp;

    #[test]
    fn test_longest_common_prefix_empty_literals() {
        let literals = Literals::empty();
        let result = literals.longest_common_prefix();
        assert_eq!(result, &[]);
    }

    #[test]
    fn test_longest_common_prefix_one_literal() {
        let literals = Literals {
            lits: vec![Literal::new(vec![1, 2, 3, 4])],
            limit_size: 250,
            limit_class: 10,
        };
        let result = literals.longest_common_prefix();
        assert_eq!(result, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_longest_common_prefix_multiple_literals() {
        let literals = Literals {
            lits: vec![Literal::new(vec![1, 2, 3, 4]), Literal::new(vec![1, 2, 5, 6])],
            limit_size: 250,
            limit_class: 10,
        };
        let result = literals.longest_common_prefix();
        assert_eq!(result, &[1, 2]);
    }

    #[test]
    fn test_longest_common_prefix_empty_literal_in_set() {
        let literals = Literals {
            lits: vec![Literal::empty(), Literal::new(vec![1, 2, 3, 4]), Literal::new(vec![1, 2, 5, 6])],
            limit_size: 250,
            limit_class: 10,
        };
        let result = literals.longest_common_prefix();
        assert_eq!(result, &[]);
    }
}#[cfg(test)]
mod tests_llm_16_488 {
    use super::*;

use crate::*;

    #[test]
    fn test_min_len_empty() {
        let lits = Literals::empty();
        assert_eq!(lits.min_len(), None);
    }

    #[test]
    fn test_min_len_single_literal() {
        let mut lits = Literals::empty();
        lits.add(Literal::new(b"abc".to_vec()));
        assert_eq!(lits.min_len(), Some(3));
    }

    #[test]
    fn test_min_len_multiple_literals() {
        let mut lits = Literals::empty();
        lits.add(Literal::new(b"abc".to_vec()));
        lits.add(Literal::new(b"defg".to_vec()));
        lits.add(Literal::new(b"hijk".to_vec()));
        assert_eq!(lits.min_len(), Some(3));
    }
}#[cfg(test)]
mod tests_llm_16_491 {
    use super::*;

use crate::*;
    use std::fmt::Debug;

    fn assert_debug_eq<T: Debug + PartialEq>(left: T, right: T) {
        assert_eq!(format!("{:?}", left), format!("{:?}", right));
    }

    #[test]
    fn test_remove_complete() {
        let mut literals = Literals {
            lits: vec![
                Literal::new(vec![65, 66, 67]),
                Literal::new(vec![68, 69, 70]),
                Literal::new(vec![71, 72, 73]),
            ],
            limit_size: 250,
            limit_class: 10,
        };

        let expected_base = vec![
            Literal::new(vec![68, 69, 70]),
            Literal::new(vec![71, 72, 73]),
        ];

        let base = literals.remove_complete();

        assert_debug_eq(expected_base, base);
    }
}#[cfg(test)]
mod tests_llm_16_494 {
    use super::*;

use crate::*;
    use hir::ClassBytes;
    use hir::ClassUnicode;
    use hir::Literal;

    #[test]
    fn test_set_limit_class() {
        let mut literals = Literals::empty();
        literals.set_limit_class(5);
        assert_eq!(literals.limit_class(), 5);
    }
}
#[cfg(test)]
mod tests_llm_16_501 {
    use super::*;

use crate::*;

    #[test]
    fn test_unambiguous_prefixes() {
        let lit1 = Literal::new(b"abc".to_vec());
        let lit2 = Literal::new(b"abcd".to_vec());
        let lit3 = Literal::new(b"abce".to_vec());
        let lit4 = Literal::new(b"def".to_vec());
        let mut lits = Literals::empty();
        lits.add(lit1.clone());
        lits.add(lit2.clone());
        lits.add(lit3.clone());
        lits.add(lit4.clone());

        let result = lits.unambiguous_prefixes();

        assert_eq!(result.lits.len(), 2);
        assert_eq!(result.lits.contains(&lit1), true);
        assert_eq!(result.lits.contains(&lit4), true);
    }
}#[cfg(test)]
mod tests_llm_16_502 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_unambiguous_suffixes() {
        let mut lits = Literals::empty();
        lits.set_limit_size(1000).set_limit_class(100);
        let lit1 = Literal::new(vec![b'a', b'b']);
        let lit2 = Literal::new(vec![b'a', b'c']);
        let lit3 = Literal::new(vec![b'b', b'c']);
        lits.add(lit1);
        lits.add(lit2);
        lits.add(lit3);
        let unambiguous = lits.unambiguous_suffixes();
        assert_eq!(unambiguous.literals(), &[Literal::new(vec![b'b', b'c']), Literal::new(vec![b'a', b'c'])]);
    }
}#[cfg(test)]
mod tests_llm_16_503 {
    use super::*;

use crate::*;

    #[test]
    fn test_union() {
        let mut literals = Literals::empty();
        literals.union(Literals::empty());
        assert_eq!(literals.lits.len(), 1);

        let mut literals2 = Literals::empty();
        literals2.union(Literals::empty());
        literals.union(literals2);
        assert_eq!(literals.lits.len(), 1);

        let mut literals3 = Literals::empty();
        literals3.union(Literals::empty());
        literals.union(literals3);
        assert_eq!(literals.lits.len(), 1);
    }
}#[cfg(test)]
mod tests_llm_16_506 {
    use super::*;

use crate::*;
    use hir::{ClassUnicode, ClassUnicodeRange};

    #[test]
    fn test_cls_char_count() {
        let mut class = ClassUnicode::new(vec![
            ClassUnicodeRange::new('a', 'z'),
            ClassUnicodeRange::new('A', 'Z'),
            ClassUnicodeRange::new('0', '9'),
        ]);

        assert_eq!(cls_char_count(&class), 62);

        class.negate();

        assert_eq!(cls_char_count(&class), 1);
    }
}#[cfg(test)]
mod tests_llm_16_508 {
    #[test]
    fn test_escape_byte() {
        assert_eq!(super::escape_byte(b'a'), "\\x61");
        assert_eq!(super::escape_byte(b'\n'), "\\x0a");
        assert_eq!(super::escape_byte(b'\r'), "\\x0d");
        assert_eq!(super::escape_byte(b'\t'), "\\x09");
    }
}#[cfg(test)]
mod tests_llm_16_510 {
    use crate::hir::literal::escape_bytes;

    fn escape_byte(b: u8) -> String {
        unimplemented!("Replace this with the implementation of escape_byte")
    }

    #[test]
    fn test_escape_bytes() {
        let bytes: [u8; 4] = [97, 98, 99, 100];
        let expected = String::from(""); // expected escaped string for the given bytes
        assert_eq!(escape_bytes(&bytes), expected);
    }
}#[cfg(test)]
mod tests_llm_16_514 {
    use crate::hir::literal::position;

    #[test]
    fn test_position_needle_present() {
        let needle = &[97, 98, 99]; // "abc"
        let haystack = &[97, 98, 99, 100, 101]; // "abcde"

        assert_eq!(position(needle, haystack), Some(0));
    }

    #[test]
    fn test_position_needle_not_present() {
        let needle = &[100, 101, 102]; // "def"
        let haystack = &[97, 98, 99, 100, 101]; // "abcde"

        assert_eq!(position(needle, haystack), None);
    }

    #[test]
    fn test_position_needle_empty() {
        let needle = &[]; // empty
        let haystack = &[97, 98, 99, 100, 101]; // "abcde"

        assert_eq!(position(needle, haystack), Some(0));
    }

    #[test]
    fn test_position_haystack_empty() {
        let needle = &[97, 98, 99]; // "abc"
        let haystack = &[]; // empty

        assert_eq!(position(needle, haystack), None);
    }

    #[test]
    fn test_position_both_empty() {
        let needle = &[]; // empty
        let haystack = &[]; // empty

        assert_eq!(position(needle, haystack), Some(0));
    }
}#[cfg(test)]
mod tests_rug_457 {
    use super::*;
    use crate::HirKind;
    use crate::hir::literal::{Literals, Literal, literal, parse};

    #[test]
    fn test_rug() {
        // prepare the first argument
        #[cfg(test)]
        mod tests_rug_457_prepare_hir {
            #[test]
            fn sample() {
                let mut v148 = literal(parse("sample").unwrap());
            }
        }
        let p0 = &v148;

        // prepare the second argument
        #[cfg(test)]
        mod tests_rug_457_prepare_literals {
            #[test]
            fn sample() {
                let mut v149 = Literals::empty();
            }
        }
        let mut p1 = &mut v149;

        crate::hir::literal::prefixes(p0, p1);

    }
}#[cfg(test)]
mod tests_rug_458 {
    use super::*;
    use crate::hir::literal::{literal, parse, Literals, Literal};
    use crate::hir::{Hir, HirKind};
    use crate::hir::Class;
    use crate::hir::RepetitionKind;
    
    #[test]
    fn test_suffixes() {
        #[cfg(test)]
        mod tests_rug_458_prepare {
            #[test]
            fn sample() {
                let mut v148 = literal(parse("sample").unwrap());

                let mut v149 = Literals::empty();
                crate::hir::literal::suffixes(&v148, &mut v149);
            }
        }
    }
}#[cfg(test)]
mod tests_rug_459 {
    use super::*;
    use crate::hir::literal::{literal, parse, Literals, Literal};
    use crate::hir::Hir;
    use crate::utf8::check_char_escape_limit;
    
    #[test]
    fn test_repeat_zero_or_one_literals() {
        let mut v148 = literal(parse("sample").unwrap()); // replace ... with the sample code
        let mut v149 = Literals::empty(); // create the local variable v149 with type hir::literal::Literals
        let mut v150 = CharEscapeDefault::default();

        repeat_zero_or_one_literals(&v148, &mut v149, |e, lits| {
            // f(e, lits) code
            let limit_size = lits.limit_size();
            lits.set_limit_size(limit_size / 2);

            let mut literals3 = lits.to_empty();
            f(&v148, &mut literals3);

            if literals3.is_empty() || !lits.cross_product(&literals3) {
                lits.cut();
                return;
            }
            lits.add(Literal::empty());
            if !lits.union(literals3) {
                lits.cut();
            }
        });
    }
}
#[cfg(test)]
mod tests_rug_460 {
    use super::*;
    use crate::hir::literal::{literal, parse, Literals, Literal, ConstFnMutClosure, Function};

    #[test]
    fn test_rug() {
        // Constructing the first argument
        #[derive(Debug)]
        struct Hir {
            literal: Literal,
        }

        impl Hir {
            fn new(literal: Literal) -> Self {
                Self { literal }
            }
        }

        impl Drop for Hir {
            fn drop(&mut self) {
                println!("Dropping {:?}", self);
            }
        }

        impl Clone for Hir {
            fn clone(&self) -> Self {
                Self::new(self.literal.clone())
            }
        }

        // Constructing the second argument
        let mut v148 = literal(parse("sample").unwrap());
        let mut v149 = Literals::empty();

        // Constructing the third argument
        
        let mut a = 10;
        let mut b = 20;
        
      let v151: ConstFnMutClosure<(&mut Literals, &mut Function), Function> = ConstFnMutClosure::new(|(x, y)| {
            //Add your logic here
            
        
            Function::new(*x, *y)
            
      });
        
       let result = v151.call_mut((&mut a, &mut b));
        
       assert_eq!(result.x, 11);
       assert_eq!(result.y, 40);

        crate::hir::literal::repeat_zero_or_more_literals(&Hir::new(v148), &mut v149, |e: &Hir,
                    l: &mut Literals| { 
                    });
    }
}#[cfg(test)]
mod tests_rug_461 {
    use super::*;
    use crate::hir::literal::{literal, parse};
    use crate::hir::literal::{Literals, Literal};
    use crate::hir::FnMutClosure;
    use crate::hir::{Hir, Function};

    #[test]
    fn test_rug() {
        let mut v148 = literal(parse("sample").unwrap());
        let mut v149 = Literals::empty();
        let mut v152: FnMutClosure<(&mut Hir, &mut Literals), Function> = FnMutClosure {
            handler: repeat_one_or_more_literals,
            state: (&mut v148, &mut v149),
        };

        repeat_one_or_more_literals(&mut v148, &mut v149, |a, b| {
            v152.handler(a, b);
            b.cut();
        });
    }
}#[cfg(test)]
mod tests_rug_462 {
    use super::*;
    use crate::hir::Hir;
    use crate::hir::literal::{Literals, Literal, literal, parse};
    use crate::hir::literal::consts::{A, B, C, D};
    use crate::hir::{FnMutClosure, Function};
    
    #[test]
    fn test_rug() {
        let mut v148 = literal(parse("sample").unwrap());
        let mut p0 = &mut v148;
        let mut p1: u32 = 42;
        let mut v154: std::option::Option<u32> = Some(42);
        let mut p2 = &mut v154;
        let mut p3: bool = true;
        let mut v149 = Literals::empty();
        let mut p4 = &mut v149;
        let mut v152: FnMutClosure<(&mut A, &mut B, &mut C, &mut D), Function> =
            FnMutClosure {
                handler: literal,
                state: (&mut A, &mut B, &mut C, &mut D),
            };
        let mut p5 = &mut v152;

                
        crate::hir::literal::repeat_range_literals(p0, p1, p2, p3, p4, p5);

    }
}#[cfg(test)]
mod tests_rug_463 {
    use super::*;
    use crate::hir::Hir;
    use crate::hir::literal::{Literals, Literal};
    use core::str::IsNotEmpty;
    
    #[test]
    fn test_rug() {
        // Prepare the first argument: es
        let mut v155: Vec<Hir> = vec![];
        // Initialize v155 with values
        
        // Prepare the second argument: lits
        let mut v149 = Literals::empty(); // create the local variable v149 with type hir::literal::Literals
        
        // Prepare the third argument: f
        let mut v153 = IsNotEmpty::new(); // create the local variable v153 with type core::str::IsNotEmpty
        // Additional code to manipulate the v153 variable if necessary
        
        crate::hir::literal::alternate_literals(&v155, &mut v149, |e, lits3| {
            f(e, lits3);
            if lits3.is_empty() || !lits2.union(lits3) {
                lits.cut();
                return;
            }
        });
    }
}#[cfg(test)]
mod tests_rug_464 {
    use super::*;
    
    #[test]
    fn test_escape_unicode() {
        let p0: &[u8] = b"Hello, world!";
        
        crate::regex_syntax::hir::literal::escape_unicode(p0);
    }
}
#[cfg(test)]
mod tests_rug_465 {
    use super::*;
    use crate::hir::{ClassBytes, ClassBytesRange};

    #[test]
    fn test_rug() {
        let mut p0 = ClassBytes::new(vec![
            ClassBytesRange::new(b'a', b'z'),
            ClassBytesRange::new(b'A', b'Z'),
        ]);

        crate::hir::literal::cls_byte_count(&p0);
    }
}
#[cfg(test)]
mod tests_rug_466 {
    use super::*;
    use crate::hir::{Hir, Class, ClassUnicodeRange, ClassUnicode, ClassPerl};
    use crate::hir::literal::{literal, parse};
    
    #[test]
    fn test_prefixes() {
        // Construct the sample Hir
        #[cfg(test)]
        mod tests_rug_466_prepare {
            #[test]
            fn sample() {
                let mut v148 = literal(parse("sample").unwrap());
            }
        }
        
        let mut p0 = Hir::Empty;
            
        <hir::literal::Literals>::prefixes(&p0);
    }
}#[cfg(test)]
mod tests_rug_467 {
    use super::*;
    use crate::hir::literal::{literal, parse};

    #[test]
    fn test_rug() {
        let mut v148 = literal(parse("sample").unwrap());
        <hir::literal::Literals>::suffixes(&v148);
    }
}
#[cfg(test)]
mod tests_rug_468 {
    use super::*;

    #[test]
    fn test_rug() {
        use crate::hir::literal::Literals;

        let mut v149 = Literals::empty(); 

        let p0 = &mut v149;
        let p1 = 10;

        p0.set_limit_size(p1);
    }
}
        
#[cfg(test)]
mod tests_rug_469 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut v149 = Literals::empty(); // create the local variable v149 with type hir::literal::Literals
        let p0: &Literals = &v149;

        <hir::literal::Literals>::literals(p0);

    }


}
                            #[cfg(test)]
mod tests_rug_470 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty();
       
        <hir::literal::Literals>::all_complete(&p0);
    }
}#[cfg(test)]
mod tests_rug_471 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut p0 = {
            let mut v149 = Literals::empty();
            p149
        };

        <hir::literal::Literals>::any_complete(&mut p0);

    }
}
#[cfg(test)]
mod tests_rug_472 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut p0 = {
            let mut v149 = Literals::empty();
            // construct p0 using the v149
            v149
        };

        <hir::literal::Literals>::to_empty(&mut p0);
        
        // assert the result
        assert_eq!(p0, Literals::empty());
    }
}
#[cfg(test)]
mod tests_rug_473 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut v149 = Literals::empty();
        let p0: &Literals = &v149;
        <hir::literal::Literals>::longest_common_suffix(p0);
    }
}#[cfg(test)]
mod tests_rug_474 {
    use super::*;
    #[test]
    fn test_rug() {
        use crate::hir::literal::{Literals, Literal};
        let mut v149 = Literals::empty();
        let p0 = &v149;
        let p1: usize = 10;

        Literal::trim_suffix(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_475 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};
    use crate::hir::Hir;
    use crate::parse::Parser;

    #[test]
    fn test_union_prefixes() {
        let mut v149 = Literals::empty();
        let mut v148 = literal(parse("sample").unwrap());

        v149.union_prefixes(&v148);
    }
}


#[cfg(test)]
mod tests_rug_476 {
    use super::*;
    use crate::hir::literal::{Literals, Literal, literal, parse};


    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty(); // create the local variable p0 with type hir::literal::Literals
        let mut p1 = literal(parse("sample").unwrap()); // create the local variable p1 with type hir::literal::Literal

        <hir::literal::Literals>::union_suffixes(&mut p0, &p1);

    }
}

#[cfg(test)]
mod tests_rug_477 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty();
        let mut p1 = Literals::empty();

        <hir::literal::Literals>::cross_product(&mut p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_478 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};
    
    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty();
        let mut p1 = Literal::new(vec![b'a', b'b', b'c']);
        
        p0.add(p1);
    }
}#[cfg(test)]
mod tests_rug_479 {
    use super::*;
    use crate::hir::literal::{Literals, ClassUnicode, ClassUnicodeRange};

    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty();
        let mut p1 = ClassUnicode::new(vec![
            ClassUnicodeRange::single('a'),
            ClassUnicodeRange::single('b'),
        ]);
        
        <hir::literal::Literals>::add_char_class_reverse(&mut p0, &p1);
    }
}#[cfg(test)]
mod tests_rug_480 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};
    use crate::hir::literal::mod_rs::*;

    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty();

        let mut p1 = ClassUnicode::new(vec![
            ClassUnicodeRange::single('a'),
            ClassUnicodeRange::single('b'),
        ]);

        let p2 = true;

        <hir::literal::Literals>::_add_char_class(&mut p0, &p1, p2);
    }
}
#[cfg(test)]
mod tests_rug_481_prepare {
    #[test]
    fn prepare_literals() {
        use crate::hir::literal::{Literals, Literal};

        let mut literals = Literals::empty();
        literals.push(Literal::empty());
    }

    #[test]
    fn prepare_class_bytes() {
        use crate::hir::{ClassBytes, ClassBytesRange, IntervalSet};

        let mut class_bytes = ClassBytes::new(vec![
            ClassBytesRange::new(b'a', b'z'),
            ClassBytesRange::new(b'A', b'Z'),
        ]);

        for range in class_bytes.iter() {
            println!("Range: {}-{}", range.start, range.end);
        }
    }
}

#[cfg(test)]
mod tests_rug_481 {
    use super::*;
    use crate::hir::{Literals, ClassBytes};

    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty();
        p0.push(Literal::empty());
        
        let mut p1 = ClassBytes::new(vec![
            ClassBytesRange::new(b'a', b'z'),
            ClassBytesRange::new(b'A', b'Z'),
        ]);

       <hir::literal::Literals>::add_byte_class(&mut p0, &p1);
        
       assert_eq!(p0.len(), 52);
       assert_eq!(p0.iter().count(), 52);
       assert_eq!(p1.len(), 2);
       
       for literal in p0.iter() {
           println!("Literal: {:?}", literal);
       }
       
    }
}#[cfg(test)]
mod tests_rug_482 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_cut() {
        let mut v149 = Literals::empty();
        v149.cut();
    }
}
#[cfg(test)]
mod tests_rug_483_prepare {
    #[test]
    fn sample() {
        use crate::hir::literal::{Literals, Literal};

       let mut v149 = Literals::empty(); // create the local variable v149 with type hir::literal::Literals


        // construct the variables for testing
    }
}

#[cfg(test)]
mod tests_rug_483 {
    use super::*;

    #[test]
    fn test_rug() {
        let mut p0 = literals::Literals::empty();
        

        literals::Literals::reverse(&mut p0);
        
        // assert the results
    }
}
#[cfg(test)]
mod tests_rug_484 {
    use super::*;
    use crate::hir::literal::{Literals, Literal}; // add the use statement for Literals and Literal

    #[test]
    fn test_rug() {
        let mut p0 = Literals::empty(); // fill in the p0 variable

        <hir::literal::Literals>::clear(&mut p0); // call the clear method on p0
    }
}#[cfg(test)]
mod tests_rug_485 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut v149 = Literals::empty();
        <hir::literal::Literals>::num_bytes(&v149);
    }
}#[cfg(test)]
mod tests_rug_486 {
    use super::*;
    use crate::hir::literal::{Literals, Literal};

    #[test]
    fn test_rug() {
        let mut v149 = Literals::empty();
        let p0 = &mut v149; // Use mutable reference to pass to the function

        let p1: usize = 10; // Sample data for size argument

        p0.class_exceeds_limits(p1);
    }
}