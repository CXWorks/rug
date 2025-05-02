/// Define a trait as usual, and a macro that can be used to instantiate
/// implementations of it.
///
/// There *must* be section markers in the trait definition:
/// @section type for associated types
/// @section self for methods
/// @section nodelegate for arbitrary tail that is not forwarded.
macro_rules! trait_template {
    ($(#[$doc:meta])* pub trait $name:ident $($methods:tt)*) => {
        macro_rules! $name { ($m : ident $extra : tt) => { $m ! { $extra pub trait $name
        $($methods)* } } } remove_sections! { [] $(#[$doc])* pub trait $name $($methods)*
        }
    };
}
macro_rules! remove_sections_inner {
    ([$($stack:tt)*]) => {
        $($stack)*
    };
    ([$($stack:tt)*] @ escape $_x:tt $($t:tt)*) => {
        remove_sections_inner!([$($stack)*] $($t)*);
    };
    ([$($stack:tt)*] @ section $x:ident $($t:tt)*) => {
        remove_sections_inner!([$($stack)*] $($t)*);
    };
    ([$($stack:tt)*] $t:tt $($tail:tt)*) => {
        remove_sections_inner!([$($stack)* $t] $($tail)*);
    };
}
macro_rules! remove_sections {
    ([$($stack:tt)*]) => {
        $($stack)*
    };
    ([$($stack:tt)*] { $($tail:tt)* }) => {
        $($stack)* { remove_sections_inner!([] $($tail)*); }
    };
    ([$($stack:tt)*] $t:tt $($tail:tt)*) => {
        remove_sections!([$($stack)* $t] $($tail)*);
    };
}
macro_rules! deref {
    ($e:expr) => {
        *$e
    };
}
macro_rules! deref_twice {
    ($e:expr) => {
        **$e
    };
}
/// Implement a trait by delegation. By default as if we are delegating
/// from &G to G.
macro_rules! delegate_impl {
    ([] $($rest:tt)*) => {
        delegate_impl! { [['a, G], G, &'a G, deref] $($rest)* }
    };
    (
        [[$($param:tt)*], $self_type:ident, $self_wrap:ty, $self_map:ident] pub trait
        $name:ident $(: $sup:ident)* $(+ $more_sup:ident)* { $(@ escape[type
        $assoc_name_ext:ident])* $(@ section type $($(#[$_assoc_attr:meta])* type
        $assoc_name:ident $(: $assoc_bound:ty)*;)+)* $(@ section self
        $($(#[$_method_attr:meta])* fn $method_name:ident (self $(: $self_selftype:ty)*
        $(,$marg:ident : $marg_ty:ty)*) $(-> $mret:ty)?;)+)* $(@ section nodelegate
        $($tail:tt)*)* }
    ) => {
        impl <$($param)*> $name for $self_wrap where $self_type : $name { $($(type
        $assoc_name = $self_type ::$assoc_name;)*)* $(type $assoc_name_ext = $self_type
        ::$assoc_name_ext;)* $($(fn $method_name (self $(: $self_selftype)* $(,$marg :
        $marg_ty)*) $(-> $mret)? { $self_map ! (self).$method_name ($($marg),*) })*)* }
    };
}
#[cfg(test)]
mod tests_rug_634 {
    use super::*;
    use crate::visit::NodeIndexable;
    use crate::graph::IndexType;
    use crate::prelude::{StableGraph, Dfs, GraphMap};
    use crate::csr::EdgesNotSorted;
    use std::net::Ipv6Addr;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(usize, usize, usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: StableGraph<Ipv6Addr, EdgesNotSorted> = StableGraph::<
            Ipv6Addr,
            EdgesNotSorted,
        >::with_capacity(rug_fuzz_0, rug_fuzz_1);
        let p1: usize = rug_fuzz_2;
        p0.from_index(p1);
             }
});    }
}
