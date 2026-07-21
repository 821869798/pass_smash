//! Charset construction and password space enumeration.

/// User-selected character classes for brute-force.
#[derive(Debug, Clone)]
pub struct CharsetOptions {
    pub min_len: usize,
    pub max_len: usize,
    pub digits: bool,
    pub lowercase: bool,
    pub uppercase: bool,
    pub symbols: bool,
    /// Optional custom charset characters (appended, de-duplicated).
    pub custom: String,
}

impl Default for CharsetOptions {
    fn default() -> Self {
        Self {
            min_len: 1,
            max_len: 4,
            digits: true,
            lowercase: false,
            uppercase: false,
            symbols: false,
            custom: String::new(),
        }
    }
}

impl CharsetOptions {
    pub fn build_charset(&self) -> String {
        let mut chars = String::new();
        if self.digits {
            chars.push_str("0123456789");
        }
        if self.lowercase {
            chars.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        if self.uppercase {
            chars.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if self.symbols {
            chars.push_str("!@#$%^&*()-_=+[]{}|;:',.<>?/`~\\\" ");
        }
        for c in self.custom.chars() {
            if !chars.contains(c) {
                chars.push(c);
            }
        }
        chars
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.min_len == 0 {
            return Err("最短密码长度至少为 1".into());
        }
        if self.max_len < self.min_len {
            return Err("最长密码长度不能小于最短密码长度".into());
        }
        if self.max_len > 12 {
            return Err("最长密码长度建议不超过 12（搜索空间会爆炸式增长）".into());
        }
        if self.build_charset().is_empty() {
            return Err("请至少勾选一种字符类型，或填写自定义字符".into());
        }
        Ok(())
    }

    /// Total number of candidates in the search space.
    pub fn total_candidates(&self) -> u64 {
        let base = self.build_charset().chars().count() as u64;
        if base == 0 {
            return 0;
        }
        let mut total: u64 = 0;
        for len in self.min_len..=self.max_len {
            // base^len, saturating
            let mut space: u64 = 1;
            for _ in 0..len {
                space = space.saturating_mul(base);
            }
            total = total.saturating_add(space);
        }
        total
    }
}

/// Generates passwords in length-major, lexicographic order over the charset.
pub struct PasswordGenerator {
    charset: Vec<char>,
    max_len: usize,
    /// Current password as indices into charset.
    indices: Vec<usize>,
    done: bool,
    total: u64,
}

impl PasswordGenerator {
    pub fn new(opts: &CharsetOptions) -> Result<Self, String> {
        opts.validate()?;
        let charset: Vec<char> = opts.build_charset().chars().collect();
        let total = opts.total_candidates();
        Ok(Self {
            charset,
            max_len: opts.max_len,
            indices: vec![0; opts.min_len],
            done: false,
            total,
        })
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    fn current_password(&self) -> String {
        self.indices
            .iter()
            .map(|&i| self.charset[i])
            .collect()
    }

    /// Advance to the next password. Returns false when exhausted.
    fn advance(&mut self) -> bool {
        if self.done {
            return false;
        }
        let base = self.charset.len();
        if base == 0 {
            self.done = true;
            return false;
        }

        // Increment like an odometer.
        let mut i = self.indices.len();
        loop {
            if i == 0 {
                // Need longer password.
                let next_len = self.indices.len() + 1;
                if next_len > self.max_len {
                    self.done = true;
                    return false;
                }
                self.indices = vec![0; next_len];
                return true;
            }
            i -= 1;
            if self.indices[i] + 1 < base {
                self.indices[i] += 1;
                // reset trailing
                for j in (i + 1)..self.indices.len() {
                    self.indices[j] = 0;
                }
                return true;
            }
        }
    }
}

impl Iterator for PasswordGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        // First call: emit current (initialized to min_len zeros) without advance first.
        let password = self.current_password();
        // Prepare next; if advance fails after this emit, mark done for subsequent calls.
        if !self.advance() {
            self.done = true;
        }
        Some(password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_len1() {
        let opts = CharsetOptions {
            min_len: 1,
            max_len: 1,
            digits: true,
            lowercase: false,
            uppercase: false,
            symbols: false,
            custom: String::new(),
        };
        let generator = PasswordGenerator::new(&opts).unwrap();
        assert_eq!(generator.total(), 10);
        let all: Vec<_> = PasswordGenerator::new(&opts).unwrap().collect();
        assert_eq!(all.len(), 10);
        assert_eq!(all[0], "0");
        assert_eq!(all[9], "9");
    }

    #[test]
    fn digits_len1_to_2() {
        let opts = CharsetOptions {
            min_len: 1,
            max_len: 2,
            digits: true,
            ..Default::default()
        };
        let all: Vec<_> = PasswordGenerator::new(&opts).unwrap().collect();
        assert_eq!(all.len(), 10 + 100);
        assert_eq!(all[0], "0");
        assert_eq!(all[10], "00");
        assert_eq!(all.last().unwrap(), "99");
    }
}
