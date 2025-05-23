#![feature(box_into_inner)]

use std::any;
use std::cmp::min;
use std::io::{Read, Write};
use std::str::FromStr;
use syn::{parse::Parser, visit::Visit, Expr, ExprLit, ItemFn, Lit, LitBool, LitChar, LitFloat, LitInt, LitStr, Stmt, visit, Ident, fold::{self, Fold}, ExprMacro, MacroDelimiter, Item, ExprCall, Token, ItemMod, ExprRepeat, Type};
use quote::{quote, ToTokens};
use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;

fn lit_to_string(lit: &Lit) -> String {
    match lit {
        Lit::Str(lit_str) => lit_str.value(), // Gets the actual string without quotes
        Lit::ByteStr(lit_bytestr) => {
            // Format as a byte string literal, including the prefix `b`
            format!("b{:?}", &lit_bytestr.value())
        }
        Lit::Byte(lit_byte) => {
            // Format as a byte literal, including the prefix `b`
            format!("b'{}'", lit_byte.value() as char)
        }
        Lit::Char(lit_char) => {
            // Format as a char literal
            format!("'{}'", lit_char.value())
        }
        Lit::Int(lit_int) => {
            // Gets the string representation of the integer
            lit_int.to_string()
        }
        Lit::Float(lit_float) => {
            // Gets the string representation of the float
            lit_float.to_string()
        }
        Lit::Bool(lit_bool) => {
            // Gets the string representation of the boolean
            lit_bool.value().to_string()
        }
        Lit::Verbatim(lit_verbatim) => {
            // Gets the direct string representation of the literal
            lit_verbatim.to_string()
        }
        &_ => todo!()
    }
}


// impl<'ast> Visit<'ast> for PrimitiveExtractor {
//     fn visit_lit(&mut self, i: &'ast Lit) {
//         let var_name = format!("extracted_value_{}", self.primitives.len());
//         let (lit, ty) = match &i {
//             Lit::Str(_) => (i.clone(), quote!(&str)),
//             Lit::ByteStr(_) => (i.clone(), quote!(&[u8])),
//             Lit::Byte(_) => (i.clone(), quote!(u8)),
//             Lit::Char(_) => (i.clone(), quote!(char)),
//             Lit::Int(lit_int) => {
//                 let ty = quote!(u8);
//                 (i.clone(), ty)
//             }
//             Lit::Float(lit_float) => {
//                 let ty = quote!(f32);
//                 (i.clone(), ty)
//             }
//             Lit::Bool(_) => (i.clone(), quote!(bool)),
//             Lit::Verbatim(_) => return,  // Verbatim literals are not handled here
//         };
//
//         self.primitives.push((lit, var_name.clone(), ty));
//         self.replacements.push((i.clone(), Ident::new(&var_name.clone(), proc_macro2::Span::call_site())));
//
//         visit::visit_lit(self, i);
//     }
//
//     fn visit_expr(&mut self, expr: &'ast Expr) {
//         match expr {
//             Expr::Macro(expr_macro) => {
//                 // Apply some heuristic transformations to the macro
//                 // For demonstration, let's just clone the macro expression as is.
//                 println!("{}", expr_macro.mac.path.segments.last().unwrap().ident.to_string());
//             }
//             _ =>{}, // Delegate to the default fold implementation
//         }
//         visit::visit_expr(self, expr);
//     }
// }

fn are_lits_equal(lit1: &Lit, lit2: &Lit) -> bool {
    match (lit1, lit2) {
        (Lit::Str(lit_str1), Lit::Str(lit_str2)) => lit_str1.value() == lit_str2.value(),
        (Lit::ByteStr(lit_byte_str1), Lit::ByteStr(lit_byte_str2)) => {
            lit_byte_str1.value() == lit_byte_str2.value()
        }
        (Lit::Byte(lit_byte1), Lit::Byte(lit_byte2)) => lit_byte1.value() == lit_byte2.value(),
        (Lit::Char(lit_char1), Lit::Char(lit_char2)) => lit_char1.value() == lit_char2.value(),
        (Lit::Int(lit_int1), Lit::Int(lit_int2)) => {
            lit_int1.base10_digits() == lit_int2.base10_digits() && lit_int1.suffix() == lit_int2.suffix()
        }
        (Lit::Float(lit_float1), Lit::Float(lit_float2)) => {
            lit_float1.base10_digits() == lit_float2.base10_digits() && lit_float1.suffix() == lit_float2.suffix()
        }
        (Lit::Bool(lit_bool1), Lit::Bool(lit_bool2)) => lit_bool1.value == lit_bool2.value,
        (Lit::Verbatim(lit_verbatim1), Lit::Verbatim(lit_verbatim2)) => {
            lit_verbatim1.to_string() == lit_verbatim2.to_string()
        }
        // If they are different variants, they are not equal
        _ => false,
    }
}


// A folder to actually replace the literals with the newly created variables.
struct LiteralReplacer {
    primitives: Vec<(Lit, String, TokenStream)>,
    // This holds the mapping of literals to variable identifiers.
    replacements: Vec<(Lit, Ident)>,
    in_mod: bool,
    in_fn: bool,
    target_mod: Option<String>,
    cur_fn: String
}

impl LiteralReplacer {
    fn new() -> Self {
        LiteralReplacer { primitives: vec![],     replacements: Vec::<(Lit, Ident)>::new(),  in_mod:false, in_fn: false, target_mod: None, cur_fn: String::new()}
    }

    fn new_with_target(t: String) -> Self {
        LiteralReplacer { primitives: vec![],     replacements: Vec::<(Lit, Ident)>::new(),  in_mod:false, in_fn: false, target_mod: Some(t), cur_fn: String::new()}
    }


    fn clean(&mut self){
        self.replacements.clear();
        self.primitives.clear();
    }
    fn generate_declarations(&self) -> (Vec<Stmt>, Stmt, Stmt) {

        let ans1 = self.primitives.iter().map(|(lit, var_name, ty)| {
            let var_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
            let declaration = quote! {
                let #var_ident = #lit;
            };
            print!("{}~{}~{}~{}_rrrruuuugggg_", self.target_mod.clone().unwrap(), self.cur_fn, var_name, lit.to_token_stream().to_string());
            syn::parse2(declaration).unwrap()
        }).collect();
        let start_comment = syn::parse_str::<Stmt>(&*format!("let _rug_st_{}_rrrruuuugggg_{}=0;", self.target_mod.clone().unwrap(), self.cur_fn)).unwrap();
        let end_comment = syn::parse_str::<Stmt>(&*format!("let _rug_ed_{}_rrrruuuugggg_{}=0;", self.target_mod.clone().unwrap(), self.cur_fn)).unwrap();
        return (ans1, start_comment, end_comment);
    }
}

impl Fold for LiteralReplacer {


    fn fold_expr(&mut self, expr: Expr) -> Expr {
        // println!("exprrrrrrr {} {} {}", self.in_mod, self.in_fn, expr.to_token_stream().to_string());
        if !self.in_mod || !self.in_fn {
            return fold::fold_expr(self, expr);
        }
        // println!("in {}", expr.to_token_stream().to_string());
        match expr {
            Expr::Lit(expr_lit) => {
                let var_name = format!("rug_fuzz_{}", self.primitives.len());
                let i = expr_lit.lit;
                let (lit, ty) = match &i {
                    Lit::Str(_) => (i.clone(), quote!(&str)),
                    Lit::ByteStr(_) => (i.clone(), quote!(&[u8])),
                    Lit::Byte(_) => (i.clone(), quote!(u8)),
                    Lit::Char(_) => (i.clone(), quote!(char)),
                    Lit::Int(lit_int) => {
                        let ty = quote!(u8);
                        (i.clone(), ty)
                    }
                    Lit::Float(lit_float) => {
                        let ty = quote!(f32);
                        (i.clone(), ty)
                    }
                    Lit::Bool(_) => (i.clone(), quote!(bool)),
                    Lit::Verbatim(_) => (i.clone(), quote!()),
                        &_ => todo!()
                };

                self.primitives.push((lit, var_name.clone(), ty));
                self.replacements.push((i.clone(), Ident::new(&var_name.clone(), proc_macro2::Span::call_site())));

                let var_ident = syn::Ident::new(&*var_name, proc_macro2::Span::call_site());

                    let replaced_expr = Expr::Path(syn::ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: var_ident.into(),
                    });
                return fold::fold_expr(self, replaced_expr);

            },
            // Delegate to the default fold implementation
            _ => fold::fold_expr(self, expr),
        }
    }
    fn fold_expr_repeat(&mut self, i: ExprRepeat) -> ExprRepeat {
        let mut ans = i.clone();
        ans.expr = Box::new(self.fold_expr( *i.expr));
        ans
    }

    fn fold_type(&mut self, i: Type) -> Type {
        i
    }

    fn fold_expr_macro(&mut self, node: ExprMacro) -> ExprMacro {
        // println!("{} {} {}", self.in_mod, self.in_fn, node.to_token_stream().to_string());
        if !self.in_mod || !self.in_fn {
            return fold::fold_expr_macro(self, node);
        }
        let macro_name = node.mac.path.segments.last().unwrap().ident.to_string();

        let new_macro_name = match macro_name.as_str() {
            "assert" => String::from("debug_assert"),
            "assert_eq" => String::from("debug_assert_eq"),
            "assert_ne" => String::from("debug_assert_ne"),
            "assert_matches" => String::from("debug_assert_matches"),
            _ => {
                macro_name.clone()
            }
        };
        if new_macro_name.starts_with("assert"){
            println!("missing {}", new_macro_name);
        }
        let mut new_macro = node.clone();
        let tmp = format!("f({})", node.mac.tokens.to_string());
        // Parse the code into a syntax tree
        if let Ok(syntax_tree) = syn::parse_str::<ExprCall>(&tmp){
            let mut first = true;
            let mut parsed = String::new();
            if macro_name.eq("format") || macro_name.eq("println") || macro_name.eq("print") || macro_name.eq("panic"){
                for ex in syntax_tree.args {
                    if first {
                        parsed += &*ex.to_token_stream().to_string();
                        first = false;
                    } else {
                        let nf = self.fold_expr(ex);
                        parsed += &*nf.to_token_stream().to_string();

                    }
                    parsed += ", ";
                }
            }else {
                for ex in syntax_tree.args {
                    if first {
                        let nf = self.fold_expr(ex);
                        parsed += &*nf.to_token_stream().to_string();
                        first = false;
                    } else {
                        parsed += &*ex.to_token_stream().to_string();
                    }
                    parsed += ", ";
                }
            }
            let t = &*parsed;
            let end = if parsed.len() > 2 { parsed.len() - 2} else { parsed.len() };
            new_macro.mac.tokens = proc_macro2::TokenStream::from_str(&t[0..end]).unwrap();
        }
        if !macro_name.eq(&new_macro_name) {
            // Create an identifier for `sel_println`
            let new_ident = Ident::new(&*new_macro_name.clone(), proc_macro2::Span::call_site());
            new_macro.mac.path = new_ident.into();
        }

        // Return the original macro if it's not `println!`
        return fold::fold_expr_macro(self, new_macro);
    }
    fn fold_item_mod(&mut self, i: ItemMod) -> ItemMod {

        if i.ident.to_string().starts_with("tests_llm_16") || i.ident.to_string().starts_with("tests_rug"){
            // println!("item mod {}", i.ident.to_string());
            if let Some(tar) = &self.target_mod{
                if tar.eq(&i.ident.to_string()){
                    self.in_mod = true;
                    let ans = fold::fold_item_mod(self,i);
                    self.in_mod = false;
                    return ans
                }
                return fold::fold_item_mod(self, i);
            }else {
                println!("{}",i.ident.to_string());
                return fold::fold_item_mod(self, i);
            }

        }else {
            return fold::fold_item_mod(self, i);
        }
    }

    fn fold_item_fn(&mut self, i: ItemFn) -> ItemFn {
        self.cur_fn = i.sig.ident.to_string();
        if self.in_mod {
            // println!("mod f {}", i.to_token_stream().to_string());
            self.clean();
            self.in_fn = true;
            let mut ans = fold::fold_item_fn(self,i);
            self.in_fn = false;
            let (t, st, ed) = self.generate_declarations();
            // Insert the new variable declarations at the beginning of the block
            ans.block.stmts.splice(0..0, t);
            ans.block.stmts.insert(0, st);
            ans.block.stmts.push(ed);
            return ans;
        }else {
            return fold::fold_item_fn(self, i);
        }

    }
}


fn main()-> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(loc) = args.get(1){
        let mut file = std::fs::File::open(loc)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let syntax_tree = syn::parse_file(&*contents).unwrap();
        let mut replacer = LiteralReplacer::new();
        if let Some(tar) = args.get(2){
            replacer = LiteralReplacer::new_with_target(tar.clone());
            let n_tree = replacer.fold_file(syntax_tree);
            let formt = prettyplease::unparse(&n_tree);
            // println!("{}", formt);
            std::fs::write(loc, formt)?;
        }else {
            let _ = replacer.fold_file(syntax_tree);
        }
    }

    // Parse the code into a syntax tree

    Ok(())
}
