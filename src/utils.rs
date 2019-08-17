
pub struct FirstLastIter<I, T> {
    inner: I,
    next: Option<T>,
    first: bool,
}

pub fn first_last_iter<I: Iterator>(mut iter: I) -> FirstLastIter<I, I::Item> {
    let next = iter.next();
    FirstLastIter {
        inner: iter,
        next,
        first: true,
    }
}

impl<I, T> Iterator for FirstLastIter<I, T>
    where I: Iterator<Item=T>
{
    type Item = (T, bool, bool); // (value, is_first, is_last)

    fn next(&mut self) -> Option<Self::Item> {
        match self.next.take() {
            Some(val) => {
                self.next = self.inner.next();
                let first = self.first;
                self.first = false;
                Some((val, first, self.next.is_none()))
            }
            None => {
                None
            }
        }
    }
}
