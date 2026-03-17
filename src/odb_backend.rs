use std::ffi::c_void;

use libc::{c_int, c_uint, size_t};
use libgit2_sys::git_oid;
pub use libgit2_sys::{
    git_object_t, git_odb, git_odb_backend, git_odb_backend_data_alloc, git_odb_backend_data_free,
    git_odb_backend_malloc, git_odb_init_backend,
};

use crate::Buf;

// TODO: Lifetimes: with phantom data!!!!

// Relevant Functions for research
//
// git_odb__add_default_backends
//
// git_odb__backend_loose
// git_odb__backend_packed
//
// loose_backend__read ... etc..

// ======================= ODB Backend, FOR REFERENCE ================================
mod ee {
    use std::ffi::c_void;

    use libc::{c_int, c_uint, size_t};
    use libgit2_sys::{
        git_indexer_progress_cb, git_object_size_t, git_object_t, git_odb, git_odb_backend,
        git_odb_foreach_cb, git_odb_stream, git_odb_writepack, git_oid,
    };

    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct git_odb_backendd {
        pub version: c_uint,
        pub odb: *mut git_odb,
        pub read: Option<
            // returns GIT_ENOTFOUND or GIT_PASSTHROUGH,
            // negative is error, 0 and positive is success
            extern "C" fn(
                // allocate a git_rawobj here
                *mut *mut c_void,
                // set it's size in bytes here
                *mut size_t,
                // set it's type here
                *mut git_object_t,
                // self referenz kindof?
                *mut git_odb_backend,
                // Git OID die abgeholt werden soll
                *const git_oid,
            ) -> c_int,
        >,

        pub read_prefix: Option<
            extern "C" fn(
                *mut git_oid,
                *mut *mut c_void,
                *mut size_t,
                *mut git_object_t,
                *mut git_odb_backend,
                *const git_oid,
                size_t,
            ) -> c_int,
        >,
        pub read_header: Option<
            extern "C" fn(
                *mut size_t,
                *mut git_object_t,
                *mut git_odb_backend,
                *const git_oid,
            ) -> c_int,
        >,

        pub write: Option<
            extern "C" fn(
                *mut git_odb_backend,
                *const git_oid,
                *const c_void,
                size_t,
                git_object_t,
            ) -> c_int,
        >,

        pub writestream: Option<
            extern "C" fn(
                *mut *mut git_odb_stream,
                *mut git_odb_backend,
                git_object_size_t,
                git_object_t,
            ) -> c_int,
        >,

        pub readstream: Option<
            extern "C" fn(
                *mut *mut git_odb_stream,
                *mut size_t,
                *mut git_object_t,
                *mut git_odb_backend,
                *const git_oid,
            ) -> c_int,
        >,

        pub exists: Option<extern "C" fn(*mut git_odb_backend, *const git_oid) -> c_int>,

        pub exists_prefix: Option<
            extern "C" fn(*mut git_oid, *mut git_odb_backend, *const git_oid, size_t) -> c_int,
        >,

        pub refresh: Option<extern "C" fn(*mut git_odb_backend) -> c_int>,

        pub foreach:
            Option<extern "C" fn(*mut git_odb_backend, git_odb_foreach_cb, *mut c_void) -> c_int>,

        pub writepack: Option<
            extern "C" fn(
                *mut *mut git_odb_writepack,
                *mut git_odb_backend,
                *mut git_odb,
                git_indexer_progress_cb,
                *mut c_void,
            ) -> c_int,
        >,

        pub writemidx: Option<extern "C" fn(*mut git_odb_backend) -> c_int>,

        pub freshen: Option<extern "C" fn(*mut git_odb_backend, *const git_oid) -> c_int>,

        pub free: Option<extern "C" fn(*mut git_odb_backend)>,
    }
}
// ======================= FOR REFERENCE END ================================

/*
struct MemObject {
    oid: git_oid,
    len: size_t,
    type: git_object_t ,
    data: char[GIT_FLEX_ARRAY],
};*/

#[repr(C)]
struct GitRawObjC {
    data: *mut c_void,
    len: size_t,
    r#type: git_object_t,
}

struct GitRawObj {
    data: Vec<u8>,
    git_object_type: git_object_t,
}

struct GitObjectInfo {
    git_object_type: git_object_t,
    size: size_t,
}

pub struct GitBox {}
impl GitBox {
    pub fn new() {}
}

pub struct OdbBackendWrapper {
    handle: OdbBackendHandle,
    rust_odb_impl: Box<&dyn OdbBackend>,
}

pub struct OdbBackendHandle {
    raw_odb_backend: *mut git_odb_backend,
}

pub trait OdbBackend {
    const VERSION: c_uint;

    pub fn new() -> Self;

    pub fn read(oid: &git_oid) -> GitRawObj;

    //pub fn read_prefix(oid: &git_oid) -> GitRawObj {}

    //pub fn find_from_short_oid(short_oid: &git_oid, len: size_t)

    pub fn read_header(oid: &git_oid) -> GitObjectInfo;

    pub fn exists(oid: &git_oid) -> bool;

    pub fn exists_prefix(oid: &git_oid) -> Option<git_oid>;

    pub fn write(oid: &git_oid, buffer: &Vec<u8>, object_type: git_object_t);

    pub fn refresh();

    pub fn into_odb_backend() -> OdbBackendWrapper {
        let mut odb_backend: Arc<git_odb_backend> = Arc::new(unsafe { std::mem::zeroed() });
        unsafe { git_odb_init_backend(&mut odb_backend, Self::VERSION) }

        OdbBackendWrapper {
            handle: OdbBackendHandle {
                raw_odb_backend: &mut odb_backend,
            },
            rust_odb_impl: (),
        }
    }

    pub unsafe extern "C" fn _git_read(
        // allocate a git_rawobj here
        output_buffer: *mut *mut c_void,
        // set it's size in bytes here
        output_buffer_size: *mut size_t,
        // set it's type here
        output_type: *mut git_object_t,
        // self referenz kindof?
        backend_ref: *mut git_odb_backend,
        // Git OID die abgeholt werden soll
        requested_oid: *const git_oid,
    ) -> c_int {
        let read = Self::read(requested_oid); // TODO: Allow various errors

        let buffer: *mut u8 = git_odb_backend_data_alloc(backend_ref, read.data.len());
        let buffer = unsafe { std::slice::from_raw_parts_mut(buffer, read.data.len()) };

        buffer.copy_from_slice(&read.data);

        output_buffer = buffer;
        output_buffer_size = read.data.len(); // Special treatement for size_t ?
        output_type = read.git_object_type;

        return 0;
    }

    pub unsafe extern "C" fn _git_read_prefix() -> c_int {}
}
