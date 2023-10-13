use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Arc<T> {
    ptr: NonNull<ArcInner<T>>,
    _marker: PhantomData<ArcInner<T>>,
}

pub struct ArcInner<T> {
    rc: AtomicUsize,
    data: T,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            ptr: NonNull::new(
                Box::into_raw(
                    Box::new(
                        ArcInner {
                            rc: AtomicUsize::new(1),
                            data,
                        }
                    )
                )
            ).unwrap(),
            _marker: PhantomData,
        }
    }
}

unsafe impl<T: Sync + Send> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

impl<T> std::ops::Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &unsafe {
            self.ptr.as_ref()
        }.data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe {
            self.ptr.as_ref()
        };

        let old_rc = inner.rc.fetch_add(1, Ordering::Relaxed);

        if old_rc >= isize::MAX as usize {
            std::process::abort();
        }

        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        let inner = unsafe {
            self.ptr.as_ref()
        };

        if inner.rc.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        atomic::fence(Ordering::Acquire);

        unsafe {
            let _ = Box::from_raw(self.ptr.as_ptr());
        }
    }
}