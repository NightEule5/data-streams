// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "std")]

use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Read, Write};
use crate::{DataSink, DataSource, Result};

impl<R: Read + ?Sized> DataSource for BufReader<R> {
	fn available(&self) -> usize {
		self.buffer().len()
	}

	fn request(&mut self, count: usize) -> Result<bool> {
		if self.available() < count {
			Ok(self.fill_buf()?.len() >= count)
		} else {
			Ok(true)
		}
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

	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		unsafe {
			super::append_utf8(buf, |b|
				Ok(self.read_to_end(b)?)
			)
		}
	}
}

impl<W: Write + ?Sized> DataSink for BufWriter<W> {
	#[inline(always)]
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.write_all(buf)?;
		Ok(())
	}
}
