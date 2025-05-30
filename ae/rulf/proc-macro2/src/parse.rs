use crate::fallback::{
    is_ident_continue, is_ident_start, Group, LexError, Literal, Span, TokenStream,
};
use crate::{Delimiter, Punct, Spacing, TokenTree};
use std::char;
use std::str::{Bytes, CharIndices, Chars};
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) struct Cursor<'a> {
    pub rest: &'a str,
    #[cfg(span_locations)]
    pub off: u32,
}
impl<'a> Cursor<'a> {
    fn advance(&self, bytes: usize) -> Cursor<'a> {
        let (_front, rest) = self.rest.split_at(bytes);
        Cursor {
            rest,
            #[cfg(span_locations)]
            off: self.off + _front.chars().count() as u32,
        }
    }
    fn starts_with(&self, s: &str) -> bool {
        self.rest.starts_with(s)
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }
    fn len(&self) -> usize {
        self.rest.len()
    }
    fn as_bytes(&self) -> &'a [u8] {
        self.rest.as_bytes()
    }
    fn bytes(&self) -> Bytes<'a> {
        self.rest.bytes()
    }
    fn chars(&self) -> Chars<'a> {
        self.rest.chars()
    }
    fn char_indices(&self) -> CharIndices<'a> {
        self.rest.char_indices()
    }
    fn parse(&self, tag: &str) -> Result<Cursor<'a>, LexError> {
        if self.starts_with(tag) { Ok(self.advance(tag.len())) } else { Err(LexError) }
    }
}
type PResult<'a, O> = Result<(Cursor<'a>, O), LexError>;
fn skip_whitespace(input: Cursor) -> Cursor {
    let mut s = input;
    while !s.is_empty() {
        let byte = s.as_bytes()[0];
        if byte == b'/' {
            if s.starts_with("//") && (!s.starts_with("///") || s.starts_with("////"))
                && !s.starts_with("//!")
            {
                let (cursor, _) = take_until_newline_or_eof(s);
                s = cursor;
                continue;
            } else if s.starts_with("/**/") {
                s = s.advance(4);
                continue;
            } else if s.starts_with("/*")
                && (!s.starts_with("/**") || s.starts_with("/***"))
                && !s.starts_with("/*!")
            {
                match block_comment(s) {
                    Ok((rest, _)) => {
                        s = rest;
                        continue;
                    }
                    Err(LexError) => return s,
                }
            }
        }
        match byte {
            b' ' | 0x09..=0x0d => {
                s = s.advance(1);
                continue;
            }
            b if b <= 0x7f => {}
            _ => {
                let ch = s.chars().next().unwrap();
                if is_whitespace(ch) {
                    s = s.advance(ch.len_utf8());
                    continue;
                }
            }
        }
        return s;
    }
    s
}
fn block_comment(input: Cursor) -> PResult<&str> {
    if !input.starts_with("/*") {
        return Err(LexError);
    }
    let mut depth = 0;
    let bytes = input.as_bytes();
    let mut i = 0;
    let upper = bytes.len() - 1;
    while i < upper {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 1;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            if depth == 0 {
                return Ok((input.advance(i + 2), &input.rest[..i + 2]));
            }
            i += 1;
        }
        i += 1;
    }
    Err(LexError)
}
fn is_whitespace(ch: char) -> bool {
    ch.is_whitespace() || ch == '\u{200e}' || ch == '\u{200f}'
}
fn word_break(input: Cursor) -> Result<Cursor, LexError> {
    match input.chars().next() {
        Some(ch) if is_ident_continue(ch) => Err(LexError),
        Some(_) | None => Ok(input),
    }
}
pub(crate) fn token_stream(mut input: Cursor) -> PResult<TokenStream> {
    let mut trees = Vec::new();
    let mut stack = Vec::new();
    loop {
        input = skip_whitespace(input);
        if let Ok((rest, tt)) = doc_comment(input) {
            trees.extend(tt);
            input = rest;
            continue;
        }
        #[cfg(span_locations)]
        let lo = input.off;
        let first = match input.bytes().next() {
            Some(first) => first,
            None => break,
        };
        if let Some(open_delimiter)
            = match first {
                b'(' => Some(Delimiter::Parenthesis),
                b'[' => Some(Delimiter::Bracket),
                b'{' => Some(Delimiter::Brace),
                _ => None,
            } {
            input = input.advance(1);
            let frame = (open_delimiter, trees);
            #[cfg(span_locations)]
            let frame = (lo, frame);
            stack.push(frame);
            trees = Vec::new();
        } else if let Some(close_delimiter)
            = match first {
                b')' => Some(Delimiter::Parenthesis),
                b']' => Some(Delimiter::Bracket),
                b'}' => Some(Delimiter::Brace),
                _ => None,
            } {
            input = input.advance(1);
            let frame = stack.pop().ok_or(LexError)?;
            #[cfg(span_locations)]
            let (lo, frame) = frame;
            let (open_delimiter, outer) = frame;
            if open_delimiter != close_delimiter {
                return Err(LexError);
            }
            let mut g = Group::new(open_delimiter, TokenStream { inner: trees });
            g.set_span(Span {
                #[cfg(span_locations)]
                lo,
                #[cfg(span_locations)]
                hi: input.off,
            });
            trees = outer;
            trees.push(TokenTree::Group(crate::Group::_new_stable(g)));
        } else {
            let (rest, mut tt) = leaf_token(input)?;
            tt.set_span(
                crate::Span::_new_stable(Span {
                    #[cfg(span_locations)]
                    lo,
                    #[cfg(span_locations)]
                    hi: rest.off,
                }),
            );
            trees.push(tt);
            input = rest;
        }
    }
    if stack.is_empty() {
        Ok((input, TokenStream { inner: trees }))
    } else {
        Err(LexError)
    }
}
fn leaf_token(input: Cursor) -> PResult<TokenTree> {
    if let Ok((input, l)) = literal(input) {
        Ok((input, TokenTree::Literal(crate::Literal::_new_stable(l))))
    } else if let Ok((input, p)) = punct(input) {
        Ok((input, TokenTree::Punct(p)))
    } else if let Ok((input, i)) = ident(input) {
        Ok((input, TokenTree::Ident(i)))
    } else {
        Err(LexError)
    }
}
fn ident(input: Cursor) -> PResult<crate::Ident> {
    if ["r\"", "r#\"", "r##", "b\"", "b\'", "br\"", "br#"]
        .iter()
        .any(|prefix| input.starts_with(prefix))
    {
        Err(LexError)
    } else {
        ident_any(input)
    }
}
fn ident_any(input: Cursor) -> PResult<crate::Ident> {
    let raw = input.starts_with("r#");
    let rest = input.advance((raw as usize) << 1);
    let (rest, sym) = ident_not_raw(rest)?;
    if !raw {
        let ident = crate::Ident::new(sym, crate::Span::call_site());
        return Ok((rest, ident));
    }
    if sym == "_" {
        return Err(LexError);
    }
    let ident = crate::Ident::_new_raw(sym, crate::Span::call_site());
    Ok((rest, ident))
}
fn ident_not_raw(input: Cursor) -> PResult<&str> {
    let mut chars = input.char_indices();
    match chars.next() {
        Some((_, ch)) if is_ident_start(ch) => {}
        _ => return Err(LexError),
    }
    let mut end = input.len();
    for (i, ch) in chars {
        if !is_ident_continue(ch) {
            end = i;
            break;
        }
    }
    Ok((input.advance(end), &input.rest[..end]))
}
fn literal(input: Cursor) -> PResult<Literal> {
    match literal_nocapture(input) {
        Ok(a) => {
            let end = input.len() - a.len();
            Ok((a, Literal::_new(input.rest[..end].to_string())))
        }
        Err(LexError) => Err(LexError),
    }
}
fn literal_nocapture(input: Cursor) -> Result<Cursor, LexError> {
    if let Ok(ok) = string(input) {
        Ok(ok)
    } else if let Ok(ok) = byte_string(input) {
        Ok(ok)
    } else if let Ok(ok) = byte(input) {
        Ok(ok)
    } else if let Ok(ok) = character(input) {
        Ok(ok)
    } else if let Ok(ok) = float(input) {
        Ok(ok)
    } else if let Ok(ok) = int(input) {
        Ok(ok)
    } else {
        Err(LexError)
    }
}
fn literal_suffix(input: Cursor) -> Cursor {
    match ident_not_raw(input) {
        Ok((input, _)) => input,
        Err(LexError) => input,
    }
}
fn string(input: Cursor) -> Result<Cursor, LexError> {
    if let Ok(input) = input.parse("\"") {
        cooked_string(input)
    } else if let Ok(input) = input.parse("r") {
        raw_string(input)
    } else {
        Err(LexError)
    }
}
fn cooked_string(input: Cursor) -> Result<Cursor, LexError> {
    let mut chars = input.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '"' => {
                let input = input.advance(i + 1);
                return Ok(literal_suffix(input));
            }
            '\r' => {
                match chars.next() {
                    Some((_, '\n')) => {}
                    _ => break,
                }
            }
            '\\' => {
                match chars.next() {
                    Some((_, 'x')) => {
                        if !backslash_x_char(&mut chars) {
                            break;
                        }
                    }
                    Some((_, 'n'))
                    | Some((_, 'r'))
                    | Some((_, 't'))
                    | Some((_, '\\'))
                    | Some((_, '\''))
                    | Some((_, '"'))
                    | Some((_, '0')) => {}
                    Some((_, 'u')) => {
                        if !backslash_u(&mut chars) {
                            break;
                        }
                    }
                    Some((_, ch @ '\n')) | Some((_, ch @ '\r')) => {
                        let mut last = ch;
                        loop {
                            if last == '\r'
                                && chars.next().map_or(true, |(_, ch)| ch != '\n')
                            {
                                return Err(LexError);
                            }
                            match chars.peek() {
                                Some((_, ch)) if ch.is_whitespace() => {
                                    last = *ch;
                                    chars.next();
                                }
                                _ => break,
                            }
                        }
                    }
                    _ => break,
                }
            }
            _ch => {}
        }
    }
    Err(LexError)
}
fn byte_string(input: Cursor) -> Result<Cursor, LexError> {
    if let Ok(input) = input.parse("b\"") {
        cooked_byte_string(input)
    } else if let Ok(input) = input.parse("br") {
        raw_string(input)
    } else {
        Err(LexError)
    }
}
fn cooked_byte_string(mut input: Cursor) -> Result<Cursor, LexError> {
    let mut bytes = input.bytes().enumerate();
    while let Some((offset, b)) = bytes.next() {
        match b {
            b'"' => {
                let input = input.advance(offset + 1);
                return Ok(literal_suffix(input));
            }
            b'\r' => {
                match bytes.next() {
                    Some((_, b'\n')) => {}
                    _ => break,
                }
            }
            b'\\' => {
                match bytes.next() {
                    Some((_, b'x')) => {
                        if !backslash_x_byte(&mut bytes) {
                            break;
                        }
                    }
                    Some((_, b'n'))
                    | Some((_, b'r'))
                    | Some((_, b't'))
                    | Some((_, b'\\'))
                    | Some((_, b'0'))
                    | Some((_, b'\''))
                    | Some((_, b'"')) => {}
                    Some((newline, b @ b'\n')) | Some((newline, b @ b'\r')) => {
                        let mut last = b as char;
                        let rest = input.advance(newline + 1);
                        let mut chars = rest.char_indices();
                        loop {
                            if last == '\r'
                                && chars.next().map_or(true, |(_, ch)| ch != '\n')
                            {
                                return Err(LexError);
                            }
                            match chars.next() {
                                Some((_, ch)) if ch.is_whitespace() => last = ch,
                                Some((offset, _)) => {
                                    input = rest.advance(offset);
                                    bytes = input.bytes().enumerate();
                                    break;
                                }
                                None => return Err(LexError),
                            }
                        }
                    }
                    _ => break,
                }
            }
            b if b < 0x80 => {}
            _ => break,
        }
    }
    Err(LexError)
}
fn raw_string(input: Cursor) -> Result<Cursor, LexError> {
    let mut chars = input.char_indices();
    let mut n = 0;
    while let Some((i, ch)) = chars.next() {
        match ch {
            '"' => {
                n = i;
                break;
            }
            '#' => {}
            _ => return Err(LexError),
        }
    }
    while let Some((i, ch)) = chars.next() {
        match ch {
            '"' if input.rest[i + 1..].starts_with(&input.rest[..n]) => {
                let rest = input.advance(i + 1 + n);
                return Ok(literal_suffix(rest));
            }
            '\r' => {
                match chars.next() {
                    Some((_, '\n')) => {}
                    _ => break,
                }
            }
            _ => {}
        }
    }
    Err(LexError)
}
fn byte(input: Cursor) -> Result<Cursor, LexError> {
    let input = input.parse("b'")?;
    let mut bytes = input.bytes().enumerate();
    let ok = match bytes.next().map(|(_, b)| b) {
        Some(b'\\') => {
            match bytes.next().map(|(_, b)| b) {
                Some(b'x') => backslash_x_byte(&mut bytes),
                Some(b'n')
                | Some(b'r')
                | Some(b't')
                | Some(b'\\')
                | Some(b'0')
                | Some(b'\'')
                | Some(b'"') => true,
                _ => false,
            }
        }
        b => b.is_some(),
    };
    if !ok {
        return Err(LexError);
    }
    let (offset, _) = bytes.next().ok_or(LexError)?;
    if !input.chars().as_str().is_char_boundary(offset) {
        return Err(LexError);
    }
    let input = input.advance(offset).parse("'")?;
    Ok(literal_suffix(input))
}
fn character(input: Cursor) -> Result<Cursor, LexError> {
    let input = input.parse("'")?;
    let mut chars = input.char_indices();
    let ok = match chars.next().map(|(_, ch)| ch) {
        Some('\\') => {
            match chars.next().map(|(_, ch)| ch) {
                Some('x') => backslash_x_char(&mut chars),
                Some('u') => backslash_u(&mut chars),
                Some('n')
                | Some('r')
                | Some('t')
                | Some('\\')
                | Some('0')
                | Some('\'')
                | Some('"') => true,
                _ => false,
            }
        }
        ch => ch.is_some(),
    };
    if !ok {
        return Err(LexError);
    }
    let (idx, _) = chars.next().ok_or(LexError)?;
    let input = input.advance(idx).parse("'")?;
    Ok(literal_suffix(input))
}
macro_rules! next_ch {
    ($chars:ident @ $pat:pat $(| $rest:pat)*) => {
        match $chars .next() { Some((_, ch)) => match ch { $pat $(| $rest)* => ch, _ =>
        return false, }, None => return false, }
    };
}
fn backslash_x_char<I>(chars: &mut I) -> bool
where
    I: Iterator<Item = (usize, char)>,
{
    next_ch!(chars @ '0'..='7');
    next_ch!(chars @ '0'..='9' | 'a'..='f' | 'A'..='F');
    true
}
fn backslash_x_byte<I>(chars: &mut I) -> bool
where
    I: Iterator<Item = (usize, u8)>,
{
    next_ch!(chars @ b'0'..= b'9' | b'a'..= b'f' | b'A'..= b'F');
    next_ch!(chars @ b'0'..= b'9' | b'a'..= b'f' | b'A'..= b'F');
    true
}
fn backslash_u<I>(chars: &mut I) -> bool
where
    I: Iterator<Item = (usize, char)>,
{
    next_ch!(chars @ '{');
    let mut value = 0;
    let mut len = 0;
    while let Some((_, ch)) = chars.next() {
        let digit = match ch {
            '0'..='9' => ch as u8 - b'0',
            'a'..='f' => 10 + ch as u8 - b'a',
            'A'..='F' => 10 + ch as u8 - b'A',
            '_' if len > 0 => continue,
            '}' if len > 0 => return char::from_u32(value).is_some(),
            _ => return false,
        };
        if len == 6 {
            return false;
        }
        value *= 0x10;
        value += u32::from(digit);
        len += 1;
    }
    false
}
fn float(input: Cursor) -> Result<Cursor, LexError> {
    let mut rest = float_digits(input)?;
    if let Some(ch) = rest.chars().next() {
        if is_ident_start(ch) {
            rest = ident_not_raw(rest)?.0;
        }
    }
    word_break(rest)
}
fn float_digits(input: Cursor) -> Result<Cursor, LexError> {
    let mut chars = input.chars().peekable();
    match chars.next() {
        Some(ch) if ch >= '0' && ch <= '9' => {}
        _ => return Err(LexError),
    }
    let mut len = 1;
    let mut has_dot = false;
    let mut has_exp = false;
    while let Some(&ch) = chars.peek() {
        match ch {
            '0'..='9' | '_' => {
                chars.next();
                len += 1;
            }
            '.' => {
                if has_dot {
                    break;
                }
                chars.next();
                if chars
                    .peek()
                    .map(|&ch| ch == '.' || is_ident_start(ch))
                    .unwrap_or(false)
                {
                    return Err(LexError);
                }
                len += 1;
                has_dot = true;
            }
            'e' | 'E' => {
                chars.next();
                len += 1;
                has_exp = true;
                break;
            }
            _ => break,
        }
    }
    if !(has_dot || has_exp) {
        return Err(LexError);
    }
    if has_exp {
        let token_before_exp = if has_dot {
            Ok(input.advance(len - 1))
        } else {
            Err(LexError)
        };
        let mut has_sign = false;
        let mut has_exp_value = false;
        while let Some(&ch) = chars.peek() {
            match ch {
                '+' | '-' => {
                    if has_exp_value {
                        break;
                    }
                    if has_sign {
                        return token_before_exp;
                    }
                    chars.next();
                    len += 1;
                    has_sign = true;
                }
                '0'..='9' => {
                    chars.next();
                    len += 1;
                    has_exp_value = true;
                }
                '_' => {
                    chars.next();
                    len += 1;
                }
                _ => break,
            }
        }
        if !has_exp_value {
            return token_before_exp;
        }
    }
    Ok(input.advance(len))
}
fn int(input: Cursor) -> Result<Cursor, LexError> {
    let mut rest = digits(input)?;
    if let Some(ch) = rest.chars().next() {
        if is_ident_start(ch) {
            rest = ident_not_raw(rest)?.0;
        }
    }
    word_break(rest)
}
fn digits(mut input: Cursor) -> Result<Cursor, LexError> {
    let base = if input.starts_with("0x") {
        input = input.advance(2);
        16
    } else if input.starts_with("0o") {
        input = input.advance(2);
        8
    } else if input.starts_with("0b") {
        input = input.advance(2);
        2
    } else {
        10
    };
    let mut len = 0;
    let mut empty = true;
    for b in input.bytes() {
        match b {
            b'0'..=b'9' => {
                let digit = (b - b'0') as u64;
                if digit >= base {
                    return Err(LexError);
                }
            }
            b'a'..=b'f' => {
                let digit = 10 + (b - b'a') as u64;
                if digit >= base {
                    break;
                }
            }
            b'A'..=b'F' => {
                let digit = 10 + (b - b'A') as u64;
                if digit >= base {
                    break;
                }
            }
            b'_' => {
                if empty && base == 10 {
                    return Err(LexError);
                }
                len += 1;
                continue;
            }
            _ => break,
        };
        len += 1;
        empty = false;
    }
    if empty { Err(LexError) } else { Ok(input.advance(len)) }
}
fn punct(input: Cursor) -> PResult<Punct> {
    match punct_char(input) {
        Ok((rest, '\'')) => {
            if ident_any(rest)?.0.starts_with("'") {
                Err(LexError)
            } else {
                Ok((rest, Punct::new('\'', Spacing::Joint)))
            }
        }
        Ok((rest, ch)) => {
            let kind = match punct_char(rest) {
                Ok(_) => Spacing::Joint,
                Err(LexError) => Spacing::Alone,
            };
            Ok((rest, Punct::new(ch, kind)))
        }
        Err(LexError) => Err(LexError),
    }
}
fn punct_char(input: Cursor) -> PResult<char> {
    if input.starts_with("//") || input.starts_with("/*") {
        return Err(LexError);
    }
    let mut chars = input.chars();
    let first = match chars.next() {
        Some(ch) => ch,
        None => {
            return Err(LexError);
        }
    };
    let recognized = "~!@#$%^&*-=+|;:,<.>/?'";
    if recognized.contains(first) {
        Ok((input.advance(first.len_utf8()), first))
    } else {
        Err(LexError)
    }
}
fn doc_comment(input: Cursor) -> PResult<Vec<TokenTree>> {
    #[cfg(span_locations)]
    let lo = input.off;
    let (rest, (comment, inner)) = doc_comment_contents(input)?;
    let span = crate::Span::_new_stable(Span {
        #[cfg(span_locations)]
        lo,
        #[cfg(span_locations)]
        hi: rest.off,
    });
    let mut scan_for_bare_cr = comment;
    while let Some(cr) = scan_for_bare_cr.find('\r') {
        let rest = &scan_for_bare_cr[cr + 1..];
        if !rest.starts_with('\n') {
            return Err(LexError);
        }
        scan_for_bare_cr = rest;
    }
    let mut trees = Vec::new();
    trees.push(TokenTree::Punct(Punct::new('#', Spacing::Alone)));
    if inner {
        trees.push(Punct::new('!', Spacing::Alone).into());
    }
    let mut stream = vec![
        TokenTree::Ident(crate ::Ident::new("doc", span)),
        TokenTree::Punct(Punct::new('=', Spacing::Alone)), TokenTree::Literal(crate
        ::Literal::string(comment)),
    ];
    for tt in stream.iter_mut() {
        tt.set_span(span);
    }
    let group = Group::new(Delimiter::Bracket, stream.into_iter().collect());
    trees.push(crate::Group::_new_stable(group).into());
    for tt in trees.iter_mut() {
        tt.set_span(span);
    }
    Ok((rest, trees))
}
fn doc_comment_contents(input: Cursor) -> PResult<(&str, bool)> {
    if input.starts_with("//!") {
        let input = input.advance(3);
        let (input, s) = take_until_newline_or_eof(input);
        Ok((input, (s, true)))
    } else if input.starts_with("/*!") {
        let (input, s) = block_comment(input)?;
        Ok((input, (&s[3..s.len() - 2], true)))
    } else if input.starts_with("///") {
        let input = input.advance(3);
        if input.starts_with("/") {
            return Err(LexError);
        }
        let (input, s) = take_until_newline_or_eof(input);
        Ok((input, (s, false)))
    } else if input.starts_with("/**") && !input.rest[3..].starts_with('*') {
        let (input, s) = block_comment(input)?;
        Ok((input, (&s[3..s.len() - 2], false)))
    } else {
        Err(LexError)
    }
}
fn take_until_newline_or_eof(input: Cursor) -> (Cursor, &str) {
    let chars = input.char_indices();
    for (i, ch) in chars {
        if ch == '\n' {
            return (input.advance(i), &input.rest[..i]);
        } else if ch == '\r' && input.rest[i + 1..].starts_with('\n') {
            return (input.advance(i + 1), &input.rest[..i]);
        }
    }
    (input.advance(input.len()), input.rest)
}
#[cfg(test)]
mod tests_llm_16_484 {
    use super::*;
    use crate::*;
    #[test]
    fn test_advance() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_advance = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 7;
        let rug_fuzz_3 = "Hello, world!";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let new_cursor = cursor.advance(rug_fuzz_2);
        debug_assert_eq!(new_cursor.rest, "world!");
        #[cfg(span_locations)] debug_assert_eq!(new_cursor.off, 7);
        let cursor = Cursor {
            rest: rug_fuzz_3,
            #[cfg(span_locations)]
            off: rug_fuzz_4,
        };
        let new_cursor = cursor.advance(rug_fuzz_5);
        debug_assert_eq!(new_cursor.rest, "Hello, world!");
        #[cfg(span_locations)] debug_assert_eq!(new_cursor.off, 0);
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_advance = 0;
    }
    #[test]
    fn test_starts_with() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_starts_with = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "Hello";
        let rug_fuzz_3 = "foo";
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        debug_assert!(cursor.starts_with(rug_fuzz_2));
        debug_assert!(! cursor.starts_with(rug_fuzz_3));
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_starts_with = 0;
    }
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "Hello, world!";
        let rug_fuzz_3 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        debug_assert!(cursor.is_empty());
        let cursor = Cursor {
            rest: rug_fuzz_2,
            #[cfg(span_locations)]
            off: rug_fuzz_3,
        };
        debug_assert!(! cursor.is_empty());
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_is_empty = 0;
    }
    #[test]
    fn test_len() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_len = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        debug_assert_eq!(cursor.len(), 13);
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_len = 0;
    }
    #[test]
    fn test_as_bytes() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_as_bytes = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        debug_assert_eq!(cursor.as_bytes(), b"Hello, world!");
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_as_bytes = 0;
    }
    #[test]
    fn test_bytes() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_bytes = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let mut iter = cursor.bytes();
        debug_assert_eq!(iter.next(), Some(b'H'));
        debug_assert_eq!(iter.next(), Some(b'e'));
        debug_assert_eq!(iter.next(), Some(b'l'));
        debug_assert_eq!(iter.next(), Some(b'l'));
        debug_assert_eq!(iter.next(), Some(b'o'));
        debug_assert_eq!(iter.next(), Some(b','));
        debug_assert_eq!(iter.next(), Some(b' '));
        debug_assert_eq!(iter.next(), Some(b'w'));
        debug_assert_eq!(iter.next(), Some(b'o'));
        debug_assert_eq!(iter.next(), Some(b'r'));
        debug_assert_eq!(iter.next(), Some(b'l'));
        debug_assert_eq!(iter.next(), Some(b'd'));
        debug_assert_eq!(iter.next(), Some(b'!'));
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_bytes = 0;
    }
    #[test]
    fn test_chars() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_chars = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let mut iter = cursor.chars();
        debug_assert_eq!(iter.next(), Some('H'));
        debug_assert_eq!(iter.next(), Some('e'));
        debug_assert_eq!(iter.next(), Some('l'));
        debug_assert_eq!(iter.next(), Some('l'));
        debug_assert_eq!(iter.next(), Some('o'));
        debug_assert_eq!(iter.next(), Some(','));
        debug_assert_eq!(iter.next(), Some(' '));
        debug_assert_eq!(iter.next(), Some('w'));
        debug_assert_eq!(iter.next(), Some('o'));
        debug_assert_eq!(iter.next(), Some('r'));
        debug_assert_eq!(iter.next(), Some('l'));
        debug_assert_eq!(iter.next(), Some('d'));
        debug_assert_eq!(iter.next(), Some('!'));
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_chars = 0;
    }
    #[test]
    fn test_char_indices() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_char_indices = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let mut iter = cursor.char_indices();
        debug_assert_eq!(iter.next(), Some((0, 'H')));
        debug_assert_eq!(iter.next(), Some((1, 'e')));
        debug_assert_eq!(iter.next(), Some((2, 'l')));
        debug_assert_eq!(iter.next(), Some((3, 'l')));
        debug_assert_eq!(iter.next(), Some((4, 'o')));
        debug_assert_eq!(iter.next(), Some((5, ',')));
        debug_assert_eq!(iter.next(), Some((6, ' ')));
        debug_assert_eq!(iter.next(), Some((7, 'w')));
        debug_assert_eq!(iter.next(), Some((8, 'o')));
        debug_assert_eq!(iter.next(), Some((9, 'r')));
        debug_assert_eq!(iter.next(), Some((10, 'l')));
        debug_assert_eq!(iter.next(), Some((11, 'd')));
        debug_assert_eq!(iter.next(), Some((12, '!')));
        debug_assert_eq!(iter.next(), None);
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_char_indices = 0;
    }
    #[test]
    fn test_parse() {
        let _rug_st_tests_llm_16_484_rrrruuuugggg_test_parse = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "Hello";
        let rug_fuzz_3 = "Hello, world!";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = "foo";
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let new_cursor = cursor.parse(rug_fuzz_2).unwrap();
        debug_assert_eq!(new_cursor.rest, ", world!");
        #[cfg(span_locations)] debug_assert_eq!(new_cursor.off, 5);
        let cursor = Cursor {
            rest: rug_fuzz_3,
            #[cfg(span_locations)]
            off: rug_fuzz_4,
        };
        let result = cursor.parse(rug_fuzz_5);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_484_rrrruuuugggg_test_parse = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_491 {
    use super::*;
    use crate::*;
    #[test]
    fn test_chars() {
        let _rug_st_tests_llm_16_491_rrrruuuugggg_test_chars = 0;
        let rug_fuzz_0 = "Hello, world!";
        let rug_fuzz_1 = 0;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let mut chars = cursor.chars();
        debug_assert_eq!(chars.next(), Some('H'));
        debug_assert_eq!(chars.next(), Some('e'));
        debug_assert_eq!(chars.next(), Some('l'));
        debug_assert_eq!(chars.next(), Some('l'));
        debug_assert_eq!(chars.next(), Some('o'));
        debug_assert_eq!(chars.next(), Some(','));
        debug_assert_eq!(chars.next(), Some(' '));
        debug_assert_eq!(chars.next(), Some('w'));
        debug_assert_eq!(chars.next(), Some('o'));
        debug_assert_eq!(chars.next(), Some('r'));
        debug_assert_eq!(chars.next(), Some('l'));
        debug_assert_eq!(chars.next(), Some('d'));
        debug_assert_eq!(chars.next(), Some('!'));
        debug_assert_eq!(chars.next(), None);
        let _rug_ed_tests_llm_16_491_rrrruuuugggg_test_chars = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_492 {
    use super::*;
    use crate::*;
    use std::boxed::Box;
    #[test]
    fn test_is_empty() {
        let _rug_st_tests_llm_16_492_rrrruuuugggg_test_is_empty = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = true;
        let rug_fuzz_3 = "hello";
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = false;
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        debug_assert_eq!(rug_fuzz_2, cursor.is_empty());
        let cursor = Cursor {
            rest: rug_fuzz_3,
            #[cfg(span_locations)]
            off: rug_fuzz_4,
        };
        debug_assert_eq!(rug_fuzz_5, cursor.is_empty());
        let _rug_ed_tests_llm_16_492_rrrruuuugggg_test_is_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_496 {
    use super::*;
    use crate::*;
    use crate::parse::Cursor;
    use crate::LexError;
    #[test]
    fn parse_success() {
        let _rug_st_tests_llm_16_496_rrrruuuugggg_parse_success = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = "Hello";
        let cursor = Cursor { rest: rug_fuzz_0 };
        let tag = rug_fuzz_1;
        let result = cursor.parse(tag);
        debug_assert!(result.is_ok());
        let parsed = result.unwrap();
        debug_assert_eq!(parsed.rest, ", World!");
        let _rug_ed_tests_llm_16_496_rrrruuuugggg_parse_success = 0;
    }
    #[test]
    fn parse_failure() {
        let _rug_st_tests_llm_16_496_rrrruuuugggg_parse_failure = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = "Hi";
        let cursor = Cursor { rest: rug_fuzz_0 };
        let tag = rug_fuzz_1;
        let result = cursor.parse(tag);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_496_rrrruuuugggg_parse_failure = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_497 {
    use super::*;
    use crate::*;
    #[test]
    fn test_starts_with_returns_true_if_rest_starts_with_given_str() {
        let _rug_st_tests_llm_16_497_rrrruuuugggg_test_starts_with_returns_true_if_rest_starts_with_given_str = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "Hello";
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let result = cursor.starts_with(rug_fuzz_2);
        debug_assert_eq!(result, true);
        let _rug_ed_tests_llm_16_497_rrrruuuugggg_test_starts_with_returns_true_if_rest_starts_with_given_str = 0;
    }
    #[test]
    fn test_starts_with_returns_false_if_rest_does_not_start_with_given_str() {
        let _rug_st_tests_llm_16_497_rrrruuuugggg_test_starts_with_returns_false_if_rest_does_not_start_with_given_str = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "World";
        let cursor = Cursor {
            rest: rug_fuzz_0,
            #[cfg(span_locations)]
            off: rug_fuzz_1,
        };
        let result = cursor.starts_with(rug_fuzz_2);
        debug_assert_eq!(result, false);
        let _rug_ed_tests_llm_16_497_rrrruuuugggg_test_starts_with_returns_false_if_rest_does_not_start_with_given_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_535 {
    use crate::parse::is_whitespace;
    #[test]
    fn test_is_whitespace() {
        let _rug_st_tests_llm_16_535_rrrruuuugggg_test_is_whitespace = 0;
        let rug_fuzz_0 = ' ';
        let rug_fuzz_1 = '\t';
        let rug_fuzz_2 = '\n';
        let rug_fuzz_3 = '\r';
        let rug_fuzz_4 = '\u{200e}';
        let rug_fuzz_5 = '\u{200f}';
        let rug_fuzz_6 = 'a';
        let rug_fuzz_7 = '1';
        let rug_fuzz_8 = '_';
        debug_assert_eq!(is_whitespace(rug_fuzz_0), true);
        debug_assert_eq!(is_whitespace(rug_fuzz_1), true);
        debug_assert_eq!(is_whitespace(rug_fuzz_2), true);
        debug_assert_eq!(is_whitespace(rug_fuzz_3), true);
        debug_assert_eq!(is_whitespace(rug_fuzz_4), true);
        debug_assert_eq!(is_whitespace(rug_fuzz_5), true);
        debug_assert_eq!(is_whitespace(rug_fuzz_6), false);
        debug_assert_eq!(is_whitespace(rug_fuzz_7), false);
        debug_assert_eq!(is_whitespace(rug_fuzz_8), false);
        let _rug_ed_tests_llm_16_535_rrrruuuugggg_test_is_whitespace = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_549 {
    use super::*;
    use crate::*;
    use crate::parse::{Cursor as OtherCursor, LexError as OtherLexError};
    #[test]
    fn test_raw_string_with_valid_input() {
        let _rug_st_tests_llm_16_549_rrrruuuugggg_test_raw_string_with_valid_input = 0;
        let rug_fuzz_0 = "#\"Hello, World!\" World!";
        let input = OtherCursor { rest: rug_fuzz_0 };
        let result = raw_string(input);
        debug_assert!(result.is_ok());
        let rest = result.unwrap();
        debug_assert_eq!(rest.rest, " World!");
        let _rug_ed_tests_llm_16_549_rrrruuuugggg_test_raw_string_with_valid_input = 0;
    }
    #[test]
    fn test_raw_string_with_invalid_input() {
        let _rug_st_tests_llm_16_549_rrrruuuugggg_test_raw_string_with_invalid_input = 0;
        let rug_fuzz_0 = "Hello, World!";
        let input = OtherCursor { rest: rug_fuzz_0 };
        let result = raw_string(input);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_549_rrrruuuugggg_test_raw_string_with_invalid_input = 0;
    }
}
#[cfg(test)]
mod tests_rug_11 {
    use super::*;
    use crate::parse::Cursor;
    #[test]
    fn test_literal_suffix() {
        let mut p0: Cursor<'_> = unimplemented!("Cursor<'_> value");
        crate::parse::literal_suffix(p0);
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::parse::Cursor;
    #[test]
    fn test_bytes() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_bytes = 0;
        let mut p0: Cursor<'static> = unimplemented!();
        <Cursor<'static>>::bytes(&p0);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_bytes = 0;
    }
}
