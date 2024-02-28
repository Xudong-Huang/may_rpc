use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::BytesMut;
use may::queue::mpsc::Queue;
use may::sync::Mutex;

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
    fn put_data(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data)
    }

    #[inline]
    fn write_all(&mut self) -> std::io::Result<()> {
        let ret = self.writer.write_all(&self.buf);
        self.buf.clear();
        let capacity = self.buf.capacity();
        self.buf.reserve(capacity);
        ret
    }
}

#[derive(Debug)]
pub struct QueuedWriter<W: Write> {
    data_count: AtomicUsize,
    data_queue: Queue<Vec<u8>>,
    writer: Mutex<BufWriter<W>>,
}

impl<W: Write> QueuedWriter<W> {
    pub fn new(writer: W) -> Self {
        QueuedWriter {
            data_count: AtomicUsize::new(0),
            data_queue: Queue::new(),
            writer: Mutex::new(BufWriter::new(writer)),
        }
    }

    /// it's safe and efficient to call this API concurrently
    pub fn write(&self, data: Vec<u8>) {
        self.data_queue.push(data);
        // only allow the first writer perform the write operation
        // other concurrent writers would just push the data
        if self.data_count.fetch_add(1, Ordering::AcqRel) == 0 {
            // in any cases this should not block since we have only one writer
            let mut writer = self.writer.lock().unwrap();

            loop {
                let mut cnt = 0;
                while let Some(data) = self.data_queue.pop() {
                    writer.put_data(&data);
                    cnt += 1;
                }

                // detect if there are more packet need to deal with
                if self.data_count.fetch_sub(cnt, Ordering::AcqRel) == cnt {
                    break;
                }
            }

            if let Err(e) = writer.write_all() {
                // FIXME: handle the error
                error!("QueuedWriter failed, err={}", e);
            }
        }
    }
}
