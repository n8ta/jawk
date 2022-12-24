mod tests {
    use crate::Regex;

    #[test]
    fn simple_test_eol() {
        for _ in 0..10000 {
            assert!(Regex::new("z+").matches("abz"));
        }
    }

    #[test]
    fn simple_test_bol() {
        for _ in 0..10000 {
            assert!(Regex::new("z+").matches("zABC"));
        }
    }

    #[test]
    fn indices() {
        for _ in 0..10000 {
            assert_eq!(Regex::new("z+").match_idx("zABC").unwrap().start, 0);
            assert_eq!(Regex::new("z+").match_idx("zABC").unwrap().len, 1);
            assert_eq!(Regex::new("z+").match_idx("zzzzzzABC").unwrap().start, 0);
            assert_eq!(Regex::new("z+").match_idx("zzzzzzABC").unwrap().len, 6);
            assert_eq!(Regex::new("z+").match_idx("AAAzzzzzzABC").unwrap().start, 3);
            assert_eq!(Regex::new("z+").match_idx("AAAzzzzzzABC").unwrap().len, 6);
        }
    }
}