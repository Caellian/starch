#[derive(Debug, Copy, Clone)]
pub struct LiteralMatcher<'s> {
    target: &'s str,
    pos: usize,
    valid: bool,
}

impl<'s> LiteralMatcher<'s> {
    pub fn new(literal: &'s str) -> LiteralMatcher<'s> {
        LiteralMatcher {
            target: literal,
            pos: 0,
            valid: true,
        }
    }

    pub fn reset(&mut self) {
        self.pos = 0;
        self.valid = true;
    }

    pub fn expected(&self) -> Option<char> {
        self.target.chars().skip(self.pos).next()
    }

    pub fn check_next(&self, test: char) -> bool {
        self.expected() == Some(test)
    }

    pub fn next(&mut self, test: char) -> bool {
        if self.is_done() {
            return false;
        }

        if self.check_next(test) {
            self.pos += 1;
            true
        } else {
            self.valid = false;
            false
        }
    }

    pub fn is_done(&self) -> bool {
        self.pos >= self.target.len()
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}
