# Cell

## `UnsafeCell`

`UnsafeCell`是用于实现内部可变性的一个重要的结构， 它的定义如下：

```rust
#[repr(transparent)]
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}
```

它实现的方法如下：

```rust
impl<T> UnsafeCell<T> {
    pub fn new(value: T) -> UnsafeCell<T> {
        UnsafeCell {
            value
        }
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized> UnsafeCell<T> {
    pub fn get(&self) -> *mut T {
        self as *const UnsafeCell<T> as *const T as *mut T
    }

    pub fn get_mut(&mut self) -> *mut T {
        &mut self.value
    }

    pub fn raw_get(this: *const Self) -> *mut T {
        this as *const T as *mut T
    }
}

impl <T:?Sized> !Sync for UnsafeCell<T> {}
```

其中`fn get(&self) -> *mut T`是核心方法， 它返回内部数据的一个可变裸指针， 这并不是不安全行为： 创建裸指针是安全的， 解引用它才是不安全的。

`UnsafeCell<T>`和它内部的`value`有相同的内存布局`T`, 这是因为`#[repr(transparent)]`

它的目的其实是提供共享引用转换为可变引用的方法， 因为共享引用并不能转换为可变引用， 但是可以将常指针转换为可变指针。 再通过可变指针获得可变引用。

## `Cell`

`Cell`是对`UnsafeCall`的一个包装， 它实现了单线程下的内部可变性， 但是只适用于`Copy`类型：

```rust
#[repr(transparent)]
struct Cell<T> {
    value: UnsafeCell<T>,
}
```

它实现的方法如下：

```rust
#[repr(transparent)]
struct Cell<T: ?Sized> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    pub fn new(value: T) -> Cell<T> {
        Self {
            value: UnsafeCell::new(value)
        }
    }

    pub fn replace(&self, val: T) -> T {
        std::mem::replace(unsafe {
            &mut *self.value.get()
        }, val)
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    pub fn set(&self, val: T) {
        let old = self.replace(val);
        drop(old);
    }

    pub fn swap(&self, other: &Self) {
        if ptr::eq(self, other) {
            return;
        }

        unsafe {
            ptr::swap(self.value.get(), other.value.get())
        }
    }
}

impl<T: ?Sized> Cell<T> {
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    pub fn from_mut(t: &mut T) -> &Cell<T> {
        unsafe {
            &*(t as *mut T as *const Cell<T>)
        }
    }
}

impl<T: Copy> Cell<T> {
    pub fn get(&self) -> T {
        unsafe {
            *self.value.get()
        }
    }

    pub fn update<F>(&self, f: F) -> T
    where F: FnOnce(T) -> T {
        let old = self.get();
        let new = f(old);

        self.set(new);

        new
    }
}

impl<T> From<T> for Cell<T> {
    fn from(value: T) -> Self {
        Cell::new(value)
    }
}

impl<T: Copy> Clone for Cell<T> {
    fn clone(&self) -> Self {
        Cell::new(self.get())
    }
}

impl<T: Default> Cell<T> {
    pub fn take(&self) -> T {
        self.replace(Default::default())
    }
}
```

所谓内部可变性， 就是一个不可变结构体的内部支持改变。 它的核心方法是

```rust
pub fn set(&self, val: T) {
    let old = self.replace(val);
    drop(old);
}

pub fn replace(&self, val: T) -> T {
    std::mem::replace(unsafe {
        &mut *self.value.get()
    }, val)
}
```

通过交换内部的指针来实现内部可变性， 因为`replace`并不要求`mut`, 所以可以实现