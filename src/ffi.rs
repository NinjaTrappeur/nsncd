/*
 * Copyright 2020 Two Sigma Open Source, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use nix::libc;
use nix::sys::socket::AddressFamily;
use std::ffi::CStr;
use std::ptr;

#[allow(non_camel_case_types)]
type size_t = ::std::os::raw::c_ulonglong;

extern "C" {
    /// This is the function signature of the glibc internal function to
    /// disable using nscd for this process.
    fn __nss_disable_nscd(hell: unsafe extern "C" fn(size_t, *mut libc::c_void));
}

/// Copied from
/// [unscd](https://github.com/bytedance/unscd/blob/3a4df8de6723bc493e9cd94bb3e3fd831e48b8ca/nscd.c#L2469)
///
/// This internal glibc function is called to disable trying to contact nscd.
/// We _are_ nscd, so we need to do the lookups, and not recurse.
/// Until 2.14, this function was taking no parameters.
/// In 2.15, it takes a function pointer from hell.
unsafe extern "C" fn do_nothing(_dbidx: size_t, _finfo: *mut libc::c_void) {}

/// Disable nscd inside our own glibc to prevent recursion.
pub fn disable_internal_nscd() {
    unsafe {
        __nss_disable_nscd(do_nothing);
    }
}

mod ffi {
    use nix::libc;
    extern "C" {
        pub fn gethostbyname2_r(name: *const libc::c_char,
                                af: libc::c_int,
                                result_buf: *mut libc::hostent,
                                buf: *mut libc::c_char, buflen: libc::size_t,
                                result: *mut *mut libc::hostent,
                                h_errnop: *mut libc::c_int) -> libc::c_int;
    }
}

pub struct Hostent {
    pub name: String,
    pub aliases: Vec<String>,
    pub addrType: i32,
    pub length: usize,
    pub addrList: Vec<i32>

}

pub struct GetHostByNameIterator {
    buf: Vec<i8>,
    hostent_ret_ptr: *mut *mut libc::hostent
}

impl Iterator for GetHostByNameIterator {
    // TODO: iterator over h_addr_list which is terminated by null
    type Item = Hostent;
    fn next(&mut self) -> Option<Hostent> {
        unsafe {
            self.hostent_ret_ptr = self.hostent_ret_ptr.add(1);
            if *self.hostent_ret_ptr == std::ptr::null_mut() {
                None
            } else {
                Some(HostEnt{})
            }
        }
    }
}


///
///
/// The stream is positioned at the first entry in the directory.
///
/// af is nix::libc::AF_INET6 or nix::libc::AF_INET6
pub fn gethostbyname2_r(name: &CStr, af: libc::c_int) -> nix::Result<GetHostByNameIterator> {
    let mut hostent : libc::hostent = libc::hostent {
        h_name:  ptr::null_mut(), // <- points to buf
        h_aliases: ptr::null_mut(),
        h_addrtype: 0,
        h_length: 0,
        h_addr_list: ptr::null_mut(),
    };
    let hostent_ret : *mut *mut libc::hostent = ptr::null_mut();
    let mut err : libc::c_int = 0;
    let mut buf : Vec<i8> = Vec::with_capacity(256);
    // TODO: check AF_INET
    let mut ret: i32;
    loop {
        ret = unsafe {
            ffi::gethostbyname2_r(name.as_ptr(), af, &mut hostent, buf.as_mut_ptr(), buf.capacity(), hostent_ret, &mut err)
        };
        if err == libc::ERANGE {
            // The buffer is too small. Let's x2 its capacity and retry.
            buf.reserve(buf.capacity());
        } else {
            break;
        }
    }
    if ret == 0 {
        Ok(GetHostByNameIterator { hostent_ret_ptr: hostent_ret, buf })
    } else {
        // errno
        Err(nix::Error::last())
    }
}

#[test]
fn test_gethostbyname2_r() {

}
