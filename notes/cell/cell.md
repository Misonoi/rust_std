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

## `RefCell`

用于给未实现`Copy`的类型实现内部可变性

```rust
pub struct RefCell<T> {
    value: UnsafeCell<T>,
    state: Cell<RefState>,
    _marker: PhantomData<UnsafeCell<T>>,
}
```

核心方法如下：

```rust
impl<T> RefCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            state: Cell::new(RefState::Unshared),
            _marker: PhantomData,
        }
    }
}

impl<T> RefCell<T> {
    pub fn borrow(&self) -> Option<Ref<'_, T>> {
        match self.state.get() {
            RefState::Unshared => {
                self.state.set(RefState::Shared(1));

                Some(Ref {
                    ref_cell: self,
                })
            }
            RefState::Shared(n) => {
                self.state.set(RefState::Shared(n + 1));

                Some(Ref {
                    ref_cell: self,
                })
            }
            _ => None,
        }
    }

    pub fn borrow_mut(&self) -> Option<RefMut<'_, T>> {
        if let RefState::Unshared = self.state.get() {
            self.state.set(RefState::Exclusive);

            Some(RefMut {
                refcell: self,
            })
        } else {
            None
        }
    }
}

pub struct Ref<'refcell, T> {
    ref_cell: &'refcell RefCell<T>,
}

impl<T> std::ops::Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ref_cell.value.get() }
    }
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        match self.ref_cell.state.get() {
            RefState::Exclusive | RefState::Unshared => unreachable!(),
            RefState::Shared(1) => {
                self.ref_cell.state.set(RefState::Unshared);
            }
            RefState::Shared(n) => {
                self.ref_cell.state.set(RefState::Shared(n - 1));
            }
        }
    }
}

pub struct RefMut<'refcell, T> {
    refcell: &'refcell RefCell<T>,
}

impl<T> std::ops::Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.refcell.value.get() }
    }
}

impl<T> std::ops::DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.refcell.value.get() }
    }
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        match self.refcell.state.get() {
            RefState::Shared(_) | RefState::Unshared => unreachable!(),
            RefState::Exclusive => {
                self.refcell.state.set(RefState::Unshared);
            }
        }
    }
}
```

`borrow`和`borrow_mut`用于向外部提供共享引用和可变引用。 值得注意的是， 需要声明结构体为`mut`的条件是， 结构体的方法中有`&mut self`, 或是直接修改结构体的字段。 

共享引用可以有很多， 但是只有没有共享引用的时候才可以借出可变引用

当`drop`时要修改引用计数， 但是又不能直接`drop RefCell`, 于是考虑使用`Ref`和`RefMut`进行包装。 

>任何计算机科学中的问题， 都可以通过添加一个抽象层来解决。