// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use simdutf8::compat::from_utf8;
use crate::{DataSink, Error, Result};
use crate::source::{BufferAccess, DataSource};

impl DataSource for &[u8] {
	#[inline(always)]
	fn available(&self) -> usize { self.len() }
	#[inline(always)]
	fn request(&mut self, count: usize) -> Result<bool> {
		Ok(self.len() >= count)
	}

	fn skip(&mut self, mut count: usize) -> Result<usize> {
		count = count.min(self.len());
		self.consume(count);
		Ok(count)
	}
	
	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let len = self.len().min(buf.len());
		buf[..len].copy_from_slice(&self[..len]);
		*self = &self[len..];
		Ok(&buf[..len])
	}

	#[cfg(feature = "alloc")]
	fn read_utf8<'a>(&mut self, mut count: usize, buf: &'a mut String) -> Result<&'a str> {
		count = count.min(self.len());
		let result = from_utf8(&self[..count]);
		*self = &self[count..];
		let start = buf.len();
		buf.push_str(result?);
		Ok(&buf[start..])
	}

	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		self.read_utf8(self.len(), buf)
	}
}

impl BufferAccess for &[u8] {
	fn buf_capacity(&self) -> usize { self.len() }

	fn buf(&self) -> &[u8] { self }

	fn fill_buf(&mut self) -> Result<&[u8]> { Ok(self) }

	fn clear_buf(&mut self) {
		*self = &[];
	}

	fn consume(&mut self, count: usize) {
		*self = &self[..count];
	}
}

impl DataSink for &mut [u8] {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		let len = buf.len().min(self.len());
		// From <[_]>::take_mut
		let (target, empty) = core::mem::take(self).split_at_mut(len);
		*self = empty;
		target.copy_from_slice(&buf[..len]);
		let remaining = buf.len() - len;
		if remaining > 0 {
			Err(Error::Overflow { remaining })
		} else {
			Ok(())
		}
	}
}
