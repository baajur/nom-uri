use super::*;
use nom::{
    branch::*, bytes::complete::*, character::complete::*, character::*, combinator::*,
    error::ErrorKind, multi::*, number::complete::*, sequence::*, IResult,
};
macro_rules! fold_closure {
    ($i:ident, $pos:ident) => {
        if peek(pct_encoded)($i.split_at($pos).1).is_ok() {
            $pos + 3
        } else {
            $pos + 1
        }
    };
}
// http://www.faqs.org/rfcs/rfc3986.html
// Appendix A.  Collected ABNF for URI

// URI           = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
// hier-part     = "//" authority path-abempty
//               / path-absolute
//               / path-rootless
//               / path-empty
// URI-reference = URI / relative-ref
// absolute-URI  = scheme ":" hier-part [ "?" query ]
// relative-ref  = relative-part [ "?" query ] [ "#" fragment ]
// relative-part = "//" authority path-abempty
//               / path-absolute
//               / path-noscheme
//               / path-empty
/// scheme        = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
fn scheme(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, (_, position)) = pair(
        alpha,
        fold_many0(
            alt((alphanumeric, one_of("+-."))),
            0,
            |mut pos: usize, _| {
                pos = fold_closure!(i, pos);
                pos
            },
        ),
    )(i)?;
    let (o, i) = i.split_at(position + 1); // one alpha at the start
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// authority     = [ userinfo "@" ] host [ ":" port ]
fn authotity(i: &[u8]) -> IResult<&[u8], Authority> {
    let (_, (user_info, hos_t, por_t)) =
        tuple((opt(userinfo), host, opt(preceded(char(':'), port))))(i)?;
    let auth = Authority {
        userinfo: user_info,
        host: hos_t,
        port: por_t.flatten(),
    };
    Ok((i, auth))
}
/// userinfo      = *( unreserved / pct-encoded / sub-delims / ":" )
fn userinfo(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, position) = fold_many1(
        alt((alt((unreserved, pct_encoded)), alt((sub_delims, char(':'))))),
        0,
        |mut pos: usize, _| {
            pos = fold_closure!(i, pos);
            pos
        },
    )(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// host          = IP-literal / IPv4address / reg-name
fn host(i: &[u8]) -> IResult<&[u8], Host> {
    alt((alt((ip_literal, ip_v4_address)), reg_name))(i)
}
/// port          = *DIGIT
fn port(i: &[u8]) -> IResult<&[u8], Option<u16>> {
    let (rest, o) = digit0(i)?;
    if o.len() == 0 {
        // port can be empty
        return Ok((i, None));
    };
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    let o = match u16::from_str_radix(o, 10) {
        // u16 max_value() = port_max => no extra value check
        Err(_) => return Err(nom::Err::Error((i, ErrorKind::Digit))),
        Ok(port) => port,
    };
    Ok((rest, Some(o)))
}
/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
fn ip_literal(i: &[u8]) -> IResult<&[u8], Host> {
    let (rest, (_, ip, _)) = tuple((char('['), alt((ip_v_future, ip_v6_address)), char(']')))(i)?;
    Ok((rest, ip))
}
/// IPvFuture     = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
/// Unimplemented!
fn ip_v_future(i: &[u8]) -> IResult<&[u8], Host> {
    unimplemented!();
}
/// IPv6address   =                            6( h16 ":" ) (ls32 / IPv4address)
///               /                       "::" 5( h16 ":" ) (ls32 / IPv4address)
///               / [               h16 ] "::" 4( h16 ":" ) (ls32 / IPv4address)
///               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) (ls32 / IPv4address)
///               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) (ls32 / IPv4address)
///               / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   (ls32 / IPv4address)
///               / [ *4( h16 ":" ) h16 ] "::"              (ls32 / IPv4address)
///               / [ *5( h16 ":" ) h16 ] "::"              h16
///               / [ *6( h16 ":" ) h16 ] "::"
fn ip_v6_address(i: &[u8]) -> IResult<&[u8], Host> {
    unimplemented!();
    //Ok((i, Host::V6(o)))
}
/// ( h16 ":" )
fn h16_colon(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, (o1, _)) = pair(h16, char(':'))(i)?;
    let (o, i) = i.split_at(o1.len() + 1); // one colon
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// (ls32 / IPv4address)
fn ip_v6_end(i: &[u8]) -> IResult<&[u8], &str> {
    match opt(ip_v4_address)(i)? {
        (rest, Some(Host::V4(o))) => Ok((rest, o)),
        (_, None) => Ok(ls32(i)?),
        _ => unreachable!(),
    }
}
/// h16           = 1*4HEXDIG
/// 16 bits of address represented in hexadecimal
fn h16(i: &[u8]) -> IResult<&[u8], &str> {
    let (rest, o) = hex_digit1(i)?;
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    match u16::from_str_radix(o, 16) {
        // u16 max_value() = FFFF => no extra value check
        Err(_) => return Err(nom::Err::Error((i, ErrorKind::Digit))),
        _ => {}
    };
    Ok((rest, o))
}
/// ls32          = ( h16 ":" h16 )
/// least-significant 32 bits of address
/// According to rfc3986 this part can also be an IPv4Address,
/// but we parse that option separatly in ip_v6_end().
fn ls32(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, (o1, _, o2)) = tuple((h16, char(':'), h16))(i)?;
    let (o, i) = i.split_at(o1.len() + o2.len() + 1); // one colon
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
fn ip_v4_address(i: &[u8]) -> IResult<&[u8], Host> {
    let (_, (o1, _, o2, _, o3, _, o4)) = tuple((
        dec_octet,
        char('.'),
        dec_octet,
        char('.'),
        dec_octet,
        char('.'),
        dec_octet,
    ))(i)?;
    let (o, i) = i.split_at(o1.len() + o2.len() + o3.len() + o4.len() + 3); // three dots
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, Host::V4(o)))
}

/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
fn dec_octet(i: &[u8]) -> IResult<&[u8], &str> {
    let (rest, o) = digit1(i)?;
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    match u8::from_str_radix(o, 10) {
        // u8 max_value() = 255 => no extra value check
        Err(_) => return Err(nom::Err::Error((i, ErrorKind::Digit))),
        _ => {}
    };
    Ok((rest, o))
}
/// reg-name      = *( unreserved / pct-encoded / sub-delims )
fn reg_name(i: &[u8]) -> IResult<&[u8], Host> {
    let (_, position) = fold_many1(
        alt((alt((unreserved, pct_encoded)), sub_delims)),
        0,
        |mut pos: usize, _| {
            pos = fold_closure!(i, pos);
            pos
        },
    )(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, Host::RegistryName(o)))
}
/// path          = path-abempty    ; begins with "/" or is empty
///               / path-absolute   ; begins with "/" but not "//"
///               / path-noscheme   ; begins with a non-colon segment
///               / path-rootless   ; begins with a segment
///               / path-empty      ; zero u8acters
fn path(i: &[u8]) -> IResult<&[u8], Path> {
    alt((
        alt((path_abempty, path_absolute)),
        alt((alt((path_noscheme, path_rootless)), path_empty)),
    ))(i)
}
/// path-absolute = "/" [ segment-nz *( "/" segment ) ]
fn path_absolute(i: &[u8]) -> IResult<&[u8], Path> {
    let (rest, (_, segments)) = pair(char('/'), opt(path_rootless))(i)?;
    let segments = match segments {
        Some(Path::Rootless(path)) => path,
        None => "",
        _ => unreachable!(),
    };
    let (o, _) = i.split_at(1 + segments.len());
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((rest, Path::Absolute(o)))
}
/// path-noscheme = segment-nz-nc *( "/" segment )
fn path_noscheme(i: &[u8]) -> IResult<&[u8], Path> {
    let (rest, (nz, segments)) = pair(segment_nz_nc, path_abempty)(i)?;
    let segments = match segments {
        Path::AbEmpty(path) => path,
        _ => unreachable!(),
    };
    let (o, _) = i.split_at(nz.len() + segments.len());
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((rest, Path::NoScheme(o)))
}
/// path-rootless = segment-nz *( "/" segment )
fn path_rootless(i: &[u8]) -> IResult<&[u8], Path> {
    let (rest, (nz, segments)) = pair(segment_nz, path_abempty)(i)?;
    let segments = match segments {
        Path::AbEmpty(path) => path,
        _ => unreachable!(),
    };
    let (o, _) = i.split_at(nz.len() + segments.len());
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((rest, Path::Rootless(o)))
}
/// path-abempty  = *( "/" segment )
fn path_abempty(i: &[u8]) -> IResult<&[u8], Path> {
    let (_, position) = fold_many0(
        preceded(char('/'), cut(segment)),
        0,
        |mut pos: usize, segment| {
            pos += 1 + segment.len(); //add one for the '/'
            pos
        },
    )(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, Path::AbEmpty(o)))
}

/// path-empty    = 0<pchar>
fn path_empty(i: &[u8]) -> IResult<&[u8], Path> {
    not(peek(pchar))(i)?;
    Ok((i, Path::Empty))
}
/// segment       = *pchar
fn segment(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, position) = fold_many0(pchar, 0, |mut pos: usize, _| {
        pos = fold_closure!(i, pos);
        pos
    })(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// segment-nz    = 1*pchar
fn segment_nz(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, position) = fold_many1(pchar, 0, |mut pos: usize, _| {
        pos = fold_closure!(i, pos);
        pos
    })(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
/// non-zero-length segment without any colon ":"
fn segment_nz_nc(i: &[u8]) -> IResult<&[u8], &str> {
    let (_, position) = fold_many1(
        alt((alt((unreserved, pct_encoded)), alt((sub_delims, char('@'))))),
        0,
        |mut pos: usize, _| {
            pos = fold_closure!(i, pos);
            pos
        },
    )(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, o))
}
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
fn pchar(i: &[u8]) -> IResult<&[u8], char> {
    alt((
        alt((unreserved, pct_encoded)),
        alt((sub_delims, one_of(":@"))),
    ))(i)
}
/// query         = *( pchar / "/" / "?" )
fn query(i: &[u8]) -> IResult<&[u8], UriPart> {
    let (_, position) = fold_many0(alt((pchar, one_of("/?"))), 0, |mut pos: usize, _| {
        pos = fold_closure!(i, pos);
        pos
    })(i)?;
    let (o, i) = i.split_at(position);
    let o = unsafe { core::str::from_utf8_unchecked(o) }; // already parsed -> cannot fail
    Ok((i, UriPart::Query(o)))
}
/// fragment      = *( pchar / "/" / "?" )
fn fragment(i: &[u8]) -> IResult<&[u8], UriPart> {
    let (i, o) = match query(i)? {
        (i, UriPart::Query(o)) => (i, o),
        _ => return Err(nom::Err::Error((i, ErrorKind::Many0))), //TODO: What error?
    };
    Ok((i, UriPart::Fragment(o)))
}
/// percentage encoded u32
/// pct-encoded   = "%" HEXDIG HEXDIG
fn pct_encoded(i: &[u8]) -> IResult<&[u8], char> {
    use core::char::from_u32;
    let (i, (high, low)) = preceded(char('%'), pair(hexdig, hexdig))(i)?;
    let hex_val = match hex_u32(&[high as u8, low as u8]) {
        Ok((_, o)) => o,
        Err(e) => match e {
            nom::Err::Incomplete(Needed) => return Err(nom::Err::Incomplete(Needed)),
            nom::Err::Error((_, e)) => return Err(nom::Err::Error((i, e))),
            nom::Err::Failure((_, e)) => return Err(nom::Err::Failure((i, e))),
        },
    };
    let o = match from_u32(hex_val) {
        Some(o) => o,
        None => return Err(nom::Err::Error((i, ErrorKind::HexDigit))),
    };
    Ok((i, o))
}
/// unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
fn unreserved(i: &[u8]) -> IResult<&[u8], char> {
    alt((alphanumeric, one_of("-._~")))(i)
}
/// reserved      = gen-delims / sub-delims
fn reserved(i: &[u8]) -> IResult<&[u8], char> {
    alt((gen_delims, sub_delims))(i)
}
/// gen-delims    = ":" / "/" / "?" / "#" / "[" / "]" / "@"
fn gen_delims(i: &[u8]) -> IResult<&[u8], char> {
    one_of(":/?#[]@")(i)
}
/// sub-delims    = "!" / "$" / "&" / "'" / "(" / ")"
///               / "*" / "+" / "," / ";" / "="
fn sub_delims(i: &[u8]) -> IResult<&[u8], char> {
    one_of("!$&'()*+,;=")(i)
}
fn alphanumeric(i: &[u8]) -> IResult<&[u8], char> {
    alt((alpha, digit))(i)
}
fn alpha(i: &[u8]) -> IResult<&[u8], char> {
    one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")(i)
}
fn digit(i: &[u8]) -> IResult<&[u8], char> {
    one_of("0123456789")(i)
}
fn hexdig(i: &[u8]) -> IResult<&[u8], char> {
    one_of("0123456789ABCDEFabcdef")(i)
}
fn is_hex_digit_u8(i: u8) -> bool {
    is_hex_digit(i as u8)
}
const pchar_no_pct: &[u8] =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~!$&'()*+,;=:@".as_bytes();
#[test]
fn port_test() {
    assert_eq!(port(b""), Ok((&b""[..], None)));
}
#[test]
fn ip_v4_test() {
    assert_eq!(
        ip_v4_address(b"24.4.34"),
        Err(nom::Err::Error((&b""[..], ErrorKind::Char)))
    );
    assert_eq!(
        ip_v4_address(b"256.24.4.34"),
        Err(nom::Err::Error((&b"256.24.4.34"[..], ErrorKind::Digit)))
    );
    assert_eq!(
        ip_v4_address(b"255.255.255.255.255"),
        Ok((&b".255"[..], Host::V4("255.255.255.255")))
    );
    assert_eq!(
        ip_v4_address(b"255.255.255.255"),
        Ok((&b""[..], Host::V4("255.255.255.255")))
    );
    assert_eq!(
        ip_v4_address(b"0.0.0.0"),
        Ok((&b""[..], Host::V4("0.0.0.0")))
    );
}
#[test]
fn path_absolute_test() {
    assert_eq!(
        path_absolute(b"abc/def//"),
        Err(nom::Err::Error((&b"abc/def//"[..], ErrorKind::Char)))
    );
    assert_eq!(
        path_absolute(b"/abc/def//"),
        Ok((&b""[..], Path::Absolute("/abc/def//")))
    );
}
#[test]
fn path_rootless_test() {
    assert_eq!(
        path_rootless(b"/abc/def//"),
        Err(nom::Err::Error((&b"/abc/def//"[..], ErrorKind::Many1)))
    );
    assert_eq!(
        path_rootless(b"abc/def//"),
        Ok((&b""[..], Path::Rootless("abc/def//")))
    );
}
#[test]
fn path_abempty_test() {
    assert_eq!(
        path_abempty(b"/abc/def//"),
        Ok((&[][..], Path::AbEmpty("/abc/def//")))
    );
    assert_eq!(
        path_abempty(b"abc/def//"),
        Ok((&b"abc/def//"[..], Path::AbEmpty("")))
    );
}
#[test]
fn fragment_test() {
    unsafe {
        assert_eq!(
            fragment(pchar_no_pct),
            Ok((
                &[][..],
                UriPart::Fragment(core::str::from_utf8_unchecked(&pchar_no_pct))
            ))
        )
    };
    assert_eq!(fragment(b"/?{"), Ok((&b"{"[..], UriPart::Fragment("/?"))));
    assert_eq!(
        fragment(b"%30%41#"),
        Ok((&b"#"[..], UriPart::Fragment("%30%41")))
    );
}
#[test]
fn pct_encoded_test() {
    assert_eq!(pct_encoded(b"%30*"), Ok((&b"*"[..], '0')));
    assert_eq!(pct_encoded(b"%41g"), Ok((&b"g"[..], 'A')));
    assert_eq!(
        pct_encoded(b"41"),
        Err(nom::Err::Error((&b"41"[..], ErrorKind::Char)))
    );
    assert_eq!(
        pct_encoded(b"%4"),
        Err(nom::Err::Error((&[][..], ErrorKind::OneOf)))
    );
}