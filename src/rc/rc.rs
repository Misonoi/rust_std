use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use crate::cell::Cell;

pub struct Rc<T> {
    inner: NonNull<RcInner<T>>,
    _marker: PhantomData<RcInner<T>>,
}

struct RcInner<T> {
    value: T,
    ref_count: Cell<usize>,
}

impl<T> Rc<T> {
    pub fn new(v: T) -> Self {
        let leaked = Box::leak(Box::new(RcInner {
            value: v,
            ref_count: Cell::new(1),
        }));

        Self {
            inner: NonNull::new(leaked).unwrap(),
            _marker: PhantomData,
        }
    }

    pub fn count(this: &Self) -> usize {
        unsafe {
            this.inner.as_ref().ref_count.get()
        }
    }
}

impl<T> std::ops::Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &unsafe {
            self.inner.as_ref()
        }.value
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe {
            self.inner.as_ref()
        };
        
        let c = inner.ref_count.get();
        
        inner.ref_count.set(c + 1);
        
        Rc {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let inner = unsafe {
            self.inner.as_ref()
        };

        let c = inner.ref_count.get();

        unsafe {
            if c == 1 {
                let _ = Box::from_raw(self.inner.as_ptr());
            } else {
                inner.ref_count.set(c - 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rc::rc::Rc;

    #[test]
    fn test_rc() {
        let t = Rc::new(String::from("abcd"));
        let b = t.clone();
        let s = b.clone();

        assert_eq!(Rc::count(&s), 3);
        assert_eq!(b.to_uppercase(), "ABCD");
        drop(s);
        drop(b);
        assert_eq!(Rc::count(&t), 1);
    }
}