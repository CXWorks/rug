use std::collections::HashMap;
use std::fmt;
use std::iter;
use std::result;
use std::sync::Arc;
use syntax::hir::{self, Hir};
use syntax::is_word_byte;
use syntax::utf8::{Utf8Range, Utf8Sequence, Utf8Sequences};
use prog::{
    EmptyLook, Inst, InstBytes, InstChar, InstEmptyLook, InstPtr, InstRanges, InstSave,
    InstSplit, Program,
};
use Error;
type Result = result::Result<Patch, Error>;
type ResultOrEmpty = result::Result<Option<Patch>, Error>;
#[derive(Debug)]
struct Patch {
    hole: Hole,
    entry: InstPtr,
}
/// A compiler translates a regular expression AST to a sequence of
/// instructions. The sequence of instructions represents an NFA.
#[allow(missing_debug_implementations)]
pub struct Compiler {
    insts: Vec<MaybeInst>,
    compiled: Program,
    capture_name_idx: HashMap<String, usize>,
    num_exprs: usize,
    size_limit: usize,
    suffix_cache: SuffixCache,
    utf8_seqs: Option<Utf8Sequences>,
    byte_classes: ByteClassSet,
}
impl Compiler {
    /// Create a new regular expression compiler.
    ///
    /// Various options can be set before calling `compile` on an expression.
    pub fn new() -> Self {
        Compiler {
            insts: vec![],
            compiled: Program::new(),
            capture_name_idx: HashMap::new(),
            num_exprs: 0,
            size_limit: 10 * (1 << 20),
            suffix_cache: SuffixCache::new(1000),
            utf8_seqs: Some(Utf8Sequences::new('\x00', '\x00')),
            byte_classes: ByteClassSet::new(),
        }
    }
    /// The size of the resulting program is limited by size_limit. If
    /// the program approximately exceeds the given size (in bytes), then
    /// compilation will stop and return an error.
    pub fn size_limit(mut self, size_limit: usize) -> Self {
        self.size_limit = size_limit;
        self
    }
    /// If bytes is true, then the program is compiled as a byte based
    /// automaton, which incorporates UTF-8 decoding into the machine. If it's
    /// false, then the automaton is Unicode scalar value based, e.g., an
    /// engine utilizing such an automaton is responsible for UTF-8 decoding.
    ///
    /// The specific invariant is that when returning a byte based machine,
    /// the neither the `Char` nor `Ranges` instructions are produced.
    /// Conversely, when producing a Unicode scalar value machine, the `Bytes`
    /// instruction is never produced.
    ///
    /// Note that `dfa(true)` implies `bytes(true)`.
    pub fn bytes(mut self, yes: bool) -> Self {
        self.compiled.is_bytes = yes;
        self
    }
    /// When disabled, the program compiled may match arbitrary bytes.
    ///
    /// When enabled (the default), all compiled programs exclusively match
    /// valid UTF-8 bytes.
    pub fn only_utf8(mut self, yes: bool) -> Self {
        self.compiled.only_utf8 = yes;
        self
    }
    /// When set, the machine returned is suitable for use in the DFA matching
    /// engine.
    ///
    /// In particular, this ensures that if the regex is not anchored in the
    /// beginning, then a preceding `.*?` is included in the program. (The NFA
    /// based engines handle the preceding `.*?` explicitly, which is difficult
    /// or impossible in the DFA engine.)
    pub fn dfa(mut self, yes: bool) -> Self {
        self.compiled.is_dfa = yes;
        self
    }
    /// When set, the machine returned is suitable for matching text in
    /// reverse. In particular, all concatenations are flipped.
    pub fn reverse(mut self, yes: bool) -> Self {
        self.compiled.is_reverse = yes;
        self
    }
    /// Compile a regular expression given its AST.
    ///
    /// The compiler is guaranteed to succeed unless the program exceeds the
    /// specified size limit. If the size limit is exceeded, then compilation
    /// stops and returns an error.
    pub fn compile(mut self, exprs: &[Hir]) -> result::Result<Program, Error> {
        debug_assert!(! exprs.is_empty());
        self.num_exprs = exprs.len();
        if exprs.len() == 1 {
            self.compile_one(&exprs[0])
        } else {
            self.compile_many(exprs)
        }
    }
    fn compile_one(mut self, expr: &Hir) -> result::Result<Program, Error> {
        let mut dotstar_patch = Patch {
            hole: Hole::None,
            entry: 0,
        };
        self.compiled.is_anchored_start = expr.is_anchored_start();
        self.compiled.is_anchored_end = expr.is_anchored_end();
        if self.compiled.needs_dotstar() {
            dotstar_patch = self.c_dotstar()?;
            self.compiled.start = dotstar_patch.entry;
        }
        self.compiled.captures = vec![None];
        let patch = self.c_capture(0, expr)?.unwrap_or(self.next_inst());
        if self.compiled.needs_dotstar() {
            self.fill(dotstar_patch.hole, patch.entry);
        } else {
            self.compiled.start = patch.entry;
        }
        self.fill_to_next(patch.hole);
        self.compiled.matches = vec![self.insts.len()];
        self.push_compiled(Inst::Match(0));
        self.compile_finish()
    }
    fn compile_many(mut self, exprs: &[Hir]) -> result::Result<Program, Error> {
        debug_assert!(exprs.len() > 1);
        self.compiled.is_anchored_start = exprs.iter().all(|e| e.is_anchored_start());
        self.compiled.is_anchored_end = exprs.iter().all(|e| e.is_anchored_end());
        let mut dotstar_patch = Patch {
            hole: Hole::None,
            entry: 0,
        };
        if self.compiled.needs_dotstar() {
            dotstar_patch = self.c_dotstar()?;
            self.compiled.start = dotstar_patch.entry;
        } else {
            self.compiled.start = 0;
        }
        self.fill_to_next(dotstar_patch.hole);
        let mut prev_hole = Hole::None;
        for (i, expr) in exprs[0..exprs.len() - 1].iter().enumerate() {
            self.fill_to_next(prev_hole);
            let split = self.push_split_hole();
            let Patch { hole, entry } = self
                .c_capture(0, expr)?
                .unwrap_or(self.next_inst());
            self.fill_to_next(hole);
            self.compiled.matches.push(self.insts.len());
            self.push_compiled(Inst::Match(i));
            prev_hole = self.fill_split(split, Some(entry), None);
        }
        let i = exprs.len() - 1;
        let Patch { hole, entry } = self
            .c_capture(0, &exprs[i])?
            .unwrap_or(self.next_inst());
        self.fill(prev_hole, entry);
        self.fill_to_next(hole);
        self.compiled.matches.push(self.insts.len());
        self.push_compiled(Inst::Match(i));
        self.compile_finish()
    }
    fn compile_finish(mut self) -> result::Result<Program, Error> {
        self.compiled.insts = self.insts.into_iter().map(|inst| inst.unwrap()).collect();
        self.compiled.byte_classes = self.byte_classes.byte_classes();
        self.compiled.capture_name_idx = Arc::new(self.capture_name_idx);
        Ok(self.compiled)
    }
    /// Compile expr into self.insts, returning a patch on success,
    /// or an error if we run out of memory.
    ///
    /// All of the c_* methods of the compiler share the contract outlined
    /// here.
    ///
    /// The main thing that a c_* method does is mutate `self.insts`
    /// to add a list of mostly compiled instructions required to execute
    /// the given expression. `self.insts` contains MaybeInsts rather than
    /// Insts because there is some backpatching required.
    ///
    /// The `Patch` value returned by each c_* method provides metadata
    /// about the compiled instructions emitted to `self.insts`. The
    /// `entry` member of the patch refers to the first instruction
    /// (the entry point), while the `hole` member contains zero or
    /// more offsets to partial instructions that need to be backpatched.
    /// The c_* routine can't know where its list of instructions are going to
    /// jump to after execution, so it is up to the caller to patch
    /// these jumps to point to the right place. So compiling some
    /// expression, e, we would end up with a situation that looked like:
    ///
    /// ```text
    /// self.insts = [ ..., i1, i2, ..., iexit1, ..., iexitn, ...]
    ///                     ^              ^             ^
    ///                     |                \         /
    ///                   entry                \     /
    ///                                         hole
    /// ```
    ///
    /// To compile two expressions, e1 and e2, concatenated together we
    /// would do:
    ///
    /// ```ignore
    /// let patch1 = self.c(e1);
    /// let patch2 = self.c(e2);
    /// ```
    ///
    /// while leaves us with a situation that looks like
    ///
    /// ```text
    /// self.insts = [ ..., i1, ..., iexit1, ..., i2, ..., iexit2 ]
    ///                     ^        ^            ^        ^
    ///                     |        |            |        |
    ///                entry1        hole1   entry2        hole2
    /// ```
    ///
    /// Then to merge the two patches together into one we would backpatch
    /// hole1 with entry2 and return a new patch that enters at entry1
    /// and has hole2 for a hole. In fact, if you look at the c_concat
    /// method you will see that it does exactly this, though it handles
    /// a list of expressions rather than just the two that we use for
    /// an example.
    ///
    /// Ok(None) is returned when an expression is compiled to no
    /// instruction, and so no patch.entry value makes sense.
    fn c(&mut self, expr: &Hir) -> ResultOrEmpty {
        use prog;
        use syntax::hir::HirKind::*;
        self.check_size()?;
        match *expr.kind() {
            Empty => Ok(None),
            Literal(hir::Literal::Unicode(c)) => self.c_char(c),
            Literal(hir::Literal::Byte(b)) => {
                assert!(self.compiled.uses_bytes());
                self.c_byte(b)
            }
            Class(hir::Class::Unicode(ref cls)) => self.c_class(cls.ranges()),
            Class(hir::Class::Bytes(ref cls)) => {
                if self.compiled.uses_bytes() {
                    self.c_class_bytes(cls.ranges())
                } else {
                    assert!(cls.is_all_ascii());
                    let mut char_ranges = vec![];
                    for r in cls.iter() {
                        let (s, e) = (r.start() as char, r.end() as char);
                        char_ranges.push(hir::ClassUnicodeRange::new(s, e));
                    }
                    self.c_class(&char_ranges)
                }
            }
            Anchor(hir::Anchor::StartLine) if self.compiled.is_reverse => {
                self.byte_classes.set_range(b'\n', b'\n');
                self.c_empty_look(prog::EmptyLook::EndLine)
            }
            Anchor(hir::Anchor::StartLine) => {
                self.byte_classes.set_range(b'\n', b'\n');
                self.c_empty_look(prog::EmptyLook::StartLine)
            }
            Anchor(hir::Anchor::EndLine) if self.compiled.is_reverse => {
                self.byte_classes.set_range(b'\n', b'\n');
                self.c_empty_look(prog::EmptyLook::StartLine)
            }
            Anchor(hir::Anchor::EndLine) => {
                self.byte_classes.set_range(b'\n', b'\n');
                self.c_empty_look(prog::EmptyLook::EndLine)
            }
            Anchor(hir::Anchor::StartText) if self.compiled.is_reverse => {
                self.c_empty_look(prog::EmptyLook::EndText)
            }
            Anchor(hir::Anchor::StartText) => {
                self.c_empty_look(prog::EmptyLook::StartText)
            }
            Anchor(hir::Anchor::EndText) if self.compiled.is_reverse => {
                self.c_empty_look(prog::EmptyLook::StartText)
            }
            Anchor(hir::Anchor::EndText) => self.c_empty_look(prog::EmptyLook::EndText),
            WordBoundary(hir::WordBoundary::Unicode) => {
                if !cfg!(feature = "unicode-perl") {
                    return Err(
                        Error::Syntax(
                            "Unicode word boundaries are unavailable when \
                         the unicode-perl feature is disabled"
                                .to_string(),
                        ),
                    );
                }
                self.compiled.has_unicode_word_boundary = true;
                self.byte_classes.set_word_boundary();
                self.c_empty_look(prog::EmptyLook::WordBoundary)
            }
            WordBoundary(hir::WordBoundary::UnicodeNegate) => {
                if !cfg!(feature = "unicode-perl") {
                    return Err(
                        Error::Syntax(
                            "Unicode word boundaries are unavailable when \
                         the unicode-perl feature is disabled"
                                .to_string(),
                        ),
                    );
                }
                self.compiled.has_unicode_word_boundary = true;
                self.byte_classes.set_word_boundary();
                self.c_empty_look(prog::EmptyLook::NotWordBoundary)
            }
            WordBoundary(hir::WordBoundary::Ascii) => {
                self.byte_classes.set_word_boundary();
                self.c_empty_look(prog::EmptyLook::WordBoundaryAscii)
            }
            WordBoundary(hir::WordBoundary::AsciiNegate) => {
                self.byte_classes.set_word_boundary();
                self.c_empty_look(prog::EmptyLook::NotWordBoundaryAscii)
            }
            Group(ref g) => {
                match g.kind {
                    hir::GroupKind::NonCapturing => self.c(&g.hir),
                    hir::GroupKind::CaptureIndex(index) => {
                        if index as usize >= self.compiled.captures.len() {
                            self.compiled.captures.push(None);
                        }
                        self.c_capture(2 * index as usize, &g.hir)
                    }
                    hir::GroupKind::CaptureName { index, ref name } => {
                        if index as usize >= self.compiled.captures.len() {
                            let n = name.to_string();
                            self.compiled.captures.push(Some(n.clone()));
                            self.capture_name_idx.insert(n, index as usize);
                        }
                        self.c_capture(2 * index as usize, &g.hir)
                    }
                }
            }
            Concat(ref es) => {
                if self.compiled.is_reverse {
                    self.c_concat(es.iter().rev())
                } else {
                    self.c_concat(es)
                }
            }
            Alternation(ref es) => self.c_alternate(&**es),
            Repetition(ref rep) => self.c_repeat(rep),
        }
    }
    fn c_capture(&mut self, first_slot: usize, expr: &Hir) -> ResultOrEmpty {
        if self.num_exprs > 1 || self.compiled.is_dfa {
            self.c(expr)
        } else {
            let entry = self.insts.len();
            let hole = self.push_hole(InstHole::Save { slot: first_slot });
            let patch = self.c(expr)?.unwrap_or(self.next_inst());
            self.fill(hole, patch.entry);
            self.fill_to_next(patch.hole);
            let hole = self
                .push_hole(InstHole::Save {
                    slot: first_slot + 1,
                });
            Ok(Some(Patch { hole: hole, entry: entry }))
        }
    }
    fn c_dotstar(&mut self) -> Result {
        Ok(
            if !self.compiled.only_utf8() {
                self.c(
                        &Hir::repetition(hir::Repetition {
                            kind: hir::RepetitionKind::ZeroOrMore,
                            greedy: false,
                            hir: Box::new(Hir::any(true)),
                        }),
                    )?
                    .unwrap()
            } else {
                self.c(
                        &Hir::repetition(hir::Repetition {
                            kind: hir::RepetitionKind::ZeroOrMore,
                            greedy: false,
                            hir: Box::new(Hir::any(false)),
                        }),
                    )?
                    .unwrap()
            },
        )
    }
    fn c_char(&mut self, c: char) -> ResultOrEmpty {
        if self.compiled.uses_bytes() {
            if c.is_ascii() {
                let b = c as u8;
                let hole = self
                    .push_hole(InstHole::Bytes {
                        start: b,
                        end: b,
                    });
                self.byte_classes.set_range(b, b);
                Ok(
                    Some(Patch {
                        hole,
                        entry: self.insts.len() - 1,
                    }),
                )
            } else {
                self.c_class(&[hir::ClassUnicodeRange::new(c, c)])
            }
        } else {
            let hole = self.push_hole(InstHole::Char { c: c });
            Ok(
                Some(Patch {
                    hole,
                    entry: self.insts.len() - 1,
                }),
            )
        }
    }
    fn c_class(&mut self, ranges: &[hir::ClassUnicodeRange]) -> ResultOrEmpty {
        assert!(! ranges.is_empty());
        if self.compiled.uses_bytes() {
            Ok(
                Some(
                    CompileClass {
                        c: self,
                        ranges: ranges,
                    }
                        .compile()?,
                ),
            )
        } else {
            let ranges: Vec<(char, char)> = ranges
                .iter()
                .map(|r| (r.start(), r.end()))
                .collect();
            let hole = if ranges.len() == 1 && ranges[0].0 == ranges[0].1 {
                self.push_hole(InstHole::Char { c: ranges[0].0 })
            } else {
                self.push_hole(InstHole::Ranges { ranges: ranges })
            };
            Ok(
                Some(Patch {
                    hole: hole,
                    entry: self.insts.len() - 1,
                }),
            )
        }
    }
    fn c_byte(&mut self, b: u8) -> ResultOrEmpty {
        self.c_class_bytes(&[hir::ClassBytesRange::new(b, b)])
    }
    fn c_class_bytes(&mut self, ranges: &[hir::ClassBytesRange]) -> ResultOrEmpty {
        debug_assert!(! ranges.is_empty());
        let first_split_entry = self.insts.len();
        let mut holes = vec![];
        let mut prev_hole = Hole::None;
        for r in &ranges[0..ranges.len() - 1] {
            self.fill_to_next(prev_hole);
            let split = self.push_split_hole();
            let next = self.insts.len();
            self.byte_classes.set_range(r.start(), r.end());
            holes
                .push(
                    self
                        .push_hole(InstHole::Bytes {
                            start: r.start(),
                            end: r.end(),
                        }),
                );
            prev_hole = self.fill_split(split, Some(next), None);
        }
        let next = self.insts.len();
        let r = &ranges[ranges.len() - 1];
        self.byte_classes.set_range(r.start(), r.end());
        holes
            .push(
                self
                    .push_hole(InstHole::Bytes {
                        start: r.start(),
                        end: r.end(),
                    }),
            );
        self.fill(prev_hole, next);
        Ok(
            Some(Patch {
                hole: Hole::Many(holes),
                entry: first_split_entry,
            }),
        )
    }
    fn c_empty_look(&mut self, look: EmptyLook) -> ResultOrEmpty {
        let hole = self.push_hole(InstHole::EmptyLook { look: look });
        Ok(
            Some(Patch {
                hole: hole,
                entry: self.insts.len() - 1,
            }),
        )
    }
    fn c_concat<'a, I>(&mut self, exprs: I) -> ResultOrEmpty
    where
        I: IntoIterator<Item = &'a Hir>,
    {
        let mut exprs = exprs.into_iter();
        let Patch { mut hole, entry } = loop {
            match exprs.next() {
                None => return Ok(None),
                Some(e) => {
                    if let Some(p) = self.c(e)? {
                        break p;
                    }
                }
            }
        };
        for e in exprs {
            if let Some(p) = self.c(e)? {
                self.fill(hole, p.entry);
                hole = p.hole;
            }
        }
        Ok(Some(Patch { hole: hole, entry: entry }))
    }
    fn c_alternate(&mut self, exprs: &[Hir]) -> ResultOrEmpty {
        debug_assert!(exprs.len() >= 2, "alternates must have at least 2 exprs");
        let first_split_entry = self.insts.len();
        let mut holes = vec![];
        let mut prev_hole = (Hole::None, false);
        for e in &exprs[0..exprs.len() - 1] {
            if prev_hole.1 {
                let next = self.insts.len();
                self.fill_split(prev_hole.0, None, Some(next));
            } else {
                self.fill_to_next(prev_hole.0);
            }
            let split = self.push_split_hole();
            if let Some(Patch { hole, entry }) = self.c(e)? {
                holes.push(hole);
                prev_hole = (self.fill_split(split, Some(entry), None), false);
            } else {
                let (split1, split2) = split.dup_one();
                holes.push(split1);
                prev_hole = (split2, true);
            }
        }
        if let Some(Patch { hole, entry }) = self.c(&exprs[exprs.len() - 1])? {
            holes.push(hole);
            if prev_hole.1 {
                self.fill_split(prev_hole.0, None, Some(entry));
            } else {
                self.fill(prev_hole.0, entry);
            }
        } else {
            holes.push(prev_hole.0);
        }
        Ok(
            Some(Patch {
                hole: Hole::Many(holes),
                entry: first_split_entry,
            }),
        )
    }
    fn c_repeat(&mut self, rep: &hir::Repetition) -> ResultOrEmpty {
        use syntax::hir::RepetitionKind::*;
        match rep.kind {
            ZeroOrOne => self.c_repeat_zero_or_one(&rep.hir, rep.greedy),
            ZeroOrMore => self.c_repeat_zero_or_more(&rep.hir, rep.greedy),
            OneOrMore => self.c_repeat_one_or_more(&rep.hir, rep.greedy),
            Range(hir::RepetitionRange::Exactly(min_max)) => {
                self.c_repeat_range(&rep.hir, rep.greedy, min_max, min_max)
            }
            Range(hir::RepetitionRange::AtLeast(min)) => {
                self.c_repeat_range_min_or_more(&rep.hir, rep.greedy, min)
            }
            Range(hir::RepetitionRange::Bounded(min, max)) => {
                self.c_repeat_range(&rep.hir, rep.greedy, min, max)
            }
        }
    }
    fn c_repeat_zero_or_one(&mut self, expr: &Hir, greedy: bool) -> ResultOrEmpty {
        let split_entry = self.insts.len();
        let split = self.push_split_hole();
        let Patch { hole: hole_rep, entry: entry_rep } = match self.c(expr)? {
            Some(p) => p,
            None => return self.pop_split_hole(),
        };
        let split_hole = if greedy {
            self.fill_split(split, Some(entry_rep), None)
        } else {
            self.fill_split(split, None, Some(entry_rep))
        };
        let holes = vec![hole_rep, split_hole];
        Ok(
            Some(Patch {
                hole: Hole::Many(holes),
                entry: split_entry,
            }),
        )
    }
    fn c_repeat_zero_or_more(&mut self, expr: &Hir, greedy: bool) -> ResultOrEmpty {
        let split_entry = self.insts.len();
        let split = self.push_split_hole();
        let Patch { hole: hole_rep, entry: entry_rep } = match self.c(expr)? {
            Some(p) => p,
            None => return self.pop_split_hole(),
        };
        self.fill(hole_rep, split_entry);
        let split_hole = if greedy {
            self.fill_split(split, Some(entry_rep), None)
        } else {
            self.fill_split(split, None, Some(entry_rep))
        };
        Ok(
            Some(Patch {
                hole: split_hole,
                entry: split_entry,
            }),
        )
    }
    fn c_repeat_one_or_more(&mut self, expr: &Hir, greedy: bool) -> ResultOrEmpty {
        let Patch { hole: hole_rep, entry: entry_rep } = match self.c(expr)? {
            Some(p) => p,
            None => return Ok(None),
        };
        self.fill_to_next(hole_rep);
        let split = self.push_split_hole();
        let split_hole = if greedy {
            self.fill_split(split, Some(entry_rep), None)
        } else {
            self.fill_split(split, None, Some(entry_rep))
        };
        Ok(
            Some(Patch {
                hole: split_hole,
                entry: entry_rep,
            }),
        )
    }
    fn c_repeat_range_min_or_more(
        &mut self,
        expr: &Hir,
        greedy: bool,
        min: u32,
    ) -> ResultOrEmpty {
        let min = u32_to_usize(min);
        let patch_concat = self
            .c_concat(iter::repeat(expr).take(min))?
            .unwrap_or(self.next_inst());
        if let Some(patch_rep) = self.c_repeat_zero_or_more(expr, greedy)? {
            self.fill(patch_concat.hole, patch_rep.entry);
            Ok(
                Some(Patch {
                    hole: patch_rep.hole,
                    entry: patch_concat.entry,
                }),
            )
        } else {
            Ok(None)
        }
    }
    fn c_repeat_range(
        &mut self,
        expr: &Hir,
        greedy: bool,
        min: u32,
        max: u32,
    ) -> ResultOrEmpty {
        let (min, max) = (u32_to_usize(min), u32_to_usize(max));
        debug_assert!(min <= max);
        let patch_concat = self.c_concat(iter::repeat(expr).take(min))?;
        if min == max {
            return Ok(patch_concat);
        }
        let patch_concat = patch_concat.unwrap_or(self.next_inst());
        let initial_entry = patch_concat.entry;
        let mut holes = vec![];
        let mut prev_hole = patch_concat.hole;
        for _ in min..max {
            self.fill_to_next(prev_hole);
            let split = self.push_split_hole();
            let Patch { hole, entry } = match self.c(expr)? {
                Some(p) => p,
                None => return self.pop_split_hole(),
            };
            prev_hole = hole;
            if greedy {
                holes.push(self.fill_split(split, Some(entry), None));
            } else {
                holes.push(self.fill_split(split, None, Some(entry)));
            }
        }
        holes.push(prev_hole);
        Ok(
            Some(Patch {
                hole: Hole::Many(holes),
                entry: initial_entry,
            }),
        )
    }
    /// Can be used as a default value for the c_* functions when the call to
    /// c_function is followed by inserting at least one instruction that is
    /// always executed after the ones written by the c* function.
    fn next_inst(&self) -> Patch {
        Patch {
            hole: Hole::None,
            entry: self.insts.len(),
        }
    }
    fn fill(&mut self, hole: Hole, goto: InstPtr) {
        match hole {
            Hole::None => {}
            Hole::One(pc) => {
                self.insts[pc].fill(goto);
            }
            Hole::Many(holes) => {
                for hole in holes {
                    self.fill(hole, goto);
                }
            }
        }
    }
    fn fill_to_next(&mut self, hole: Hole) {
        let next = self.insts.len();
        self.fill(hole, next);
    }
    fn fill_split(
        &mut self,
        hole: Hole,
        goto1: Option<InstPtr>,
        goto2: Option<InstPtr>,
    ) -> Hole {
        match hole {
            Hole::None => Hole::None,
            Hole::One(pc) => {
                match (goto1, goto2) {
                    (Some(goto1), Some(goto2)) => {
                        self.insts[pc].fill_split(goto1, goto2);
                        Hole::None
                    }
                    (Some(goto1), None) => {
                        self.insts[pc].half_fill_split_goto1(goto1);
                        Hole::One(pc)
                    }
                    (None, Some(goto2)) => {
                        self.insts[pc].half_fill_split_goto2(goto2);
                        Hole::One(pc)
                    }
                    (None, None) => {
                        unreachable!(
                            "at least one of the split \
                     holes must be filled"
                        )
                    }
                }
            }
            Hole::Many(holes) => {
                let mut new_holes = vec![];
                for hole in holes {
                    new_holes.push(self.fill_split(hole, goto1, goto2));
                }
                if new_holes.is_empty() {
                    Hole::None
                } else if new_holes.len() == 1 {
                    new_holes.pop().unwrap()
                } else {
                    Hole::Many(new_holes)
                }
            }
        }
    }
    fn push_compiled(&mut self, inst: Inst) {
        self.insts.push(MaybeInst::Compiled(inst));
    }
    fn push_hole(&mut self, inst: InstHole) -> Hole {
        let hole = self.insts.len();
        self.insts.push(MaybeInst::Uncompiled(inst));
        Hole::One(hole)
    }
    fn push_split_hole(&mut self) -> Hole {
        let hole = self.insts.len();
        self.insts.push(MaybeInst::Split);
        Hole::One(hole)
    }
    fn pop_split_hole(&mut self) -> ResultOrEmpty {
        self.insts.pop();
        Ok(None)
    }
    fn check_size(&self) -> result::Result<(), Error> {
        use std::mem::size_of;
        if self.insts.len() * size_of::<Inst>() > self.size_limit {
            Err(Error::CompiledTooBig(self.size_limit))
        } else {
            Ok(())
        }
    }
}
#[derive(Debug)]
enum Hole {
    None,
    One(InstPtr),
    Many(Vec<Hole>),
}
impl Hole {
    fn dup_one(self) -> (Self, Self) {
        match self {
            Hole::One(pc) => (Hole::One(pc), Hole::One(pc)),
            Hole::None | Hole::Many(_) => unreachable!("must be called on single hole"),
        }
    }
}
#[derive(Clone, Debug)]
enum MaybeInst {
    Compiled(Inst),
    Uncompiled(InstHole),
    Split,
    Split1(InstPtr),
    Split2(InstPtr),
}
impl MaybeInst {
    fn fill(&mut self, goto: InstPtr) {
        let maybeinst = match *self {
            MaybeInst::Split => MaybeInst::Split1(goto),
            MaybeInst::Uncompiled(ref inst) => MaybeInst::Compiled(inst.fill(goto)),
            MaybeInst::Split1(goto1) => {
                MaybeInst::Compiled(
                    Inst::Split(InstSplit {
                        goto1: goto1,
                        goto2: goto,
                    }),
                )
            }
            MaybeInst::Split2(goto2) => {
                MaybeInst::Compiled(
                    Inst::Split(InstSplit {
                        goto1: goto,
                        goto2: goto2,
                    }),
                )
            }
            _ => {
                unreachable!(
                    "not all instructions were compiled! \
                 found uncompiled instruction: {:?}",
                    self
                )
            }
        };
        *self = maybeinst;
    }
    fn fill_split(&mut self, goto1: InstPtr, goto2: InstPtr) {
        let filled = match *self {
            MaybeInst::Split => {
                Inst::Split(InstSplit {
                    goto1: goto1,
                    goto2: goto2,
                })
            }
            _ => {
                unreachable!(
                    "must be called on Split instruction, \
                 instead it was called on: {:?}",
                    self
                )
            }
        };
        *self = MaybeInst::Compiled(filled);
    }
    fn half_fill_split_goto1(&mut self, goto1: InstPtr) {
        let half_filled = match *self {
            MaybeInst::Split => goto1,
            _ => {
                unreachable!(
                    "must be called on Split instruction, \
                 instead it was called on: {:?}",
                    self
                )
            }
        };
        *self = MaybeInst::Split1(half_filled);
    }
    fn half_fill_split_goto2(&mut self, goto2: InstPtr) {
        let half_filled = match *self {
            MaybeInst::Split => goto2,
            _ => {
                unreachable!(
                    "must be called on Split instruction, \
                 instead it was called on: {:?}",
                    self
                )
            }
        };
        *self = MaybeInst::Split2(half_filled);
    }
    fn unwrap(self) -> Inst {
        match self {
            MaybeInst::Compiled(inst) => inst,
            _ => {
                unreachable!(
                    "must be called on a compiled instruction, \
                 instead it was called on: {:?}",
                    self
                )
            }
        }
    }
}
#[derive(Clone, Debug)]
enum InstHole {
    Save { slot: usize },
    EmptyLook { look: EmptyLook },
    Char { c: char },
    Ranges { ranges: Vec<(char, char)> },
    Bytes { start: u8, end: u8 },
}
impl InstHole {
    fn fill(&self, goto: InstPtr) -> Inst {
        match *self {
            InstHole::Save { slot } => Inst::Save(InstSave { goto: goto, slot: slot }),
            InstHole::EmptyLook { look } => {
                Inst::EmptyLook(InstEmptyLook {
                    goto: goto,
                    look: look,
                })
            }
            InstHole::Char { c } => Inst::Char(InstChar { goto: goto, c: c }),
            InstHole::Ranges { ref ranges } => {
                Inst::Ranges(InstRanges {
                    goto: goto,
                    ranges: ranges.clone(),
                })
            }
            InstHole::Bytes { start, end } => {
                Inst::Bytes(InstBytes {
                    goto: goto,
                    start: start,
                    end: end,
                })
            }
        }
    }
}
struct CompileClass<'a, 'b> {
    c: &'a mut Compiler,
    ranges: &'b [hir::ClassUnicodeRange],
}
impl<'a, 'b> CompileClass<'a, 'b> {
    fn compile(mut self) -> Result {
        let mut holes = vec![];
        let mut initial_entry = None;
        let mut last_split = Hole::None;
        let mut utf8_seqs = self.c.utf8_seqs.take().unwrap();
        self.c.suffix_cache.clear();
        for (i, range) in self.ranges.iter().enumerate() {
            let is_last_range = i + 1 == self.ranges.len();
            utf8_seqs.reset(range.start(), range.end());
            let mut it = (&mut utf8_seqs).peekable();
            loop {
                let utf8_seq = match it.next() {
                    None => break,
                    Some(utf8_seq) => utf8_seq,
                };
                if is_last_range && it.peek().is_none() {
                    let Patch { hole, entry } = self.c_utf8_seq(&utf8_seq)?;
                    holes.push(hole);
                    self.c.fill(last_split, entry);
                    last_split = Hole::None;
                    if initial_entry.is_none() {
                        initial_entry = Some(entry);
                    }
                } else {
                    if initial_entry.is_none() {
                        initial_entry = Some(self.c.insts.len());
                    }
                    self.c.fill_to_next(last_split);
                    last_split = self.c.push_split_hole();
                    let Patch { hole, entry } = self.c_utf8_seq(&utf8_seq)?;
                    holes.push(hole);
                    last_split = self.c.fill_split(last_split, Some(entry), None);
                }
            }
        }
        self.c.utf8_seqs = Some(utf8_seqs);
        Ok(Patch {
            hole: Hole::Many(holes),
            entry: initial_entry.unwrap(),
        })
    }
    fn c_utf8_seq(&mut self, seq: &Utf8Sequence) -> Result {
        if self.c.compiled.is_reverse {
            self.c_utf8_seq_(seq)
        } else {
            self.c_utf8_seq_(seq.into_iter().rev())
        }
    }
    fn c_utf8_seq_<'r, I>(&mut self, seq: I) -> Result
    where
        I: IntoIterator<Item = &'r Utf8Range>,
    {
        let mut from_inst = ::std::usize::MAX;
        let mut last_hole = Hole::None;
        for byte_range in seq {
            let key = SuffixCacheKey {
                from_inst: from_inst,
                start: byte_range.start,
                end: byte_range.end,
            };
            {
                let pc = self.c.insts.len();
                if let Some(cached_pc) = self.c.suffix_cache.get(key, pc) {
                    from_inst = cached_pc;
                    continue;
                }
            }
            self.c.byte_classes.set_range(byte_range.start, byte_range.end);
            if from_inst == ::std::usize::MAX {
                last_hole = self
                    .c
                    .push_hole(InstHole::Bytes {
                        start: byte_range.start,
                        end: byte_range.end,
                    });
            } else {
                self.c
                    .push_compiled(
                        Inst::Bytes(InstBytes {
                            goto: from_inst,
                            start: byte_range.start,
                            end: byte_range.end,
                        }),
                    );
            }
            from_inst = self.c.insts.len().checked_sub(1).unwrap();
            debug_assert!(from_inst < ::std::usize::MAX);
        }
        debug_assert!(from_inst < ::std::usize::MAX);
        Ok(Patch {
            hole: last_hole,
            entry: from_inst,
        })
    }
}
/// `SuffixCache` is a simple bounded hash map for caching suffix entries in
/// UTF-8 automata. For example, consider the Unicode range \u{0}-\u{FFFF}.
/// The set of byte ranges looks like this:
///
/// [0-7F]
/// [C2-DF][80-BF]
/// [E0][A0-BF][80-BF]
/// [E1-EC][80-BF][80-BF]
/// [ED][80-9F][80-BF]
/// [EE-EF][80-BF][80-BF]
///
/// Each line above translates to one alternate in the compiled regex program.
/// However, all but one of the alternates end in the same suffix, which is
/// a waste of an instruction. The suffix cache facilitates reusing them across
/// alternates.
///
/// Note that a HashMap could be trivially used for this, but we don't need its
/// overhead. Some small bounded space (LRU style) is more than enough.
///
/// This uses similar idea to [`SparseSet`](../sparse/struct.SparseSet.html),
/// except it uses hashes as original indices and then compares full keys for
/// validation against `dense` array.
#[derive(Debug)]
struct SuffixCache {
    sparse: Box<[usize]>,
    dense: Vec<SuffixCacheEntry>,
}
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct SuffixCacheEntry {
    key: SuffixCacheKey,
    pc: InstPtr,
}
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct SuffixCacheKey {
    from_inst: InstPtr,
    start: u8,
    end: u8,
}
impl SuffixCache {
    fn new(size: usize) -> Self {
        SuffixCache {
            sparse: vec![0usize; size].into(),
            dense: Vec::with_capacity(size),
        }
    }
    fn get(&mut self, key: SuffixCacheKey, pc: InstPtr) -> Option<InstPtr> {
        let hash = self.hash(&key);
        let pos = &mut self.sparse[hash];
        if let Some(entry) = self.dense.get(*pos) {
            if entry.key == key {
                return Some(entry.pc);
            }
        }
        *pos = self.dense.len();
        self.dense
            .push(SuffixCacheEntry {
                key: key,
                pc: pc,
            });
        None
    }
    fn clear(&mut self) {
        self.dense.clear();
    }
    fn hash(&self, suffix: &SuffixCacheKey) -> usize {
        const FNV_PRIME: u64 = 1099511628211;
        let mut h = 14695981039346656037;
        h = (h ^ (suffix.from_inst as u64)).wrapping_mul(FNV_PRIME);
        h = (h ^ (suffix.start as u64)).wrapping_mul(FNV_PRIME);
        h = (h ^ (suffix.end as u64)).wrapping_mul(FNV_PRIME);
        (h as usize) % self.sparse.len()
    }
}
struct ByteClassSet([bool; 256]);
impl ByteClassSet {
    fn new() -> Self {
        ByteClassSet([false; 256])
    }
    fn set_range(&mut self, start: u8, end: u8) {
        debug_assert!(start <= end);
        if start > 0 {
            self.0[start as usize - 1] = true;
        }
        self.0[end as usize] = true;
    }
    fn set_word_boundary(&mut self) {
        let iswb = is_word_byte;
        let mut b1: u16 = 0;
        let mut b2: u16;
        while b1 <= 255 {
            b2 = b1 + 1;
            while b2 <= 255 && iswb(b1 as u8) == iswb(b2 as u8) {
                b2 += 1;
            }
            self.set_range(b1 as u8, (b2 - 1) as u8);
            b1 = b2;
        }
    }
    fn byte_classes(&self) -> Vec<u8> {
        let mut byte_classes = vec![0; 256];
        let mut class = 0u8;
        let mut i = 0;
        loop {
            byte_classes[i] = class as u8;
            if i >= 255 {
                break;
            }
            if self.0[i] {
                class = class.checked_add(1).unwrap();
            }
            i += 1;
        }
        byte_classes
    }
}
impl fmt::Debug for ByteClassSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ByteClassSet").field(&&self.0[..]).finish()
    }
}
fn u32_to_usize(n: u32) -> usize {
    if (n as u64) > (::std::usize::MAX as u64) {
        panic!("BUG: {} is too big to be pointer sized", n)
    }
    n as usize
}
#[cfg(test)]
mod tests {
    use super::ByteClassSet;
    #[test]
    fn byte_classes() {
        let mut set = ByteClassSet::new();
        set.set_range(b'a', b'z');
        let classes = set.byte_classes();
        assert_eq!(classes[0], 0);
        assert_eq!(classes[1], 0);
        assert_eq!(classes[2], 0);
        assert_eq!(classes[b'a' as usize - 1], 0);
        assert_eq!(classes[b'a' as usize], 1);
        assert_eq!(classes[b'm' as usize], 1);
        assert_eq!(classes[b'z' as usize], 1);
        assert_eq!(classes[b'z' as usize + 1], 2);
        assert_eq!(classes[254], 2);
        assert_eq!(classes[255], 2);
        let mut set = ByteClassSet::new();
        set.set_range(0, 2);
        set.set_range(4, 6);
        let classes = set.byte_classes();
        assert_eq!(classes[0], 0);
        assert_eq!(classes[1], 0);
        assert_eq!(classes[2], 0);
        assert_eq!(classes[3], 1);
        assert_eq!(classes[4], 2);
        assert_eq!(classes[5], 2);
        assert_eq!(classes[6], 2);
        assert_eq!(classes[7], 3);
        assert_eq!(classes[255], 3);
    }
    #[test]
    fn full_byte_classes() {
        let mut set = ByteClassSet::new();
        for i in 0..256u16 {
            set.set_range(i as u8, i as u8);
        }
        assert_eq!(set.byte_classes().len(), 256);
    }
}
#[cfg(test)]
mod tests_llm_16_198 {
    use super::*;
    use crate::*;
    #[test]
    fn test_byte_classes() {
        let _rug_st_tests_llm_16_198_rrrruuuugggg_test_byte_classes = 0;
        let rug_fuzz_0 = 65;
        let rug_fuzz_1 = 90;
        let rug_fuzz_2 = 97;
        let rug_fuzz_3 = 122;
        let mut byte_class_set = ByteClassSet::new();
        byte_class_set.set_range(rug_fuzz_0, rug_fuzz_1);
        byte_class_set.set_range(rug_fuzz_2, rug_fuzz_3);
        byte_class_set.set_word_boundary();
        let byte_classes = byte_class_set.byte_classes();
        debug_assert_eq!(byte_classes, vec![0, 0, 1, 2, 2, 2, 3, 3,]);
        let _rug_ed_tests_llm_16_198_rrrruuuugggg_test_byte_classes = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_199 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_199_rrrruuuugggg_test_new = 0;
        let byte_class_set = ByteClassSet::new();
        debug_assert_eq!(byte_class_set.0, [false; 256]);
        let _rug_ed_tests_llm_16_199_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_200 {
    use super::*;
    use crate::*;
    #[test]
    fn test_set_range() {
        let _rug_st_tests_llm_16_200_rrrruuuugggg_test_set_range = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 10;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 10;
        let mut byte_class_set = ByteClassSet::new();
        byte_class_set.set_range(rug_fuzz_0, rug_fuzz_1);
        debug_assert_eq!(byte_class_set.0[rug_fuzz_2], true);
        debug_assert_eq!(byte_class_set.0[rug_fuzz_3], true);
        let _rug_ed_tests_llm_16_200_rrrruuuugggg_test_set_range = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_201 {
    use super::*;
    use crate::*;
    use crate::compile::ByteClassSet;
    #[test]
    fn set_word_boundary_test() {
        let _rug_st_tests_llm_16_201_rrrruuuugggg_set_word_boundary_test = 0;
        let rug_fuzz_0 = 1;
        let rug_fuzz_1 = 255;
        let mut set = ByteClassSet::new();
        set.set_word_boundary();
        let byte_classes = set.byte_classes();
        for i in rug_fuzz_0..rug_fuzz_1 {
            debug_assert_eq!(
                byte_classes[i], byte_classes[i + 1],
                "Byte classes differ for byte {} and byte {}", i, i + 1
            );
        }
        let _rug_ed_tests_llm_16_201_rrrruuuugggg_set_word_boundary_test = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_217_llm_16_216 {
    use super::*;
    use crate::*;
    #[derive(Debug)]
    struct InstPtr(u64);
    #[derive(Debug)]
    struct SuffixCacheKey {
        from_inst: usize,
        start: usize,
        end: usize,
    }
    #[derive(Debug)]
    struct SuffixCacheEntry {
        key: SuffixCacheKey,
        pc: InstPtr,
    }
    struct SuffixCache {
        sparse: Box<[usize]>,
        dense: Vec<SuffixCacheEntry>,
    }
    impl SuffixCache {
        fn new(size: usize) -> Self {
            SuffixCache {
                sparse: vec![0usize; size].into(),
                dense: Vec::with_capacity(size),
            }
        }
        fn clear(&mut self) {
            self.dense.clear();
        }
        fn hash(&self, suffix: &SuffixCacheKey) -> usize {
            const FNV_PRIME: u64 = 1099511628211;
            let mut h = 14695981039346656037;
            h = (h ^ (suffix.from_inst as u64)).wrapping_mul(FNV_PRIME);
            h = (h ^ (suffix.start as u64)).wrapping_mul(FNV_PRIME);
            h = (h ^ (suffix.end as u64)).wrapping_mul(FNV_PRIME);
            (h as usize) % self.sparse.len()
        }
    }
    #[test]
    fn test_clear() {
        let _rug_st_tests_llm_16_217_llm_16_216_rrrruuuugggg_test_clear = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 1;
        let rug_fuzz_2 = 2;
        let rug_fuzz_3 = 3;
        let rug_fuzz_4 = 4;
        let mut cache = SuffixCache::new(rug_fuzz_0);
        cache
            .dense
            .push(SuffixCacheEntry {
                key: SuffixCacheKey {
                    from_inst: rug_fuzz_1,
                    start: rug_fuzz_2,
                    end: rug_fuzz_3,
                },
                pc: InstPtr(rug_fuzz_4),
            });
        cache.clear();
        debug_assert!(cache.dense.is_empty());
        let _rug_ed_tests_llm_16_217_llm_16_216_rrrruuuugggg_test_clear = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_218 {
    use crate::compile::{SuffixCache, SuffixCacheKey};
    #[test]
    fn test_get_cache_hit() {
        let _rug_st_tests_llm_16_218_rrrruuuugggg_test_get_cache_hit = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 10;
        let mut cache = SuffixCache::new(rug_fuzz_0);
        let key = SuffixCacheKey {
            from_inst: rug_fuzz_1,
            start: rug_fuzz_2,
            end: rug_fuzz_3,
        };
        let pc = rug_fuzz_4;
        cache.get(key, pc);
        let result = cache.get(key, pc);
        debug_assert_eq!(result, Some(pc));
        let _rug_ed_tests_llm_16_218_rrrruuuugggg_test_get_cache_hit = 0;
    }
    #[test]
    fn test_get_cache_miss() {
        let _rug_st_tests_llm_16_218_rrrruuuugggg_test_get_cache_miss = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 42;
        let rug_fuzz_2 = 1;
        let rug_fuzz_3 = 2;
        let rug_fuzz_4 = 10;
        let mut cache = SuffixCache::new(rug_fuzz_0);
        let key = SuffixCacheKey {
            from_inst: rug_fuzz_1,
            start: rug_fuzz_2,
            end: rug_fuzz_3,
        };
        let pc = rug_fuzz_4;
        let result = cache.get(key, pc);
        debug_assert_eq!(result, None);
        let _rug_ed_tests_llm_16_218_rrrruuuugggg_test_get_cache_miss = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_221 {
    use super::*;
    use crate::*;
    use compile::SuffixCache;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_221_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = 10;
        let size = rug_fuzz_0;
        let sc = SuffixCache::new(size);
        debug_assert_eq!(sc.sparse.len(), size);
        debug_assert_eq!(sc.dense.len(), 0);
        let _rug_ed_tests_llm_16_221_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_rug_8 {
    use super::*;
    #[test]
    fn test_u32_to_usize() {
        let _rug_st_tests_rug_8_rrrruuuugggg_test_u32_to_usize = 0;
        let rug_fuzz_0 = 42;
        let p0: u32 = rug_fuzz_0;
        crate::compile::u32_to_usize(p0);
        let _rug_ed_tests_rug_8_rrrruuuugggg_test_u32_to_usize = 0;
    }
}
#[cfg(test)]
mod tests_rug_9 {
    use super::*;
    use crate::compile::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_9_rrrruuuugggg_test_rug = 0;
        Compiler::new();
        let _rug_ed_tests_rug_9_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_10 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_size_limit() {
        let _rug_st_tests_rug_10_rrrruuuugggg_test_size_limit = 0;
        let rug_fuzz_0 = 100;
        let mut p0 = Compiler::new();
        let p1: usize = rug_fuzz_0;
        p0.size_limit(p1);
        let _rug_ed_tests_rug_10_rrrruuuugggg_test_size_limit = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_11_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Compiler::new();
        let p1: bool = rug_fuzz_0;
        p0.bytes(p1);
        let _rug_ed_tests_rug_11_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Compiler::new();
        let p1: bool = rug_fuzz_0;
        p0.only_utf8(p1);
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_13_rrrruuuugggg_sample = 0;
        let rug_fuzz_0 = true;
        #[cfg(test)]
        mod tests_rug_13_prepare {
            use crate::internal::Compiler;
            #[test]
            fn sample() {
                let _rug_st_tests_rug_13_prepare_rrrruuuugggg_sample = 0;
                let rug_fuzz_0 = 0;
                let rug_fuzz_1 = true;
                let rug_fuzz_2 = 0;
                let _rug_st_tests_rug_13_rrrruuuugggg_sample = rug_fuzz_0;
                let rug_fuzz_0 = rug_fuzz_1;
                let mut p0 = Compiler::new();
                let p1 = rug_fuzz_0;
                p0.dfa(p1);
                let _rug_ed_tests_rug_13_rrrruuuugggg_sample = rug_fuzz_2;
                let _rug_ed_tests_rug_13_prepare_rrrruuuugggg_sample = 0;
            }
        }
        let _rug_ed_tests_rug_13_rrrruuuugggg_sample = 0;
    }
}
#[cfg(test)]
mod tests_rug_14 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_14_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = true;
        let mut p0 = Compiler::new();
        let p1: bool = rug_fuzz_0;
        p0.reverse(p1);
        let _rug_ed_tests_rug_14_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::internal::Compiler;
    use crate::syntax::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let mut p1: Vec<Hir> = Vec::new();
        p0.compile(&p1);
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    use crate::internal::Compiler;
    use crate::syntax::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let p1: &Hir = unimplemented!();
        p0.compile_one(p1).unwrap();
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::internal::Compiler;
    use syntax::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_17_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let mut p1: Vec<Hir> = Vec::new();
        crate::compile::Compiler::compile_many(p0, &p1);
        let _rug_ed_tests_rug_17_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        p0.compile_finish();
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_19 {
    use super::*;
    use crate::internal::Compiler;
    use syntax::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_19_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let p1: &Hir = unimplemented!();
        Compiler::c(&mut p0, &p1).unwrap();
        let _rug_ed_tests_rug_19_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::internal::Compiler;
    use crate::syntax::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = Compiler::new();
        let p1: usize = rug_fuzz_0;
        let p2: Hir = unimplemented!();
        p0.c_capture(p1, &p2);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_21 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_21_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        crate::compile::Compiler::c_dotstar(&mut p0).unwrap();
        let _rug_ed_tests_rug_21_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let mut p0 = Compiler::new();
        let p1: char = rug_fuzz_0;
        p0.c_char(p1);
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_23 {
    use crate::internal::Compiler;
    use crate::syntax::hir::ClassUnicodeRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_23_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let mut p1: Vec<ClassUnicodeRange> = Vec::new();
        crate::compile::Compiler::c_class(&mut p0, &p1);
        let _rug_ed_tests_rug_23_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_24 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_24_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 65;
        let mut p0 = Compiler::new();
        let mut p1: u8 = rug_fuzz_0;
        p0.c_byte(p1).unwrap();
        let _rug_ed_tests_rug_24_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::internal::Compiler;
    use syntax::hir::ClassBytesRange;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let p1: Vec<ClassBytesRange> = vec![];
        Compiler::c_class_bytes(&mut p0, &p1);
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_26 {
    use super::*;
    use crate::internal::{Compiler, EmptyLook};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_26_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let p1: EmptyLook = EmptyLook::StartLine;
        p0.c_empty_look(p1);
        let _rug_ed_tests_rug_26_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_28 {
    use crate::internal::Compiler;
    use syntax::hir::Hir;
    #[test]
    fn test_c_alternate() {
        let _rug_st_tests_rug_28_rrrruuuugggg_test_c_alternate = 0;
        let mut compiler = Compiler::new();
        let mut exprs: Vec<Hir> = Vec::new();
        compiler.c_alternate(&exprs);
        let _rug_ed_tests_rug_28_rrrruuuugggg_test_c_alternate = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::internal::Compiler;
    use crate::syntax::hir::Hir;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        let p1: Hir = unimplemented!();
        let p2: bool = unimplemented!();
        let p3: u32 = unimplemented!();
        p0.c_repeat_range_min_or_more(&p1, p2, p3);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_34 {
    use super::*;
    use crate::internal::Compiler;
    use crate::syntax::hir::Hir;
    #[test]
    fn test_c_repeat_range() {
        let _rug_st_tests_rug_34_rrrruuuugggg_test_c_repeat_range = 0;
        let mut compiler = Compiler::new();
        let expression: Hir = unimplemented!();
        let greedy: bool = unimplemented!();
        let min: u32 = unimplemented!();
        let max: u32 = unimplemented!();
        compiler.c_repeat_range(&expression, greedy, min, max);
        let _rug_ed_tests_rug_34_rrrruuuugggg_test_c_repeat_range = 0;
    }
}
#[cfg(test)]
mod tests_rug_35 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_35_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        p0.next_inst();
        let _rug_ed_tests_rug_35_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_39 {
    use super::*;
    use crate::internal::Compiler;
    use crate::internal::Inst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_39_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let mut p0 = Compiler::new();
        let p1 = Inst::Match(rug_fuzz_0);
        p0.push_compiled(p1);
        let _rug_ed_tests_rug_39_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_40 {
    use super::super::compile::{Compiler, InstHole};
    #[test]
    fn test_push_hole() {
        let _rug_st_tests_rug_40_rrrruuuugggg_test_push_hole = 0;
        let rug_fuzz_0 = 'a';
        let mut v8 = Compiler::new();
        let mut v20 = InstHole::Char { c: rug_fuzz_0 };
        v8.push_hole(v20);
        let _rug_ed_tests_rug_40_rrrruuuugggg_test_push_hole = 0;
    }
}
#[cfg(test)]
mod tests_rug_41 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_41_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        crate::compile::Compiler::push_split_hole(&mut p0);
        let _rug_ed_tests_rug_41_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_42 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_42_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        crate::compile::Compiler::pop_split_hole(&mut p0).unwrap();
        let _rug_ed_tests_rug_42_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_43 {
    use super::*;
    use crate::internal::Compiler;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_43_rrrruuuugggg_test_rug = 0;
        let mut p0 = Compiler::new();
        p0.check_size().unwrap();
        let _rug_ed_tests_rug_43_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_45 {
    use super::*;
    use crate::compile::{Inst, InstSplit, MaybeInst};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_45_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 123;
        let mut p0 = MaybeInst::Split;
        let p1: InstPtr = rug_fuzz_0;
        <MaybeInst>::fill(&mut p0, p1);
        let _rug_ed_tests_rug_45_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::compile::{Inst, InstPtr, InstSplit, MaybeInst};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_46_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 24;
        let mut p0 = MaybeInst::Split;
        let p1: InstPtr = rug_fuzz_0;
        let p2: InstPtr = rug_fuzz_1;
        p0.fill_split(p1, p2);
        let _rug_ed_tests_rug_46_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use compile::MaybeInst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_47_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut p0 = MaybeInst::Split;
        let mut p1: usize = rug_fuzz_0;
        MaybeInst::half_fill_split_goto1(&mut p0, p1);
        let _rug_ed_tests_rug_47_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use compile::MaybeInst;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 42;
        let mut v21 = MaybeInst::Split;
        let p0 = &mut v21;
        let p1: usize = rug_fuzz_0;
        p0.half_fill_split_goto2(p1);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::compile;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_49_rrrruuuugggg_test_rug = 0;
        let mut p0 = MaybeInst::Split;
        <compile::MaybeInst>::unwrap(p0);
        let _rug_ed_tests_rug_49_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use super::super::compile::{
        InstHole, InstPtr, Inst, InstSave, InstEmptyLook, InstChar, InstRanges, InstBytes,
    };
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_50_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 123;
        let mut p0 = InstHole::Char { c: rug_fuzz_0 };
        let p1: InstPtr = rug_fuzz_1;
        p0.fill(p1);
        let _rug_ed_tests_rug_50_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_52 {
    use super::*;
    use crate::compile::CompileClass;
    use crate::syntax::utf8::Utf8Sequence;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_52_rrrruuuugggg_test_rug = 0;
        let mut p0: CompileClass<'static, 'static> = unimplemented!();
        let mut p1: Utf8Sequence = unimplemented!();
        p0.c_utf8_seq(&p1);
        let _rug_ed_tests_rug_52_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_54 {
    use super::*;
    use crate::compile::{SuffixCache, SuffixCacheKey};
    #[test]
    fn test_hash() {
        let _rug_st_tests_rug_54_rrrruuuugggg_test_hash = 0;
        let rug_fuzz_0 = 10;
        let mut p0 = SuffixCache::new(rug_fuzz_0);
        let p1 = SuffixCacheKey::default();
        p0.hash(&p1);
        let _rug_ed_tests_rug_54_rrrruuuugggg_test_hash = 0;
    }
}
