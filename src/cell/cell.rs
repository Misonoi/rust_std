
use std::marker::PhantomData;
use std::ptr;

#[repr(transparent)]
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}

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

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn raw_get(this: *const Self) -> *mut T {
        this as *const T as *mut T
    }
}

impl <T: ?Sized> !Sync for UnsafeCell<T> {}

impl<T> From<T> for UnsafeCell<T> {
    fn from(value: T) -> Self {
        UnsafeCell::new(value)
    }
}

#[repr(transparent)]
pub struct Cell<T: ?Sized> {
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

    pub fn set(&self, val:  T) {
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

#[derive(Clone, Copy)]
enum RefState {
    Unshared,
    Shared(usize),
    Exclusive,
}

pub struct RefCell<T> {
    value: UnsafeCell<T>,
    state: Cell<RefState>,
    _marker: PhantomData<UnsafeCell<T>>,
}

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

#[cfg(test)]
mod tests {
    use crate::cell::{Cell, RefCell};
    use crate::rc::Rc;

    #[test]
    fn test_cell() {
        let t = Cell::new(5);
        t.set(3);
        println!("{}", t.get());
    }

    #[test]
    fn test_ref_cell() {
        let s = RefCell::new(String::from("abc"));

        let mut t = s.borrow_mut().unwrap();
        t.push_str("acv");

        drop(t);

        assert_eq!(s.borrow().unwrap().to_uppercase(), "ABCACV");
    }
}