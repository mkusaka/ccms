use lru::LruCache;
use regex::Regex;
use std::num::NonZeroUsize;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGEX_CACHE: Mutex<LruCache<String, Regex>> = {
        let capacity = NonZeroUsize::new(128).unwrap();
        Mutex::new(LruCache::new(capacity))
    };
}

pub fn get_or_compile_regex(pattern: &str, flags: &str) -> Result<Regex, regex::Error> {
    let cache_key = format!("{}\0{}", pattern, flags);
    
    // Try to get from cache first
    if let Ok(mut cache) = REGEX_CACHE.try_lock() {
        if let Some(regex) = cache.get(&cache_key) {
            return Ok(regex.clone());
        }
    }
    
    // Compile regex
    let mut regex_builder = regex::RegexBuilder::new(pattern);
    
    if flags.contains('i') {
        regex_builder.case_insensitive(true);
    }
    if flags.contains('m') {
        regex_builder.multi_line(true);
    }
    if flags.contains('s') {
        regex_builder.dot_matches_new_line(true);
    }
    
    let regex = regex_builder.build()?;
    
    // Try to cache it
    if let Ok(mut cache) = REGEX_CACHE.try_lock() {
        cache.put(cache_key, regex.clone());
    }
    
    Ok(regex)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_regex_cache() {
        // First call should compile
        let regex1 = get_or_compile_regex("test", "i").unwrap();
        assert!(regex1.is_match("TEST"));
        
        // Second call should use cache
        let regex2 = get_or_compile_regex("test", "i").unwrap();
        assert!(regex2.is_match("TEST"));
        
        // Different pattern should compile new regex
        let regex3 = get_or_compile_regex("other", "i").unwrap();
        assert!(regex3.is_match("OTHER"));
    }
    
    #[test]
    fn test_regex_flags() {
        let regex_i = get_or_compile_regex("test", "i").unwrap();
        assert!(regex_i.is_match("TEST"));
        
        let regex_m = get_or_compile_regex("^test", "m").unwrap();
        assert!(regex_m.is_match("line1\ntest"));
        
        let regex_s = get_or_compile_regex("a.b", "s").unwrap();
        assert!(regex_s.is_match("a\nb"));
    }
}