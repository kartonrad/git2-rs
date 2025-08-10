pub use libgit2_sys::{git_odb, git_odb_backend, git_odb_backend_malloc, git_odb_init_backend,
    git_odb_backend_data_free,
    git_odb_backend_data_alloc
};

use crate::Buf;

// TODO: Lifetimes: with phantom data!!!!


struct memobject {
	git_oid oid;
	size_t len;
	git_object_t type;
	char data[GIT_FLEX_ARRAY];
};

pub struct GitBox {}
impl GitBox {
    pub fn new() {

    }
}


pub struct OdbBackendWrapper {
    handle: OdbBackendHandle,
    rust_odb_impl: Box<&dyn OdbBackend>,
}



pub struct OdbBackendHandle {
    raw_odb_backend: *mut git_odb_backend,
}

impl OdbBackendHandle {
    pub fn alloc_object(&mut self, buffer_size: usize) -> &mut [u8] {
        git_odb_backend_data_alloc(self.raw_odb_backend, len)
    }

    pub fn free_object(&mut self) {
        git_odb_backend_data_free(self.raw_odb_backend)
    }
}



pub trait OdbBackend {
    pub fn new() -> Self;

    pub fn read() -> Git;

    pub fn into_odb_backend() -> OdbBackendWrapper {
    }

}
