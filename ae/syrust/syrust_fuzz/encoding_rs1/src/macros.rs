macro_rules! decoder_function {
    (
        $preamble:block, $loop_preable:block, $eof:block, $body:block, $slf:ident,
        $src_consumed:ident, $dest:ident, $source:ident, $b:ident,
        $destination_handle:ident, $unread_handle:ident, $destination_check:ident,
        $name:ident, $code_unit:ty, $dest_struct:ident
    ) => {
        pub fn $name (& mut $slf, src : & [u8], dst : & mut [$code_unit], last : bool) ->
        (DecoderResult, usize, usize) { let mut $source = ByteSource::new(src); let mut
        $dest = $dest_struct ::new(dst); loop { { $preamble } loop { { $loop_preable }
        match $source .check_available() { Space::Full($src_consumed) => { if last { $eof
        } return (DecoderResult::InputEmpty, $src_consumed, $dest .written()); }
        Space::Available(source_handle) => { match $dest .$destination_check () {
        Space::Full(dst_written) => { return (DecoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available($destination_handle) => { let ($b,
        $unread_handle) = source_handle.read(); $body } } } } } } }
    };
}
macro_rules! decoder_functions {
    (
        $preamble:block, $loop_preable:block, $eof:block, $body:block, $slf:ident,
        $src_consumed:ident, $dest:ident, $source:ident, $b:ident,
        $destination_handle:ident, $unread_handle:ident, $destination_check:ident
    ) => {
        decoder_function!($preamble, $loop_preable, $eof, $body, $slf, $src_consumed,
        $dest, $source, $b, $destination_handle, $unread_handle, $destination_check,
        decode_to_utf8_raw, u8, Utf8Destination); decoder_function!($preamble,
        $loop_preable, $eof, $body, $slf, $src_consumed, $dest, $source, $b,
        $destination_handle, $unread_handle, $destination_check, decode_to_utf16_raw,
        u16, Utf16Destination);
    };
}
macro_rules! ascii_compatible_two_byte_decoder_function {
    (
        $lead:block, $trail:block, $slf:ident, $non_ascii:ident, $byte:ident,
        $lead_minus_offset:ident, $unread_handle_trail:ident, $source:ident,
        $handle:ident, $outermost:tt, $copy_ascii:ident, $destination_check:ident,
        $name:ident, $code_unit:ty, $dest_struct:ident, $ascii_punctuation:expr
    ) => {
        pub fn $name (& mut $slf, src : & [u8], dst : & mut [$code_unit], last : bool) ->
        (DecoderResult, usize, usize) { let mut $source = ByteSource::new(src); let mut
        dest_prolog = $dest_struct ::new(dst); let dest = match $slf .lead { Some(lead)
        => { let $lead_minus_offset = lead; $slf .lead = None; match $source
        .check_available() { Space::Full(src_consumed_prolog) => { if last { return
        (DecoderResult::Malformed(1, 0), src_consumed_prolog, dest_prolog.written()); }
        return (DecoderResult::InputEmpty, src_consumed_prolog, dest_prolog.written()); }
        Space::Available(source_handle_prolog) => { match dest_prolog.$destination_check
        () { Space::Full(dst_written_prolog) => { return (DecoderResult::OutputFull,
        source_handle_prolog.consumed(), dst_written_prolog); } Space::Available($handle)
        => { let ($byte, $unread_handle_trail) = source_handle_prolog.read(); $trail } }
        } } }, None => { & mut dest_prolog } }; $outermost : loop { match dest
        .$copy_ascii (& mut $source) { CopyAsciiResult::Stop(ret) => return ret,
        CopyAsciiResult::GoOn((mut $non_ascii, mut $handle)) => { 'middle : loop { let
        dest_again = { let $lead_minus_offset = { $lead }; match $source
        .check_available() { Space::Full(src_consumed_trail) => { if last { return
        (DecoderResult::Malformed(1, 0), src_consumed_trail, $handle .written()); } $slf
        .lead = Some($lead_minus_offset); return (DecoderResult::InputEmpty,
        src_consumed_trail, $handle .written()); } Space::Available(source_handle_trail)
        => { let ($byte, $unread_handle_trail) = source_handle_trail.read(); $trail } }
        }; match $source .check_available() { Space::Full(src_consumed) => { return
        (DecoderResult::InputEmpty, src_consumed, dest_again.written()); }
        Space::Available(source_handle) => { match dest_again.$destination_check () {
        Space::Full(dst_written) => { return (DecoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available(mut destination_handle) => { let
        (mut b, unread_handle) = source_handle.read(); let source_again = unread_handle
        .commit(); 'innermost : loop { if b > 127 { $non_ascii = b; $handle =
        destination_handle; continue 'middle; } let dest_again_again = destination_handle
        .write_ascii(b); if $ascii_punctuation && b < 60 { match source_again
        .check_available() { Space::Full(src_consumed_again) => { return
        (DecoderResult::InputEmpty, src_consumed_again, dest_again_again.written()); }
        Space::Available(source_handle_again) => { match dest_again_again
        .$destination_check () { Space::Full(dst_written_again) => { return
        (DecoderResult::OutputFull, source_handle_again.consumed(), dst_written_again); }
        Space::Available(destination_handle_again) => { { let (b_again,
        _unread_handle_again) = source_handle_again.read(); b = b_again;
        destination_handle = destination_handle_again; continue 'innermost; } } } } } }
        continue $outermost; } } } } } } } } } }
    };
}
macro_rules! ascii_compatible_two_byte_decoder_functions {
    (
        $lead:block, $trail:block, $slf:ident, $non_ascii:ident, $byte:ident,
        $lead_minus_offset:ident, $unread_handle_trail:ident, $source:ident,
        $handle:ident, $outermost:tt, $copy_ascii:ident, $destination_check:ident,
        $ascii_punctuation:expr
    ) => {
        ascii_compatible_two_byte_decoder_function!($lead, $trail, $slf, $non_ascii,
        $byte, $lead_minus_offset, $unread_handle_trail, $source, $handle, $outermost,
        $copy_ascii, $destination_check, decode_to_utf8_raw, u8, Utf8Destination,
        $ascii_punctuation); ascii_compatible_two_byte_decoder_function!($lead, $trail,
        $slf, $non_ascii, $byte, $lead_minus_offset, $unread_handle_trail, $source,
        $handle, $outermost, $copy_ascii, $destination_check, decode_to_utf16_raw, u16,
        Utf16Destination, $ascii_punctuation);
    };
}
macro_rules! gb18030_decoder_function {
    (
        $first_body:block, $second_body:block, $third_body:block, $fourth_body:block,
        $slf:ident, $non_ascii:ident, $first_minus_offset:ident, $second:ident,
        $second_minus_offset:ident, $unread_handle_second:ident, $third:ident,
        $third_minus_offset:ident, $unread_handle_third:ident, $fourth:ident,
        $fourth_minus_offset:ident, $unread_handle_fourth:ident, $source:ident,
        $handle:ident, $outermost:tt, $name:ident, $code_unit:ty, $dest_struct:ident
    ) => {
        #[cfg_attr(feature = "cargo-clippy", allow(never_loop))] pub fn $name (& mut
        $slf, src : & [u8], dst : & mut [$code_unit], last : bool) -> (DecoderResult,
        usize, usize) { let mut $source = ByteSource::new(src); let mut dest =
        $dest_struct ::new(dst); { if let Some(ascii) = $slf .pending_ascii { match dest
        .check_space_bmp() { Space::Full(_) => { return (DecoderResult::OutputFull, 0,
        0); } Space::Available(pending_ascii_handle) => { $slf .pending_ascii = None;
        pending_ascii_handle.write_ascii(ascii); } } } } while !$slf .pending.is_none() {
        match $source .check_available() { Space::Full(src_consumed) => { if last { let
        count = $slf .pending.count(); $slf .pending = Gb18030Pending::None; return
        (DecoderResult::Malformed(count as u8, 0), src_consumed, dest.written()); }
        return (DecoderResult::InputEmpty, src_consumed, dest.written()); }
        Space::Available(source_handle) => { match dest.check_space_astral() {
        Space::Full(dst_written) => { return (DecoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available($handle) => { let (byte,
        unread_handle) = source_handle.read(); match $slf .pending {
        Gb18030Pending::One($first_minus_offset) => { $slf .pending =
        Gb18030Pending::None; let $second = byte; let $unread_handle_second =
        unread_handle; let $second_minus_offset = $second .wrapping_sub(0x30); if
        $second_minus_offset > (0x39 - 0x30) { $second_body } else { $slf .pending =
        Gb18030Pending::Two($first_minus_offset, $second_minus_offset); $handle .commit()
        } } Gb18030Pending::Two($first_minus_offset, $second_minus_offset) => { $slf
        .pending = Gb18030Pending::None; let $third = byte; let $unread_handle_third =
        unread_handle; let $third_minus_offset = { $third_body }; $slf .pending =
        Gb18030Pending::Three($first_minus_offset, $second_minus_offset,
        $third_minus_offset); $handle .commit() }
        Gb18030Pending::Three($first_minus_offset, $second_minus_offset,
        $third_minus_offset) => { $slf .pending = Gb18030Pending::None; let $fourth =
        byte; let $unread_handle_fourth = unread_handle; $fourth_body }
        Gb18030Pending::None => unreachable!("Checked in loop condition"), }; } } } } }
        $outermost : loop { match dest.copy_ascii_from_check_space_astral(& mut $source)
        { CopyAsciiResult::Stop(ret) => return ret, CopyAsciiResult::GoOn((mut
        $non_ascii, mut $handle)) => { 'middle : loop { let dest_again = { let
        $first_minus_offset = { $first_body }; match $source .check_available() {
        Space::Full(src_consumed_trail) => { if last { return
        (DecoderResult::Malformed(1, 0), src_consumed_trail, $handle .written()); } $slf
        .pending = Gb18030Pending::One($first_minus_offset); return
        (DecoderResult::InputEmpty, src_consumed_trail, $handle .written()); }
        Space::Available(source_handle_trail) => { let ($second, $unread_handle_second) =
        source_handle_trail.read(); let $second_minus_offset = $second
        .wrapping_sub(0x30); if $second_minus_offset > (0x39 - 0x30) { $second_body }
        else { match $unread_handle_second .commit().check_available() {
        Space::Full(src_consumed_third) => { if last { return
        (DecoderResult::Malformed(2, 0), src_consumed_third, $handle .written()); } $slf
        .pending = Gb18030Pending::Two($first_minus_offset, $second_minus_offset); return
        (DecoderResult::InputEmpty, src_consumed_third, $handle .written()); }
        Space::Available(source_handle_third) => { let ($third, $unread_handle_third) =
        source_handle_third.read(); let $third_minus_offset = { $third_body }; match
        $unread_handle_third .commit().check_available() {
        Space::Full(src_consumed_fourth) => { if last { return
        (DecoderResult::Malformed(3, 0), src_consumed_fourth, $handle .written()); } $slf
        .pending = Gb18030Pending::Three($first_minus_offset, $second_minus_offset,
        $third_minus_offset); return (DecoderResult::InputEmpty, src_consumed_fourth,
        $handle .written()); } Space::Available(source_handle_fourth) => { let ($fourth,
        $unread_handle_fourth) = source_handle_fourth.read(); $fourth_body } } } } } } }
        }; match $source .check_available() { Space::Full(src_consumed) => { return
        (DecoderResult::InputEmpty, src_consumed, dest_again.written()); }
        Space::Available(source_handle) => { match dest_again.check_space_astral() {
        Space::Full(dst_written) => { return (DecoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available(destination_handle) => { let (b, _)
        = source_handle.read(); 'innermost : loop { if b > 127 { $non_ascii = b; $handle
        = destination_handle; continue 'middle; } destination_handle.write_ascii(b);
        continue $outermost; } } } } } } } } } }
    };
}
macro_rules! gb18030_decoder_functions {
    (
        $first_body:block, $second_body:block, $third_body:block, $fourth_body:block,
        $slf:ident, $non_ascii:ident, $first_minus_offset:ident, $second:ident,
        $second_minus_offset:ident, $unread_handle_second:ident, $third:ident,
        $third_minus_offset:ident, $unread_handle_third:ident, $fourth:ident,
        $fourth_minus_offset:ident, $unread_handle_fourth:ident, $source:ident,
        $handle:ident, $outermost:tt
    ) => {
        gb18030_decoder_function!($first_body, $second_body, $third_body, $fourth_body,
        $slf, $non_ascii, $first_minus_offset, $second, $second_minus_offset,
        $unread_handle_second, $third, $third_minus_offset, $unread_handle_third,
        $fourth, $fourth_minus_offset, $unread_handle_fourth, $source, $handle,
        $outermost, decode_to_utf8_raw, u8, Utf8Destination);
        gb18030_decoder_function!($first_body, $second_body, $third_body, $fourth_body,
        $slf, $non_ascii, $first_minus_offset, $second, $second_minus_offset,
        $unread_handle_second, $third, $third_minus_offset, $unread_handle_third,
        $fourth, $fourth_minus_offset, $unread_handle_fourth, $source, $handle,
        $outermost, decode_to_utf16_raw, u16, Utf16Destination);
    };
}
macro_rules! euc_jp_decoder_function {
    (
        $jis0802_trail_body:block, $jis0812_lead_body:block, $jis0812_trail_body:block,
        $half_width_katakana_body:block, $slf:ident, $non_ascii:ident,
        $jis0208_lead_minus_offset:ident, $byte:ident, $unread_handle_trail:ident,
        $jis0212_lead_minus_offset:ident, $lead:ident, $unread_handle_jis0212:ident,
        $source:ident, $handle:ident, $name:ident, $code_unit:ty, $dest_struct:ident
    ) => {
        #[cfg_attr(feature = "cargo-clippy", allow(never_loop))] pub fn $name (& mut
        $slf, src : & [u8], dst : & mut [$code_unit], last : bool) -> (DecoderResult,
        usize, usize) { let mut $source = ByteSource::new(src); let mut dest =
        $dest_struct ::new(dst); while !$slf .pending.is_none() { match $source
        .check_available() { Space::Full(src_consumed) => { if last { let count = $slf
        .pending.count(); $slf .pending = EucJpPending::None; return
        (DecoderResult::Malformed(count as u8, 0), src_consumed, dest.written()); }
        return (DecoderResult::InputEmpty, src_consumed, dest.written()); }
        Space::Available(source_handle) => { match dest.check_space_bmp() {
        Space::Full(dst_written) => { return (DecoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available($handle) => { let ($byte,
        $unread_handle_trail) = source_handle.read(); match $slf .pending {
        EucJpPending::Jis0208Lead($jis0208_lead_minus_offset) => { $slf .pending =
        EucJpPending::None; $jis0802_trail_body } EucJpPending::Jis0212Shift => { $slf
        .pending = EucJpPending::None; let $lead = $byte; let $unread_handle_jis0212 =
        $unread_handle_trail; let $jis0212_lead_minus_offset = { $jis0812_lead_body };
        $slf .pending = EucJpPending::Jis0212Lead($jis0212_lead_minus_offset); $handle
        .commit() } EucJpPending::Jis0212Lead($jis0212_lead_minus_offset) => { $slf
        .pending = EucJpPending::None; $jis0812_trail_body }
        EucJpPending::HalfWidthKatakana => { $slf .pending = EucJpPending::None;
        $half_width_katakana_body } EucJpPending::None =>
        unreachable!("Checked in loop condition"), }; } } } } } 'outermost : loop { match
        dest.copy_ascii_from_check_space_bmp(& mut $source) { CopyAsciiResult::Stop(ret)
        => return ret, CopyAsciiResult::GoOn((mut $non_ascii, mut $handle)) => { 'middle
        : loop { let dest_again = { let $jis0208_lead_minus_offset = $non_ascii
        .wrapping_sub(0xA1); if $jis0208_lead_minus_offset <= (0xFE - 0xA1) { match
        $source .check_available() { Space::Full(src_consumed_trail) => { if last {
        return (DecoderResult::Malformed(1, 0), src_consumed_trail, $handle .written());
        } $slf .pending = EucJpPending::Jis0208Lead($jis0208_lead_minus_offset); return
        (DecoderResult::InputEmpty, src_consumed_trail, $handle .written()); }
        Space::Available(source_handle_trail) => { let ($byte, $unread_handle_trail) =
        source_handle_trail.read(); $jis0802_trail_body } } } else if $non_ascii == 0x8F
        { match $source .check_available() { Space::Full(src_consumed_jis0212) => { if
        last { return (DecoderResult::Malformed(1, 0), src_consumed_jis0212, $handle
        .written()); } $slf .pending = EucJpPending::Jis0212Shift; return
        (DecoderResult::InputEmpty, src_consumed_jis0212, $handle .written()); }
        Space::Available(source_handle_jis0212) => { let ($lead, $unread_handle_jis0212)
        = source_handle_jis0212.read(); let $jis0212_lead_minus_offset = {
        $jis0812_lead_body }; match $unread_handle_jis0212 .commit().check_available() {
        Space::Full(src_consumed_trail) => { if last { return
        (DecoderResult::Malformed(2, 0), src_consumed_trail, $handle .written()); } $slf
        .pending = EucJpPending::Jis0212Lead($jis0212_lead_minus_offset); return
        (DecoderResult::InputEmpty, src_consumed_trail, $handle .written()); }
        Space::Available(source_handle_trail) => { let ($byte, $unread_handle_trail) =
        source_handle_trail.read(); $jis0812_trail_body } } } } } else if $non_ascii ==
        0x8E { match $source .check_available() { Space::Full(src_consumed_trail) => { if
        last { return (DecoderResult::Malformed(1, 0), src_consumed_trail, $handle
        .written()); } $slf .pending = EucJpPending::HalfWidthKatakana; return
        (DecoderResult::InputEmpty, src_consumed_trail, $handle .written()); }
        Space::Available(source_handle_trail) => { let ($byte, $unread_handle_trail) =
        source_handle_trail.read(); $half_width_katakana_body } } } else { return
        (DecoderResult::Malformed(1, 0), $source .consumed(), $handle .written()); } };
        match $source .check_available() { Space::Full(src_consumed) => { return
        (DecoderResult::InputEmpty, src_consumed, dest_again.written()); }
        Space::Available(source_handle) => { match dest_again.check_space_bmp() {
        Space::Full(dst_written) => { return (DecoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available(destination_handle) => { let (b, _)
        = source_handle.read(); 'innermost : loop { if b > 127 { $non_ascii = b; $handle
        = destination_handle; continue 'middle; } destination_handle.write_ascii(b);
        continue 'outermost; } } } } } } } } } }
    };
}
macro_rules! euc_jp_decoder_functions {
    (
        $jis0802_trail_body:block, $jis0812_lead_body:block, $jis0812_trail_body:block,
        $half_width_katakana_body:block, $slf:ident, $non_ascii:ident,
        $jis0208_lead_minus_offset:ident, $byte:ident, $unread_handle_trail:ident,
        $jis0212_lead_minus_offset:ident, $lead:ident, $unread_handle_jis0212:ident,
        $source:ident, $handle:ident
    ) => {
        euc_jp_decoder_function!($jis0802_trail_body, $jis0812_lead_body,
        $jis0812_trail_body, $half_width_katakana_body, $slf, $non_ascii,
        $jis0208_lead_minus_offset, $byte, $unread_handle_trail,
        $jis0212_lead_minus_offset, $lead, $unread_handle_jis0212, $source, $handle,
        decode_to_utf8_raw, u8, Utf8Destination);
        euc_jp_decoder_function!($jis0802_trail_body, $jis0812_lead_body,
        $jis0812_trail_body, $half_width_katakana_body, $slf, $non_ascii,
        $jis0208_lead_minus_offset, $byte, $unread_handle_trail,
        $jis0212_lead_minus_offset, $lead, $unread_handle_jis0212, $source, $handle,
        decode_to_utf16_raw, u16, Utf16Destination);
    };
}
macro_rules! encoder_function {
    (
        $eof:block, $body:block, $slf:ident, $src_consumed:ident, $source:ident,
        $dest:ident, $c:ident, $destination_handle:ident, $unread_handle:ident,
        $destination_check:ident, $name:ident, $input:ty, $source_struct:ident
    ) => {
        pub fn $name (& mut $slf, src : &$input, dst : & mut [u8], last : bool) ->
        (EncoderResult, usize, usize) { let mut $source = $source_struct ::new(src); let
        mut $dest = ByteDestination::new(dst); loop { match $source .check_available() {
        Space::Full($src_consumed) => { if last { $eof } return
        (EncoderResult::InputEmpty, $src_consumed, $dest .written()); }
        Space::Available(source_handle) => { match $dest .$destination_check () {
        Space::Full(dst_written) => { return (EncoderResult::OutputFull, source_handle
        .consumed(), dst_written); } Space::Available($destination_handle) => { let ($c,
        $unread_handle) = source_handle.read(); $body } } } } } }
    };
}
macro_rules! encoder_functions {
    (
        $eof:block, $body:block, $slf:ident, $src_consumed:ident, $source:ident,
        $dest:ident, $c:ident, $destination_handle:ident, $unread_handle:ident,
        $destination_check:ident
    ) => {
        encoder_function!($eof, $body, $slf, $src_consumed, $source, $dest, $c,
        $destination_handle, $unread_handle, $destination_check, encode_from_utf8_raw,
        str, Utf8Source); encoder_function!($eof, $body, $slf, $src_consumed, $source,
        $dest, $c, $destination_handle, $unread_handle, $destination_check,
        encode_from_utf16_raw, [u16], Utf16Source);
    };
}
macro_rules! ascii_compatible_encoder_function {
    (
        $bmp_body:block, $astral_body:block, $bmp:ident, $astral:ident, $slf:ident,
        $source:ident, $handle:ident, $copy_ascii:ident, $destination_check:ident,
        $name:ident, $input:ty, $source_struct:ident, $ascii_punctuation:expr
    ) => {
        pub fn $name (& mut $slf, src : &$input, dst : & mut [u8], _last : bool) ->
        (EncoderResult, usize, usize) { let mut $source = $source_struct ::new(src); let
        mut dest = ByteDestination::new(dst); 'outermost : loop { match $source
        .$copy_ascii (& mut dest) { CopyAsciiResult::Stop(ret) => return ret,
        CopyAsciiResult::GoOn((mut non_ascii, mut $handle)) => { 'middle : loop { let
        dest_again = match non_ascii { NonAscii::BmpExclAscii($bmp) => { $bmp_body }
        NonAscii::Astral($astral) => { $astral_body } }; match $source .check_available()
        { Space::Full(src_consumed) => { return (EncoderResult::InputEmpty, src_consumed,
        dest_again.written()); } Space::Available(source_handle) => { match dest_again
        .$destination_check () { Space::Full(dst_written) => { return
        (EncoderResult::OutputFull, source_handle.consumed(), dst_written); }
        Space::Available(mut destination_handle) => { let (mut c, unread_handle) =
        source_handle.read_enum(); let source_again = unread_handle.commit(); 'innermost
        : loop { let ascii = match c { Unicode::NonAscii(non_ascii_again) => { non_ascii
        = non_ascii_again; $handle = destination_handle; continue 'middle; }
        Unicode::Ascii(a) => a, }; let dest_again_again = destination_handle
        .write_one(ascii); if $ascii_punctuation && ascii < 60 { match source_again
        .check_available() { Space::Full(src_consumed_again) => { return
        (EncoderResult::InputEmpty, src_consumed_again, dest_again_again.written()); }
        Space::Available(source_handle_again) => { match dest_again_again
        .$destination_check () { Space::Full(dst_written_again) => { return
        (EncoderResult::OutputFull, source_handle_again.consumed(), dst_written_again); }
        Space::Available(destination_handle_again) => { { let (c_again,
        _unread_handle_again) = source_handle_again.read_enum(); c = c_again;
        destination_handle = destination_handle_again; continue 'innermost; } } } } } }
        continue 'outermost; } } } } } } } } } }
    };
}
macro_rules! ascii_compatible_encoder_functions {
    (
        $bmp_body:block, $astral_body:block, $bmp:ident, $astral:ident, $slf:ident,
        $source:ident, $handle:ident, $copy_ascii:ident, $destination_check:ident,
        $ascii_punctuation:expr
    ) => {
        ascii_compatible_encoder_function!($bmp_body, $astral_body, $bmp, $astral, $slf,
        $source, $handle, $copy_ascii, $destination_check, encode_from_utf8_raw, str,
        Utf8Source, $ascii_punctuation); ascii_compatible_encoder_function!($bmp_body,
        $astral_body, $bmp, $astral, $slf, $source, $handle, $copy_ascii,
        $destination_check, encode_from_utf16_raw, [u16], Utf16Source,
        $ascii_punctuation);
    };
}
macro_rules! ascii_compatible_bmp_encoder_function {
    (
        $bmp_body:block, $bmp:ident, $slf:ident, $source:ident, $handle:ident,
        $copy_ascii:ident, $destination_check:ident, $name:ident, $input:ty,
        $source_struct:ident, $ascii_punctuation:expr
    ) => {
        ascii_compatible_encoder_function!($bmp_body, { return
        (EncoderResult::Unmappable(astral), $source .consumed(), $handle .written(),); },
        $bmp, astral, $slf, $source, $handle, $copy_ascii, $destination_check, $name,
        $input, $source_struct, $ascii_punctuation);
    };
}
macro_rules! ascii_compatible_bmp_encoder_functions {
    (
        $bmp_body:block, $bmp:ident, $slf:ident, $source:ident, $handle:ident,
        $copy_ascii:ident, $destination_check:ident, $ascii_punctuation:expr
    ) => {
        ascii_compatible_encoder_functions!($bmp_body, { return
        (EncoderResult::Unmappable(astral), $source .consumed(), $handle .written(),); },
        $bmp, astral, $slf, $source, $handle, $copy_ascii, $destination_check,
        $ascii_punctuation);
    };
}
macro_rules! public_decode_function {
    (
        $(#[$meta:meta])*, $decode_to_utf:ident, $decode_to_utf_raw:ident,
        $decode_to_utf_checking_end:ident,
        $decode_to_utf_after_one_potential_bom_byte:ident,
        $decode_to_utf_after_two_potential_bom_bytes:ident,
        $decode_to_utf_checking_end_with_offset:ident, $code_unit:ty
    ) => {
        $(#[$meta])* pub fn $decode_to_utf (& mut self, src : & [u8], dst : & mut
        [$code_unit], last : bool) -> (DecoderResult, usize, usize) { let mut offset =
        0usize; loop { match self.life_cycle { DecoderLifeCycle::Converting => { return
        self.$decode_to_utf_checking_end (src, dst, last); } DecoderLifeCycle::AtStart =>
        { debug_assert_eq!(offset, 0usize); if src.is_empty() { return
        (DecoderResult::InputEmpty, 0, 0); } match src[0] { 0xEFu8 => { self.life_cycle =
        DecoderLifeCycle::SeenUtf8First; offset += 1; continue; } 0xFEu8 => { self
        .life_cycle = DecoderLifeCycle::SeenUtf16BeFirst; offset += 1; continue; } 0xFFu8
        => { self.life_cycle = DecoderLifeCycle::SeenUtf16LeFirst; offset += 1; continue;
        } _ => { self.life_cycle = DecoderLifeCycle::Converting; continue; } } }
        DecoderLifeCycle::AtUtf8Start => { debug_assert_eq!(offset, 0usize); if src
        .is_empty() { return (DecoderResult::InputEmpty, 0, 0); } match src[0] { 0xEFu8
        => { self.life_cycle = DecoderLifeCycle::SeenUtf8First; offset += 1; continue; }
        _ => { self.life_cycle = DecoderLifeCycle::Converting; continue; } } }
        DecoderLifeCycle::AtUtf16BeStart => { debug_assert_eq!(offset, 0usize); if src
        .is_empty() { return (DecoderResult::InputEmpty, 0, 0); } match src[0] { 0xFEu8
        => { self.life_cycle = DecoderLifeCycle::SeenUtf16BeFirst; offset += 1; continue;
        } _ => { self.life_cycle = DecoderLifeCycle::Converting; continue; } } }
        DecoderLifeCycle::AtUtf16LeStart => { debug_assert_eq!(offset, 0usize); if src
        .is_empty() { return (DecoderResult::InputEmpty, 0, 0); } match src[0] { 0xFFu8
        => { self.life_cycle = DecoderLifeCycle::SeenUtf16LeFirst; offset += 1; continue;
        } _ => { self.life_cycle = DecoderLifeCycle::Converting; continue; } } }
        DecoderLifeCycle::SeenUtf8First => { if offset >= src.len() { if last { return
        self.$decode_to_utf_after_one_potential_bom_byte (src, dst, last, offset,
        0xEFu8); } return (DecoderResult::InputEmpty, offset, 0); } if src[offset] ==
        0xBBu8 { self.life_cycle = DecoderLifeCycle::SeenUtf8Second; offset += 1;
        continue; } return self.$decode_to_utf_after_one_potential_bom_byte (src, dst,
        last, offset, 0xEFu8); } DecoderLifeCycle::SeenUtf8Second => { if offset >= src
        .len() { if last { return self.$decode_to_utf_after_two_potential_bom_bytes (src,
        dst, last, offset); } return (DecoderResult::InputEmpty, offset, 0); } if
        src[offset] == 0xBFu8 { self.life_cycle = DecoderLifeCycle::Converting; offset +=
        1; if self.encoding != UTF_8 { self.encoding = UTF_8; self.variant = UTF_8
        .new_variant_decoder(); } return self.$decode_to_utf_checking_end_with_offset
        (src, dst, last, offset); } return self
        .$decode_to_utf_after_two_potential_bom_bytes (src, dst, last, offset); }
        DecoderLifeCycle::SeenUtf16BeFirst => { if offset >= src.len() { if last { return
        self.$decode_to_utf_after_one_potential_bom_byte (src, dst, last, offset,
        0xFEu8); } return (DecoderResult::InputEmpty, offset, 0); } if src[offset] ==
        0xFFu8 { self.life_cycle = DecoderLifeCycle::Converting; offset += 1; if self
        .encoding != UTF_16BE { self.encoding = UTF_16BE; self.variant = UTF_16BE
        .new_variant_decoder(); } return self.$decode_to_utf_checking_end_with_offset
        (src, dst, last, offset); } return self
        .$decode_to_utf_after_one_potential_bom_byte (src, dst, last, offset, 0xFEu8); }
        DecoderLifeCycle::SeenUtf16LeFirst => { if offset >= src.len() { if last { return
        self.$decode_to_utf_after_one_potential_bom_byte (src, dst, last, offset,
        0xFFu8); } return (DecoderResult::InputEmpty, offset, 0); } if src[offset] ==
        0xFEu8 { self.life_cycle = DecoderLifeCycle::Converting; offset += 1; if self
        .encoding != UTF_16LE { self.encoding = UTF_16LE; self.variant = UTF_16LE
        .new_variant_decoder(); } return self.$decode_to_utf_checking_end_with_offset
        (src, dst, last, offset); } return self
        .$decode_to_utf_after_one_potential_bom_byte (src, dst, last, offset, 0xFFu8); }
        DecoderLifeCycle::ConvertingWithPendingBB => { debug_assert_eq!(offset, 0usize);
        return self.$decode_to_utf_after_one_potential_bom_byte (src, dst, last, 0usize,
        0xBBu8); } DecoderLifeCycle::Finished =>
        panic!("Must not use a decoder that has finished."), } } } fn
        $decode_to_utf_after_one_potential_bom_byte (& mut self, src : & [u8], dst : &
        mut [$code_unit], last : bool, offset : usize, first_byte : u8) ->
        (DecoderResult, usize, usize) { self.life_cycle = DecoderLifeCycle::Converting;
        if offset == 0usize { let first = [first_byte]; let mut out_read = 0usize; let
        (mut first_result, _, mut first_written) = self.variant.$decode_to_utf_raw (&
        first[..], dst, false); match first_result { DecoderResult::InputEmpty => { let
        (result, read, written) = self.$decode_to_utf_checking_end (src, & mut
        dst[first_written..], last); first_result = result; out_read = read;
        first_written += written; } DecoderResult::Malformed(_, _) => {}
        DecoderResult::OutputFull => { panic!("Output buffer must have been too small.");
        } } return (first_result, out_read, first_written); } debug_assert_eq!(offset,
        1usize); self.$decode_to_utf_checking_end (src, dst, last) } fn
        $decode_to_utf_after_two_potential_bom_bytes (& mut self, src : & [u8], dst : &
        mut [$code_unit], last : bool, offset : usize) -> (DecoderResult, usize, usize) {
        self.life_cycle = DecoderLifeCycle::Converting; if offset == 0usize { let ef_bb =
        [0xEFu8, 0xBBu8]; let (mut first_result, mut first_read, mut first_written) =
        self.variant.$decode_to_utf_raw (& ef_bb[..], dst, false); match first_result {
        DecoderResult::InputEmpty => { let (result, read, written) = self
        .$decode_to_utf_checking_end (src, & mut dst[first_written..], last);
        first_result = result; first_read = read; first_written += written; }
        DecoderResult::Malformed(_, _) => { if first_read == 1usize { self.life_cycle =
        DecoderLifeCycle::ConvertingWithPendingBB; } first_read = 0usize; }
        DecoderResult::OutputFull => { panic!("Output buffer must have been too small.");
        } } return (first_result, first_read, first_written); } if offset == 1usize {
        return self.$decode_to_utf_after_one_potential_bom_byte (src, dst, last, 0usize,
        0xEFu8); } debug_assert_eq!(offset, 2usize); self.$decode_to_utf_checking_end
        (src, dst, last) } #[doc =
        " Calls `$decode_to_utf_checking_end` with `offset` bytes omitted from"] #[doc =
        " the start of `src` but adjusting the return values to show those bytes"] #[doc
        = " as having been consumed."] fn $decode_to_utf_checking_end_with_offset (& mut
        self, src : & [u8], dst : & mut [$code_unit], last : bool, offset : usize) ->
        (DecoderResult, usize, usize) { debug_assert_eq!(self.life_cycle,
        DecoderLifeCycle::Converting); let (result, read, written) = self
        .$decode_to_utf_checking_end (& src[offset..], dst, last); (result, read +
        offset, written) } #[doc =
        " Calls through to the delegate and adjusts life cycle iff `last` is"] #[doc =
        " `true` and result is `DecoderResult::InputEmpty`."] fn
        $decode_to_utf_checking_end (& mut self, src : & [u8], dst : & mut [$code_unit],
        last : bool) -> (DecoderResult, usize, usize) { debug_assert_eq!(self.life_cycle,
        DecoderLifeCycle::Converting); let (result, read, written) = self.variant
        .$decode_to_utf_raw (src, dst, last); if last { if let DecoderResult::InputEmpty
        = result { self.life_cycle = DecoderLifeCycle::Finished; } } (result, read,
        written) }
    };
}
#[cfg(test)]
mod tests_rug_410 {
    use super::*;
    use crate::{big5, DecoderResult};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 17], bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut p0 = big5::Big5Decoder::new();
        let p1 = rug_fuzz_0;
        let mut p2 = vec![0u8; 10];
        let p3 = rug_fuzz_1;
        let result = p0.decode_to_utf8_raw(p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_411 {
    use crate::big5::Big5Decoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 11], u16, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut p0 = Big5Decoder::new();
        let p1 = rug_fuzz_0;
        let mut p2 = [rug_fuzz_1; 10];
        let p3 = rug_fuzz_2;
        p0.decode_to_utf16_raw(p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_413 {
    use super::*;
    use crate::{EncoderResult, big5, DecoderResult};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3)) = <(u16, u16, u16, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = big5::Big5Encoder;
        let p1 = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        let mut p2 = vec![0; 10];
        let p3 = rug_fuzz_3;
        let (_result, _src_chars, _dst_bytes) = big5::Big5Encoder::encode_from_utf16_raw(
            &mut p0,
            &p1,
            &mut p2,
            p3,
        );
             }
});    }
}
#[cfg(test)]
mod tests_rug_416 {
    use super::*;
    use crate::euc_jp::EucJpEncoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2)) = <(&str, u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = EucJpEncoder;
        let p1 = rug_fuzz_0;
        let mut p2 = [rug_fuzz_1; 10];
        let p3 = rug_fuzz_2;
        EucJpEncoder::encode_from_utf8_raw(&mut p0, &p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_418 {
    use crate::{euc_kr, DecoderResult};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 11], u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut v5 = euc_kr::EucKrDecoder::new();
        let src_data: &[u8] = rug_fuzz_0;
        let mut dst_data: [u8; 10] = [rug_fuzz_1; 10];
        let last: bool = rug_fuzz_2;
        v5.decode_to_utf8_raw(src_data, &mut dst_data, last);
             }
});    }
}
#[cfg(test)]
mod tests_rug_419 {
    use super::*;
    use crate::euc_kr::EucKrDecoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 22], u16, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        #[allow(unused_mut)]
        let mut v5 = EucKrDecoder::new();
        let mut src: &[u8] = rug_fuzz_0;
        let mut dst: [u16; 10] = [rug_fuzz_1; 10];
        let last: bool = rug_fuzz_2;
        v5.decode_to_utf16_raw(src, &mut dst, last);
             }
});    }
}
#[cfg(test)]
mod tests_rug_422 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 17], bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut p0 = gb18030::Gb18030Decoder::new();
        let p1 = rug_fuzz_0;
        let mut p2 = vec![0; 10];
        let p3 = rug_fuzz_1;
        p0.decode_to_utf8_raw(p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_423 {
    use super::*;
    use crate::gb18030;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6, mut rug_fuzz_7)) = <(u8, u8, u8, u8, u8, u8, u16, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut p0 = gb18030::Gb18030Decoder::new();
        let p1 = &[
            rug_fuzz_0,
            rug_fuzz_1,
            rug_fuzz_2,
            rug_fuzz_3,
            rug_fuzz_4,
            rug_fuzz_5,
        ];
        let mut p2 = [rug_fuzz_6; 10];
        let p3 = rug_fuzz_7;
        p0.decode_to_utf16_raw(p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_435 {
    use super::*;
    use crate::utf_16;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(bool, u8, u8, u8, u8, u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut v56 = utf_16::Utf16Decoder::new(rug_fuzz_0);
        let p1 = &[rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let p2 = &mut [rug_fuzz_5; 8];
        let p3 = rug_fuzz_6;
        v56.decode_to_utf8_raw(p1, p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_436 {
    use crate::utf_16::Utf16Decoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1_ext, mut rug_fuzz_2, mut rug_fuzz_3)) = <(bool, [u8; 5], u16, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_1 = & rug_fuzz_1_ext;
        let mut p0 = Utf16Decoder::new(rug_fuzz_0);
        let p1: &[u8] = rug_fuzz_1.as_ref();
        let mut p2: [u16; 10] = [rug_fuzz_2; 10];
        let p3: bool = rug_fuzz_3;
        p0.decode_to_utf16_raw(p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_438 {
    use super::*;
    use crate::utf_8;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1, mut rug_fuzz_2, mut rug_fuzz_3, mut rug_fuzz_4, mut rug_fuzz_5, mut rug_fuzz_6)) = <(u8, u8, u8, u8, u8, u16, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let mut decoder = utf_8::Utf8Decoder::new_inner();
        let src_data = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2, rug_fuzz_3, rug_fuzz_4];
        let src_slice = &src_data;
        let mut dst_data = [rug_fuzz_5; 5];
        let dst_slice = &mut dst_data[..];
        let last = rug_fuzz_6;
        utf_8::Utf8Decoder::decode_to_utf16_raw(
            &mut decoder,
            src_slice,
            dst_slice,
            last,
        );
             }
});    }
}
#[cfg(test)]
mod tests_rug_439 {
    use super::*;
    use x_user_defined::UserDefinedDecoder;
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1, mut rug_fuzz_2)) = <([u8; 11], u8, bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut p0 = UserDefinedDecoder::new();
        let p1 = rug_fuzz_0;
        let mut p2 = [rug_fuzz_1; 16];
        let p3 = rug_fuzz_2;
        p0.decode_to_utf8_raw(p1, &mut p2, p3);
             }
});    }
}
#[cfg(test)]
mod tests_rug_446 {
    use super::*;
    use crate::{Decoder, DecoderResult, DecoderLifeCycle, UTF_8};
    #[test]
    fn test_rug() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0_ext, mut rug_fuzz_1)) = <([u8; 11], bool) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

let rug_fuzz_0 = & rug_fuzz_0_ext;
        let mut p0 = UTF_8.new_decoder_without_bom_handling();
        let p1 = rug_fuzz_0;
        let mut p2 = vec![0u8; 20];
        let p3 = rug_fuzz_1;
        p0.decode_to_utf8_checking_end(p1, &mut p2, p3);
             }
});    }
}
