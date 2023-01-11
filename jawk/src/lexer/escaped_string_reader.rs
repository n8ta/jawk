use std::iter::Peekable;
use std::str::Chars;
use crate::printable_error::PrintableError;


const ZERO: char = 0x30 as char;
const SEVEN: char = 0x37 as char;
const NINE: char = 0x39 as char;

// Escape chars:
// https://pubs.opengroup.org/onlinepubs/009604499/basedefs/xbd_chap05.html
// AWK specific ones:
// https://pubs.opengroup.org/onlinepubs/009604499/utilities/awk.html "Table: Escape Sequences in awk"
// The starting " is already consumed from the iterator and a non-escaped closing quote must exist or returns Err
pub fn escaped_string_reader(characters: &mut Peekable<Chars>) -> Result<Vec<u8>, PrintableError> {
    let mut output = vec![];
    let mut escaped = false;
    let mut scratch_bytes: [u8; 4] = [0; 4];
    let mut finished = false;
    while let Some(char) = characters.next() {
        if escaped {
            let result: u8 = match char {
                '\\' => 0x5c, // back slash
                '"' => 0x22, // quote
                '/' => 0x2f, // forward slash
                'a' => 0x7,  // alert
                'b' => 0x8,  // backspace
                't' => 0x9,  // tab
                'n' => 0xa,  // new line
                'v' => 0xb,  // vertical tab
                'f' => 0xc,  // form feed
                'r' => 0xd,  // carriage return
                ZERO..=SEVEN => octal_escape(char as u8 - ZERO as u8, characters),
                _ => {
                    return Err(PrintableError::new(format!("\\{} is not a known ask escape sequence", char)));
                }
            };
            output.push(result);
            escaped = false;
        } else {
            if char == '\\' {
                escaped = true;
            } else if char == '\"' {
                finished = true;
                break;
            } else if char == '\n' {
                return Err(PrintableError::new("String literals may not contains a new line"));
            } else {
                let str = char.encode_utf8(&mut scratch_bytes);
                let bytes_used = str.as_bytes().len();
                debug_assert!(bytes_used > 0);
                debug_assert!(bytes_used <= 4);
                output.extend_from_slice(&scratch_bytes[0..bytes_used])
            }
        }
    }
    if !finished {
        return Err(PrintableError::new(format!("Unterminated string literal")));
    }
    Ok(output)
}

// Saturates at 255
fn saturating_octal_parse(char1: u8, char2: u8, char3: u8) -> u8 {
    debug_assert!(char1 <= 7);
    debug_assert!(char2 <= 7);
    debug_assert!(char3 <= 7);
    let c1 = char1.saturating_mul(64);
    let c2 = char2.saturating_mul(8);
    c1.saturating_add(c2).saturating_add(char3)
}

fn next_is_octal(characters: &mut Peekable<Chars>) -> Option<u8> {
    let is_octal = if let Some(peeked) = characters.peek() {
        if (ZERO..=SEVEN).contains(&peeked) {
            true
        } else {
            false
        }
    } else {
        false
    };
    if is_octal { Some(characters.next().unwrap() as u8 - ZERO as u8) } else { None }
}

fn octal_escape(char1: u8, characters: &mut Peekable<Chars>) -> u8 {
    if let Some(char2) = next_is_octal(characters) {
        if let Some(char3) = next_is_octal(characters) {
            saturating_octal_parse(char1, char2, char3)
        } else {
            saturating_octal_parse(0, char1, char2)
        }
    } else {
        saturating_octal_parse(0, 0, char1)
    }
}

#[cfg(test)]
mod string_read_tests {
    use crate::lexer::escaped_string_reader::{escaped_string_reader, saturating_octal_parse};

    #[test]
    fn test_parse_octals() {
        assert_eq!(saturating_octal_parse(0, 0, 7), 7);
        assert_eq!(saturating_octal_parse(0, 0, 0), 0);
        assert_eq!(saturating_octal_parse(0, 1, 3), 11);
        assert_eq!(saturating_octal_parse(0, 2, 3), 19);
        assert_eq!(saturating_octal_parse(0, 7, 7), 63);
        assert_eq!(saturating_octal_parse(3, 7, 7), 255);
        assert_eq!(saturating_octal_parse(3, 7, 7), 255);
        assert_eq!(saturating_octal_parse(4, 0, 0), 255);
    }

    fn test_helper<T: Into<Vec<u8>>>(input: &str, oracle: T, expected_err: &str) {
        let oracle = oracle.into();
        let res = escaped_string_reader(&mut input.chars().peekable());
        match res {
            Ok(result) => assert_eq!(oracle, result),
            Err(err) => {
                if expected_err == "" {
                    assert!(false, "Failed, input {} expected output {:?} but got err {}", input, oracle, expected_err)
                } else {
                    let err_str = format!("{}", err);
                    assert!(err_str.contains(expected_err), "Expected to get an error including msg {} but got: {}", expected_err, err_str)
                }
            }
        }
    }

    macro_rules! test {
        ($name:ident,$input:expr,$oracle:expr,$expected_err:expr) => {
            #[test]
            fn $name() {
                test_helper($input, $oracle, $expected_err);
            }
        };
    }

    // These look a little messed up b/c we're dealing with rust's escaping rules
    // and awk's. Luckily they mostly match!

    // All tests end with a " b/c that's how the lexer knows the string is over.
    test!(test_simple, "abc\"", "abc", "");
    test!(test_spaces, "abc  def\"", "abc  def", "");

    // Escape sequences
    test!(test_line_break, r#"abc\nv""#, "abc\nv", "");
    test!(test_tab, r#"abc\tv""#, "abc\tv", "");
    test!(test_escape, r#"abc\\tv""#, r"abc\tv", "");
    test!(test_alert, r#"\a""#, "\x07", "");
    test!(test_backspace, r#"\b""#, "\x08", "");
    test!(test_linefeed, r#"\f""#, "\x0c", "");
    test!(test_carraige_return, r#"\r""#, "\r", "");
    test!(test_vertical_tab, r#"\v""#, "\x0b", "");
    test!(test_quote_0, r#"\"""#, "\"", "");
    test!(test_quoted_str, r#"\"abc\"""#, "\"abc\"", "");
    test!(test_backslash, r#"\\""#, r"\", "");

    // Octal escapes
    test!(test_octal_escape_0, r#"\1""#, "\x01", "");
    test!(test_octal_escape_1, r#"start\1end""#, "start\x01end", "");
    test!(test_octal_escape_2, r#"\12""#, "\x0a", ""); // Octal 12 = Dec 10 = Hex 0a
test!(test_octal_escape_3, r#"start\123end""#, "start\x53end", ""); // Octal 123 = Dec 64+16+3 == 83 = Hex 53
test!(test_octal_escape_long, r#"start\1234end""#, "start\x534end", ""); // 4 is not part of the octal escape seq
test!(test_three_ocals, r#"start\1\2\3end""#, "start\x01\x02\x03end", "");
    test!(test_octal_max_utf8_0, r#"\177\1""#, "\x7f\x01", ""); // max allowed utf-8 byte
test!(test_octal_max_utf8_1, r#"start\177\1end""#, "start\x7f\x01end", ""); // max allowed utf-8 byte
test!(test_octal_null_byte, r#"a\000a""#, "a\x00a", "");

    test!(test_overflowing_octal_0, r#"\777""#, vec![0xff], ""); // higher than max byte
test!(test_overflowing_octal_1, r#"\377""#, vec![0xff], " "); // higher than max byte
test!(test_overflowing_octal_2, r#"\376""#, vec![0xfe], " "); // one lower than max byte

    test!(test_newline_in_str, "abc\n", "", "String literals may not contains a new line"); // max utf-8 byte + 1
}

