// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "std")]

use std::io::{
	BufRead,
	BufReader,
	BufWriter,
	Cursor,
	Empty,
	ErrorKind,
	Read,
	Repeat,
	Sink,
	Take,
	Write,
};
use crate::{
	Result,
	Error,
	DataSink,
	BufferAccess,
	DataSource,
	source::default_skip,
};

impl<R: Read + ?Sized> DataSource for BufReader<R> {
	#[cfg(not(feature = "unstable_specialization"))]
	fn available(&self) -> usize { self.buffer_count() }

	#[cfg(not(feature = "unstable_specialization"))]
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
}

impl<R: Read + ?Sized> BufferAccess for BufReader<R> {
	fn buffer_capacity(&self) -> usize { self.capacity() }

	fn buffer(&self) -> &[u8] { self.buffer() }

	fn fill_buffer(&mut self) -> Result<&[u8]> {
		Ok(self.fill_buf()?)
	}

	fn drain_buffer(&mut self, count: usize) {
		self.consume(count);
	}
}

impl<W: Write + ?Sized> DataSink for BufWriter<W> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.write_all(buf)?;
		Ok(())
	}
}

impl<T: AsRef<[u8]>> DataSource for Cursor<T> {
	#[cfg(not(feature = "unstable_specialization"))]
	fn available(&self) -> usize { self.buffer_count() }

	fn request(&mut self, count: usize) -> Result<bool> {
		Ok(self.available() >= count)
	}

	fn skip(&mut self, mut count: usize) -> Result<usize> {
		count = count.min(self.available());
		self.consume(count);
		Ok(count)
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let count = self.read(buf)?;
		Ok(&buf[..count])
	}

	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}
}

impl<T: AsRef<[u8]>> BufferAccess for Cursor<T> {
	fn buffer_capacity(&self) -> usize { cursor_as_slice(self).len() }

	fn buffer_count(&self) -> usize {
		self.buffer_capacity()
			.min(self.position() as usize)
	}

	fn buffer(&self) -> &[u8] {
		// See Cursor::fill_buf and Cursor::split
		let slice = cursor_as_slice(self);
		let start = self.buffer_count();
		&slice[start..]
	}

	fn fill_buffer(&mut self) -> Result<&[u8]> {
		Ok((*self).buffer()) // Nothing to read
	}

	fn drain_buffer(&mut self, count: usize) {
		self.consume(count);
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
	#[cfg(not(feature = "unstable_specialization"))]
	fn available(&self) -> usize { self.buffer_count() }

	#[cfg(not(feature = "unstable_specialization"))]
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
}

impl<T: BufferAccess + BufRead> BufferAccess for Take<T> {
	fn buffer_capacity(&self) -> usize { self.get_ref().buffer_capacity() }

	fn buffer_count(&self) -> usize {
		self.get_ref()
			.buffer_count()
			.min(self.limit() as usize)
	}
	
	fn buffer(&self) -> &[u8] {
		let buf = self.get_ref().buffer();
		let len = self.buffer_count();
		&buf[..len]
	}

	fn fill_buffer(&mut self) -> Result<&[u8]> {
		Ok(self.fill_buf()?)
	}

	fn drain_buffer(&mut self, count: usize) {
		self.consume(count);
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

	#[cfg(feature = "utf8")]
	fn read_utf8<'a>(&mut self, _: &'a mut [u8]) -> Result<&'a str> {
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
		Read::read(self, buf).unwrap(); // Repeat doesn't return an error
		Ok(buf)
	}

	#[cfg(feature = "utf8")]
	fn read_utf8<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a str> {
		match self.read_bytes(buf).unwrap() {
			[] => Ok(""),
			bytes @ [byte, ..] if byte.is_ascii() => Ok(unsafe {
				// Safety: the byte is valid ASCII, which is valid UTF-8.
				core::str::from_utf8_unchecked(bytes)
			}),
			bytes =>
				// Use from_utf8 to convert the byte into a UTF-8 error.
				// Unwrap is safe because non-ASCII bytes are not valid UTF-8.
				Err(simdutf8::compat::from_utf8(&bytes[..1]).unwrap_err().into())
		}
	}
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

fn buf_read_bytes<'a>(source: &mut (impl Read + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
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

#[cfg(all(feature = "alloc", feature = "utf8"))]
fn buf_read_utf8_to_end<'a>(source: &mut (impl Read + ?Sized), buf: &'a mut String) -> Result<&'a str> {
	unsafe {
		crate::source::append_utf8(buf, |b|
			Ok(source.read_to_end(b)?)
		)
	}
}
