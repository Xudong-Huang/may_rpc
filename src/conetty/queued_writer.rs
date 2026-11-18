use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::BytesMut;
use may::queue::mpsc::Queue;

mod spin_lock {
    use std::cell::UnsafeCell;
    use std::hint::spin_loop;
    use std::ops::{Deref, DerefMut};
    use std::sync::atomic::{AtomicBool, Ordering};
    /// a minimal spin lock
    ///
    /// it must not be held across any operation that could yield or block,
    /// otherwise the spinning threads could starve the coroutine runtime
    #[derive(Debug)]
    pub(super) struct SpinLock<T> {
        locked: AtomicBool,
        data: UnsafeCell<T>,
    }

    // safety: the data can only be accessed through `lock`, which guarantees
    // exclusive access for the lifetime of the returned guard
    unsafe impl<T: Send> Sync for SpinLock<T> {}

    #[derive(Debug)]
    pub(super) struct SpinLockGuard<'a, T> {
        lock: &'a SpinLock<T>,
        data: &'a mut T,
    }

    impl<T> SpinLock<T> {
        pub(super) fn new(data: T) -> Self {
            SpinLock {
                locked: AtomicBool::new(false),
                data: UnsafeCell::new(data),
            }
        }

        #[inline]
        #[must_use = "if unused the lock will be released immediately"]
        pub(super) fn lock(&self) -> SpinLockGuard<'_, T> {
            // fast path, try to acquire the lock without spinning
            while self
                .locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                // spin on a plain load to avoid bouncing the cache line
                // between all the waiters
                while self.locked.load(Ordering::Relaxed) {
                    spin_loop();
                }
            }
            SpinLockGuard {
                lock: self,
                // safety: only one thread can reach here at a time, so the
                // data is exclusively borrowed for the lifetime of the guard
                data: unsafe { &mut *self.data.get() },
            }
        }
    }

    impl<T> Drop for SpinLockGuard<'_, T> {
        #[inline]
        fn drop(&mut self) {
            self.lock.locked.store(false, Ordering::Release);
        }
    }

    impl<T> Deref for SpinLockGuard<'_, T> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &Self::Target {
            self.data
        }
    }

    impl<T> DerefMut for SpinLockGuard<'_, T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.data
        }
    }
}

#[derive(Debug)]
struct BufWriter<W: Write> {
    writer: W,
    buf: BytesMut,
}

impl<W: Write> BufWriter<W> {
    fn new(writer: W) -> Self {
        BufWriter {
            writer,
            buf: BytesMut::with_capacity(1024 * 32),
        }
    }
    #[inline]
    fn reserve_buf(&mut self) {
        let remain = self.buf.capacity() - self.buf.len();
        if remain < 1024 {
            self.buf.reserve(1024 * 32 - remain);
        }
    }

    #[inline]
    fn put_data(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data)
    }

    #[inline]
    fn write_all(&mut self) -> std::io::Result<()> {
        let ret = self.writer.write_all(&self.buf);
        self.buf.clear();
        ret
    }
}

use spin_lock::SpinLock;

#[derive(Debug)]
pub struct QueuedWriter<W: Write> {
    data_count: AtomicUsize,
    data_queue: Queue<Vec<u8>>,
    writer: SpinLock<BufWriter<W>>,
}

impl<W: Write> QueuedWriter<W> {
    pub fn new(writer: W) -> Self {
        QueuedWriter {
            data_count: AtomicUsize::new(0),
            data_queue: Queue::new(),
            writer: SpinLock::new(BufWriter::new(writer)),
        }
    }

    /// it's safe and efficient to call this API concurrently
    pub fn write(&self, data: Vec<u8>) -> std::io::Result<()> {
        self.data_queue.push(data);
        // only allow the first writer perform the write operation
        // other concurrent writers would just push the data
        if self.data_count.fetch_add(1, Ordering::AcqRel) == 0 {
            // the lock could still be held by the previous writer
            // for a few instructions after its `data_count.fetch_sub`
            // before its guard is dropped, so use a spin lock to
            // acquire it as soon as it's released
            let mut writer = self.writer.lock();
            writer.reserve_buf();

            loop {
                let mut cnt = 0;
                while let Some(data) = self.data_queue.pop() {
                    writer.put_data(&data);
                    cnt += 1;
                }

                // write all buffered data to the underlying writer
                // this happens before we update the data_count
                // so other threads can not contend the mutex
                // also ensures that data is written as soon as possible
                let ret = writer.write_all();

                // detect if there are more packet need to deal with
                // decrease the data_count after processing cnt packets
                let left = self.data_count.fetch_sub(cnt, Ordering::AcqRel);
                // now check the result and return if error occurred
                ret?;

                if left == cnt {
                    break;
                }
            }
            // we still hold the lock here,
            // but the data_count could be zero now
            // so there is a small window where other
            // threads might try to acquire the lock
        }
        Ok(())
    }
}
