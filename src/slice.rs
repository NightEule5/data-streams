// SPDX-License-Identifier: Apache-2.0

use alloc::string::String;
use simdutf8::compat::from_utf8;
use crate::{DataSource, Result};

impl DataSource for &[u8] {
	#[inline(always)]
	fn available(&self) -> usize { self.len() }
	#[inline(always)]
	fn request(&mut self, count: usize) -> Result<bool> {
		Ok(self.len() >= count)
	}
	
	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let len = self.len().min(buf.len());
		buf[..len].copy_from_slice(&self[..len]);
		*self = &self[len..];
		Ok(&buf[..len])
	}

	fn read_utf8<'a>(&mut self, mut count: usize, buf: &'a mut String) -> Result<&'a str> {
		count = count.min(self.len());
		let result = from_utf8(&self[..count]);
		*self = &self[count..];
		let start = buf.len();
		buf.push_str(result?);
		Ok(&buf[start..])
	}

	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		self.read_utf8(self.len(), buf)
	}
}
