mod tests {
    use crate::Regex;

    #[test]
    fn simple_test_eol() {
        for _ in 0..10000 {
            assert!(Regex::new("z+".as_bytes()).matches("abz".as_bytes()));
        }
    }

    #[test]
    fn simple_test_bol() {
        for _ in 0..10000 {
            assert!(Regex::new("z+".as_bytes()).matches("zABC".as_bytes()));
        }
    }

    #[test]
    fn indices() {
        for _ in 0..10000 {
            assert_eq!(Regex::new("z+".as_bytes()).match_idx("zABC".as_bytes()).unwrap().start, 0);
            assert_eq!(Regex::new("z+".as_bytes()).match_idx("zABC".as_bytes()).unwrap().len, 1);
            assert_eq!(Regex::new("z+".as_bytes()).match_idx("zzzzzzABC".as_bytes()).unwrap().start, 0);
            assert_eq!(Regex::new("z+".as_bytes()).match_idx("zzzzzzABC".as_bytes()).unwrap().len, 6);
            assert_eq!(Regex::new("z+".as_bytes()).match_idx("AAAzzzzzzABC".as_bytes()).unwrap().start, 3);
            assert_eq!(Regex::new("z+".as_bytes()).match_idx("AAAzzzzzzABC".as_bytes()).unwrap().len, 6);
        }
    }
}