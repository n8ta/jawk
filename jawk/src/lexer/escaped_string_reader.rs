use std::iter::Peekable;
use std::str::Chars;
use crate::printable_error::PrintableError;

// Escape chars:
// https://pubs.opengroup.org/onlinepubs/009604499/basedefs/xbd_chap05.html
// AWK specific ones:
// https://pubs.opengroup.org/onlinepubs/009604499/utilities/awk.html "Table: Escape Sequences in awk"
pub fn escaped_string_reader(_chars: &mut Peekable<Chars>) -> Result<String, PrintableError> {
    todo!()
}

#[cfg(test)]
mod string_read_tests {
    use crate::lexer::escaped_string_reader::escaped_string_reader;

    fn test_helper(input: &str, oracle: &str, expected_err: &str) {
        let res = escaped_string_reader(&mut input.chars().peekable());
        match res {
            Ok(result) => assert_eq!(oracle, result),
            Err(err) => {
                if expected_err == "" {
                    assert!(false, "Failed, input {} expected output {} but got err {}", input, oracle, expected_err)
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
    // test!(test_simple, "abc", "abc", "");
    // test!(test_spaces, "abc  def", "abc def", "");
    //
    // // Escape sequences
    // test!(test_line_break, "abc\\nv", "abc\nv", "");
    // test!(test_tab, "abc\\tv", "abc\tv", "");
    // test!(test_escape  , "abc\\\\tv", "abc\\tv", "");
    // test!(test_alert, "\\a", "\x07", "");
    // test!(test_backspace, "\\b", "\x08", "");
    // test!(test_linefeed, "\\f", "\x0c", "");
    // test!(test_carraige_return, "\\r", "\r", "");
    // test!(test_vertical_tab, "\\v", "\x0b", "");
    // test!(test_quote, "\\\"", "\"", "");
    // test!(test_quoted_str, "\\\"abc\\\\", "\"abc\"", "");
    // test!(test_backslash, "\\\\", "\\", "");
    //
    // // Octal escapes
    // test!(test_octal_escape_1, "start\\1end", "start\x01end", "");
    // test!(test_octal_escape_2, "start\\12end", "start\x0aend", ""); // Octal 12 = Dec 10 = Hex 0a
    // test!(test_octal_escape_3, "start\\123end", "start\x53end", ""); // Octal 123 = Dec 64+16+3 == 83 = Hex 53
    // test!(test_octal_escape_long, "start\\1234end", "start\x534end", ""); // 4 is not part of the octal escape seq
    // test!(test_three_ocals, "start\\1\\2\\3end", "start\x01\x02\x03end", "");
    // test!(test_octal_max_utf8, "start\\177\\1end", "start\x7f1end", ""); // max allowed utf-8 byte
    // test!(test_octal_null_byte, "a\\000\\b", "", "Using the null byte \\000 in a string is undefined behavior");
    //
    // // TODO: Is this correct? All other awks accept \777 and print it as 0xff.
    // // Existential panic: can a valid awk use utf-8 exclusively?
    // // test!(test_overflowing_octal, "start\\777\\1end", "", "\\777 (octal) is not a valid utf-8 character"); // higher than max byte
    // // test!(test_overflowing_octal, "start\\178\\1end", "", "\\178 (octal) is not a valid utf-8 character "); // max utf-8 byte + 1

}

