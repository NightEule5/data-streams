// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "nightly_borrowed_buf")]

use core::io::{BorrowedBuf, BorrowedCursor};
use crate::{DataSink, Error, Result};

impl DataSink for BorrowedBuf<'_> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.unfilled().write_bytes(buf)
	}
}

impl DataSink for BorrowedCursor<'_> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		if buf.len() > self.capacity() {
			return Err(Error::overflow(self.capacity() - buf.len()))
		}
		
		self.append(buf);
		Ok(())
	}
}
