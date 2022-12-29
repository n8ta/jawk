use std::fmt::{Display, Formatter};

#[repr(i8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    FloatTag = 0,
    StringTag = 1,
    StrnumTag = 2,
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = (match self {
            Tag::FloatTag => "flt",
            Tag::StringTag => "str",
            Tag::StrnumTag => "strnum",
        });
        f.write_str(s)
    }
}

impl Tag {
    #[inline(always)]
    pub fn has_ptr(&self) -> bool {
        *self != Tag::FloatTag
    }
}