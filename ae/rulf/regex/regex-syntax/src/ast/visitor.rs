use std::fmt;

use ast::{self, Ast};

/// A trait for visiting an abstract syntax tree (AST) in depth first order.
///
/// The principle aim of this trait is to enable callers to perform case
/// analysis on an abstract syntax tree without necessarily using recursion.
/// In particular, this permits callers to do case analysis with constant stack
/// usage, which can be important since the size of an abstract syntax tree
/// may be proportional to end user input.
///
/// Typical usage of this trait involves providing an implementation and then
/// running it using the [`visit`](fn.visit.html) function.
///
/// Note that the abstract syntax tree for a regular expression is quite
/// complex. Unless you specifically need it, you might be able to use the
/// much simpler
/// [high-level intermediate representation](../hir/struct.Hir.html)
/// and its
/// [corresponding `Visitor` trait](../hir/trait.Visitor.html)
/// instead.
pub trait Visitor {
    /// The result of visiting an AST.
    type Output;
    /// An error that visiting an AST might return.
    type Err;

    /// All implementors of `Visitor` must provide a `finish` method, which
    /// yields the result of visiting the AST or an error.
    fn finish(self) -> Result<Self::Output, Self::Err>;

    /// This method is called before beginning traversal of the AST.
    fn start(&mut self) {}

    /// This method is called on an `Ast` before descending into child `Ast`
    /// nodes.
    fn visit_pre(&mut self, _ast: &Ast) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called on an `Ast` after descending all of its child
    /// `Ast` nodes.
    fn visit_post(&mut self, _ast: &Ast) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called between child nodes of an
    /// [`Alternation`](struct.Alternation.html).
    fn visit_alternation_in(&mut self) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called on every
    /// [`ClassSetItem`](enum.ClassSetItem.html)
    /// before descending into child nodes.
    fn visit_class_set_item_pre(
        &mut self,
        _ast: &ast::ClassSetItem,
    ) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called on every
    /// [`ClassSetItem`](enum.ClassSetItem.html)
    /// after descending into child nodes.
    fn visit_class_set_item_post(
        &mut self,
        _ast: &ast::ClassSetItem,
    ) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called on every
    /// [`ClassSetBinaryOp`](struct.ClassSetBinaryOp.html)
    /// before descending into child nodes.
    fn visit_class_set_binary_op_pre(
        &mut self,
        _ast: &ast::ClassSetBinaryOp,
    ) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called on every
    /// [`ClassSetBinaryOp`](struct.ClassSetBinaryOp.html)
    /// after descending into child nodes.
    fn visit_class_set_binary_op_post(
        &mut self,
        _ast: &ast::ClassSetBinaryOp,
    ) -> Result<(), Self::Err> {
        Ok(())
    }

    /// This method is called between the left hand and right hand child nodes
    /// of a [`ClassSetBinaryOp`](struct.ClassSetBinaryOp.html).
    fn visit_class_set_binary_op_in(
        &mut self,
        _ast: &ast::ClassSetBinaryOp,
    ) -> Result<(), Self::Err> {
        Ok(())
    }
}

/// Executes an implementation of `Visitor` in constant stack space.
///
/// This function will visit every node in the given `Ast` while calling the
/// appropriate methods provided by the
/// [`Visitor`](trait.Visitor.html) trait.
///
/// The primary use case for this method is when one wants to perform case
/// analysis over an `Ast` without using a stack size proportional to the depth
/// of the `Ast`. Namely, this method will instead use constant stack size, but
/// will use heap space proportional to the size of the `Ast`. This may be
/// desirable in cases where the size of `Ast` is proportional to end user
/// input.
///
/// If the visitor returns an error at any point, then visiting is stopped and
/// the error is returned.
pub fn visit<V: Visitor>(ast: &Ast, visitor: V) -> Result<V::Output, V::Err> {
    HeapVisitor::new().visit(ast, visitor)
}

/// HeapVisitor visits every item in an `Ast` recursively using constant stack
/// size and a heap size proportional to the size of the `Ast`.
struct HeapVisitor<'a> {
    /// A stack of `Ast` nodes. This is roughly analogous to the call stack
    /// used in a typical recursive visitor.
    stack: Vec<(&'a Ast, Frame<'a>)>,
    /// Similar to the `Ast` stack above, but is used only for character
    /// classes. In particular, character classes embed their own mini
    /// recursive syntax.
    stack_class: Vec<(ClassInduct<'a>, ClassFrame<'a>)>,
}

/// Represents a single stack frame while performing structural induction over
/// an `Ast`.
enum Frame<'a> {
    /// A stack frame allocated just before descending into a repetition
    /// operator's child node.
    Repetition(&'a ast::Repetition),
    /// A stack frame allocated just before descending into a group's child
    /// node.
    Group(&'a ast::Group),
    /// The stack frame used while visiting every child node of a concatenation
    /// of expressions.
    Concat {
        /// The child node we are currently visiting.
        head: &'a Ast,
        /// The remaining child nodes to visit (which may be empty).
        tail: &'a [Ast],
    },
    /// The stack frame used while visiting every child node of an alternation
    /// of expressions.
    Alternation {
        /// The child node we are currently visiting.
        head: &'a Ast,
        /// The remaining child nodes to visit (which may be empty).
        tail: &'a [Ast],
    },
}

/// Represents a single stack frame while performing structural induction over
/// a character class.
enum ClassFrame<'a> {
    /// The stack frame used while visiting every child node of a union of
    /// character class items.
    Union {
        /// The child node we are currently visiting.
        head: &'a ast::ClassSetItem,
        /// The remaining child nodes to visit (which may be empty).
        tail: &'a [ast::ClassSetItem],
    },
    /// The stack frame used while a binary class operation.
    Binary { op: &'a ast::ClassSetBinaryOp },
    /// A stack frame allocated just before descending into a binary operator's
    /// left hand child node.
    BinaryLHS {
        op: &'a ast::ClassSetBinaryOp,
        lhs: &'a ast::ClassSet,
        rhs: &'a ast::ClassSet,
    },
    /// A stack frame allocated just before descending into a binary operator's
    /// right hand child node.
    BinaryRHS { op: &'a ast::ClassSetBinaryOp, rhs: &'a ast::ClassSet },
}

/// A representation of the inductive step when performing structural induction
/// over a character class.
///
/// Note that there is no analogous explicit type for the inductive step for
/// `Ast` nodes because the inductive step is just an `Ast`. For character
/// classes, the inductive step can produce one of two possible child nodes:
/// an item or a binary operation. (An item cannot be a binary operation
/// because that would imply binary operations can be unioned in the concrete
/// syntax, which is not possible.)
enum ClassInduct<'a> {
    Item(&'a ast::ClassSetItem),
    BinaryOp(&'a ast::ClassSetBinaryOp),
}

impl<'a> HeapVisitor<'a> {
    fn new() -> HeapVisitor<'a> {
        HeapVisitor { stack: vec![], stack_class: vec![] }
    }

    fn visit<V: Visitor>(
        &mut self,
        mut ast: &'a Ast,
        mut visitor: V,
    ) -> Result<V::Output, V::Err> {
        self.stack.clear();
        self.stack_class.clear();

        visitor.start();
        loop {
            visitor.visit_pre(ast)?;
            if let Some(x) = self.induct(ast, &mut visitor)? {
                let child = x.child();
                self.stack.push((ast, x));
                ast = child;
                continue;
            }
            // No induction means we have a base case, so we can post visit
            // it now.
            visitor.visit_post(ast)?;

            // At this point, we now try to pop our call stack until it is
            // either empty or we hit another inductive case.
            loop {
                let (post_ast, frame) = match self.stack.pop() {
                    None => return visitor.finish(),
                    Some((post_ast, frame)) => (post_ast, frame),
                };
                // If this is a concat/alternate, then we might have additional
                // inductive steps to process.
                if let Some(x) = self.pop(frame) {
                    if let Frame::Alternation { .. } = x {
                        visitor.visit_alternation_in()?;
                    }
                    ast = x.child();
                    self.stack.push((post_ast, x));
                    break;
                }
                // Otherwise, we've finished visiting all the child nodes for
                // this AST, so we can post visit it now.
                visitor.visit_post(post_ast)?;
            }
        }
    }

    /// Build a stack frame for the given AST if one is needed (which occurs if
    /// and only if there are child nodes in the AST). Otherwise, return None.
    ///
    /// If this visits a class, then the underlying visitor implementation may
    /// return an error which will be passed on here.
    fn induct<V: Visitor>(
        &mut self,
        ast: &'a Ast,
        visitor: &mut V,
    ) -> Result<Option<Frame<'a>>, V::Err> {
        Ok(match *ast {
            Ast::Class(ast::Class::Bracketed(ref x)) => {
                self.visit_class(x, visitor)?;
                None
            }
            Ast::Repetition(ref x) => Some(Frame::Repetition(x)),
            Ast::Group(ref x) => Some(Frame::Group(x)),
            Ast::Concat(ref x) if x.asts.is_empty() => None,
            Ast::Concat(ref x) => {
                Some(Frame::Concat { head: &x.asts[0], tail: &x.asts[1..] })
            }
            Ast::Alternation(ref x) if x.asts.is_empty() => None,
            Ast::Alternation(ref x) => Some(Frame::Alternation {
                head: &x.asts[0],
                tail: &x.asts[1..],
            }),
            _ => None,
        })
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

    fn visit_class<V: Visitor>(
        &mut self,
        ast: &'a ast::ClassBracketed,
        visitor: &mut V,
    ) -> Result<(), V::Err> {
        let mut ast = ClassInduct::from_bracketed(ast);
        loop {
            self.visit_class_pre(&ast, visitor)?;
            if let Some(x) = self.induct_class(&ast) {
                let child = x.child();
                self.stack_class.push((ast, x));
                ast = child;
                continue;
            }
            self.visit_class_post(&ast, visitor)?;

            // At this point, we now try to pop our call stack until it is
            // either empty or we hit another inductive case.
            loop {
                let (post_ast, frame) = match self.stack_class.pop() {
                    None => return Ok(()),
                    Some((post_ast, frame)) => (post_ast, frame),
                };
                // If this is a union or a binary op, then we might have
                // additional inductive steps to process.
                if let Some(x) = self.pop_class(frame) {
                    if let ClassFrame::BinaryRHS { ref op, .. } = x {
                        visitor.visit_class_set_binary_op_in(op)?;
                    }
                    ast = x.child();
                    self.stack_class.push((post_ast, x));
                    break;
                }
                // Otherwise, we've finished visiting all the child nodes for
                // this class node, so we can post visit it now.
                self.visit_class_post(&post_ast, visitor)?;
            }
        }
    }

    /// Call the appropriate `Visitor` methods given an inductive step.
    fn visit_class_pre<V: Visitor>(
        &self,
        ast: &ClassInduct<'a>,
        visitor: &mut V,
    ) -> Result<(), V::Err> {
        match *ast {
            ClassInduct::Item(item) => {
                visitor.visit_class_set_item_pre(item)?;
            }
            ClassInduct::BinaryOp(op) => {
                visitor.visit_class_set_binary_op_pre(op)?;
            }
        }
        Ok(())
    }

    /// Call the appropriate `Visitor` methods given an inductive step.
    fn visit_class_post<V: Visitor>(
        &self,
        ast: &ClassInduct<'a>,
        visitor: &mut V,
    ) -> Result<(), V::Err> {
        match *ast {
            ClassInduct::Item(item) => {
                visitor.visit_class_set_item_post(item)?;
            }
            ClassInduct::BinaryOp(op) => {
                visitor.visit_class_set_binary_op_post(op)?;
            }
        }
        Ok(())
    }

    /// Build a stack frame for the given class node if one is needed (which
    /// occurs if and only if there are child nodes). Otherwise, return None.
    fn induct_class(&self, ast: &ClassInduct<'a>) -> Option<ClassFrame<'a>> {
        match *ast {
            ClassInduct::Item(&ast::ClassSetItem::Bracketed(ref x)) => {
                match x.kind {
                    ast::ClassSet::Item(ref item) => {
                        Some(ClassFrame::Union { head: item, tail: &[] })
                    }
                    ast::ClassSet::BinaryOp(ref op) => {
                        Some(ClassFrame::Binary { op: op })
                    }
                }
            }
            ClassInduct::Item(&ast::ClassSetItem::Union(ref x)) => {
                if x.items.is_empty() {
                    None
                } else {
                    Some(ClassFrame::Union {
                        head: &x.items[0],
                        tail: &x.items[1..],
                    })
                }
            }
            ClassInduct::BinaryOp(op) => Some(ClassFrame::BinaryLHS {
                op: op,
                lhs: &op.lhs,
                rhs: &op.rhs,
            }),
            _ => None,
        }
    }

    /// Pops the given frame. If the frame has an additional inductive step,
    /// then return it, otherwise return `None`.
    fn pop_class(&self, induct: ClassFrame<'a>) -> Option<ClassFrame<'a>> {
        match induct {
            ClassFrame::Union { tail, .. } => {
                if tail.is_empty() {
                    None
                } else {
                    Some(ClassFrame::Union {
                        head: &tail[0],
                        tail: &tail[1..],
                    })
                }
            }
            ClassFrame::Binary { .. } => None,
            ClassFrame::BinaryLHS { op, rhs, .. } => {
                Some(ClassFrame::BinaryRHS { op: op, rhs: rhs })
            }
            ClassFrame::BinaryRHS { .. } => None,
        }
    }
}

impl<'a> Frame<'a> {
    /// Perform the next inductive step on this frame and return the next
    /// child AST node to visit.
    fn child(&self) -> &'a Ast {
        match *self {
            Frame::Repetition(rep) => &rep.ast,
            Frame::Group(group) => &group.ast,
            Frame::Concat { head, .. } => head,
            Frame::Alternation { head, .. } => head,
        }
    }
}

impl<'a> ClassFrame<'a> {
    /// Perform the next inductive step on this frame and return the next
    /// child class node to visit.
    fn child(&self) -> ClassInduct<'a> {
        match *self {
            ClassFrame::Union { head, .. } => ClassInduct::Item(head),
            ClassFrame::Binary { op, .. } => ClassInduct::BinaryOp(op),
            ClassFrame::BinaryLHS { ref lhs, .. } => {
                ClassInduct::from_set(lhs)
            }
            ClassFrame::BinaryRHS { ref rhs, .. } => {
                ClassInduct::from_set(rhs)
            }
        }
    }
}

impl<'a> ClassInduct<'a> {
    fn from_bracketed(ast: &'a ast::ClassBracketed) -> ClassInduct<'a> {
        ClassInduct::from_set(&ast.kind)
    }

    fn from_set(ast: &'a ast::ClassSet) -> ClassInduct<'a> {
        match *ast {
            ast::ClassSet::Item(ref item) => ClassInduct::Item(item),
            ast::ClassSet::BinaryOp(ref op) => ClassInduct::BinaryOp(op),
        }
    }
}

impl<'a> fmt::Debug for ClassFrame<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let x = match *self {
            ClassFrame::Union { .. } => "Union",
            ClassFrame::Binary { .. } => "Binary",
            ClassFrame::BinaryLHS { .. } => "BinaryLHS",
            ClassFrame::BinaryRHS { .. } => "BinaryRHS",
        };
        write!(f, "{}", x)
    }
}

impl<'a> fmt::Debug for ClassInduct<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let x = match *self {
            ClassInduct::Item(it) => match *it {
                ast::ClassSetItem::Empty(_) => "Item(Empty)",
                ast::ClassSetItem::Literal(_) => "Item(Literal)",
                ast::ClassSetItem::Range(_) => "Item(Range)",
                ast::ClassSetItem::Ascii(_) => "Item(Ascii)",
                ast::ClassSetItem::Perl(_) => "Item(Perl)",
                ast::ClassSetItem::Unicode(_) => "Item(Unicode)",
                ast::ClassSetItem::Bracketed(_) => "Item(Bracketed)",
                ast::ClassSetItem::Union(_) => "Item(Union)",
            },
            ClassInduct::BinaryOp(it) => match it.kind {
                ast::ClassSetBinaryOpKind::Intersection => {
                    "BinaryOp(Intersection)"
                }
                ast::ClassSetBinaryOpKind::Difference => {
                    "BinaryOp(Difference)"
                }
                ast::ClassSetBinaryOpKind::SymmetricDifference => {
                    "BinaryOp(SymmetricDifference)"
                }
            },
        };
        write!(f, "{}", x)
    }
}
#[cfg(test)]
mod tests_llm_16_266 {
    use crate::ast;
    use crate::ast::visitor::ClassInduct;

    #[test]
    fn test_from_set() {
        let item = ast::ClassSetItem::Literal(ast::Literal {
            span: ast::Span::splat(ast::Position::new(0, 0, 0)),
            kind: ast::LiteralKind::Verbatim,
            c: 'a',
        });

        let ast = ast::ClassSet::Item(item);
        let result = ClassInduct::from_set(&ast);

        match result {
            ClassInduct::Item(_) => assert!(true),
            _ => assert!(false),
        }
    }
}#[cfg(test)]
mod tests_llm_16_267 {
    use super::*;

use crate::*;
    use crate::ast::*;
    
    #[test]
    fn test_child() {
        let rep = Repetition {
            span: Span::splat(Position::new(0, 0, 0)),
            op: RepetitionOp {
                span: Span::splat(Position::new(0, 0, 0)),
                kind: RepetitionKind::ZeroOrOne,
            },
            greedy: true,
            ast: Box::new(Ast::Empty(Span::splat(Position::new(0, 0, 0)))),
        };
        let frame = Frame::Repetition(&rep);
        let child = frame.child();
        assert_eq!(child, &Ast::Empty(Span::splat(Position::new(0, 0, 0))));
    }
}#[cfg(test)]
mod tests_llm_16_269 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_new() {
        let heap_visitor: HeapVisitor<'static> = HeapVisitor::new();
    }
}
#[cfg(test)]
mod tests_rug_429 {
    use super::*;
    use crate::ast::{Ast, Concat};
    use crate::ast::print::Writer;
    use crate::ast::visitor::{Visitor, Span};

    struct Printer;

    impl std::fmt::Write for Printer {
        fn write_str(&mut self, _: &str) -> std::fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_rug() {
        let v125 = Ast::Concat(Concat {
            asts: vec![], // fill in the desired value for the `asts` field
            span: Span::splat(Position::new(0, 0, 0)), // fill in the desired `Span` value
        });

        let mut v130: Writer<std::fmt::String> = Writer {
            printer: &mut Printer,
            wtr: String::new(),
        };


        crate::ast::visitor::visit(&v125, &mut v130).unwrap();
        
    }
}

#[cfg(test)]
mod tests_rug_430 {
    use super::*;
    use crate::ast::visitor::Visitor;
    use crate::ast::visitor::{AstVisitor, Visit};
    
    #[test]
    fn test_rug() {
        let mut p0: AstVisitor<NestLimiter> = AstVisitor {
            nest_limiter: NestLimiter::new(&p),
        };

        Visitor::start(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_431 {
    use super::*;
    use crate::ast::print::Writer;
    use crate::ast::visitor::Visitor;
    use crate::ast::{Ast, Concat};
    use crate::parser::{Partial, Parser};
    use crate::Span;

    #[test]
    fn test_visit_pre() {
        struct Printer;

        impl std::fmt::Write for Printer {
            fn write_str(&mut self, _: &str) -> std::fmt::Result {
                Ok(())
            }
        }

        let mut p0: Writer<String> = Writer {
            printer: &mut Printer,
            wtr: String::new(),
        };

        let p1: Ast = Ast::Concat(Concat {
            asts: vec![Ast::Literal(Partial { ast: Partial { node: 'a', span: Span }, repeat: None })],
            span: Span,
        });

        let mut visitor = crate::ast::visitor;
        visitor.visit_pre(&mut p0, &p1).unwrap();
    }
}
               
#[cfg(test)]
mod tests_rug_432 {
    use super::*;
    use crate::ast::visitor::Visitor;
    use crate::ast::{Ast, Span};

    #[test]
    fn test_visit_post() {
        let mut p0 = RegexVisitor {}; // Customize the value for this variable
        let p1 = Ast::Concat(Concat {
            asts: vec![], // Customize the value for this field
            span: Span::splat(Position::new(0, 0, 0)), // Customize the value for this field
        });

        Visitor::visit_post::<RegexVisitor>(&mut p0, &p1).unwrap();
    }
}#[cfg(test)]
mod tests_rug_433 {
    use super::*;
    use crate::ast::visitor::Visitor;
    
    #[test]
    fn test_visit_alternation_in() {
        let mut p0 = ();

        let result = Visitor::visit_alternation_in(&mut p0);
        assert_eq!(result, Ok(()));
    }
}
#[cfg(test)]
mod tests_rug_434 {
    use super::*;
    use crate::ast::visitor::Visitor;
    use crate::ast::{ClassSetItem, ClassSetRange, Literal, Span};
    use crate::ast::print::Writer;
    use std::fmt;

    struct Printer;

    impl fmt::Write for Printer {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_visit_class_set_item_pre() {
        let mut v130: Writer<fmt::String> = Writer {
            printer: &mut Printer,
            wtr: String::new(),
        };

        let mut v126 = ClassSetItem::Range(ClassSetRange {
            start: Literal::Unicode('\u{0041}'),
            end: Literal::Unicode('\u{005A}'),
            span: Span::default(),
        });

        let result = v130
            .visit_class_set_item_pre(&v126)
            .expect("visit_class_set_item_pre failed");

        assert_eq!(result, ());
    }
}
#[cfg(test)]
mod tests_rug_435 {
    use super::*;
    use crate::ast;

    #[test]
    fn test_regex_syntax() {
        let mut p0 = ast::parse::NestLimiter::new(&ast::parse::ParserI {
            parser: std::sync::Arc::new(T {}),
        });

        let mut p1 = ast::ClassSetItem::Range(ast::ClassSetRange {
            start: ast::Literal::Unicode('\u{0041}'),
            end: ast::Literal::Unicode('\u{005A}'),
            span: ast::Span::default(),
        });

        crate::regex_syntax::
            ast::
            visitor::
            Visitor::
            visit_class_set_item_post(&mut p0, &p1)
            .unwrap();
    }
}#[cfg(test)]
mod tests_rug_436 {
    use super::*;
    use crate::ast::visitor::Visitor;

    #[test]
    fn test_rug() {
        use crate::ast::print::Writer;
        use crate::ast::{ClassSetBinaryOp, ClassSetBinaryOpKind, ClassSet, Span};

        struct Printer;

        impl std::fmt::Write for Printer {
            fn write_str(&mut self, _: &str) -> std::fmt::Result {
                Ok(())
            }
        }

        let mut v130: Writer<std::fmt::String> = Writer {
            printer: &mut Printer,
            wtr: String::new(),
        };

        let mut v127 = ClassSetBinaryOp {
            span: Span { start: 0, end: 10 },  // Replace with the actual span value
            kind: ClassSetBinaryOpKind::Union,  // Replace with desired kind value
            lhs: Box::new(ClassSet {}),  // Replace with desired lhs value
            rhs: Box::new(ClassSet {}),  // Replace with desired rhs value
        };

        crate::ast::visitor::Visitor::<std::fmt::String>::visit_class_set_binary_op_pre(&mut v130, &v127);
    }
}#[cfg(test)]
mod tests_rug_437 {
    use super::*;
    use std::sync::Arc;
    use crate::ast::parse::NestLimiter;
    use crate::ast::{Ast, Span};
    use crate::ast::{ClassSetItem, ClassSetBinaryOp, Class};

    #[test]
    fn test_regex_syntax_visit_class_set_binary_op_post() {
        let parser: Arc<T> = Arc::new(T {});
        let p = ParserI { parser: parser.borrow() };
        let p0 = NestLimiter::new(&p);
        
        let p1 = ClassSetBinaryOp {
            span: Span::default(),
            kind: ClassSetBinaryOpKind::Union,
            lhs: Box::new(ClassSet {}),
            rhs: Box::new(ClassSet {}),
        };

        crate::ast::visitor::Visitor::visit_class_set_binary_op_post(&mut p0, &p1).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_438 {
    use super::*;
    use crate::ast::print::Writer;
    use crate::ast::visitor::Visitor;
    

    impl std::fmt::Write for Printer {
        fn write_str(&mut self, _: &str) -> std::fmt::Result {
            Ok(())
        }
    }

    #[test]
    fn test_visit_class_set_binary_op_in() {
        struct Printer;

        let mut p0: Writer<std::fmt::String> = Writer {
            printer: &mut Printer,
            wtr: String::new(),
        };

        let mut p1 = regex_syntax::ast::ClassSetBinaryOp {
            span: Span {
                start: ..,
                end: ..,
            },
            kind: regex_syntax::ast::ClassSetBinaryOpKind::Union,
            lhs: Box::new(regex_syntax::ast::ClassSet {}),
            rhs: Box::new(regex_syntax :: ast :: ClassSet {}),
                };
                
        
        let result = crate :: ast :: visitor ::Visitor :: visit_class_set_binary_op_in(&mut p0, &p1);
        
        assert_eq!(result, Ok(()));
    }
}

#[cfg(test)]
mod tests_rug_439 {
    use super::*;
    use crate::Ast;
    use crate::ast::{Position, Span};
    use crate::ast::visitor::HeapVisitor;
    use crate::hir::translate::TranslatorI;
    
    #[test]
    fn test_rug() {
        let asts = vec![]; // fill in the desired value for the `asts` field
        let span = Span::splat(Position::new(0, 0, 0)); // fill in the desired `Span` value
        let ast = Ast::Concat(Concat { asts, span });
        let mut translator = TranslatorI::new(&Translator, "pattern");
        let mut visitor = HeapVisitor::new();
        
        visitor.visit(&ast, &mut translator).unwrap();
    }
}#[cfg(test)]
mod tests_rug_440 {
    use super::*;
    use crate::{
        ast::{Ast, Class, Concat, Group, Repetition},
        Span, Position,
        ast::visitor::{Visitor, HeapVisitor},
    };

    #[test]
    fn test_rug() {
        let mut p0: HeapVisitor<'static> = HeapVisitor::new();

        let p1 = Ast::Concat(Concat {
            asts: vec![Ast::Repetition(Repetition { ast: Ast::Group(Group {}) })],
            span: Span::splat(Position::new(0, 0, 0)),
        });

        let mut p2 = hir::translate::TranslatorI::new(&Translator, "pattern");

        <ast::visitor::HeapVisitor<'_>>::induct(&mut p0, &p1, &mut p2);
    }
}#[cfg(test)]
mod tests_rug_441 {
    use super::*;
    use crate::ast::visitor::{HeapVisitor, Frame};

    #[test]
    fn test_rug() {
        let heap_visitor: HeapVisitor<'static> = HeapVisitor::new();
        let frame: Frame<'static> = Frame::new();

        heap_visitor.pop(frame);
    }
}
#[cfg(test)]
mod tests_rug_442 {
    use super::*;

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_442_prepare {
            use crate::ast::visitor::HeapVisitor;
            use crate::ast::{ClassBracketed, ClassSet, Span};
            use crate::ast::print::Writer;
            use crate::ast::visitor::Visitor;
            use std::fmt;

            struct Printer;

            impl fmt::Write for Printer {
                fn write_str(&mut self, _: &str) -> fmt::Result {
                    Ok(())
                }
            }

            #[test]
            fn sample() {
                let mut v138: HeapVisitor<'static> = HeapVisitor::new();

                let mut v140 = ClassBracketed {
                    span: Span::default(), // use default span
                    negated: false, // set negated to false
                    kind: ClassSet::None, // set kind to None
                };

                let mut v130: Writer<fmt::String> = Writer {
                    printer: &mut Printer,
                    wtr: String::new(),
                };
                
                <HeapVisitor<'_>>::visit_class(&mut v138, &mut v140, &mut v130);
            }
        }
    }
}#[cfg(test)]
mod tests_rug_443 {
    use super::*;
    use crate::ast::visitor::{HeapVisitor, ClassInduct, Visitor};
    use crate::hir::translate::TranslatorI;
    
    #[test]
    fn test_visit_class_pre() {
        let mut visitor = HeapVisitor::new();
        let ast = ClassInduct::new();
        let translator = TranslatorI::new(&Translator, "pattern");
        
        assert_eq!(
            HeapVisitor::<'_>::visit_class_pre(&visitor, &ast, &mut translator),
            Ok(())
        );
    }
}
#[cfg(test)]
mod tests_rug_444 {
    use super::*;
    use crate::ast::visitor::HeapVisitor;
    use crate::ast::visitor::{Visitor, ClassInduct};
    use crate::ast::parse::{ParserI, NestLimiter};
    use std::borrow::Borrow;
    use std::sync::Arc;

    #[test]
    fn test_visit_class_post() {
        // Construct the first argument
        let mut p0: HeapVisitor<'static> = HeapVisitor::new();

        // Construct the second argument
        let mut p1: ClassInduct<'static> = ClassInduct::Item(ClassSetItem {});
        
        // Construct the third argument
        let parser: Arc<T> = Arc::new(T {});
        let p = ParserI { parser: parser.borrow() };
        let p2 = NestLimiter::new(&p);

        p0.visit_class_post(&p1, &mut p2).unwrap();
    }
}
#[cfg(test)]
mod tests_rug_445 {
    use super::*;
    use crate::ast::{ClassInduct, ClassSetItem, ClassSet, ClassFrame};

    #[test]
    fn test_rug() {
        let mut v138: HeapVisitor<'static> = HeapVisitor::new();
        let mut v141 = ClassInduct::new();

        induct_class(&v138, &v141);
    }
}#[cfg(test)]
mod tests_rug_446 {
    use super::*;
    use crate::ast::visitor::{HeapVisitor, ClassFrame};

    #[test]
    fn test_pop_class() {
        let mut visitor: HeapVisitor<'static> = HeapVisitor::new();
        let frame: ClassFrame<'static> = ClassFrame::new(/* provide any necessary parameters */);
        let result = visitor.pop_class(frame);
        // Perform assertions on the result
        // assert_eq!(result, Some(/* expected value */));
    }
}
#[cfg(test)]
mod tests_rug_447 {
    use super::*;
    // use statements

    #[test]
    fn test_rug() {
        // test data
        let mut v142: ClassFrame<'static> = ClassFrame::new(/* provide any necessary parameters */);
        // fill in any necessary data for the ClassFrame instance

        let result = v142.child();
        
        // assertions
        match result {
            ClassInduct::Item(_) => (),
            ClassInduct::BinaryOp(_) => (),
            ClassInduct::from_set(_) => (),
        }
    }
}
#[cfg(test)]
mod tests_rug_448 {
    use super::*;
    use crate::ast::{ClassBracketed, ClassSet, Span};

    #[test]
    fn test_rug() {
        let mut p0 = ClassBracketed {
            span: Span::default(),
            negated: false,
            kind: ClassSet::None,
        };

        <ast::visitor::ClassInduct<'_>>::from_bracketed(&p0);
    }
}