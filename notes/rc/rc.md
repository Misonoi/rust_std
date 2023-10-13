# Rc

`Rc`是用于引用计数的智能指针， 常用于多所有权或者是与`Cell`与`RefCell`一起实现内部可变性。

```rust
pub struct Rc<T> {
    inner: NonNull<RcInner<T>>,
    _marker: PhantomData<RcInner<T>>,
}

struct RcInner<T> {
    value: T,
    ref_count: Cell<usize>,
}
```

主要的方法如下：

```rust
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
```

思想很简单， 根据引用来计数， 且使用`Deref`来隐式转换。