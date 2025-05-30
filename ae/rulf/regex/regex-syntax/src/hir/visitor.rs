use hir::{self, Hir, HirKind};

/// A trait for visiting the high-level IR (HIR) in depth first order.
///
/// The principle aim of this trait is to enable callers to perform case
/// analysis on a high-level intermediate representation of a regular
/// expression without necessarily using recursion. In particular, this permits
/// callers to do case analysis with constant stack usage, which can be
/// important since the size of an HIR may be proportional to end user input.
///
/// Typical usage of this trait involves providing an implementation and then
/// running it using the [`visit`](fn.visit.html) function.
pub trait Visitor {
    /// The result of visiting an HIR.
    type Output;
    /// An error that visiting an HIR might return.
    type Err;

    /// All implementors of `Visitor` must provide a `finish` method, which
    /// yields the result of visiting the HIR or an error.
    fn finish(self) -> Result<Self::Output, Self::Err>;

    /// This method is called before beginning traversal of the HIR.
    fn start(&mut self) {}

    /// This method is called on an `Hir` before descending into child `Hir`
    /// nodes.
    fn visit_pre(&mut self, _hir: &Hir) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called on an `Hir` after descending all of its child
    /// `Hir` nodes.
    fn visit_post(&mut self, _hir: &Hir) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called between child nodes of an alternation.
    fn visit_alternation_in(&mut self) -> Result<(), Self::Err> {
        Ok(())
    }
}

/// Executes an implementation of `Visitor` in constant stack space.
///
/// This function will visit every node in the given `Hir` while calling
/// appropriate methods provided by the
/// [`Visitor`](trait.Visitor.html) trait.
///
/// The primary use case for this method is when one wants to perform case
/// analysis over an `Hir` without using a stack size proportional to the depth
/// of the `Hir`. Namely, this method will instead use constant stack space,
/// but will use heap space proportional to the size of the `Hir`. This may be
/// desirable in cases where the size of `Hir` is proportional to end user
/// input.
///
/// If the visitor returns an error at any point, then visiting is stopped and
/// the error is returned.
pub fn visit<V: Visitor>(hir: &Hir, visitor: V) -> Result<V::Output, V::Err> {
    HeapVisitor::new().visit(hir, visitor)
}

/// HeapVisitor visits every item in an `Hir` recursively using constant stack
/// size and a heap size proportional to the size of the `Hir`.
struct HeapVisitor<'a> {
    /// A stack of `Hir` nodes. This is roughly analogous to the call stack
    /// used in a typical recursive visitor.
    stack: Vec<(&'a Hir, Frame<'a>)>,
}

/// Represents a single stack frame while performing structural induction over
/// an `Hir`.
enum Frame<'a> {
    /// A stack frame allocated just before descending into a repetition
    /// operator's child node.
    Repetition(&'a hir::Repetition),
    /// A stack frame allocated just before descending into a group's child
    /// node.
    Group(&'a hir::Group),
    /// The stack frame used while visiting every child node of a concatenation
    /// of expressions.
    Concat {
        /// The child node we are currently visiting.
        head: &'a Hir,
        /// The remaining child nodes to visit (which may be empty).
        tail: &'a [Hir],
    },
    /// The stack frame used while visiting every child node of an alternation
    /// of expressions.
    Alternation {
        /// The child node we are currently visiting.
        head: &'a Hir,
        /// The remaining child nodes to visit (which may be empty).
        tail: &'a [Hir],
    },
}

impl<'a> HeapVisitor<'a> {
    fn new() -> HeapVisitor<'a> {
        HeapVisitor { stack: vec![] }
    }

    fn visit<V: Visitor>(
        &mut self,
        mut hir: &'a Hir,
        mut visitor: V,
    ) -> Result<V::Output, V::Err> {
        self.stack.clear();

        visitor.start();
        loop {
            visitor.visit_pre(hir)?;
            if let Some(x) = self.induct(hir) {
                let child = x.child();
                self.stack.push((hir, x));
                hir = child;
                continue;
            }
            // No induction means we have a base case, so we can post visit
            // it now.
            visitor.visit_post(hir)?;

            // At this point, we now try to pop our call stack until it is
            // either empty or we hit another inductive case.
            loop {
                let (post_hir, frame) = match self.stack.pop() {
                    None => return visitor.finish(),
                    Some((post_hir, frame)) => (post_hir, frame),
                };
                // If this is a concat/alternate, then we might have additional
                // inductive steps to process.
                if let Some(x) = self.pop(frame) {
                    if let Frame::Alternation { .. } = x {
                        visitor.visit_alternation_in()?;
                    }
                    hir = x.child();
                    self.stack.push((post_hir, x));
                    break;
                }
                // Otherwise, we've finished visiting all the child nodes for
                // this HIR, so we can post visit it now.
                visitor.visit_post(post_hir)?;
            }
        }
    }

    /// Build a stack frame for the given HIR if one is needed (which occurs if
    /// and only if there are child nodes in the HIR). Otherwise, return None.
    fn induct(&mut self, hir: &'a Hir) -> Option<Frame<'a>> {
        match *hir.kind() {
            HirKind::Repetition(ref x) => Some(Frame::Repetition(x)),
            HirKind::Group(ref x) => Some(Frame::Group(x)),
            HirKind::Concat(ref x) if x.is_empty() => None,
            HirKind::Concat(ref x) => {
                Some(Frame::Concat { head: &x[0], tail: &x[1..] })
            }
            HirKind::Alternation(ref x) if x.is_empty() => None,
            HirKind::Alternation(ref x) => {
                Some(Frame::Alternation { head: &x[0], tail: &x[1..] })
            }
            _ => None,
        }
    }

    /// Pops the given frame. If the frame has an additional inductive step,
    /// then return it, otherwise return `None`.
    fn pop(&self, induct: Frame<'a>) -> Option<Frame<'a>> {
        match induct {
            Frame::Repetition(_) => None,
            Frame::Group(_) => None,
            Frame::Concat { tail, .. } => {
                if tail.is_empty() {
                    None
                } else {
                    Some(Frame::Concat { head: &tail[0], tail: &tail[1..] })
                }
            }
            Frame::Alternation { tail, .. } => {
                if tail.is_empty() {
                    None
                } else {
                    Some(Frame::Alternation {
                        head: &tail[0],
                        tail: &tail[1..],
                    })
                }
            }
        }
    }
}

impl<'a> Frame<'a> {
    /// Perform the next inductive step on this frame and return the next
    /// child HIR node to visit.
    fn child(&self) -> &'a Hir {
        match *self {
            Frame::Repetition(rep) => &rep.hir,
            Frame::Group(group) => &group.hir,
            Frame::Concat { head, .. } => head,
            Frame::Alternation { head, .. } => head,
        }
    }
}
#[cfg(test)]
mod tests_rug_526 {
    use super::*;
    use crate::hir::{self, Hir};
    use crate::print::{self, Writer};
    use std::fmt;

    struct DummyPrinter;

    impl print::Printer for DummyPrinter {
        fn write(&mut self, _s: &str) -> fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_526_prepare {
            #[test]
            fn sample_hir() {
                let mut v148 = hir::literal(hir::parse("sample").unwrap());
            }

            #[test]
            fn sample_writer() {
                struct DummyPrinter;

                impl print::Printer for DummyPrinter {
                    fn write(&mut self, _s: &str) -> fmt::Result {
                        Ok(())
                    }
                }

                let printer = DummyPrinter;
                let wtr = String::new();

                let mut v174: Writer<_, String> = Writer { printer: &mut printer, wtr };
            }
        }

        tests_prepare::sample_hir();
        tests_prepare::sample_writer();

        let mut p0 = hir;
        let mut p1: Writer<_, String> = Writer { printer: &mut DummyPrinter, wtr: String::new() };

        crate::hir::visitor::visit(&mut p0, &mut p1);
    }
}
#[cfg(test)]
mod tests_rug_527 {
    use super::*;
    use crate::print::Writer;
    use crate::print::Printer;
    use std::fmt;
        
    struct DummyPrinter;
        
    impl Printer for DummyPrinter {
        fn write(&mut self, _s: &str) -> fmt::Result {
            Ok(())
        }
    }
        
    #[test]
    fn test_rug() {
        let printer = DummyPrinter;
        let wtr = String::new();
        
        let mut p0: Writer<_, String> = Writer { printer: &mut printer, wtr };
                    
        crate::hir::visitor::Visitor::start(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_528 {
    use super::*;
    use crate::hir::Hir;
    use crate::print::Writer;
    use crate::print::Printer;
    use std::fmt;

    struct DummyPrinter;

    impl Printer for DummyPrinter {
        fn write(&mut self, _s: &str) -> fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_visit_pre() {
        let mut printer = DummyPrinter;
        let wtr = String::new();

        let mut visitor = Writer { printer: &mut printer, wtr };

        let hir = // construct your Hir here

        visitor.visit_pre(&hir).unwrap();
    }
}#[cfg(test)]
mod tests_rug_529 {
    use super::*;
    use crate::print::Writer;
    use crate::print::Printer;
    use std::fmt;
    use crate::hir::{Hir, literal, parse};
    
    struct DummyPrinter;
        
    impl Printer for DummyPrinter {
        fn write(&mut self, _s: &str) -> fmt::Result {
            Ok(())
        }
    }
    
    #[test]
    fn test_visit_post() {
        let mut printer = DummyPrinter;
        let wtr = String::new();
        
        let mut p0: Writer<_, String> = Writer { printer: &mut printer, wtr };
        
        let mut p1 = literal(parse("sample").unwrap());
        
        crate::hir::visitor::Visitor::visit_post(&mut p0, &mut p1).unwrap();
    }
}#[cfg(test)]
mod tests_rug_530 {
    use super::*;
    use crate::print::Writer;
    use crate::print::Printer;
    use std::fmt;

    #[test]
    fn test_rug() {
        struct DummyPrinter;

        impl Printer for DummyPrinter {
            fn write(&mut self, _s: &str) -> fmt::Result {
                Ok(())
            }
        }

        let printer = DummyPrinter;
        let wtr = String::new();

        let mut p0: Writer<_, String> = Writer { printer: &mut printer, wtr };

        crate::hir::visitor::Visitor::visit_alternation_in(&mut p0).unwrap();
    }
}#[cfg(test)]
mod tests_rug_531 {
    use super::*;
    use crate::regex_syntax::hir::visitor::HeapVisitor;
    
    #[test]
    fn test_rug() {
        HeapVisitor::<'static>::new();
    }
}
#[cfg(test)]
mod tests_rug_532 {
    use super::*;
    use crate::hir::visitor::HeapVisitor;
    use crate::hir::{Hir,Visitor};
    use crate::print::{Writer, Printer};
    use std::fmt;
    use crate::hir::literal::{literal, parse};

    struct DummyPrinter;

    impl Printer for DummyPrinter {
        fn write(&mut self, _s: &str) -> fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_rug() {
        let mut p0: HeapVisitor<'static> = HeapVisitor::default();

        let p1 = literal(parse("sample").unwrap());

        let printer = DummyPrinter;
        let wtr = String::new();
        let mut p2: Writer<_, String> = Writer { printer: &mut printer, wtr };

        p0.visit(&p1, &mut p2).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_533 {
    use super::*;
    use crate::hir::visitor::HeapVisitor;
    use crate::hir::{Hir, HirKind, Repetition, Group, Concat, Alternation};

    #[test]
    fn test_rug() {
        let mut p0: HeapVisitor<'static> = HeapVisitor::default();
        let mut p1: Hir = match *hir.kind() {
            HirKind::Repetition(ref x) => Repetition(x.clone()),
            HirKind::Group(ref x) => Group(x.clone()),
            HirKind::Concat(ref x) if x.is_empty() => Concat(Vec::new()),
            HirKind::Concat(ref x) => Concat(vec![x[0].clone(), x[1..].to_vec()]),
            HirKind::Alternation(ref x) if x.is_empty() => Alternation(Vec::new()),
            HirKind::Alternation(ref x) => Alternation(vec![x[0].clone(), x[1..].to_vec()]),
            _ => panic!(),
        };

        p0.induct(&p1);
    }
}#[cfg(test)]
mod tests_rug_534 {
    use super::*;
    use crate::hir::visitor::{HeapVisitor, Frame};

    #[test]
    fn test_rug() {
        let mut p0: HeapVisitor<'static> = HeapVisitor::default();
        
        let mut p1: Frame<'static> = Frame {
            op: 0,
            ast: None,
            then: None,
            else_: None,
            span_start: 0,
            span_end: 0,
            phantom: std::marker::PhantomData,
        };
                
        <hir::visitor::HeapVisitor<'static>>::pop(&p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_535 {
    use super::*;
    use crate::{hir, Hir};
    use crate::hir::visitor::Frame;
    use crate::hir::visitor::Frame::{Repetition, Group, Concat, Alternation};
    
    #[test]
    fn test_child() {
        let mut v176: Frame<'static> = Frame {
            op: 0,
            ast: None,
            then: None,
            else_: None,
            span_start: 0,
            span_end: 0,
            phantom: std::marker::PhantomData,
        };
        
        let result = v176.child();

        // assertion here
    }
}