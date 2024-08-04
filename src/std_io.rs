// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "std")]

#[cfg(feature = "alloc")]
use alloc::string::String;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Empty, ErrorKind, Read, Repeat, Sink, Take, Write};
use std::iter::repeat;
use crate::{Error, Result};
use crate::sink::DataSink;
use crate::source::{BufferAccess, DataSource, default_skip};

impl<R: Read + ?Sized> DataSource for BufReader<R> {
	#[cfg(not(feature = "nightly_specialization"))]
	fn available(&self) -> usize {
		crate::source::default_available(self)
	}

	#[cfg(not(feature = "nightly_specialization"))]
	fn request(&mut self, count: usize) -> Result<bool> {
		crate::source::default_request(self, count)
	}
	
	fn skip(&mut self, count: usize) -> Result<usize> {
		Ok(buf_read_skip(self, count))
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_bytes(self, buf)
	}

	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		buf_read_utf8_to_end(self, buf)
	}
}

impl<R: Read + ?Sized> BufferAccess for BufReader<R> {
	fn buf_capacity(&self) -> usize { self.capacity() }

	fn buf(&self) -> &[u8] { self.buffer() }

	fn fill_buf(&mut self) -> Result<&[u8]> {
		Ok(BufRead::fill_buf(self)?)
	}

	fn clear_buf(&mut self) {
		BufferAccess::consume(self, self.available());
	}

	fn consume(&mut self, count: usize) {
		BufRead::consume(self, count);
	}
}

impl<W: Write + ?Sized> DataSink for BufWriter<W> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.write_all(buf)?;
		Ok(())
	}
}

impl<T: AsRef<[u8]>> DataSource for Cursor<T> {
	#[cfg(not(feature = "nightly_specialization"))]
	fn available(&self) -> usize { crate::source::default_available(self) }

	fn request(&mut self, count: usize) -> Result<bool> {
		Ok(self.available() >= count)
	}

	fn skip(&mut self, mut count: usize) -> Result<usize> {
		count = count.min(self.available());
		BufRead::consume(self, count);
		Ok(count)
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let count = self.read(buf)?;
		Ok(&buf[..count])
	}

	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		self.read_utf8(self.available(), buf)
	}
}

impl<T: AsRef<[u8]>> BufferAccess for Cursor<T> {
	fn buf_capacity(&self) -> usize { cursor_as_slice(self).len() }

	fn buf(&self) -> &[u8] {
		// See Cursor::fill_buf and Cursor::split
		let slice = cursor_as_slice(self);
		let start = (self.position() as usize).min(slice.len());
		&slice[start..]
	}

	fn fill_buf(&mut self) -> Result<&[u8]> {
		Ok((*self).buf()) // Nothing to read
	}

	fn clear_buf(&mut self) {
		BufferAccess::consume(self, self.buf_capacity().min(self.position() as usize));
	}

	fn consume(&mut self, count: usize) {
		BufRead::consume(self, count);
	}
}

impl<T> DataSink for Cursor<T> where Self: Write {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		let count = self.write(buf)?;
		if count < buf.len() {
			let remaining = buf.len() - count;
			Err(Error::Overflow { remaining })
		} else {
			Ok(())
		}
	}
}

fn cursor_as_slice<T: AsRef<[u8]>>(cursor: &Cursor<T>) -> &[u8] {
	cursor.get_ref().as_ref()
}

impl<T: BufferAccess + BufRead> DataSource for Take<T> {
	#[cfg(not(feature = "nightly_specialization"))]
	fn available(&self) -> usize { crate::source::default_available(self) }

	#[cfg(not(feature = "nightly_specialization"))]
	fn request(&mut self, count: usize) -> Result<bool> {
		crate::source::default_request(self, count)
	}

	fn skip(&mut self, count: usize) -> Result<usize> {
		Ok(buf_read_skip(self, count))
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_bytes(self, buf)
	}

	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		buf_read_utf8_to_end(self, buf)
	}
}

impl<T: BufferAccess + BufRead> BufferAccess for Take<T> {
	fn buf_capacity(&self) -> usize { self.get_ref().buf_capacity() }
	
	fn buf(&self) -> &[u8] {
		let buf = self.get_ref().buf();
		let len = (self.limit() as usize).min(buf.len());
		&buf[..len]
	}

	fn fill_buf(&mut self) -> Result<&[u8]> {
		Ok(BufRead::fill_buf(self)?)
	}

	fn clear_buf(&mut self) {
		BufferAccess::consume(self, self.available());
	}

	fn consume(&mut self, count: usize) {
		BufRead::consume(self, count);
	}
}

macro_rules! fixed_stream_impl {
    (impl $trait:ident for $stream:ident {
		$($item:item)+
	}) => {
		impl $trait for $stream {
			$($item)+
		}
		
		impl $trait for &$stream {
			$($item)+
		}
	};
}

fixed_stream_impl! {
impl DataSource for Empty {
	fn available(&self) -> usize { 0 }

	fn request(&mut self, _: usize) -> Result<bool> {
		Ok(false)
	}

	fn skip(&mut self, _: usize) -> Result<usize> {
		Ok(0)
	}

	fn read_bytes<'a>(&mut self, _: &'a mut [u8]) -> Result<&'a [u8]> {
		Ok(&[])
	}

	#[cfg(feature = "alloc")]
	fn read_utf8<'a>(&mut self, _: usize, _: &'a mut String) -> Result<&'a str> {
		Ok("")
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, _: &'a mut String) -> Result<&'a str> {
		Ok("")
	}
}
}

impl DataSink for Empty {
	fn write_bytes(&mut self, _: &[u8]) -> Result { Ok(()) }
}
impl DataSink for &Empty {
	fn write_bytes(&mut self, _: &[u8]) -> Result { Ok(()) }
}

impl DataSink for Sink {
	fn write_bytes(&mut self, _: &[u8]) -> Result { Ok(()) }
}
impl DataSink for &Sink {
	fn write_bytes(&mut self, _: &[u8]) -> Result { Ok(()) }
}

impl DataSource for Repeat {
	fn available(&self) -> usize { usize::MAX }

	fn request(&mut self, _: usize) -> Result<bool> {
		Ok(true)
	}

	fn skip(&mut self, count: usize) -> Result<usize> {
		Ok(count)
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let byte = get_repeated_byte(self);
		buf.fill(byte);
		Ok(buf)
	}

	fn read_utf8<'a>(&mut self, count: usize, buf: &'a mut String) -> Result<&'a str> {
		let bytes @ [byte] = &[get_repeated_byte(self)];
		if byte.is_ascii() {
			buf.try_reserve(count)?;
			let start = buf.len();
			let str = unsafe {
				// Safety: the byte is valid ASCII, which is valid UTF-8.
				core::str::from_utf8_unchecked(&bytes[..])
			};
			buf.extend(repeat(str).take(count));
			Ok(&buf[start..])
		} else {
			Err(Error::Ascii(*byte))
		}

	}

	fn read_utf8_to_end<'a>(&mut self, _: &'a mut String) -> Result<&'a str> {
		Err(Error::NoEnd)
	}
}

/// A janky function which gets the private repeated byte field of [`Repeat`].
fn get_repeated_byte(repeat: &mut Repeat) -> u8 {
	let mut array @ [b] = [0];
	let _ = repeat.read(&mut array).unwrap();
	b
}

fn buf_read_skip(source: &mut (impl BufferAccess + DataSource + ?Sized), count: usize) -> usize {
	let mut skip_count = 0;
	while skip_count < count {
		let cur_skip_count = default_skip(&mut *source, count);
		skip_count += cur_skip_count;

		if cur_skip_count == 0 {
			break
		}
	}
	skip_count
}

fn buf_read_bytes<'a>(source: &mut (impl BufferAccess + DataSource + Read + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
	use ErrorKind::Interrupted;

	let mut count = 0;
	loop {
		match source.read(buf) {
			Ok(0) => break Ok(&buf[..count]),
			Ok(cur_count) => count += cur_count,
			Err(err) if err.kind() == Interrupted => { }
			Err(err) => break Err(err.into())
		}
	}
}

fn buf_read_exact_bytes<'a>(source: &mut (impl Read + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
	match source.read_exact(&mut *buf) {
		Ok(()) => Ok(buf),
		Err(error) if error.kind() == ErrorKind::UnexpectedEof =>
			Err(Error::End { required_count: buf.len() }),
		Err(error) => Err(error.into())
	}
}

fn buf_read_utf8_to_end<'a>(source: &mut (impl Read + ?Sized), buf: &'a mut String) -> Result<&'a str> {
	unsafe {
		super::append_utf8(buf, |b|
			Ok(source.read_to_end(b)?)
		)
	}
}
