//! Options for wrapping text.
use crate::{LineEnding, WordSeparator, WordSplitter, WrapAlgorithm};
/// Holds configuration options for wrapping and filling text.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// The width in columns at which the text will be wrapped.
    pub width: usize,
    /// Line ending used for breaking lines.
    pub line_ending: LineEnding,
    /// Indentation used for the first line of output. See the
    /// [`Options::initial_indent`] method.
    pub initial_indent: &'a str,
    /// Indentation used for subsequent lines of output. See the
    /// [`Options::subsequent_indent`] method.
    pub subsequent_indent: &'a str,
    /// Allow long words to be broken if they cannot fit on a line.
    /// When set to `false`, some lines may be longer than
    /// `self.width`. See the [`Options::break_words`] method.
    pub break_words: bool,
    /// Wrapping algorithm to use, see the implementations of the
    /// [`WrapAlgorithm`] trait for details.
    pub wrap_algorithm: WrapAlgorithm,
    /// The line breaking algorithm to use, see the [`WordSeparator`]
    /// trait for an overview and possible implementations.
    pub word_separator: WordSeparator,
    /// The method for splitting words. This can be used to prohibit
    /// splitting words on hyphens, or it can be used to implement
    /// language-aware machine hyphenation.
    pub word_splitter: WordSplitter,
}
impl<'a> From<&'a Options<'a>> for Options<'a> {
    fn from(options: &'a Options<'a>) -> Self {
        Self {
            width: options.width,
            line_ending: options.line_ending,
            initial_indent: options.initial_indent,
            subsequent_indent: options.subsequent_indent,
            break_words: options.break_words,
            word_separator: options.word_separator,
            wrap_algorithm: options.wrap_algorithm,
            word_splitter: options.word_splitter.clone(),
        }
    }
}
impl<'a> From<usize> for Options<'a> {
    fn from(width: usize) -> Self {
        Options::new(width)
    }
}
impl<'a> Options<'a> {
    /// Creates a new [`Options`] with the specified width.
    ///
    /// The other fields are given default values as follows:
    ///
    /// ```
    /// # use textwrap::{LineEnding, Options, WordSplitter, WordSeparator, WrapAlgorithm};
    /// # let width = 80;
    /// let options = Options::new(width);
    /// assert_eq!(options.line_ending, LineEnding::LF);
    /// assert_eq!(options.initial_indent, "");
    /// assert_eq!(options.subsequent_indent, "");
    /// assert_eq!(options.break_words, true);
    ///
    /// #[cfg(feature = "unicode-linebreak")]
    /// assert_eq!(options.word_separator, WordSeparator::UnicodeBreakProperties);
    /// #[cfg(not(feature = "unicode-linebreak"))]
    /// assert_eq!(options.word_separator, WordSeparator::AsciiSpace);
    ///
    /// #[cfg(feature = "smawk")]
    /// assert_eq!(options.wrap_algorithm, WrapAlgorithm::new_optimal_fit());
    /// #[cfg(not(feature = "smawk"))]
    /// assert_eq!(options.wrap_algorithm, WrapAlgorithm::FirstFit);
    ///
    /// assert_eq!(options.word_splitter, WordSplitter::HyphenSplitter);
    /// ```
    ///
    /// Note that the default word separator and wrap algorithms
    /// changes based on the available Cargo features. The best
    /// available algorithms are used by default.
    pub const fn new(width: usize) -> Self {
        Options {
            width,
            line_ending: LineEnding::LF,
            initial_indent: "",
            subsequent_indent: "",
            break_words: true,
            word_separator: WordSeparator::new(),
            wrap_algorithm: WrapAlgorithm::new(),
            word_splitter: WordSplitter::HyphenSplitter,
        }
    }
    /// Change [`self.line_ending`]. This specifies which of the
    /// supported line endings should be used to break the lines of the
    /// input text.
    ///
    /// # Examples
    ///
    /// ```
    /// use textwrap::{refill, LineEnding, Options};
    ///
    /// let options = Options::new(15).line_ending(LineEnding::CRLF);
    /// assert_eq!(refill("This is a little example.", options),
    ///            "This is a\r\nlittle example.");
    /// ```
    ///
    /// [`self.line_ending`]: #structfield.line_ending
    pub fn line_ending(self, line_ending: LineEnding) -> Self {
        Options { line_ending, ..self }
    }
    /// Change [`self.initial_indent`]. The initial indentation is
    /// used on the very first line of output.
    ///
    /// # Examples
    ///
    /// Classic paragraph indentation can be achieved by specifying an
    /// initial indentation and wrapping each paragraph by itself:
    ///
    /// ```
    /// use textwrap::{wrap, Options};
    ///
    /// let options = Options::new(16).initial_indent("    ");
    /// assert_eq!(wrap("This is a little example.", options),
    ///            vec!["    This is a",
    ///                 "little example."]);
    /// ```
    ///
    /// [`self.initial_indent`]: #structfield.initial_indent
    pub fn initial_indent(self, indent: &'a str) -> Self {
        Options {
            initial_indent: indent,
            ..self
        }
    }
    /// Change [`self.subsequent_indent`]. The subsequent indentation
    /// is used on lines following the first line of output.
    ///
    /// # Examples
    ///
    /// Combining initial and subsequent indentation lets you format a
    /// single paragraph as a bullet list:
    ///
    /// ```
    /// use textwrap::{wrap, Options};
    ///
    /// let options = Options::new(12)
    ///     .initial_indent("* ")
    ///     .subsequent_indent("  ");
    /// #[cfg(feature = "smawk")]
    /// assert_eq!(wrap("This is a little example.", options),
    ///            vec!["* This is",
    ///                 "  a little",
    ///                 "  example."]);
    ///
    /// // Without the `smawk` feature, the wrapping is a little different:
    /// #[cfg(not(feature = "smawk"))]
    /// assert_eq!(wrap("This is a little example.", options),
    ///            vec!["* This is a",
    ///                 "  little",
    ///                 "  example."]);
    /// ```
    ///
    /// [`self.subsequent_indent`]: #structfield.subsequent_indent
    pub fn subsequent_indent(self, indent: &'a str) -> Self {
        Options {
            subsequent_indent: indent,
            ..self
        }
    }
    /// Change [`self.break_words`]. This controls if words longer
    /// than `self.width` can be broken, or if they will be left
    /// sticking out into the right margin.
    ///
    /// See [`Options::word_splitter`] instead if you want to control
    /// hyphenation.
    ///
    /// # Examples
    ///
    /// ```
    /// use textwrap::{wrap, Options};
    ///
    /// let options = Options::new(4).break_words(true);
    /// assert_eq!(wrap("This is a little example.", options),
    ///            vec!["This",
    ///                 "is a",
    ///                 "litt",
    ///                 "le",
    ///                 "exam",
    ///                 "ple."]);
    /// ```
    ///
    /// [`self.break_words`]: #structfield.break_words
    pub fn break_words(self, setting: bool) -> Self {
        Options {
            break_words: setting,
            ..self
        }
    }
    /// Change [`self.word_separator`].
    ///
    /// See the [`WordSeparator`] trait for details on the choices.
    ///
    /// [`self.word_separator`]: #structfield.word_separator
    pub fn word_separator(self, word_separator: WordSeparator) -> Options<'a> {
        Options {
            width: self.width,
            line_ending: self.line_ending,
            initial_indent: self.initial_indent,
            subsequent_indent: self.subsequent_indent,
            break_words: self.break_words,
            word_separator: word_separator,
            wrap_algorithm: self.wrap_algorithm,
            word_splitter: self.word_splitter,
        }
    }
    /// Change [`self.wrap_algorithm`].
    ///
    /// See the [`WrapAlgorithm`] trait for details on the choices.
    ///
    /// [`self.wrap_algorithm`]: #structfield.wrap_algorithm
    pub fn wrap_algorithm(self, wrap_algorithm: WrapAlgorithm) -> Options<'a> {
        Options {
            width: self.width,
            line_ending: self.line_ending,
            initial_indent: self.initial_indent,
            subsequent_indent: self.subsequent_indent,
            break_words: self.break_words,
            word_separator: self.word_separator,
            wrap_algorithm: wrap_algorithm,
            word_splitter: self.word_splitter,
        }
    }
    /// Change [`self.word_splitter`]. The [`WordSplitter`] is used to
    /// fit part of a word into the current line when wrapping text.
    ///
    /// See [`Options::break_words`] instead if you want to control the
    /// handling of words longer than the line width.
    ///
    /// # Examples
    ///
    /// ```
    /// use textwrap::{wrap, Options, WordSplitter};
    ///
    /// // The default is WordSplitter::HyphenSplitter.
    /// let options = Options::new(5);
    /// assert_eq!(wrap("foo-bar-baz", &options),
    ///            vec!["foo-", "bar-", "baz"]);
    ///
    /// // The word is now so long that break_words kick in:
    /// let options = Options::new(5)
    ///     .word_splitter(WordSplitter::NoHyphenation);
    /// assert_eq!(wrap("foo-bar-baz", &options),
    ///            vec!["foo-b", "ar-ba", "z"]);
    ///
    /// // If you want to breaks at all, disable both:
    /// let options = Options::new(5)
    ///     .break_words(false)
    ///     .word_splitter(WordSplitter::NoHyphenation);
    /// assert_eq!(wrap("foo-bar-baz", &options),
    ///            vec!["foo-bar-baz"]);
    /// ```
    ///
    /// [`self.word_splitter`]: #structfield.word_splitter
    pub fn word_splitter(self, word_splitter: WordSplitter) -> Options<'a> {
        Options {
            width: self.width,
            line_ending: self.line_ending,
            initial_indent: self.initial_indent,
            subsequent_indent: self.subsequent_indent,
            break_words: self.break_words,
            word_separator: self.word_separator,
            wrap_algorithm: self.wrap_algorithm,
            word_splitter,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn options_agree_with_usize() {
        let opt_usize = Options::from(42_usize);
        let opt_options = Options::new(42);
        assert_eq!(opt_usize.width, opt_options.width);
        assert_eq!(opt_usize.initial_indent, opt_options.initial_indent);
        assert_eq!(opt_usize.subsequent_indent, opt_options.subsequent_indent);
        assert_eq!(opt_usize.break_words, opt_options.break_words);
        assert_eq!(
            opt_usize.word_splitter.split_points("hello-world"), opt_options
            .word_splitter.split_points("hello-world")
        );
    }
}
#[cfg(test)]
mod tests_rug_46 {
    use super::*;
    use crate::{LineEnding, Options, WordSplitter, WordSeparator, WrapAlgorithm};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: usize = rug_fuzz_0;
        Options::new(p0);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_47 {
    use super::*;
    use crate::refill;
    use crate::line_ending::LineEnding;
    use crate::options::Options;
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Options = Options::<'static>::new(rug_fuzz_0);
        let mut p1 = LineEnding::CRLF;
        p0.line_ending(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_48 {
    use super::*;
    use crate::Options;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_48_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "    ";
        let mut p0: Options<'static> = Options::new(rug_fuzz_0);
        let mut p1: &str = rug_fuzz_1;
        p0 = p0.initial_indent(p1);
        let _rug_ed_tests_rug_48_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_49 {
    use super::*;
    use crate::options::Options;
    use crate::wrap;
    #[test]
    fn test_subsequent_indent() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(usize, &str, &str, &str) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let options = Options::new(rug_fuzz_0)
            .initial_indent(rug_fuzz_1)
            .subsequent_indent(rug_fuzz_2);
        let text = rug_fuzz_3;
        let wrapped_text = wrap(text, options.clone());
        #[cfg(feature = "smawk")]
        debug_assert_eq!(wrapped_text, vec!["* This is", "  a little", "  example."]);
        #[cfg(not(feature = "smawk"))]
        debug_assert_eq!(wrapped_text, vec!["* This is a", "  little", "  example."]);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_50 {
    use super::*;
    use crate::{wrap, Options};
    #[test]
    fn test_rug() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(usize, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0: Options<'static> = Options::new(rug_fuzz_0);
        let mut p1: bool = rug_fuzz_1;
        p0.break_words(p1);
             }
}
}
}    }
}
#[cfg(test)]
mod tests_rug_53 {
    use super::*;
    use crate::{Options, WordSplitter};
    #[test]
    fn test_word_splitter() {

    extern crate arbitrary;
    if let Ok(folder) = std::env::var("FUZZ_CORPUS"){
                for f in std::fs::read_dir(folder).unwrap(){
                    if let Ok(corpus) = f{
                        let rug_data: &[u8] = &std::fs::read(corpus.path()).unwrap();
            if let Ok((mut rug_fuzz_0)) = <(usize) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Options<'static> = Options::new(rug_fuzz_0);
        let p1: WordSplitter = WordSplitter::NoHyphenation;
        Options::word_splitter(p0, p1);
             }
}
}
}    }
}
