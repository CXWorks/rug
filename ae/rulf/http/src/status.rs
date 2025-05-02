//! HTTP status codes
//!
//! This module contains HTTP-status code related structs an errors. The main
//! type in this module is `StatusCode` which is not intended to be used through
//! this module but rather the `http::StatusCode` type.
//!
//! # Examples
//!
//! ```
//! use http::StatusCode;
//!
//! assert_eq!(StatusCode::from_u16(200).unwrap(), StatusCode::OK);
//! assert_eq!(StatusCode::NOT_FOUND, 404);
//! assert!(StatusCode::OK.is_success());
//! ```
use std::convert::TryFrom;
use std::num::NonZeroU16;
use std::error::Error;
use std::fmt;
use std::str::FromStr;
/// An HTTP status code (`status-code` in RFC 7230 et al.).
///
/// Constants are provided for known status codes, including those in the IANA
/// [HTTP Status Code Registry](
/// https://www.iana.org/assignments/http-status-codes/http-status-codes.xhtml).
///
/// Status code values in the range 100-999 (inclusive) are supported by this
/// type. Values in the range 100-599 are semantically classified by the most
/// significant digit. See [`StatusCode::is_success`], etc. Values above 599
/// are unclassified but allowed for legacy compatibility, though their use is
/// discouraged. Applications may interpret such values as protocol errors.
///
/// # Examples
///
/// ```
/// use http::StatusCode;
///
/// assert_eq!(StatusCode::from_u16(200).unwrap(), StatusCode::OK);
/// assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
/// assert!(StatusCode::OK.is_success());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusCode(NonZeroU16);
/// A possible error value when converting a `StatusCode` from a `u16` or `&str`
///
/// This error indicates that the supplied input was not a valid number, was less
/// than 100, or was greater than 999.
pub struct InvalidStatusCode {
    _priv: (),
}
impl StatusCode {
    /// Converts a u16 to a status code.
    ///
    /// The function validates the correctness of the supplied u16. It must be
    /// greater or equal to 100 and less than 1000.
    ///
    /// # Example
    ///
    /// ```
    /// use http::StatusCode;
    ///
    /// let ok = StatusCode::from_u16(200).unwrap();
    /// assert_eq!(ok, StatusCode::OK);
    ///
    /// let err = StatusCode::from_u16(99);
    /// assert!(err.is_err());
    /// ```
    #[inline]
    pub fn from_u16(src: u16) -> Result<StatusCode, InvalidStatusCode> {
        if src < 100 || src >= 1000 {
            return Err(InvalidStatusCode::new());
        }
        NonZeroU16::new(src).map(StatusCode).ok_or_else(InvalidStatusCode::new)
    }
    /// Converts a &[u8] to a status code
    pub fn from_bytes(src: &[u8]) -> Result<StatusCode, InvalidStatusCode> {
        if src.len() != 3 {
            return Err(InvalidStatusCode::new());
        }
        let a = src[0].wrapping_sub(b'0') as u16;
        let b = src[1].wrapping_sub(b'0') as u16;
        let c = src[2].wrapping_sub(b'0') as u16;
        if a == 0 || a > 9 || b > 9 || c > 9 {
            return Err(InvalidStatusCode::new());
        }
        let status = (a * 100) + (b * 10) + c;
        NonZeroU16::new(status).map(StatusCode).ok_or_else(InvalidStatusCode::new)
    }
    /// Returns the `u16` corresponding to this `StatusCode`.
    ///
    /// # Note
    ///
    /// This is the same as the `From<StatusCode>` implementation, but
    /// included as an inherent method because that implementation doesn't
    /// appear in rustdocs, as well as a way to force the type instead of
    /// relying on inference.
    ///
    /// # Example
    ///
    /// ```
    /// let status = http::StatusCode::OK;
    /// assert_eq!(status.as_u16(), 200);
    /// ```
    #[inline]
    pub fn as_u16(&self) -> u16 {
        (*self).into()
    }
    /// Returns a &str representation of the `StatusCode`
    ///
    /// The return value only includes a numerical representation of the
    /// status code. The canonical reason is not included.
    ///
    /// # Example
    ///
    /// ```
    /// let status = http::StatusCode::OK;
    /// assert_eq!(status.as_str(), "200");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        let offset = (self.0.get() - 100) as usize;
        let offset = offset * 3;
        #[cfg(debug_assertions)] { &CODE_DIGITS[offset..offset + 3] }
        #[cfg(not(debug_assertions))]
        unsafe { CODE_DIGITS.get_unchecked(offset..offset + 3) }
    }
    /// Get the standardised `reason-phrase` for this status code.
    ///
    /// This is mostly here for servers writing responses, but could potentially have application
    /// at other times.
    ///
    /// The reason phrase is defined as being exclusively for human readers. You should avoid
    /// deriving any meaning from it at all costs.
    ///
    /// Bear in mind also that in HTTP/2.0 and HTTP/3.0 the reason phrase is abolished from
    /// transmission, and so this canonical reason phrase really is the only reason phrase you’ll
    /// find.
    ///
    /// # Example
    ///
    /// ```
    /// let status = http::StatusCode::OK;
    /// assert_eq!(status.canonical_reason(), Some("OK"));
    /// ```
    pub fn canonical_reason(&self) -> Option<&'static str> {
        canonical_reason(self.0.get())
    }
    /// Check if status is within 100-199.
    #[inline]
    pub fn is_informational(&self) -> bool {
        200 > self.0.get() && self.0.get() >= 100
    }
    /// Check if status is within 200-299.
    #[inline]
    pub fn is_success(&self) -> bool {
        300 > self.0.get() && self.0.get() >= 200
    }
    /// Check if status is within 300-399.
    #[inline]
    pub fn is_redirection(&self) -> bool {
        400 > self.0.get() && self.0.get() >= 300
    }
    /// Check if status is within 400-499.
    #[inline]
    pub fn is_client_error(&self) -> bool {
        500 > self.0.get() && self.0.get() >= 400
    }
    /// Check if status is within 500-599.
    #[inline]
    pub fn is_server_error(&self) -> bool {
        600 > self.0.get() && self.0.get() >= 500
    }
}
impl fmt::Debug for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
/// Formats the status code, *including* the canonical reason.
///
/// # Example
///
/// ```
/// # use http::StatusCode;
/// assert_eq!(format!("{}", StatusCode::OK), "200 OK");
/// ```
impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "{} {}", u16::from(* self), self.canonical_reason()
            .unwrap_or("<unknown status code>")
        )
    }
}
impl Default for StatusCode {
    #[inline]
    fn default() -> StatusCode {
        StatusCode::OK
    }
}
impl PartialEq<u16> for StatusCode {
    #[inline]
    fn eq(&self, other: &u16) -> bool {
        self.as_u16() == *other
    }
}
impl PartialEq<StatusCode> for u16 {
    #[inline]
    fn eq(&self, other: &StatusCode) -> bool {
        *self == other.as_u16()
    }
}
impl From<StatusCode> for u16 {
    #[inline]
    fn from(status: StatusCode) -> u16 {
        status.0.get()
    }
}
impl FromStr for StatusCode {
    type Err = InvalidStatusCode;
    fn from_str(s: &str) -> Result<StatusCode, InvalidStatusCode> {
        StatusCode::from_bytes(s.as_ref())
    }
}
impl<'a> From<&'a StatusCode> for StatusCode {
    #[inline]
    fn from(t: &'a StatusCode) -> Self {
        t.clone()
    }
}
impl<'a> TryFrom<&'a [u8]> for StatusCode {
    type Error = InvalidStatusCode;
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        StatusCode::from_bytes(t)
    }
}
impl<'a> TryFrom<&'a str> for StatusCode {
    type Error = InvalidStatusCode;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}
impl TryFrom<u16> for StatusCode {
    type Error = InvalidStatusCode;
    #[inline]
    fn try_from(t: u16) -> Result<Self, Self::Error> {
        StatusCode::from_u16(t)
    }
}
macro_rules! status_codes {
    ($($(#[$docs:meta])* ($num:expr, $konst:ident, $phrase:expr);)+) => {
        impl StatusCode { $($(#[$docs])* pub const $konst : StatusCode =
        StatusCode(unsafe { NonZeroU16::new_unchecked($num) });)+ } fn
        canonical_reason(num : u16) -> Option <&'static str > { match num { $($num =>
        Some($phrase),)+ _ => None } }
    };
}
status_codes! {
    #[doc = " 100 Continue"] #[doc =
    " [[RFC7231, Section 6.2.1](https://tools.ietf.org/html/rfc7231#section-6.2.1)]"]
    (100, CONTINUE, "Continue"); #[doc = " 101 Switching Protocols"] #[doc =
    " [[RFC7231, Section 6.2.2](https://tools.ietf.org/html/rfc7231#section-6.2.2)]"]
    (101, SWITCHING_PROTOCOLS, "Switching Protocols"); #[doc = " 102 Processing"] #[doc =
    " [[RFC2518](https://tools.ietf.org/html/rfc2518)]"] (102, PROCESSING, "Processing");
    #[doc = " 200 OK"] #[doc =
    " [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]"]
    (200, OK, "OK"); #[doc = " 201 Created"] #[doc =
    " [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]"]
    (201, CREATED, "Created"); #[doc = " 202 Accepted"] #[doc =
    " [[RFC7231, Section 6.3.3](https://tools.ietf.org/html/rfc7231#section-6.3.3)]"]
    (202, ACCEPTED, "Accepted"); #[doc = " 203 Non-Authoritative Information"] #[doc =
    " [[RFC7231, Section 6.3.4](https://tools.ietf.org/html/rfc7231#section-6.3.4)]"]
    (203, NON_AUTHORITATIVE_INFORMATION, "Non Authoritative Information"); #[doc =
    " 204 No Content"] #[doc =
    " [[RFC7231, Section 6.3.5](https://tools.ietf.org/html/rfc7231#section-6.3.5)]"]
    (204, NO_CONTENT, "No Content"); #[doc = " 205 Reset Content"] #[doc =
    " [[RFC7231, Section 6.3.6](https://tools.ietf.org/html/rfc7231#section-6.3.6)]"]
    (205, RESET_CONTENT, "Reset Content"); #[doc = " 206 Partial Content"] #[doc =
    " [[RFC7233, Section 4.1](https://tools.ietf.org/html/rfc7233#section-4.1)]"] (206,
    PARTIAL_CONTENT, "Partial Content"); #[doc = " 207 Multi-Status"] #[doc =
    " [[RFC4918](https://tools.ietf.org/html/rfc4918)]"] (207, MULTI_STATUS,
    "Multi-Status"); #[doc = " 208 Already Reported"] #[doc =
    " [[RFC5842](https://tools.ietf.org/html/rfc5842)]"] (208, ALREADY_REPORTED,
    "Already Reported"); #[doc = " 226 IM Used"] #[doc =
    " [[RFC3229](https://tools.ietf.org/html/rfc3229)]"] (226, IM_USED, "IM Used"); #[doc
    = " 300 Multiple Choices"] #[doc =
    " [[RFC7231, Section 6.4.1](https://tools.ietf.org/html/rfc7231#section-6.4.1)]"]
    (300, MULTIPLE_CHOICES, "Multiple Choices"); #[doc = " 301 Moved Permanently"] #[doc
    = " [[RFC7231, Section 6.4.2](https://tools.ietf.org/html/rfc7231#section-6.4.2)]"]
    (301, MOVED_PERMANENTLY, "Moved Permanently"); #[doc = " 302 Found"] #[doc =
    " [[RFC7231, Section 6.4.3](https://tools.ietf.org/html/rfc7231#section-6.4.3)]"]
    (302, FOUND, "Found"); #[doc = " 303 See Other"] #[doc =
    " [[RFC7231, Section 6.4.4](https://tools.ietf.org/html/rfc7231#section-6.4.4)]"]
    (303, SEE_OTHER, "See Other"); #[doc = " 304 Not Modified"] #[doc =
    " [[RFC7232, Section 4.1](https://tools.ietf.org/html/rfc7232#section-4.1)]"] (304,
    NOT_MODIFIED, "Not Modified"); #[doc = " 305 Use Proxy"] #[doc =
    " [[RFC7231, Section 6.4.5](https://tools.ietf.org/html/rfc7231#section-6.4.5)]"]
    (305, USE_PROXY, "Use Proxy"); #[doc = " 307 Temporary Redirect"] #[doc =
    " [[RFC7231, Section 6.4.7](https://tools.ietf.org/html/rfc7231#section-6.4.7)]"]
    (307, TEMPORARY_REDIRECT, "Temporary Redirect"); #[doc = " 308 Permanent Redirect"]
    #[doc = " [[RFC7238](https://tools.ietf.org/html/rfc7238)]"] (308,
    PERMANENT_REDIRECT, "Permanent Redirect"); #[doc = " 400 Bad Request"] #[doc =
    " [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]"]
    (400, BAD_REQUEST, "Bad Request"); #[doc = " 401 Unauthorized"] #[doc =
    " [[RFC7235, Section 3.1](https://tools.ietf.org/html/rfc7235#section-3.1)]"] (401,
    UNAUTHORIZED, "Unauthorized"); #[doc = " 402 Payment Required"] #[doc =
    " [[RFC7231, Section 6.5.2](https://tools.ietf.org/html/rfc7231#section-6.5.2)]"]
    (402, PAYMENT_REQUIRED, "Payment Required"); #[doc = " 403 Forbidden"] #[doc =
    " [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]"]
    (403, FORBIDDEN, "Forbidden"); #[doc = " 404 Not Found"] #[doc =
    " [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]"]
    (404, NOT_FOUND, "Not Found"); #[doc = " 405 Method Not Allowed"] #[doc =
    " [[RFC7231, Section 6.5.5](https://tools.ietf.org/html/rfc7231#section-6.5.5)]"]
    (405, METHOD_NOT_ALLOWED, "Method Not Allowed"); #[doc = " 406 Not Acceptable"] #[doc
    = " [[RFC7231, Section 6.5.6](https://tools.ietf.org/html/rfc7231#section-6.5.6)]"]
    (406, NOT_ACCEPTABLE, "Not Acceptable"); #[doc =
    " 407 Proxy Authentication Required"] #[doc =
    " [[RFC7235, Section 3.2](https://tools.ietf.org/html/rfc7235#section-3.2)]"] (407,
    PROXY_AUTHENTICATION_REQUIRED, "Proxy Authentication Required"); #[doc =
    " 408 Request Timeout"] #[doc =
    " [[RFC7231, Section 6.5.7](https://tools.ietf.org/html/rfc7231#section-6.5.7)]"]
    (408, REQUEST_TIMEOUT, "Request Timeout"); #[doc = " 409 Conflict"] #[doc =
    " [[RFC7231, Section 6.5.8](https://tools.ietf.org/html/rfc7231#section-6.5.8)]"]
    (409, CONFLICT, "Conflict"); #[doc = " 410 Gone"] #[doc =
    " [[RFC7231, Section 6.5.9](https://tools.ietf.org/html/rfc7231#section-6.5.9)]"]
    (410, GONE, "Gone"); #[doc = " 411 Length Required"] #[doc =
    " [[RFC7231, Section 6.5.10](https://tools.ietf.org/html/rfc7231#section-6.5.10)]"]
    (411, LENGTH_REQUIRED, "Length Required"); #[doc = " 412 Precondition Failed"] #[doc
    = " [[RFC7232, Section 4.2](https://tools.ietf.org/html/rfc7232#section-4.2)]"] (412,
    PRECONDITION_FAILED, "Precondition Failed"); #[doc = " 413 Payload Too Large"] #[doc
    = " [[RFC7231, Section 6.5.11](https://tools.ietf.org/html/rfc7231#section-6.5.11)]"]
    (413, PAYLOAD_TOO_LARGE, "Payload Too Large"); #[doc = " 414 URI Too Long"] #[doc =
    " [[RFC7231, Section 6.5.12](https://tools.ietf.org/html/rfc7231#section-6.5.12)]"]
    (414, URI_TOO_LONG, "URI Too Long"); #[doc = " 415 Unsupported Media Type"] #[doc =
    " [[RFC7231, Section 6.5.13](https://tools.ietf.org/html/rfc7231#section-6.5.13)]"]
    (415, UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type"); #[doc =
    " 416 Range Not Satisfiable"] #[doc =
    " [[RFC7233, Section 4.4](https://tools.ietf.org/html/rfc7233#section-4.4)]"] (416,
    RANGE_NOT_SATISFIABLE, "Range Not Satisfiable"); #[doc = " 417 Expectation Failed"]
    #[doc =
    " [[RFC7231, Section 6.5.14](https://tools.ietf.org/html/rfc7231#section-6.5.14)]"]
    (417, EXPECTATION_FAILED, "Expectation Failed"); #[doc = " 418 I'm a teapot"] #[doc =
    " [curiously not registered by IANA but [RFC2324](https://tools.ietf.org/html/rfc2324)]"]
    (418, IM_A_TEAPOT, "I'm a teapot"); #[doc = " 421 Misdirected Request"] #[doc =
    " [RFC7540, Section 9.1.2](http://tools.ietf.org/html/rfc7540#section-9.1.2)"] (421,
    MISDIRECTED_REQUEST, "Misdirected Request"); #[doc = " 422 Unprocessable Entity"]
    #[doc = " [[RFC4918](https://tools.ietf.org/html/rfc4918)]"] (422,
    UNPROCESSABLE_ENTITY, "Unprocessable Entity"); #[doc = " 423 Locked"] #[doc =
    " [[RFC4918](https://tools.ietf.org/html/rfc4918)]"] (423, LOCKED, "Locked"); #[doc =
    " 424 Failed Dependency"] #[doc =
    " [[RFC4918](https://tools.ietf.org/html/rfc4918)]"] (424, FAILED_DEPENDENCY,
    "Failed Dependency"); #[doc = " 426 Upgrade Required"] #[doc =
    " [[RFC7231, Section 6.5.15](https://tools.ietf.org/html/rfc7231#section-6.5.15)]"]
    (426, UPGRADE_REQUIRED, "Upgrade Required"); #[doc = " 428 Precondition Required"]
    #[doc = " [[RFC6585](https://tools.ietf.org/html/rfc6585)]"] (428,
    PRECONDITION_REQUIRED, "Precondition Required"); #[doc = " 429 Too Many Requests"]
    #[doc = " [[RFC6585](https://tools.ietf.org/html/rfc6585)]"] (429, TOO_MANY_REQUESTS,
    "Too Many Requests"); #[doc = " 431 Request Header Fields Too Large"] #[doc =
    " [[RFC6585](https://tools.ietf.org/html/rfc6585)]"] (431,
    REQUEST_HEADER_FIELDS_TOO_LARGE, "Request Header Fields Too Large"); #[doc =
    " 451 Unavailable For Legal Reasons"] #[doc =
    " [[RFC7725](http://tools.ietf.org/html/rfc7725)]"] (451,
    UNAVAILABLE_FOR_LEGAL_REASONS, "Unavailable For Legal Reasons"); #[doc =
    " 500 Internal Server Error"] #[doc =
    " [[RFC7231, Section 6.6.1](https://tools.ietf.org/html/rfc7231#section-6.6.1)]"]
    (500, INTERNAL_SERVER_ERROR, "Internal Server Error"); #[doc =
    " 501 Not Implemented"] #[doc =
    " [[RFC7231, Section 6.6.2](https://tools.ietf.org/html/rfc7231#section-6.6.2)]"]
    (501, NOT_IMPLEMENTED, "Not Implemented"); #[doc = " 502 Bad Gateway"] #[doc =
    " [[RFC7231, Section 6.6.3](https://tools.ietf.org/html/rfc7231#section-6.6.3)]"]
    (502, BAD_GATEWAY, "Bad Gateway"); #[doc = " 503 Service Unavailable"] #[doc =
    " [[RFC7231, Section 6.6.4](https://tools.ietf.org/html/rfc7231#section-6.6.4)]"]
    (503, SERVICE_UNAVAILABLE, "Service Unavailable"); #[doc = " 504 Gateway Timeout"]
    #[doc =
    " [[RFC7231, Section 6.6.5](https://tools.ietf.org/html/rfc7231#section-6.6.5)]"]
    (504, GATEWAY_TIMEOUT, "Gateway Timeout"); #[doc = " 505 HTTP Version Not Supported"]
    #[doc =
    " [[RFC7231, Section 6.6.6](https://tools.ietf.org/html/rfc7231#section-6.6.6)]"]
    (505, HTTP_VERSION_NOT_SUPPORTED, "HTTP Version Not Supported"); #[doc =
    " 506 Variant Also Negotiates"] #[doc =
    " [[RFC2295](https://tools.ietf.org/html/rfc2295)]"] (506, VARIANT_ALSO_NEGOTIATES,
    "Variant Also Negotiates"); #[doc = " 507 Insufficient Storage"] #[doc =
    " [[RFC4918](https://tools.ietf.org/html/rfc4918)]"] (507, INSUFFICIENT_STORAGE,
    "Insufficient Storage"); #[doc = " 508 Loop Detected"] #[doc =
    " [[RFC5842](https://tools.ietf.org/html/rfc5842)]"] (508, LOOP_DETECTED,
    "Loop Detected"); #[doc = " 510 Not Extended"] #[doc =
    " [[RFC2774](https://tools.ietf.org/html/rfc2774)]"] (510, NOT_EXTENDED,
    "Not Extended"); #[doc = " 511 Network Authentication Required"] #[doc =
    " [[RFC6585](https://tools.ietf.org/html/rfc6585)]"] (511,
    NETWORK_AUTHENTICATION_REQUIRED, "Network Authentication Required");
}
impl InvalidStatusCode {
    fn new() -> InvalidStatusCode {
        InvalidStatusCode { _priv: () }
    }
}
impl fmt::Debug for InvalidStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InvalidStatusCode").finish()
    }
}
impl fmt::Display for InvalidStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid status code")
    }
}
impl Error for InvalidStatusCode {}
const CODE_DIGITS: &'static str = "\
100101102103104105106107108109110111112113114115116117118119\
120121122123124125126127128129130131132133134135136137138139\
140141142143144145146147148149150151152153154155156157158159\
160161162163164165166167168169170171172173174175176177178179\
180181182183184185186187188189190191192193194195196197198199\
200201202203204205206207208209210211212213214215216217218219\
220221222223224225226227228229230231232233234235236237238239\
240241242243244245246247248249250251252253254255256257258259\
260261262263264265266267268269270271272273274275276277278279\
280281282283284285286287288289290291292293294295296297298299\
300301302303304305306307308309310311312313314315316317318319\
320321322323324325326327328329330331332333334335336337338339\
340341342343344345346347348349350351352353354355356357358359\
360361362363364365366367368369370371372373374375376377378379\
380381382383384385386387388389390391392393394395396397398399\
400401402403404405406407408409410411412413414415416417418419\
420421422423424425426427428429430431432433434435436437438439\
440441442443444445446447448449450451452453454455456457458459\
460461462463464465466467468469470471472473474475476477478479\
480481482483484485486487488489490491492493494495496497498499\
500501502503504505506507508509510511512513514515516517518519\
520521522523524525526527528529530531532533534535536537538539\
540541542543544545546547548549550551552553554555556557558559\
560561562563564565566567568569570571572573574575576577578579\
580581582583584585586587588589590591592593594595596597598599\
600601602603604605606607608609610611612613614615616617618619\
620621622623624625626627628629630631632633634635636637638639\
640641642643644645646647648649650651652653654655656657658659\
660661662663664665666667668669670671672673674675676677678679\
680681682683684685686687688689690691692693694695696697698699\
700701702703704705706707708709710711712713714715716717718719\
720721722723724725726727728729730731732733734735736737738739\
740741742743744745746747748749750751752753754755756757758759\
760761762763764765766767768769770771772773774775776777778779\
780781782783784785786787788789790791792793794795796797798799\
800801802803804805806807808809810811812813814815816817818819\
820821822823824825826827828829830831832833834835836837838839\
840841842843844845846847848849850851852853854855856857858859\
860861862863864865866867868869870871872873874875876877878879\
880881882883884885886887888889890891892893894895896897898899\
900901902903904905906907908909910911912913914915916917918919\
920921922923924925926927928929930931932933934935936937938939\
940941942943944945946947948949950951952953954955956957958959\
960961962963964965966967968969970971972973974975976977978979\
980981982983984985986987988989990991992993994995996997998999";
#[cfg(test)]
mod tests_llm_16_215 {
    use super::*;
    use crate::*;
    use std::convert::TryInto;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_215_rrrruuuugggg_test_eq = 0;
        let rug_fuzz_0 = 200;
        let rug_fuzz_1 = 404;
        let status = StatusCode::OK;
        let code: u16 = rug_fuzz_0;
        debug_assert_eq!(status.eq(& code), true);
        let code: u16 = rug_fuzz_1;
        debug_assert_eq!(status.eq(& code), false);
        let _rug_ed_tests_llm_16_215_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_216 {
    use crate::StatusCode;
    #[test]
    fn test_from() {
        let _rug_st_tests_llm_16_216_rrrruuuugggg_test_from = 0;
        let status_code = StatusCode::OK;
        let result = StatusCode::from(&status_code);
        debug_assert_eq!(result, StatusCode::OK);
        let _rug_ed_tests_llm_16_216_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_218_llm_16_217 {
    use crate::status::StatusCode;
    use std::convert::TryFrom;
    use crate::status::InvalidStatusCode;
    #[test]
    fn test_try_from() {
        let _rug_st_tests_llm_16_218_llm_16_217_rrrruuuugggg_test_try_from = 0;
        let rug_fuzz_0 = b"200";
        let bytes = rug_fuzz_0;
        let result: Result<StatusCode, InvalidStatusCode> = StatusCode::try_from(
            &bytes[..],
        );
        debug_assert_eq!(result.unwrap(), StatusCode::OK);
        let _rug_ed_tests_llm_16_218_llm_16_217_rrrruuuugggg_test_try_from = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_223 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default() {
        let _rug_st_tests_llm_16_223_rrrruuuugggg_test_default = 0;
        let default_status_code: StatusCode = StatusCode::default();
        debug_assert_eq!(default_status_code, StatusCode::OK);
        let _rug_ed_tests_llm_16_223_rrrruuuugggg_test_default = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_524 {
    use crate::status::StatusCode;
    #[test]
    fn test_eq() {
        let _rug_st_tests_llm_16_524_rrrruuuugggg_test_eq = 0;
        let code1 = StatusCode::OK;
        let code2 = StatusCode::ACCEPTED;
        debug_assert_eq!(code1.eq(& code2), false);
        let _rug_ed_tests_llm_16_524_rrrruuuugggg_test_eq = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_528 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_str() {
        let _rug_st_tests_llm_16_528_rrrruuuugggg_test_as_str = 0;
        let status = StatusCode::OK;
        debug_assert_eq!(status.as_str(), "200");
        let _rug_ed_tests_llm_16_528_rrrruuuugggg_test_as_str = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_529 {
    use super::*;
    use crate::*;
    #[test]
    fn test_as_u16() {
        let _rug_st_tests_llm_16_529_rrrruuuugggg_test_as_u16 = 0;
        let status = StatusCode::OK;
        debug_assert_eq!(status.as_u16(), 200);
        let _rug_ed_tests_llm_16_529_rrrruuuugggg_test_as_u16 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_530 {
    use crate::StatusCode;
    #[test]
    fn test_canonical_reason() {
        let _rug_st_tests_llm_16_530_rrrruuuugggg_test_canonical_reason = 0;
        let rug_fuzz_0 = 200;
        let status = StatusCode::from_u16(rug_fuzz_0).unwrap();
        debug_assert_eq!(status.canonical_reason(), Some("OK"));
        let _rug_ed_tests_llm_16_530_rrrruuuugggg_test_canonical_reason = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_531 {
    use super::*;
    use crate::*;
    use crate::status::StatusCode;
    #[test]
    fn test_from_bytes_valid_status_code() {
        let _rug_st_tests_llm_16_531_rrrruuuugggg_test_from_bytes_valid_status_code = 0;
        let rug_fuzz_0 = b'2';
        let rug_fuzz_1 = b'0';
        let rug_fuzz_2 = b'0';
        let bytes = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        debug_assert_eq!(StatusCode::from_bytes(& bytes[..]).unwrap(), StatusCode::OK);
        let _rug_ed_tests_llm_16_531_rrrruuuugggg_test_from_bytes_valid_status_code = 0;
    }
    #[test]
    fn test_from_bytes_invalid_status_code() {
        let _rug_st_tests_llm_16_531_rrrruuuugggg_test_from_bytes_invalid_status_code = 0;
        let rug_fuzz_0 = b'2';
        let rug_fuzz_1 = b'0';
        let rug_fuzz_2 = b'x';
        let bytes = [rug_fuzz_0, rug_fuzz_1, rug_fuzz_2];
        debug_assert!(StatusCode::from_bytes(& bytes[..]).is_err());
        let _rug_ed_tests_llm_16_531_rrrruuuugggg_test_from_bytes_invalid_status_code = 0;
    }
    #[test]
    fn test_from_bytes_invalid_status_code_length() {
        let _rug_st_tests_llm_16_531_rrrruuuugggg_test_from_bytes_invalid_status_code_length = 0;
        let rug_fuzz_0 = b'2';
        let rug_fuzz_1 = b'0';
        let bytes = [rug_fuzz_0, rug_fuzz_1];
        debug_assert!(StatusCode::from_bytes(& bytes[..]).is_err());
        let _rug_ed_tests_llm_16_531_rrrruuuugggg_test_from_bytes_invalid_status_code_length = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_534 {
    use crate::status::StatusCode;
    #[test]
    fn test_is_client_error() {
        let _rug_st_tests_llm_16_534_rrrruuuugggg_test_is_client_error = 0;
        let status_code = StatusCode::BAD_REQUEST;
        debug_assert!(status_code.is_client_error());
        let status_code = StatusCode::OK;
        debug_assert!(! status_code.is_client_error());
        let _rug_ed_tests_llm_16_534_rrrruuuugggg_test_is_client_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_535 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_informational() {
        let _rug_st_tests_llm_16_535_rrrruuuugggg_test_is_informational = 0;
        let rug_fuzz_0 = 100;
        let rug_fuzz_1 = 199;
        let rug_fuzz_2 = 200;
        let rug_fuzz_3 = 300;
        let status = StatusCode::from_u16(rug_fuzz_0).unwrap();
        debug_assert_eq!(status.is_informational(), true);
        let status = StatusCode::from_u16(rug_fuzz_1).unwrap();
        debug_assert_eq!(status.is_informational(), true);
        let status = StatusCode::from_u16(rug_fuzz_2).unwrap();
        debug_assert_eq!(status.is_informational(), false);
        let status = StatusCode::from_u16(rug_fuzz_3).unwrap();
        debug_assert_eq!(status.is_informational(), false);
        let _rug_ed_tests_llm_16_535_rrrruuuugggg_test_is_informational = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_536 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_redirection() {
        let _rug_st_tests_llm_16_536_rrrruuuugggg_test_is_redirection = 0;
        let rug_fuzz_0 = 300;
        let rug_fuzz_1 = 200;
        let rug_fuzz_2 = 400;
        let status = StatusCode::from_u16(rug_fuzz_0).unwrap();
        debug_assert_eq!(status.is_redirection(), true);
        let status = StatusCode::from_u16(rug_fuzz_1).unwrap();
        debug_assert_eq!(status.is_redirection(), false);
        let status = StatusCode::from_u16(rug_fuzz_2).unwrap();
        debug_assert_eq!(status.is_redirection(), false);
        let _rug_ed_tests_llm_16_536_rrrruuuugggg_test_is_redirection = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_537 {
    use super::*;
    use crate::*;
    use std::num::NonZeroU16;
    #[test]
    fn test_is_server_error() {
        let _rug_st_tests_llm_16_537_rrrruuuugggg_test_is_server_error = 0;
        let rug_fuzz_0 = 500;
        let rug_fuzz_1 = 599;
        let rug_fuzz_2 = 300;
        let rug_fuzz_3 = 400;
        let rug_fuzz_4 = 200;
        let rug_fuzz_5 = 600;
        let status = StatusCode(NonZeroU16::new(rug_fuzz_0).unwrap());
        debug_assert_eq!(status.is_server_error(), true);
        let status = StatusCode(NonZeroU16::new(rug_fuzz_1).unwrap());
        debug_assert_eq!(status.is_server_error(), true);
        let status = StatusCode(NonZeroU16::new(rug_fuzz_2).unwrap());
        debug_assert_eq!(status.is_server_error(), false);
        let status = StatusCode(NonZeroU16::new(rug_fuzz_3).unwrap());
        debug_assert_eq!(status.is_server_error(), false);
        let status = StatusCode(NonZeroU16::new(rug_fuzz_4).unwrap());
        debug_assert_eq!(status.is_server_error(), false);
        let status = StatusCode(NonZeroU16::new(rug_fuzz_5).unwrap());
        debug_assert_eq!(status.is_server_error(), false);
        let _rug_ed_tests_llm_16_537_rrrruuuugggg_test_is_server_error = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_538 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_success() {
        let _rug_st_tests_llm_16_538_rrrruuuugggg_test_is_success = 0;
        let status_code = StatusCode::OK;
        debug_assert_eq!(status_code.is_success(), true);
        let status_code = StatusCode::MOVED_PERMANENTLY;
        debug_assert_eq!(status_code.is_success(), false);
        let status_code = StatusCode::INTERNAL_SERVER_ERROR;
        debug_assert_eq!(status_code.is_success(), false);
        let _rug_ed_tests_llm_16_538_rrrruuuugggg_test_is_success = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_539 {
    use super::*;
    use crate::*;
    use crate::status;
    #[test]
    fn test_canonical_reason() {
        let _rug_st_tests_llm_16_539_rrrruuuugggg_test_canonical_reason = 0;
        let rug_fuzz_0 = 200;
        let rug_fuzz_1 = 404;
        let rug_fuzz_2 = 500;
        let rug_fuzz_3 = 999;
        debug_assert_eq!(status::canonical_reason(rug_fuzz_0), Some("OK"));
        debug_assert_eq!(status::canonical_reason(rug_fuzz_1), Some("Not Found"));
        debug_assert_eq!(
            status::canonical_reason(rug_fuzz_2), Some("Internal Server Error")
        );
        debug_assert_eq!(status::canonical_reason(rug_fuzz_3), None);
        let _rug_ed_tests_llm_16_539_rrrruuuugggg_test_canonical_reason = 0;
    }
}
#[cfg(test)]
mod tests_rug_139 {
    use super::*;
    use crate::status::{StatusCode, InvalidStatusCode};
    #[test]
    fn test_from_u16() {
        let _rug_st_tests_rug_139_rrrruuuugggg_test_from_u16 = 0;
        let rug_fuzz_0 = 200;
        let mut p0: u16 = rug_fuzz_0;
        StatusCode::from_u16(p0).unwrap();
        let _rug_ed_tests_rug_139_rrrruuuugggg_test_from_u16 = 0;
    }
}
#[cfg(test)]
mod tests_rug_140 {
    use super::*;
    use crate::status::StatusCode;
    use std::convert::From;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_140_rrrruuuugggg_test_from = 0;
        let p0: StatusCode = StatusCode::OK;
        <u16 as From<StatusCode>>::from(p0);
        let _rug_ed_tests_rug_140_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_143 {
    use super::*;
    use crate::status::StatusCode;
    use std::convert::TryFrom;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_143_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = 200;
        let mut p0: u16 = rug_fuzz_0;
        <StatusCode as std::convert::TryFrom<u16>>::try_from(p0);
        let _rug_ed_tests_rug_143_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_144 {
    use super::*;
    use crate::status;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_144_rrrruuuugggg_test_rug = 0;
        <status::InvalidStatusCode>::new();
        let _rug_ed_tests_rug_144_rrrruuuugggg_test_rug = 0;
    }
}
