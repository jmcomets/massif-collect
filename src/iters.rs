pub struct PrefixedIter<It: Iterator> {
    prefix: Option<It::Item>,
    it: It,
}

impl<It: Iterator> PrefixedIter<It> {
    pub fn new(prefix: Option<It::Item>, it: It) -> Self {
        PrefixedIter { prefix, it }
    }
}

pub fn prefixed<It: Iterator>(head: It::Item, it: It) -> PrefixedIter<It> {
    PrefixedIter::new(Some(head), it)
}

impl<It: Iterator> From<It> for PrefixedIter<It> {
    fn from(it: It) -> Self {
        PrefixedIter::new(None, it)
    }
}

impl<It: Iterator> Iterator for PrefixedIter<It> {
    type Item = It::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.prefix.take().or_else(|| self.it.next())
    }
}

