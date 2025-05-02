//! The simplest cache. This should eventually offer line-oriented access
//! to the remote document, and can be used as a building block for more
//! complicated caching schemes.
use memchr::memchr;
use crate::xi_core::plugin_rpc::{GetDataResponse, TextUnit};
use xi_rope::interval::IntervalBounds;
use xi_rope::{DeltaElement, Interval, LinesMetric, Rope, RopeDelta};
use xi_trace::trace_block;
use super::{Cache, DataSource, Error};
#[cfg(not(test))]
const CHUNK_SIZE: usize = 1024 * 1024;
#[cfg(test)]
const CHUNK_SIZE: usize = 16;
/// A simple cache, holding a single contiguous chunk of the document.
#[derive(Debug, Clone, Default)]
pub struct ChunkCache {
    /// The position of this chunk relative to the tracked document.
    /// All offsets are guaranteed to be valid UTF-8 character boundaries.
    pub offset: usize,
    /// A chunk of the remote buffer.
    pub contents: String,
    /// The (zero-based) line number of the line containing the start of the chunk.
    pub first_line: usize,
    /// The byte offset of the start of the chunk from the start of `first_line`.
    /// If this chunk starts at a line break, this will be 0.
    pub first_line_offset: usize,
    /// A list of indexes of newlines in this chunk.
    pub line_offsets: Vec<usize>,
    /// The total size of the tracked document.
    pub buf_size: usize,
    pub num_lines: usize,
    pub rev: u64,
}
impl Cache for ChunkCache {
    fn new(buf_size: usize, rev: u64, num_lines: usize) -> Self {
        let mut new = Self::default();
        new.buf_size = buf_size;
        new.num_lines = num_lines;
        new.rev = rev;
        new
    }
    /// Returns the line at `line_num` (zero-indexed). Returns an `Err(_)` if
    /// there is a problem connecting to the peer, or if the requested line
    /// is out of bounds.
    ///
    /// The `source` argument is some type that implements [`DataSource`]; in
    /// the general case this is backed by the remote peer.
    ///
    /// # Errors
    ///
    /// Returns an error if `line_num` is greater than the total number of lines
    /// in the document, or if there is a problem communicating with `source`.
    ///
    /// [`DataSource`]: trait.DataSource.html
    fn get_line<DS>(&mut self, source: &DS, line_num: usize) -> Result<&str, Error>
    where
        DS: DataSource,
    {
        if line_num >= self.num_lines {
            return Err(Error::BadRequest);
        }
        if self.contents.is_empty() || line_num < self.first_line
            || (line_num == self.first_line && self.first_line_offset > 0)
            || (line_num > self.first_line + self.line_offsets.len())
        {
            let resp = source.get_data(line_num, TextUnit::Line, CHUNK_SIZE, self.rev)?;
            self.reset_chunk(resp);
        }
        let mut start_off = self.cached_offset_of_line(line_num).unwrap() - self.offset;
        loop {
            if let Some(end_off) = self.cached_offset_of_line(line_num + 1) {
                return Ok(&self.contents[start_off..end_off - self.offset]);
            }
            if start_off != 0 {
                self.clear_up_to(start_off);
                start_off = 0;
            }
            let chunk_end = self.offset + self.contents.len();
            let resp = source.get_data(chunk_end, TextUnit::Utf8, CHUNK_SIZE, self.rev)?;
            self.append_chunk(&resp);
        }
    }
    fn get_region<DS, I>(&mut self, source: &DS, interval: I) -> Result<&str, Error>
    where
        DS: DataSource,
        I: IntervalBounds,
    {
        let Interval { start, end } = interval.into_interval(self.buf_size);
        if self.contents.is_empty() || start < self.offset
            || start >= self.offset + self.contents.len()
        {
            let resp = source.get_data(start, TextUnit::Utf8, CHUNK_SIZE, self.rev)?;
            self.reset_chunk(resp);
        }
        loop {
            let start_off = start - self.offset;
            let end_off = end - self.offset;
            if end_off <= self.contents.len() {
                return Ok(&self.contents[start_off..end_off]);
            }
            if start_off != 0 {
                self.clear_up_to(start_off);
            }
            let chunk_end = self.offset + self.contents.len();
            let resp = source.get_data(chunk_end, TextUnit::Utf8, CHUNK_SIZE, self.rev)?;
            self.append_chunk(&resp);
        }
    }
    fn get_document<DS: DataSource>(&mut self, source: &DS) -> Result<String, Error> {
        let mut result = String::new();
        let mut cur_idx = 0;
        while cur_idx < self.buf_size {
            if self.contents.is_empty() || cur_idx != self.offset {
                let resp = source
                    .get_data(cur_idx, TextUnit::Utf8, CHUNK_SIZE, self.rev)?;
                self.reset_chunk(resp);
            }
            result.push_str(&self.contents);
            cur_idx = self.offset + self.contents.len();
        }
        Ok(result)
    }
    fn offset_of_line<DS: DataSource>(
        &mut self,
        source: &DS,
        line_num: usize,
    ) -> Result<usize, Error> {
        if line_num > self.num_lines {
            return Err(Error::BadRequest);
        }
        match self.cached_offset_of_line(line_num) {
            Some(offset) => Ok(offset),
            None => {
                let resp = source
                    .get_data(line_num, TextUnit::Line, CHUNK_SIZE, self.rev)?;
                self.reset_chunk(resp);
                self.offset_of_line(source, line_num)
            }
        }
    }
    fn line_of_offset<DS: DataSource>(
        &mut self,
        source: &DS,
        offset: usize,
    ) -> Result<usize, Error> {
        if offset > self.buf_size {
            return Err(Error::BadRequest);
        }
        if self.contents.is_empty() || offset < self.offset
            || offset > self.offset + self.contents.len()
        {
            let resp = source.get_data(offset, TextUnit::Utf8, CHUNK_SIZE, self.rev)?;
            self.reset_chunk(resp);
        }
        let rel_offset = offset - self.offset;
        let line_num = match self.line_offsets.binary_search(&rel_offset) {
            Ok(ix) => ix + self.first_line + 1,
            Err(ix) => ix + self.first_line,
        };
        Ok(line_num)
    }
    /// Updates the chunk to reflect changes in this delta.
    fn update(
        &mut self,
        delta: Option<&RopeDelta>,
        new_len: usize,
        num_lines: usize,
        rev: u64,
    ) {
        let _t = trace_block("ChunkCache::update", &["plugin"]);
        let is_empty = self.offset == 0 && self.contents.is_empty();
        let should_clear = match delta {
            Some(delta) if !is_empty => self.should_clear(delta),
            Some(_) => true,
            None => true,
        };
        if should_clear {
            self.clear();
        } else {
            self.update_chunk(delta.unwrap());
        }
        self.buf_size = new_len;
        self.num_lines = num_lines;
        self.rev = rev;
    }
    fn clear(&mut self) {
        self.contents.clear();
        self.offset = 0;
        self.line_offsets.clear();
        self.first_line = 0;
        self.first_line_offset = 0;
    }
}
impl ChunkCache {
    /// Returns the offset of the provided `line_num` if it can be determined
    /// without fetching data. The offset of line 0 is always 0, and there
    /// is an implicit line at the last offset in the buffer.
    fn cached_offset_of_line(&self, line_num: usize) -> Option<usize> {
        if line_num < self.first_line {
            return None;
        }
        let rel_line_num = line_num - self.first_line;
        if rel_line_num == 0 {
            return Some(self.offset - self.first_line_offset);
        }
        if rel_line_num <= self.line_offsets.len() {
            return Some(self.offset + self.line_offsets[rel_line_num - 1]);
        }
        if line_num == self.num_lines
            && self.offset + self.contents.len() == self.buf_size
        {
            return Some(self.offset + self.contents.len());
        }
        None
    }
    /// Clears anything in the cache up to `offset`, which is indexed relative
    /// to `self.contents`.
    ///
    /// # Panics
    ///
    /// Panics if `offset` is not a character boundary, or if `offset` is greater than
    /// the length of `self.content`.
    fn clear_up_to(&mut self, offset: usize) {
        if offset > self.contents.len() {
            panic!(
                "offset greater than content length: {} > {}", offset, self.contents
                .len()
            )
        }
        let new_contents = self.contents.split_off(offset);
        self.contents = new_contents;
        self.offset += offset;
        let (new_line, new_line_off) = match self.line_offsets.binary_search(&offset) {
            Ok(idx) => (self.first_line + idx + 1, 0),
            Err(0) => (self.first_line, self.first_line_offset + offset),
            Err(idx) => (self.first_line + idx, offset - self.line_offsets[idx - 1]),
        };
        self
            .line_offsets = self
            .line_offsets
            .iter()
            .filter(|i| **i > offset)
            .map(|i| i - offset)
            .collect();
        self.first_line = new_line;
        self.first_line_offset = new_line_off;
    }
    /// Discard any existing cache, starting again with the new data.
    fn reset_chunk(&mut self, data: GetDataResponse) {
        self.contents = data.chunk;
        self.offset = data.offset;
        self.first_line = data.first_line;
        self.first_line_offset = data.first_line_offset;
        self.recalculate_line_offsets();
    }
    /// Append to the existing cache, leaving existing data in place.
    fn append_chunk(&mut self, data: &GetDataResponse) {
        self.contents.push_str(data.chunk.as_str());
        self.recalculate_line_offsets();
    }
    fn recalculate_line_offsets(&mut self) {
        self.line_offsets.clear();
        newline_offsets(&self.contents, &mut self.line_offsets);
    }
    /// Determine whether we should update our state with this delta,
    /// or if we should clear it. In the update case, also patches up
    /// offsets.
    fn should_clear(&mut self, delta: &RopeDelta) -> bool {
        let (iv, _) = delta.summary();
        let start = iv.start();
        let end = iv.end();
        if start < self.offset || start > self.offset + self.contents.len() {
            true
        } else if delta.is_simple_delete() {
            let end = end.min(self.offset + self.contents.len());
            self.simple_delete(start, end);
            false
        } else if let Some(text) = delta.as_simple_insert() {
            assert_eq!(iv.size(), 0);
            self.simple_insert(text, start);
            false
        } else {
            true
        }
    }
    /// Patches up `self.line_offsets` in the simple insert case.
    fn simple_insert(&mut self, text: &Rope, ins_offset: usize) {
        let has_newline = text.measure::<LinesMetric>() > 0;
        let self_off = self.offset;
        assert!(ins_offset >= self_off);
        self.line_offsets
            .iter_mut()
            .for_each(|off| {
                if *off > ins_offset - self_off {
                    *off += text.len();
                }
            });
        if has_newline {
            let mut new_offsets = Vec::new();
            newline_offsets(&String::from(text), &mut new_offsets);
            new_offsets.iter_mut().for_each(|off| *off += ins_offset - self_off);
            let split_idx = self
                .line_offsets
                .binary_search(&new_offsets[0])
                .err()
                .expect("new index cannot be occupied");
            self
                .line_offsets = [
                &self.line_offsets[..split_idx],
                &new_offsets,
                &self.line_offsets[split_idx..],
            ]
                .concat();
        }
    }
    /// Patches up `self.line_offsets` in the simple delete case.
    fn simple_delete(&mut self, start: usize, end: usize) {
        let del_size = end - start;
        let start = start - self.offset;
        let end = end - self.offset;
        let has_newline = memchr(b'\n', &self.contents.as_bytes()[start..end]).is_some();
        if has_newline {
            self
                .line_offsets = self
                .line_offsets
                .iter()
                .filter_map(|off| match *off {
                    x if x <= start => Some(x),
                    x if x > start && x <= end => None,
                    x if x > end => Some(x - del_size),
                    hmm => panic!("invariant violated {} {} {}?", start, end, hmm),
                })
                .collect();
        } else {
            self.line_offsets
                .iter_mut()
                .for_each(|off| {
                    if *off >= end {
                        *off -= del_size;
                    }
                });
        }
    }
    /// Updates `self.contents` with the given delta.
    fn update_chunk(&mut self, delta: &RopeDelta) {
        let chunk_start = self.offset;
        let chunk_end = chunk_start + self.contents.len();
        let mut new_state = String::with_capacity(self.contents.len());
        let mut prev_copy_end = 0;
        let mut del_before: usize = 0;
        let mut ins_before: usize = 0;
        for op in delta.els.as_slice() {
            match *op {
                DeltaElement::Copy(start, end) => {
                    if start < chunk_start {
                        del_before += start - prev_copy_end;
                        if end >= chunk_start {
                            let cp_end = (end - chunk_start).min(self.contents.len());
                            new_state.push_str(&self.contents[0..cp_end]);
                        }
                    } else if start <= chunk_end {
                        if prev_copy_end < chunk_start {
                            del_before += chunk_start - prev_copy_end;
                        }
                        let cp_start = start - chunk_start;
                        let cp_end = (end - chunk_start).min(self.contents.len());
                        new_state.push_str(&self.contents[cp_start..cp_end]);
                    }
                    prev_copy_end = end;
                }
                DeltaElement::Insert(ref s) => {
                    if prev_copy_end < chunk_start {
                        ins_before += s.len();
                    } else if prev_copy_end <= chunk_end {
                        let s: String = s.into();
                        new_state.push_str(&s);
                    }
                }
            }
        }
        self.offset += ins_before;
        self.offset -= del_before;
        self.contents = new_state;
    }
}
/// Calculates the offsets of newlines in `text`,
/// inserting the results into `storage`. The offsets are the offset
/// of the start of the line, not the line break character.
fn newline_offsets(text: &str, storage: &mut Vec<usize>) {
    let mut cur_idx = 0;
    while let Some(idx) = memchr(b'\n', &text.as_bytes()[cur_idx..]) {
        storage.push(cur_idx + idx + 1);
        cur_idx += idx + 1;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::xi_core::plugin_rpc::GetDataResponse;
    use xi_rope::delta::Delta;
    use xi_rope::interval::Interval;
    use xi_rope::rope::{LinesMetric, Rope};
    struct MockDataSource(Rope);
    impl DataSource for MockDataSource {
        fn get_data(
            &self,
            start: usize,
            unit: TextUnit,
            _max_size: usize,
            _rev: u64,
        ) -> Result<GetDataResponse, Error> {
            let offset = unit
                .resolve_offset(&self.0, start)
                .ok_or(Error::Other("unable to resolve offset".into()))?;
            let first_line = self.0.line_of_offset(offset);
            let first_line_offset = offset - self.0.offset_of_line(first_line);
            let end_off = (offset + CHUNK_SIZE).min(self.0.len());
            if offset > self.0.len() {
                Err(Error::Other("offset too big".into()))
            } else {
                let chunk = self.0.slice_to_cow(offset..end_off).into_owned();
                Ok(GetDataResponse {
                    chunk,
                    offset,
                    first_line,
                    first_line_offset,
                })
            }
        }
    }
    #[test]
    fn simple_chunk() {
        let mut c = ChunkCache::default();
        c.buf_size = 2;
        c.contents = "oh".into();
        let d = Delta::simple_edit(Interval::new(0, 0), "yay".into(), c.contents.len());
        c.update(Some(&d), d.new_document_len(), 1, 1);
        assert_eq!(& c.contents, "yayoh");
        assert_eq!(c.offset, 0);
        let d = Delta::simple_edit(Interval::new(0, 0), "ahh".into(), c.contents.len());
        c.update(Some(&d), d.new_document_len(), 1, 2);
        assert_eq!(& c.contents, "ahhyayoh");
        assert_eq!(c.offset, 0);
        let d = Delta::simple_edit(
            Interval::new(2, 2),
            "_oops_".into(),
            c.contents.len(),
        );
        assert_eq!(d.els.len(), 3);
        c.update(Some(&d), d.new_document_len(), 1, 3);
        assert_eq!(& c.contents, "ah_oops_hyayoh");
        assert_eq!(c.offset, 0);
        let d = Delta::simple_edit(Interval::new(9, 9), "fin".into(), c.contents.len());
        c.update(Some(&d), d.new_document_len(), 1, 5);
        assert_eq!(& c.contents, "ah_oops_hfinyayoh");
        assert_eq!(c.offset, 0);
    }
    #[test]
    fn get_lines() {
        let remote_document = MockDataSource("this\nhas\nfour\nlines!".into());
        let mut c = ChunkCache::default();
        c.buf_size = remote_document.0.len();
        c.num_lines = remote_document.0.measure::<LinesMetric>() + 1;
        assert_eq!(c.num_lines, 4);
        assert_eq!(c.buf_size, 20);
        assert_eq!(c.line_offsets.len(), 0);
        assert_eq!(c.get_line(& remote_document, 0).ok(), Some("this\n"));
        assert_eq!(c.line_offsets.len(), 3);
        assert_eq!(c.offset, 0);
        assert_eq!(c.buf_size, 20);
        assert_eq!(c.contents.len(), 16);
        assert_eq!(c.get_line(& remote_document, 2).ok(), Some("four\n"));
        assert_eq!(c.cached_offset_of_line(3), Some(14));
        assert_eq!(c.cached_offset_of_line(4), None);
        assert_eq!(c.get_line(& remote_document, 3).ok(), Some("lines!"));
        assert!(c.get_line(& remote_document, 4).is_err());
    }
    #[test]
    fn get_region() {
        let remote_document = MockDataSource(
            "but\nthis big fella\nhas\nFIVE\nlines!".into(),
        );
        let mut c = ChunkCache::default();
        c.buf_size = remote_document.0.len();
        c.num_lines = remote_document.0.measure::<LinesMetric>() + 1;
        assert_eq!(c.get_region(& remote_document, ..3).ok(), Some("but"));
        assert_eq!(c.get_region(& remote_document, 28..).ok(), Some("lines!"));
        assert!(c.offset > 0);
        assert_eq!(
            c.get_region(& remote_document, ..).ok(),
            Some("but\nthis big fella\nhas\nFIVE\nlines!")
        );
    }
    #[test]
    fn reset_chunk() {
        let data = GetDataResponse {
            chunk: "1\n2\n3\n4\n5\n6\n7".into(),
            offset: 0,
            first_line: 0,
            first_line_offset: 0,
        };
        let mut cache = ChunkCache::default();
        cache.reset_chunk(data);
        assert_eq!(cache.line_offsets.len(), 6);
        assert_eq!(cache.line_offsets, vec![2, 4, 6, 8, 10, 12]);
        let idx_1 = cache.cached_offset_of_line(1).unwrap();
        let idx_2 = cache.cached_offset_of_line(2).unwrap();
        assert_eq!(& cache.contents.as_str() [idx_1..idx_2], "2\n");
    }
    #[test]
    fn clear_up_to() {
        let mut c = ChunkCache::default();
        let data = GetDataResponse {
            chunk: "this\n has a newline at idx 4\nand at idx 28".into(),
            offset: 0,
            first_line: 0,
            first_line_offset: 0,
        };
        c.reset_chunk(data);
        assert_eq!(c.line_offsets, vec![5, 29]);
        c.clear_up_to(5);
        assert_eq!(c.offset, 5);
        assert_eq!(c.first_line, 1);
        assert_eq!(c.first_line_offset, 0);
        assert_eq!(c.line_offsets, vec![24]);
        c.clear_up_to(10);
        assert_eq!(c.offset, 15);
        assert_eq!(c.first_line, 1);
        assert_eq!(c.first_line_offset, 10);
        assert_eq!(c.line_offsets, vec![14]);
    }
    #[test]
    fn simple_insert() {
        let mut c = ChunkCache::default();
        c.contents = "some".into();
        c.buf_size = 4;
        let d = Delta::simple_edit(
            Interval::new(0, 0),
            "two\nline\nbreaks".into(),
            c.contents.len(),
        );
        assert!(d.as_simple_insert().is_some());
        assert!(! d.is_simple_delete());
        c.update(Some(&d), d.new_document_len(), 3, 1);
        assert_eq!(c.line_offsets, vec![4, 9]);
        let d = Delta::simple_edit(
            Interval::new(4, 4),
            "one\nmore".into(),
            c.contents.len(),
        );
        assert!(d.as_simple_insert().is_some());
        c.update(Some(&d), d.new_document_len(), 4, 2);
        assert_eq!(& c.contents, "two\none\nmoreline\nbreakssome");
        assert_eq!(c.line_offsets, vec![4, 8, 17]);
    }
    #[test]
    fn offset_of_line() {
        let source = MockDataSource("this\nhas\nfour\nlines!".into());
        let mut c = ChunkCache::default();
        c.buf_size = source.0.len();
        c.num_lines = source.0.measure::<LinesMetric>() + 1;
        assert_eq!(c.num_lines, 4);
        assert_eq!(c.cached_offset_of_line(0), Some(0));
        assert_eq!(c.offset_of_line(& source, 0).unwrap(), 0);
        assert_eq!(c.offset_of_line(& source, 1).unwrap(), 5);
        assert_eq!(c.offset_of_line(& source, 2).unwrap(), 9);
        assert_eq!(c.offset_of_line(& source, 3).unwrap(), 14);
    }
    #[test]
    fn cached_offset_of_line() {
        let data = GetDataResponse {
            chunk: "zer\none\ntwo\ntri".into(),
            offset: 0,
            first_line: 0,
            first_line_offset: 0,
        };
        assert_eq!(Rope::from(& data.chunk).measure::< LinesMetric > () + 1, 4);
        let mut c = ChunkCache::default();
        c.buf_size = data.chunk.len();
        c.num_lines = 4;
        c.reset_chunk(data);
        assert_eq!(& c.contents, "zer\none\ntwo\ntri");
        assert_eq!(& c.line_offsets, & [4, 8, 12]);
        assert_eq!(c.cached_offset_of_line(0), Some(0));
        assert_eq!(c.cached_offset_of_line(1), Some(4));
        assert_eq!(c.cached_offset_of_line(2), Some(8));
        assert_eq!(c.cached_offset_of_line(3), Some(12));
        assert_eq!(c.cached_offset_of_line(4), Some(15));
        assert_eq!(c.cached_offset_of_line(5), None);
        let delta = Delta::simple_edit(Interval::new(3, 4), "".into(), c.buf_size);
        assert!(delta.is_simple_delete());
        c.update(Some(&delta), delta.new_document_len(), 3, 1);
        assert_eq!(& c.contents, "zerone\ntwo\ntri");
        assert_eq!(& c.line_offsets, & [7, 11]);
        assert_eq!(c.cached_offset_of_line(0), Some(0));
        assert_eq!(c.cached_offset_of_line(1), Some(7));
        assert_eq!(c.cached_offset_of_line(2), Some(11));
        assert_eq!(c.cached_offset_of_line(3), Some(14));
        assert_eq!(c.cached_offset_of_line(4), None);
    }
    #[test]
    fn simple_delete() {
        let data = GetDataResponse {
            chunk: "zer\none\ntwo\ntri".into(),
            offset: 0,
            first_line: 0,
            first_line_offset: 0,
        };
        assert_eq!(Rope::from(& data.chunk).measure::< LinesMetric > () + 1, 4);
        let mut c = ChunkCache::default();
        c.buf_size = data.chunk.len();
        c.num_lines = 4;
        c.reset_chunk(data);
        assert_eq!(& c.contents, "zer\none\ntwo\ntri");
        assert_eq!(& c.line_offsets, & [4, 8, 12]);
        let delta = Delta::simple_edit(Interval::new(3, 4), "".into(), c.buf_size);
        assert!(delta.is_simple_delete());
        let (iv, _) = delta.summary();
        let start = iv.start();
        let end = iv.end();
        assert_eq!((start, end), (3, 4));
        assert_eq!(c.offset, 0);
        c.simple_delete(start, end);
        assert_eq!(& c.line_offsets, & [7, 11]);
    }
    #[test]
    fn large_delete() {
        let large_str = "This string literal is larger than CHUNK_SIZE.";
        assert!(large_str.len() > CHUNK_SIZE);
        let data = GetDataResponse {
            chunk: large_str.split_at(CHUNK_SIZE).0.into(),
            offset: 0,
            first_line: 0,
            first_line_offset: 0,
        };
        let mut c = ChunkCache::default();
        c.reset_chunk(data);
        let delta = Delta::simple_edit(
            Interval::new(0, large_str.len()),
            "".into(),
            large_str.len(),
        );
        assert!(delta.is_simple_delete());
        c.update(Some(&delta), delta.new_document_len(), 1, 1);
    }
    #[test]
    fn simple_edits_with_offset() {
        let mut source = MockDataSource("this\nhas\nfour\nlines!".into());
        let mut c = ChunkCache::default();
        c.buf_size = source.0.len();
        c.num_lines = source.0.measure::<LinesMetric>() + 1;
        assert_eq!(c.get_line(& source, 2).ok(), Some("four\n"));
        assert_eq!(c.offset, 9);
        assert_eq!(& c.contents, "four\nlines!");
        assert_eq!(c.offset_of_line(& source, 3).unwrap(), 14);
        let d = Delta::simple_edit(
            Interval::new(10, 10),
            "ive nice\ns".into(),
            c.contents.len() + c.offset,
        );
        c.update(Some(&d), d.new_document_len(), 5, 1);
        source.0 = "this\nhas\nfive nice\nsour\nlines!".into();
        assert_eq!(& c.contents, "five nice\nsour\nlines!");
        assert_eq!(c.offset, 9);
        assert_eq!(c.offset_of_line(& source, 3).unwrap(), 19);
        assert_eq!(c.offset_of_line(& source, 4).unwrap(), 24);
        assert_eq!(c.offset_of_line(& source, 0).unwrap(), 0);
        assert_eq!(c.offset, 0);
        assert_eq!(& c.contents, "this\nhas\nfive ni");
        assert_eq!(c.offset_of_line(& source, 1).unwrap(), 5);
        assert_eq!(c.offset_of_line(& source, 3).unwrap(), 19);
        assert_eq!(c.offset_of_line(& source, 4).unwrap(), 24);
        let _ = c.offset_of_line(&source, 0);
        c.clear_up_to(5);
        assert_eq!(& c.contents, & "this\nhas\nfive nice\nsour\nlines!"[5..CHUNK_SIZE]);
        assert_eq!(c.offset, 5);
        assert_eq!(c.first_line, 1);
        let d = Delta::simple_edit(
            Interval::new(6, 10),
            "".into(),
            c.contents.len() + c.offset,
        );
        assert!(d.is_simple_delete());
        c.update(Some(&d), d.new_document_len(), 4, 1);
        source.0 = "this\nhive nice\nsour\nlines!".into();
        assert_eq!(c.offset, 5);
        assert_eq!(c.first_line, 1);
        assert_eq!(c.get_line(& source, 1).unwrap(), "hive nice\n");
        assert_eq!(c.offset_of_line(& source, 2).unwrap(), 15);
    }
    #[test]
    fn cache_offsets() {
        let mut c = ChunkCache::default();
        c.contents = "ring\nis\nour\ntotal\nbuffer".into();
        c.buf_size = c.contents.len() + 7;
        c.offset = 7;
        c.first_line = 1;
        c.first_line_offset = 2;
        c.recalculate_line_offsets();
        assert_eq!(c.cached_offset_of_line(2), Some(12));
        assert_eq!(c.cached_offset_of_line(3), Some(15));
        assert_eq!(c.cached_offset_of_line(0), None);
        assert_eq!(c.cached_offset_of_line(1), Some(5));
    }
    #[test]
    fn get_big_line() {
        let test_str = "this\nhas one big line in the middle\nwow, multi-fetch!\nyay!";
        let source = MockDataSource(test_str.into());
        let mut c = ChunkCache::default();
        c.buf_size = source.0.len();
        c.num_lines = source.0.measure::<LinesMetric>() + 1;
        assert_eq!(c.num_lines, 4);
        assert_eq!(c.get_line(& source, 0).unwrap(), "this\n");
        assert_eq!(c.contents, test_str[..CHUNK_SIZE]);
        assert_eq!(c.get_line(& source, 1).unwrap(), "has one big line in the middle\n");
        assert_eq!(c.contents, test_str[5..CHUNK_SIZE * 3]);
        assert_eq!(c.get_line(& source, 3).unwrap(), "yay!");
        assert_eq!(c.first_line, 3);
    }
    #[test]
    fn get_last_line() {
        let base_document = "\
            one\n\
            two\n
            three\n\
            four";
        let source = MockDataSource(base_document.into());
        let mut c = ChunkCache::default();
        let delta = Delta::simple_edit(Interval::new(0, 0), base_document.into(), 0);
        c.update(Some(&delta), base_document.len(), 4, 0);
        match c.get_line(&source, 4) {
            Err(Error::BadRequest) => {}
            other => assert!(false, "expected BadRequest, found {:?}", other),
        };
    }
    #[test]
    fn convert_lines_offsets() {
        let source = MockDataSource("this\nhas\nfour\nlines!".into());
        let mut c = ChunkCache::default();
        c.buf_size = source.0.len();
        c.num_lines = source.0.measure::<LinesMetric>() + 1;
        assert_eq!(c.line_of_offset(& source, 0).unwrap(), 0);
        assert_eq!(c.line_of_offset(& source, 1).unwrap(), 0);
        eprintln!("{:?} {} {}", c.line_offsets, c.offset, c.buf_size);
        assert_eq!(c.line_of_offset(& source, 4).unwrap(), 0);
        assert_eq!(c.line_of_offset(& source, 5).unwrap(), 1);
        assert_eq!(c.line_of_offset(& source, 8).unwrap(), 1);
        assert_eq!(c.line_of_offset(& source, 9).unwrap(), 2);
        assert_eq!(c.line_of_offset(& source, 13).unwrap(), 2);
        assert_eq!(c.line_of_offset(& source, 14).unwrap(), 3);
        assert_eq!(c.line_of_offset(& source, 18).unwrap(), 3);
        assert_eq!(c.line_of_offset(& source, 20).unwrap(), 3);
        assert!(c.line_of_offset(& source, 21).is_err());
        assert_eq!(c.offset_of_line(& source, 0).unwrap(), 0);
        assert_eq!(c.offset_of_line(& source, 1).unwrap(), 5);
        assert_eq!(c.offset_of_line(& source, 2).unwrap(), 9);
        assert_eq!(c.offset_of_line(& source, 3).unwrap(), 14);
        assert_eq!(c.offset_of_line(& source, 4).unwrap(), 20);
        assert!(c.offset_of_line(& source, 5).is_err());
    }
    #[test]
    fn get_line_regression() {
        let base_document = r#"fn main() {
    let one = "one";
    let two = "two";
}"#;
        let edited_document = r#"fn main() {
    let one = "one";
    let two = "two";}"#;
        let mut source = MockDataSource(base_document.into());
        let mut c = ChunkCache::default();
        let delta = Delta::simple_edit(Interval::new(0, 0), base_document.into(), 0);
        c.update(Some(&delta), base_document.len(), 4, 0);
        assert_eq!(c.get_line(& source, 0).unwrap(), "fn main() {\n");
        assert_eq!(c.get_line(& source, 1).unwrap(), "    let one = \"one\";\n");
        assert_eq!(c.get_line(& source, 2).unwrap(), "    let two = \"two\";\n");
        assert_eq!(c.get_line(& source, 3).unwrap(), "}");
        let delta = Delta::simple_edit(Interval::new(53, 54), "".into(), c.buf_size);
        c.update(Some(&delta), base_document.len() - 1, 3, 1);
        source.0 = edited_document.into();
        assert_eq!(c.get_line(& source, 0).unwrap(), "fn main() {\n");
        assert_eq!(c.get_line(& source, 1).unwrap(), "    let one = \"one\";\n");
        assert_eq!(c.get_line(& source, 2).unwrap(), "    let two = \"two\";}");
        assert!(c.get_line(& source, 3).is_err());
    }
}
#[cfg(test)]
mod tests_llm_16_15 {
    use super::*;
    use crate::*;
    use crate::base_cache::DataSource;
    struct MockDataSource;
    impl DataSource for MockDataSource {
        fn get_data(
            &self,
            _: usize,
            _: TextUnit,
            _: usize,
            _: u64,
        ) -> Result<GetDataResponse, Error> {
            unimplemented!()
        }
    }
    #[test]
    fn test_update_should_clear_cache_with_empty_delta_if_cache_empty() {
        let _rug_st_tests_llm_16_15_rrrruuuugggg_test_update_should_clear_cache_with_empty_delta_if_cache_empty = 0;
        let rug_fuzz_0 = 10;
        let rug_fuzz_1 = 5;
        let rug_fuzz_2 = 100;
        let mut cache = ChunkCache::default();
        let delta = None;
        let new_len = rug_fuzz_0;
        let num_lines = rug_fuzz_1;
        let rev = rug_fuzz_2;
        cache.update(delta, new_len, num_lines, rev);
        let _rug_ed_tests_llm_16_15_rrrruuuugggg_test_update_should_clear_cache_with_empty_delta_if_cache_empty = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_50 {
    use super::*;
    use crate::*;
    use crate::RopeDelta;
    struct MockDataSource;
    impl DataSource for MockDataSource {
        fn get_data(
            &self,
            _offset: usize,
            _unit: TextUnit,
            _size: usize,
            _rev: u64,
        ) -> Result<GetDataResponse, Error> {
            unimplemented!()
        }
    }
    #[test]
    fn test_append_chunk() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_append_chunk = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "Hello, World!";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = "Hello, ";
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 0;
        let mut cache = ChunkCache::default();
        let data = GetDataResponse {
            offset: rug_fuzz_0,
            chunk: String::from(rug_fuzz_1),
            first_line: rug_fuzz_2,
            first_line_offset: rug_fuzz_3,
        };
        cache.append_chunk(&data);
        debug_assert_eq!(cache.contents, "Hello, World!");
        let new_data = GetDataResponse {
            offset: rug_fuzz_4,
            chunk: String::from(rug_fuzz_5),
            first_line: rug_fuzz_6,
            first_line_offset: rug_fuzz_7,
        };
        cache.append_chunk(&new_data);
        debug_assert_eq!(cache.contents, "Hello, World!Hello, ");
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_append_chunk = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;
    use crate::*;
    use crate::base_cache::{DataSource, GetDataResponse, Error, TextUnit, CHUNK_SIZE};
    use crate::base_cache::ChunkCache;
    struct DummyDataSource;
    impl DataSource for DummyDataSource {
        fn get_data(
            &self,
            line_num: usize,
            unit: TextUnit,
            size: usize,
            rev: u64,
        ) -> Result<GetDataResponse, Error> {
            unimplemented!()
        }
    }
    #[test]
    fn test_cached_offset_of_line() {
        let _rug_st_tests_llm_16_52_rrrruuuugggg_test_cached_offset_of_line = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 1;
        let rug_fuzz_5 = 10;
        let rug_fuzz_6 = 2;
        let rug_fuzz_7 = 1;
        let rug_fuzz_8 = 5;
        let rug_fuzz_9 = 2;
        let rug_fuzz_10 = 4;
        let rug_fuzz_11 = "test";
        let rug_fuzz_12 = 14;
        let rug_fuzz_13 = 4;
        let rug_fuzz_14 = 10;
        let mut cache = ChunkCache::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let line_num_1 = rug_fuzz_3;
        let result_1 = cache.cached_offset_of_line(line_num_1);
        debug_assert_eq!(result_1, None);
        cache.first_line = rug_fuzz_4;
        cache.offset = rug_fuzz_5;
        cache.first_line_offset = rug_fuzz_6;
        let line_num_2 = rug_fuzz_7;
        let result_2 = cache.cached_offset_of_line(line_num_2);
        debug_assert_eq!(result_2, Some(8));
        cache.line_offsets = vec![rug_fuzz_8, 10, 15, 20];
        let line_num_3 = rug_fuzz_9;
        let result_3 = cache.cached_offset_of_line(line_num_3);
        debug_assert_eq!(result_3, Some(25));
        cache.num_lines = rug_fuzz_10;
        cache.contents = rug_fuzz_11.to_string();
        cache.buf_size = rug_fuzz_12;
        let line_num_4 = rug_fuzz_13;
        let result_4 = cache.cached_offset_of_line(line_num_4);
        debug_assert_eq!(result_4, Some(14));
        let line_num_5 = rug_fuzz_14;
        let result_5 = cache.cached_offset_of_line(line_num_5);
        debug_assert_eq!(result_5, None);
        let _rug_ed_tests_llm_16_52_rrrruuuugggg_test_cached_offset_of_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_55 {
    use super::*;
    use crate::*;
    #[test]
    fn test_recalculate_line_offsets() {
        let _rug_st_tests_llm_16_55_rrrruuuugggg_test_recalculate_line_offsets = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "Hello\nWorld\n";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 13;
        let rug_fuzz_5 = 2;
        let rug_fuzz_6 = 0;
        let mut cache = ChunkCache {
            offset: rug_fuzz_0,
            contents: String::from(rug_fuzz_1),
            first_line: rug_fuzz_2,
            first_line_offset: rug_fuzz_3,
            line_offsets: Vec::new(),
            buf_size: rug_fuzz_4,
            num_lines: rug_fuzz_5,
            rev: rug_fuzz_6,
        };
        cache.recalculate_line_offsets();
        debug_assert_eq!(cache.line_offsets, vec![5, 11]);
        let _rug_ed_tests_llm_16_55_rrrruuuugggg_test_recalculate_line_offsets = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_60 {
    use super::*;
    use crate::*;
    struct MockDataSource;
    impl DataSource for MockDataSource {
        fn get_data(
            &self,
            offset: usize,
            unit: TextUnit,
            size: usize,
            rev: u64,
        ) -> Result<GetDataResponse, Error> {
            unimplemented!()
        }
    }
    #[test]
    fn test_simple_delete() {
        let _rug_st_tests_llm_16_60_rrrruuuugggg_test_simple_delete = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "Hello, World!";
        let rug_fuzz_2 = 0;
        let rug_fuzz_3 = 0;
        let rug_fuzz_4 = 0;
        let rug_fuzz_5 = 0;
        let rug_fuzz_6 = 0;
        let rug_fuzz_7 = 2;
        let rug_fuzz_8 = 7;
        let mut cache = ChunkCache {
            offset: rug_fuzz_0,
            contents: rug_fuzz_1.to_string(),
            first_line: rug_fuzz_2,
            first_line_offset: rug_fuzz_3,
            line_offsets: vec![],
            buf_size: rug_fuzz_4,
            num_lines: rug_fuzz_5,
            rev: rug_fuzz_6,
        };
        cache.simple_delete(rug_fuzz_7, rug_fuzz_8);
        debug_assert_eq!(cache.contents, "Horld!");
        let _rug_ed_tests_llm_16_60_rrrruuuugggg_test_simple_delete = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;
    use crate::*;
    use crate::base_cache::{
        Cache, ChunkCache, GetDataResponse, DataSource, RopeDelta, LinesMetric, Interval,
        IntervalBounds, TextUnit, Error,
    };
    struct MockDataSource;
    impl DataSource for MockDataSource {
        fn get_data(
            &self,
            _offset: usize,
            _unit: TextUnit,
            _size: usize,
            _rev: u64,
        ) -> Result<GetDataResponse, Error> {
            unimplemented!()
        }
    }
    #[test]
    fn test_simple_insert() {
        let _rug_st_tests_llm_16_61_rrrruuuugggg_test_simple_insert = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = "Hello, world!";
        let rug_fuzz_4 = 6;
        let mut cache = ChunkCache::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let text = Rope::from(rug_fuzz_3);
        let ins_offset = rug_fuzz_4;
        cache.simple_insert(&text, ins_offset);
        let _rug_ed_tests_llm_16_61_rrrruuuugggg_test_simple_insert = 0;
    }
    #[test]
    fn test_simple_insert_with_newline() {
        let _rug_st_tests_llm_16_61_rrrruuuugggg_test_simple_insert_with_newline = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = 10;
        let rug_fuzz_3 = "Hello, world!\nThis is a test.";
        let rug_fuzz_4 = 6;
        let mut cache = ChunkCache::new(rug_fuzz_0, rug_fuzz_1, rug_fuzz_2);
        let text = Rope::from(rug_fuzz_3);
        let ins_offset = rug_fuzz_4;
        cache.simple_insert(&text, ins_offset);
        let _rug_ed_tests_llm_16_61_rrrruuuugggg_test_simple_insert_with_newline = 0;
    }
}
