#![allow(dead_code)]
use std::{any::Any, ffi::c_void, marker::PhantomData, ptr::NonNull};

use libc::{c_int, c_uint, size_t};
use libgit2_sys::git_oid;
pub use libgit2_sys::{git_object_t, git_odb_backend, git_odb_backend_data_alloc};

use crate::{Binding, Error, Oid};

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
        /// Read an object from the database.
        ///
        /// This method queries all available ODB backends trying to read the given OID.
        ///
        /// The returned object is reference counted and internally cached,
        /// so it should be closed by the user once it's no longer in use.
        ///
        /// # returns
        /// `0` or an error code
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

        /// Read an object from the database, given a prefix of its identifier.
        /// This method queries all available ODB backends trying to match the 'len'
        /// first hexadecimal characters of the 'short_id'. The remaining (GIT_OID_SHA1_HEXSIZE-len)*4 bits of
        /// 'short_id' must be 0s.
        ///
        /// 'len' must be at least GIT_OID_MINPREFIXLEN,
        /// and the prefix must be long enough to identify a unique object in all the backends;
        /// the method will fail otherwise.
        ///
        /// The returned object is reference counted and internally cached,
        /// so it should be closed by the user once it's no longer in use.
        ///
        /// # returns
        /// `0` or an error code
        pub read_prefix: Option<
            extern "C" fn(
                // `obj_id`: [OUT] the id of the object that was found
                *mut git_oid,
                // `obj_buffer`: [OUT] pointer where to store the read object
                *mut *mut c_void,
                // `obj_buffer_len`: [OUT] length of read object
                *mut size_t,
                // `obj_type`: [OUT] type of the read object
                *mut git_object_t,
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `oid_prefix`: a prefix of the id of the object to read
                *const git_oid,
                // `oid_prefix_len`: the length of the prefix
                size_t,
            ) -> c_int,
        >,
        /// Read the header of an object from the database, without reading its full contents.
        ///
        /// The header includes the length and the type of an object.
        ///
        /// Note that most backends do not support reading only the header of an object,
        /// so the whole object will be read and then the header will be returned.
        ///
        /// # returns
        /// `0` or an error code
        pub read_header: Option<
            extern "C" fn(
                // `len_out`: [OUT] pointer where to store the length
                *mut size_t,
                // `type_out`: [OUT] pointer where to store the type
                *mut git_object_t,
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `id`: identity of the object to read
                *const git_oid,
            ) -> c_int,
        >,

        /// Write an object directly into the ODB
        ///
        /// This method writes a full object straight into the ODB.
        /// For most cases, it is preferred to write objects through a write stream,
        /// which is both faster and less memory intensive, specially for big objects.
        ///
        /// This method is provided for compatibility
        /// with custom backends which are not able to support streaming writes
        ///
        /// # returns
        /// `0` or an error code
        pub write: Option<
            extern "C" fn(
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `id`: under which id to store the data
                // (since this is a *const, i assume the hashing was already done.)
                *const git_oid,
                // `data`: buffer with the data to store
                *const c_void,
                // `len`: Size of the buffer
                size_t,
                // `type`: type of the data to store
                git_object_t,
            ) -> c_int,
        >,

        /// Open a stream to write an object into the ODB
        ///
        /// The type and final length of the object must be specified when opening the stream.
        ///
        /// The returned stream will be of type GIT_STREAM_WRONLY, and it won't
        /// be effective until git_odb_stream_finalize_write is called and returns without an error
        ///
        /// The stream must always be freed when done with git_odb_stream_free or will leak memory.
        ///
        /// # returns
        /// `0` if the stream was created; error code otherwise
        pub writestream: Option<
            extern "C" fn(
                // `out`: [OUT] Pointer where to store the opened stream
                *mut *mut git_odb_stream,
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `size`: final size of the object that will be written
                git_object_size_t,
                // `type`: type of the object that will be written
                git_object_t,
            ) -> c_int,
        >,

        /// Open a stream to read an object from the ODB
        ///
        /// Note that most backends do not support streaming reads,
        /// because they store their objects as compressed/delta'ed blobs.
        ///
        /// It's recommended to use git_odb_read instead, which is assured to work on all backends.
        ///
        /// The returned stream will be of type GIT_STREAM_RDONLY and will have the following methods:
        ///
        ///    stream->read: read n bytes from the stream - stream->free: free the stream
        ///
        /// The stream must always be free'd or will leak memory
        ///
        /// # returns
        /// `0` if the stream was created; error code otherwise
        pub readstream: Option<
            extern "C" fn(
                // `out`: [OUT] pointer where to store the stream
                *mut *mut git_odb_stream,
                // `len`: [OUT] pointer where to store the length of the object
                *mut size_t,
                // `type`: [OUT] pointer where to store the type of the object
                *mut git_object_t,
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `oid`: oid of the object the stream will read from
                *const git_oid,
            ) -> c_int,
        >,

        /// Determine if the given object can be found in the object database.
        ///
        /// ## returns
        /// 1 if the object was found, 0 otherwise
        pub exists: Option<
            extern "C" fn(
                // `db`: database to be searched for the given object.
                *mut git_odb_backend,
                // `id`: the object to search for.
                *const git_oid,
            ) -> c_int,
        >,

        /// Determine if an object can be found in the object database by an abbreviated object ID.
        ///
        /// ## returns
        /// 0 if found, GIT_ENOTFOUND if not found, GIT_EAMBIGUOUS if multiple matches were found, other value < 0 if there was a read error.
        pub exists_prefix: Option<
            extern "C" fn(
                // `out`: The full OID of the found object if just one is found.
                *mut git_oid,
                // `backend`: The database to be searched for the given object.
                *mut git_odb_backend,
                // `short_id`: A prefix of the id of the object to read.
                *const git_oid,
                // `len`: The length of the prefix.
                size_t,
            ) -> c_int,
        >,

        /// Refresh the object database to load newly added files.
        ///
        /// If the object databases have changed on disk while the library is running,
        /// this function will force a reload of the underlying indexes.
        ///
        /// Use this function when you're confident that an external application has tampered with the ODB.
        ///
        /// NOTE that it is not necessary to call this function at all.
        /// The library will automatically attempt to refresh the ODB when a lookup fails,
        /// to see if the looked up object exists on disk but hasn't been loaded yet.
        ///
        /// ## refresh
        /// 0 on success, error code otherwise
        pub refresh: Option<
            extern "C" fn(
                // `backend`: reference to odb backend
                *mut git_odb_backend,
            ) -> c_int,
        >,

        /// List all objects available in the database
        ///
        /// The callback will be called for each object available in the database.
        /// Note that the objects are likely to be returned in the index order,
        /// which would make accessing the objects in that order inefficient.
        /// Return a non-zero value from the callback to stop looping.
        ///
        /// ## returns
        /// 0 on success, non-zero callback return value, or error code
        pub foreach: Option<
            extern "C" fn(
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `callback`: the callback to call for each object
                git_odb_foreach_cb,
                // `payload`: data to pass to the callback
                *mut c_void,
            ) -> c_int,
        >,

        /// Open a stream for writing a pack file to the ODB.
        ///
        /// If the ODB layer understands pack files,
        /// then the given packfile will likely be streamed directly to disk (and a corresponding index created).
        /// If the ODB layer does not understand pack files,
        /// the objects will be stored in whatever format the ODB layer uses.
        ///
        /// ## returns
        /// 0 or an error code.
        pub writepack: Option<
            extern "C" fn(
                // `out`: [OUT] pointer to the writepack functions
                *mut *mut git_odb_writepack,
                // `backend`: reference to the odb backend
                *mut git_odb_backend,
                // `db`: object database where the stream will read from
                *mut git_odb,
                // `progress_cb`: function to call with progress information
                // be aware that this is called inline with network and indexing operations,
                // so performance may be affected.
                git_indexer_progress_cb,
                // `progress_payload`: payload for the progress callback
                *mut c_void,
            ) -> c_int,
        >,

        /// Write a multi-pack-index file from all the .pack files in the ODB.
        ///
        /// If the ODB layer understands pack files,
        /// then this will create a file called multi-pack-index next to the .pack and .idx files,
        /// which will contain an index of all objects stored in .pack files.
        /// This will allow for O(log n) lookup for n objects (regardless of how many packfiles there exist).
        ///
        /// ## returns
        /// 0 or an error code
        pub writemidx: Option<extern "C" fn(*mut git_odb_backend) -> c_int>,

        /// "Freshens" an already existing object, updating its last-used time.
        /// This occurs when git_odb_write was called, but the object already existed (and will not be re-written).
        /// The underlying implementation may want to update last-used timestamps.
        ///
        /// If callers implement this, they should return 0 if the object exists and was freshened, and non-zero otherwise.
        pub freshen: Option<extern "C" fn(*mut git_odb_backend, *const git_oid) -> c_int>,

        /// Frees any resources held by the odb (including the git_odb_backend itself).
        /// An odb backend implementation must provide this function.
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

pub struct GitRawObj {
    data: Vec<u8>,
    git_object_type: git_object_t,
}

pub struct GitObjectInfo {
    git_object_type: git_object_t,
    size: size_t,
}

pub struct GitBox {}
impl GitBox {
    pub fn new() {}
}

pub struct OdbBackendHandle<'a, T: OdbBackend> {
    pub pointer: NonNull<OdbBackendSneakstructure>,
    pub(crate) phantom_data: PhantomData<&'a T>,
}

impl<T: OdbBackend> OdbBackendHandle<'_, T> {
    pub fn inner_mut(&mut self) -> &mut T {
        let val_mut = unsafe { self.pointer.as_mut() };

        val_mut
            .rust_impl
            .as_any_mut()
            .downcast_mut()
            .expect("OdbBackendHandle was created with Type T, but value cannot be downcast")
    }
}

#[repr(C)]
pub struct OdbBackendSneakstructure {
    /// The Git-Structure for the ODB-Backend.
    pub backend: git_odb_backend,
    /// The Rust-Structure implementing the ODB-Backend Trait.
    pub rust_impl: Box<dyn OdbBackend>,
}

/// This Trait allows the implementer to provide an ODB-Backend-Implementation which will
/// be called by libgit2's C code.
///
/// The Type implementing OdbBackend can keep state,
/// and can be registered in a [crate::Odb] using [crate::Odb::add_backend].
pub trait OdbBackend: 'static {
    /// Get the ODB Backend Version
    fn get_version(&self) -> c_uint;

    /// Read an object from the database.
    fn read(&self, oid: &Oid) -> Result<GitRawObj, Error>;

    /// Read an object from the database, given a prefix of its identifier.
    /// This method queries the ODB backend trying to match the 'len'
    /// first hexadecimal characters of the 'short_id'. The remaining (GIT_OID_SHA1_HEXSIZE-len)*4 bits of
    /// 'short_id' must be 0s.
    ///
    /// 'len' must be at least GIT_OID_MINPREFIXLEN,
    /// the method will fail otherwise.
    /// and the prefix must be long enough to identify a unique object in all the backends;
    ///
    /// # Returns
    /// The matched OID and Object, or Error with
    /// [ErrorCode::NotFound] is not found, [ErrorCode::Ambiguous] if multiple matches were found
    fn read_prefix(&self, oid: &git_oid, oid_prefix_len: size_t)
        -> Result<(Oid, GitRawObj), Error>;

    /// Read the header of an object from the database, without reading its full contents.
    ///
    /// The header includes the length and the type of an object.
    ///
    /// Note that most backends do not support reading only the header of an object,
    /// so the whole object will be read and then the header will be returned.
    fn read_header(&self, oid: &Oid) -> Result<GitObjectInfo, Error>;

    /// Write an object directly into the ODB
    ///
    /// This method writes a full object straight into the ODB.
    /// For most cases, it is preferred to write objects through a write stream,
    /// which is both faster and less memory intensive, specially for big objects.
    ///
    /// This method is provided for compatibility
    /// with custom backends which are not able to support streaming writes
    fn write(&self, oid: &git_oid, buffer: &[u8], object_type: git_object_t) -> Result<(), Error>;

    // TODO: writestream
    //       readstream

    /// Determine if the given object can be found in the object database.
    fn exists(&self, oid: &Oid) -> bool;

    /// Determine if an object can be found in the object database by an abbreviated object ID.
    ///
    /// # Returns
    /// The full matched oid, or Error with
    /// [ErrorCode::NotFound] is not found, [ErrorCode::Ambiguous] if multiple matches were found
    fn exists_prefix(&self, oid: &git_oid, len: size_t) -> Result<Option<git_oid>, Error>;

    /// Refresh the object database to load newly added files.
    ///
    /// If the object databases have changed on disk while the library is running,
    /// this function will force a reload of the underlying indexes.
    ///
    /// Use this function when you're confident that an external application has tampered with the ODB.
    ///
    /// NOTE that it is not necessary to call this function at all.
    /// The library will automatically attempt to refresh the ODB when a lookup fails,
    /// to see if the looked up object exists on disk but hasn't been loaded yet.
    fn refresh(&self) -> Result<(), Error>;

    // TODO: fn foreach
    // TODO: fn writepack
    // TODO: fn writemidx
    // TODO: fn freshen

    /// Cast "dyn OdbBackend" to "dyn Any"
    fn as_any_mut(&self) -> &mut dyn Any;
}

// ===== Interface Methods: Static Functions that allow git2 to call back to the Rust Trait-Methods ======

/// Read an object from the database.
///
/// This method queries all available ODB backends trying to read the given OID.
///
/// The returned object is reference counted and internally cached,
/// so it should be closed by the user once it's no longer in use.
///
/// # returns
/// `0` or an error code
pub extern "C" fn _git_dyn_odbbackend_read(
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
    unsafe {
        // Obtain Reference
        let odb_backend_ref: &mut OdbBackendSneakstructure = (backend_ref
            as *mut OdbBackendSneakstructure)
            .as_mut()
            .expect("GIT-ODB-Backend read should never be called with a null pointer");

        // Call Trait Method
        let read = match odb_backend_ref
            .rust_impl
            .read(&Oid::from_raw(requested_oid))
        {
            Ok(read) => read,
            Err(git_err) => {
                let err_code = git_err.raw_set_git_error();
                return err_code;
            }
        };

        // Convert Results
        let buffer: *mut u8 = git_odb_backend_data_alloc(backend_ref, read.data.len()) as *mut u8;
        let buffer = std::slice::from_raw_parts_mut(buffer, read.data.len());

        buffer.copy_from_slice(&read.data);

        *output_buffer = buffer.as_mut_ptr().cast();
        *output_buffer_size = read.data.len(); // Special treatement for size_t ?
        *output_type = read.git_object_type;

        return 0;
    }
}

/// Read an object from the database, given a prefix of its identifier.
/// This method queries the ODB backend trying to match the 'len'
/// first hexadecimal characters of the 'short_id'. The remaining (GIT_OID_SHA1_HEXSIZE-len)*4 bits of
/// 'short_id' must be 0s.
///
/// 'len' must be at least GIT_OID_MINPREFIXLEN,
/// and the prefix must be long enough to identify a unique object in all the backends;
/// the method will fail otherwise.
///
/// The returned object is reference counted and internally cached,
/// so it should be closed by the user once it's no longer in use.
///
/// # Arguments
/// * `obj_id`: [OUT] the id of the object that was found
/// * `obj_buffer`: [OUT] pointer where to store the read object
/// * `obj_buffer_len`: [OUT] length of read object
/// * `obj_type`: [OUT] type of the read object
/// * `backend`: reference to the odb backend
/// * `oid_prefix`: a prefix of the id of the object to read
/// * `oid_prefix_len`: the length of the prefix
///
/// # returns
/// `0` or an error code
pub extern "C" fn _git_dyn_odbbackend_read_prefix(
    obj_id: *mut git_oid,
    obj_buffer: *mut *mut c_void,
    obj_buffer_len: *mut size_t,
    obj_type: *mut git_object_t,
    backend: *mut git_odb_backend,
    oid_prefix: *const git_oid,
    oid_prefix_len: size_t,
) -> c_int {
    unsafe {
        // Obtain Reference
        let odb_backend_ref: &mut OdbBackendSneakstructure = (backend
            as *mut OdbBackendSneakstructure)
            .as_mut()
            .expect("GIT-ODB-Backend read should never be called with a null pointer");

        // Call Trait Method
        let read = match odb_backend_ref
            .rust_impl
            .read_prefix(&*oid_prefix, oid_prefix_len)
        {
            Ok(read) => read,
            Err(git_err) => {
                let err_code = git_err.raw_set_git_error();
                return err_code;
            }
        };

        // Convert Results
        let buffer: *mut u8 = git_odb_backend_data_alloc(backend, read.1.data.len()) as *mut u8;
        let buffer = std::slice::from_raw_parts_mut(buffer, read.1.data.len());

        buffer.copy_from_slice(&read.1.data);

        *obj_id = *read.0.raw();
        *obj_buffer = buffer.as_mut_ptr().cast();
        *obj_buffer_len = read.1.data.len(); // Special treatement for size_t ?
        *obj_type = read.1.git_object_type;

        return 0;
    }
}

/// Frees any resources held by the odb (including the git_odb_backend itself).
/// An odb backend implementation must provide this function.
pub extern "C" fn _git_dyn_odbbackend_free(backend_ref: *mut git_odb_backend) {
    unsafe {
        // Take ownership and drop.
        let object_database_backend: Box<OdbBackendSneakstructure> =
            Box::from_raw(backend_ref.cast());
        drop(object_database_backend)
    }
}

// unsafe { sneak_odb_backend.as_mut() }.read_prefix = Some(_git_dyn_odbbackend_read_prefix);
// unsafe { sneak_odb_backend.as_mut() }.read_header = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.write  = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.writestream  = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.readstream  = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.exists  = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.exists_prefix  = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.refresh = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.foreach = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.writepack = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.writemidx = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.freshen = Some(_git_dyn_odbbackend_write);
// unsafe { sneak_odb_backend.as_mut() }.free = Some(_git_dyn_odbbackend_write);
//

//pub unsafe extern "C" fn _git_read_prefix() -> c_int {}
