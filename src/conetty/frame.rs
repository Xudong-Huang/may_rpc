use std::io::{self, Cursor, ErrorKind, Read, Write};

use crate::{Error, WireError};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::{BufMut, Bytes, BytesMut};

// Frame layout
// id(u64) + len(u64) + payload([u8; len])

// req frame layout
// id(u64) + len(u64) + req_data([u8; len])

// rsp frame layout
// id(u64) + len(u64) + ty(u8) + len1(u64) + rsp_data([u8; len1])

// max frame len
const FRAME_MAX_LEN: u64 = 1024 * 1024;

/// raw frame wrapper, low level protocol
/// TODO: add check sum check
#[derive(Debug)]
pub struct Frame {
    /// frame id, req and rsp has the same id
    pub id: u64,
    /// payload data
    data: Bytes,
}

impl Frame {
    /// decode a frame from the reader
    pub fn decode_from<R: Read>(r: &mut R, buf: &mut BytesMut) -> io::Result<Self> {
        let id = r.read_u64::<BigEndian>()?;
        info!("decode id = {:?}", id);

        let len = r.read_u64::<BigEndian>()? + 16;
        info!("decode len = {:?}", len);

        if len > FRAME_MAX_LEN {
            let s = format!("decode too big frame length. len={len}");
            error!("{s}");
            return Err(io::Error::new(ErrorKind::InvalidInput, s));
        }

        let buf_len = len as usize;

        buf.reserve(len as usize);
        let data: &mut [u8] = unsafe { std::mem::transmute(buf.chunk_mut()) };

        r.read_exact(&mut data[16..buf_len])?;
        unsafe { buf.advance_mut(buf_len) };
        let mut data = buf.split_to(buf_len);

        unsafe { data.set_len(0) };
        data.put_u64(id);
        data.put_u64(len - 16);
        unsafe { data.set_len(buf_len) };

        let data = data.freeze();

        Ok(Frame { id, data })
    }

    /// convert self into raw buf that can be re-send as a frame
    // pub fn finish(self, id: u64) -> Vec<u8> {
    //     let mut cursor = Cursor::new(self.data);

    //     // write from start
    //     cursor.write_u64::<BigEndian>(id).unwrap();
    //     info!("re-encode id = {:?}", id);

    //     cursor.into_inner()
    // }

    /// decode a request from the frame, this would return the req raw buffer
    /// you need to deserialized from it into the real type
    pub fn decode_req(&self) -> &[u8] {
        // skip the frame head
        &self.data[16..]
    }

    /// decode a response from the frame, this would return the rsp raw buffer
    /// you need to deserialized from it into the real type
    pub fn decode_rsp(&self) -> Result<&[u8], Error> {
        use Error::*;

        let mut r = Cursor::new(&self.data[..]);
        // skip the frame head
        r.set_position(16);

        let ty = r.read_u8()?;
        // we don't need to check len here, frame is checked already
        let len = r.read_u64::<BigEndian>()? as usize;

        let buf = r.into_inner();
        let data = &buf[25..len + 25];

        // info!("decode response, ty={}, len={}", ty, len);
        match ty {
            0 => Ok(data),
            1 => Err(ServerDeserialize(String::from_utf8(data.into()).unwrap())),
            2 => Err(ServerSerialize(String::from_utf8(data.into()).unwrap())),
            3 => Err(Status(String::from_utf8(data.into()).unwrap())),
            _ => {
                let s = format!("invalid response type. ty={ty}");
                error!("{s}");
                Err(ClientDeserialize(s))
            }
        }
    }
}

/// req frame buffer that can be serialized into
pub struct ReqBuf(Cursor<Vec<u8>>);

impl Default for ReqBuf {
    fn default() -> Self {
        ReqBuf::new()
    }
}

impl ReqBuf {
    /// crate a new `ReqBuf` instance
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(128);
        buf.resize(16, 0);
        let mut cursor = Cursor::new(buf);
        // leave enough space to write id and len
        cursor.set_position(16);
        ReqBuf(cursor)
    }

    /// convert self into raw buf that can be send as a frame
    pub fn finish(self, id: u64) -> Vec<u8> {
        let mut cursor = self.0;
        let len = cursor.get_ref().len() as u64;
        assert!(len <= FRAME_MAX_LEN);

        // write from start
        cursor.set_position(0);
        cursor.write_u64::<BigEndian>(id).unwrap();
        info!("encode id = {:?}", id);

        // adjust the data length
        cursor.write_u64::<BigEndian>(len - 16).unwrap();
        info!("encode len = {:?}", len);

        cursor.into_inner()
    }
}

impl Write for ReqBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// rsp frame buffer that can be serialized into
pub struct RspBuf(Cursor<Vec<u8>>);

impl Default for RspBuf {
    fn default() -> Self {
        RspBuf::new()
    }
}

pub const SERVER_POLL_ENCODE: u8 = 200;
impl RspBuf {
    /// crate a new `RspBuf` instance
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(64);
        // id + len + ty + len + data
        buf.resize(25, 0);
        let mut cursor = Cursor::new(buf);
        // leave enough space to write id and len
        cursor.set_position(25);
        RspBuf(cursor)
    }

    /// convert self into raw buf that can be send as a frame
    pub fn finish(self, id: u64, ret: Result<(), WireError>) -> Vec<u8> {
        let mut cursor = self.0;
        let dummy = Vec::new();

        let (ty, len, data) = match ret {
            Ok(_) => (0, cursor.get_ref().len() - 25, dummy.as_slice()),
            Err(ref e) => match *e {
                WireError::ServerDeserialize(ref s) => (1, s.len(), s.as_bytes()),
                WireError::ServerSerialize(ref s) => (2, s.len(), s.as_bytes()),
                WireError::Status(ref s) => (3, s.len(), s.as_bytes()),
                WireError::Polling => (SERVER_POLL_ENCODE, 0, dummy.as_slice()),
            },
        };

        let len = len as u64;
        assert!(len < FRAME_MAX_LEN);

        // write from start
        cursor.set_position(0);
        cursor.write_u64::<BigEndian>(id).unwrap();
        info!("encode id = {:?}", id);

        // adjust the data length
        cursor.write_u64::<BigEndian>(len + 9).unwrap();
        info!("encode len = {:?}", len);

        // write the type
        cursor.write_u8(ty).unwrap();
        // write the len
        cursor.write_u64::<BigEndian>(len).unwrap();
        // write the data into the writer
        match ty {
            0 => {} // the normal ret already wrote
            SERVER_POLL_ENCODE => {
                // the server need to poll the client, will be filtered out by multiplex_client
            }
            1..=3 => {
                cursor.get_mut().resize(len as usize + 25, 0);
                cursor.write_all(data).unwrap();
            }
            _ => unreachable!("unknown rsp type"),
        }

        cursor.into_inner()
    }
}

impl Write for RspBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
