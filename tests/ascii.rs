// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(all(feature = "unstable", feature = "unstable_ascii_char"))]
#![feature(ascii_char)]
#![feature(concat_bytes)]
#![feature(assert_matches)]

mod slice {
	use std::assert_matches::assert_matches;
	use data_streams::{AsciiError, DataSource, Error};

	#[test]
	fn empty() -> data_streams::Result {
		assert_eq!((&[][..]).read_ascii(&mut [0; 256])?, &[]);
		Ok(())
	}
	
	#[test]
	fn nominal() -> data_streams::Result {
		const STR: &[u8] = b"hello world!";
		
		let mut input = STR;
		assert_eq!(
			input.read_ascii(&mut [0; 256])?,
			unsafe { STR.as_ascii_unchecked() }
		);
		assert_eq!(input, &[]);
		Ok(())
	}
	
	#[test]
	fn error_end() -> data_streams::Result {
		const STR: &[u8] = concat_bytes!(b"hello world!", [0xFF]);
		
		let mut input = STR;
		let mut buf = [0; 256];
		assert_matches!(
			input.read_ascii(&mut buf),
			Err(Error::Ascii(AsciiError { invalid_byte: 0xFF, valid_up_to: 12, consumed_count: 12 }))
		);
		assert_eq!(&buf[..12], b"hello world!");
		assert_eq!(input, [0xFF]);
		Ok(())
	}
	
	#[test]
	fn error_middle() -> data_streams::Result {
		const STR: &[u8] = concat_bytes!(b"hello", [0xFF], b" world!");
		
		let mut input = STR;
		let mut buf = [0; 256];
		assert_matches!(
			input.read_ascii(&mut buf),
			Err(Error::Ascii(AsciiError { invalid_byte: 0xFF, valid_up_to: 5, consumed_count: 5 }))
		);
		assert_eq!(&buf[..5], b"hello");
		assert_eq!(input, &STR[5..]);
		Ok(())
	}
}
