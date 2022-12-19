// Compact representation of an Option<usize>

#[derive(Debug, Clone, Copy)]
pub struct OptUsize {
    data: usize,
}

impl OptUsize {
    pub fn none() -> Self {
        Self { data: 0 }
    }
    pub fn some(data: usize) -> Self {
        Self {
            data: (data << 1) + 1
        }
    }
    pub fn get(&self) -> Option<usize> {
        if self.data & 0b1 == 1 {
            Some(self.data >> 1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::optional_usize::OptUsize;

    #[test]
    fn test_opt_usize_none() {
        let u = OptUsize::none();
        assert_eq!(u.get(), None)
    }

    #[test]
    fn test_opt_usize_some() {
        let u = OptUsize::some(0);
        assert_eq!(u.get(), Some(0))
    }

    #[test]
    fn test_opt_usize_some_large() {
        let u = OptUsize::some(1<<31);
        assert_eq!(u.get(), Some(1<<31))
    }

    #[test]
    fn test_opt_usize_some_bits() {
        let u = OptUsize::some(0b1111000011110000);
        assert_eq!(u.get(), Some(0b1111000011110000))
    }
}

