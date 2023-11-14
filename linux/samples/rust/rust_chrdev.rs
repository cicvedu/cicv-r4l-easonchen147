// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.

use core::result::Result::Err;

use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::{chrdev, file};

const GLOBALMEM_SIZE: usize = 0x1000;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Rust for Linux Contributors",
    description: "Rust character device sample",
    license: "GPL",
}

static GLOBALMEM_BUF: Mutex<[u8;GLOBALMEM_SIZE]> = unsafe {
    Mutex::new([0u8;GLOBALMEM_SIZE])
};

struct RustFile {
    #[allow(dead_code)]
    inner: &'static Mutex<[u8; GLOBALMEM_SIZE]>,
}

#[vtable]
impl file::Operations for RustFile {
    type Data = Box<Self>;

    fn open(_shared: &(), _file: &file::File) -> Result<Box<Self>> {
        Ok(
            Box::try_new(RustFile {
                inner: &GLOBALMEM_BUF
            })?
        )
    }

    fn write(_this: &Self, _file: &file::File, _reader: &mut impl kernel::io_buffer::IoBufferReader, _offset: u64) -> Result<usize> {
        if _reader.is_empty() {
            return Ok(0);
        }
        let mut inner = _this.inner.lock();
        let data = _reader.read_all()?;

        // pr_info!("data len: {}\n", data.len());

        inner[..data.len()].copy_from_slice(&data);
        Ok(data.len())
    }

    fn read(_this: &Self, _file: &file::File, _writer: &mut impl kernel::io_buffer::IoBufferWriter, _offset: u64) -> Result<usize> {
        let mut inner = _this.inner.lock();
        if is_zero_filled(&(*inner)) {
            return Ok(0);
        }
        let all = inner.len();
        _writer.write_slice(&mut *inner)?;

        for element in inner.iter_mut() {
            *element = 0;
        }

        Ok(all)
    }
}

fn is_zero_filled(array: &[u8]) -> bool {
    array.iter().all(|&x| x == 0)
}

struct RustChrdev {
    _dev: Pin<Box<chrdev::Registration<2>>>,
}

impl kernel::Module for RustChrdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;

        // Register the same kind of device twice, we're just demonstrating
        // that you can use multiple minors. There are two minors in this case
        // because its type is `chrdev::Registration<2>`
        chrdev_reg.as_mut().register::<RustFile>()?;
        chrdev_reg.as_mut().register::<RustFile>()?;

        Ok(RustChrdev { _dev: chrdev_reg })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
