// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "std")]

#[cfg(feature = "alloc")]
use alloc::string::String;
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Read, Write};
use crate::Result;
use crate::sink::DataSink;
use crate::source::{BufferAccess, DataSource, default_available, default_request, default_skip};

impl<R: Read + ?Sized> DataSource for BufReader<R> {
	fn available(&self) -> usize {
		default_available(self)
	}

	fn request(&mut self, count: usize) -> Result<bool> {
		default_request(self, count)
	}
	
	fn skip(&mut self, count: usize) -> Result<usize> {
		let mut skip_count = 0;
		while skip_count < count {
			let cur_skip_count = default_skip(&mut *self, count)?;
			skip_count += cur_skip_count;
			
			if cur_skip_count == 0 {
				break
			}
		}
		Ok(skip_count)
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		use ErrorKind::Interrupted;

		let mut count = 0;
		loop {
			match self.read(buf) {
				Ok(0) => break Ok(&buf[..count]),
				Ok(cur_count) => count += cur_count,
				Err(err) if err.kind() == Interrupted => { }
				Err(err) => break Err(err.into())
			}
		}
	}

	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		self.read_exact(buf)?;
		Ok(buf)
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		unsafe {
			super::append_utf8(buf, |b|
				Ok(self.read_to_end(b)?)
			)
		}
	}
}

impl<R: Read + ?Sized> BufferAccess for BufReader<R> {
	fn buf_capacity(&self) -> usize { self.capacity() }

	fn buf(&self) -> &[u8] { self.buffer() }

	fn fill_buf(&mut self) -> Result<&[u8]> {
		Ok(BufRead::fill_buf(self)?)
	}

	fn clear_buf(&mut self) {
		BufferAccess::consume(self, self.available())
	}

	fn consume(&mut self, count: usize) {
		BufRead::consume(self, count)
	}
}

impl<W: Write + ?Sized> DataSink for BufWriter<W> {
	#[inline(always)]
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.write_all(buf)?;
		Ok(())
	}
}
