use crate::columns::index_of::index_of;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Split {
    start: usize,
    end: usize,
}

impl Split {
    pub fn new(start: usize, end: usize) -> Self { Self { start, end } }
    pub fn get_slice<'a, T>(&self, src: &'a [T]) -> &'a [T] {
        &src[self.start..self.end]
    }
    pub fn len(&self) -> usize { self.end - self.start }
}

pub fn borrowing_split<T: PartialEq>(content: &[T], split: &[T]) -> Vec<Split> {
    let mut start: usize = 0;
    let mut splits = vec![];
    while content.len() > start {
        let haystack = &content[start..];
        match index_of(split, haystack) {
            None => {
                let split = Split { start, end: content.len() };
                if split.len() > 0 {
                    splits.push(split);
                }
                break;
            }
            Some(next_split) => {
                let new_split = Split { start, end: next_split + start };
                if new_split.len() > 0 {
                    splits.push(new_split);
                }
                start += next_split + split.len();
            }
        }
    }
    splits
}

#[cfg(test)]
mod index_splitter_tests {
    use crate::columns::borrrowing_split::{borrowing_split, Split};

    #[test]
    fn test_index_splitting_normal_case() {
        let content = [1, 2, 3, 4, 5, 6];
        assert_eq!(borrowing_split(&content, &[2, 3]), vec![Split::new(0,1), Split::new(3,6)])
    }

    #[test]
    fn test_index_splitting_only_splitter() {
        let content = [1, 2, 3, 4, 5, 6];
        assert_eq!(borrowing_split(&content, &[1, 2, 3, 4, 5, 6]), vec![]);
    }

    #[test]
    fn test_index_splitting_ends_in_splitter_harder() {
        let content = [1, 2, 3, 4, 1, 2, 3, 4];
        assert_eq!(borrowing_split(&content, &[3, 4]), vec![Split::new(0,2), Split::new(4,6)]);
    }


    #[test]
    fn test_index_splitting_ends_in_splitter_simpler() {
        let content = [1, 2];
        assert_eq!(borrowing_split(&content, &[1, 2]), vec![]);
    }

    #[test]
    fn test_index_splitting_starts_w_split() {
        let content = [1, 2, 3, 4, 5, 6];
        assert_eq!(borrowing_split(&content, &[1, 2, 3]), vec![Split::new(3, 6)]);
        assert_eq!(borrowing_split(&content, &[1, 2, 3])[0].get_slice(&content), &content[3..]);
    }
}