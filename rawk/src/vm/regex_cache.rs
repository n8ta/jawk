use lru_cache::LruCache;
use mawk_regex::Regex;
use crate::awk_str::AwkStr;

pub struct RegexCache {
    cache: LruCache<AwkStr, Regex>,
}

impl RegexCache {
    pub fn new() -> Self { Self { cache: LruCache::new(32) } }

    pub fn get(&mut self, reg_str: &AwkStr) -> &mut Regex {
        if self.cache.contains_key(&reg_str) {
            self.cache.get_mut(&reg_str).unwrap()
        } else {
            let re = Regex::new(&reg_str);
            self.cache.insert((reg_str).clone(), re);
            self.cache.get_mut(&reg_str).unwrap()
        }
    }
}