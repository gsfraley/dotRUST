//! Hosting the CLR from unix with Rust

extern crate libloading as libl;

use std::collections::HashMap;
use std::env::current_exe;
use std::ffi::CString;
use std::io::{self, Error, ErrorKind};
use std::mem;
use std::os::raw::{c_char, c_int, c_uint, c_void};

use super::ClrHost;


fn to_c_str<T: Into<Vec<u8>>>(t: T) -> *const c_char {
    CString::new(t).unwrap().as_ptr() as *const c_char
}


/// A function pointer type for the init function
type CoreClrInitializeFn = unsafe extern fn(
    *const c_char,
    *const c_char,
    c_int,
    *const *const c_char,
    *const *const c_char,
    *const *const c_void,
    *const c_uint) -> c_int;

/// A function pointer type for the shutdown function
type CoreClrShutdownFn = unsafe extern fn(*const c_void, c_uint) -> c_int;

/// A function pointer type for the second shutdown function
type CoreClrShutdown2Fn = unsafe extern fn(*const c_void, c_uint, *const c_int) -> c_int;

/// A function pointer type for the create delegate function
type CoreClrCreateDelegateFn = unsafe extern fn(
    *const c_void,
    c_uint,
    *const c_char,
    *const c_char,
    *const c_char,
    *const *const c_void) -> c_int;

/// The UnixCoreClr object represents a binding to the CLR maintained by a private handle and domain id
pub struct UnixCoreClrHost {
    host_handle: *const c_void,
    domain_id: c_uint
}

impl UnixCoreClrHost {
    /// Private helper function to grab a reference to the library in the current context
    fn library() -> io::Result<libl::Library> {
        libl::Library::new(
            "/usr/local/share/dotnet/shared/Microsoft.NETCore.App/2.0.0/libcoreclr.dylib")
    }

    /// Creates a new CLR object
    pub fn init(
            app_domain_friendly_name: &str,
            properties: HashMap<&str, &str>)
            -> io::Result<UnixCoreClrHost>
    {
        let exe_path = current_exe()?;
        let exe_path_str = exe_path.to_str().unwrap();

        // Create the host handle and its ref
        let host_handle = 0 as *const c_void;
        let host_handle_ref = &host_handle as *const *const c_void;
        
        // Create the domain id and its ref
        let domain_id = 0 as c_uint;
        let domain_id_ref = &domain_id as *const c_uint;

        // Raw C string refs from exe_path and app_domain_friendly_name slices
        let exe_path_raw = to_c_str(exe_path_str);
        let app_domain_friendly_name_raw = to_c_str(app_domain_friendly_name);

        // Count for upcoming vecs
        let properties_count = properties.len() as c_int;

        // Collect property keys into a vec
        let properties_keys: Vec<*const c_char> = properties.keys()
                .map(|&k| to_c_str(k))
                .collect();
        
        // Collect equivalent property values into another vec
        let properties_values: Vec<*const c_char> = properties.values()
                .map(|&v| to_c_str(v))
                .collect();

        // Grab refs of the two to pass into the actual function call
        let properties_keys_ref = properties_keys.as_ptr();
        let properties_values_ref = properties_values.as_ptr();

        // Open up an unsafe block for actually loading functions from the CLR libs
        unsafe {
            let coreclr_library = UnixCoreClrHost::library()?;
            let coreclr_initialize: libl::Symbol<CoreClrInitializeFn> = coreclr_library.get(b"coreclr_initialize")?;

            // Initialize the CLR
            match coreclr_initialize(
                exe_path_raw,
                app_domain_friendly_name_raw,
                properties_count,
                properties_keys_ref,
                properties_values_ref,
                host_handle_ref,
                domain_id_ref)
            {
                // If healthy exit code, return a model of the CLR
                0 => Ok(UnixCoreClrHost {
                    host_handle: host_handle,
                    domain_id: domain_id
                }),
                // Else error out
                code => Err(Error::new(ErrorKind::Other, format!("Failed to initialize ({:X}).  Host handle: {:?}, domain id: {:?}.", code, host_handle, domain_id)))
            }
        }
    }
}

impl ClrHost for UnixCoreClrHost {
    fn get_app_domain_id(self: &Self) -> io::Result<i32> {
        Ok(self.domain_id as i32)
    }

    fn shutdown(self: Self) -> io::Result<()> {
        unsafe {
            let coreclr_library = UnixCoreClrHost::library()?;
            let coreclr_shutdown: libl::Symbol<CoreClrShutdownFn> = coreclr_library.get(b"coreclr_shutdown")?;

            // Shutdown the CLR
            match coreclr_shutdown(self.host_handle, self.domain_id) {
                // If healthy exit code, return unit
                0 => Ok(()),
                // Else error out
                _ => Err(Error::new(ErrorKind::Other, "Failed to shutdown"))
            }
        }
    }

    fn execute_assembly(
        self: &Self,
        _assembly_path: &str,
        _args: Vec<&str>) -> io::Result<i32>
    {
        unimplemented!()
    }

    unsafe fn create_delegate<T>(
        self: &Self,
        assembly_name: &str,
        class_name: &str,
        method_name: &str) -> io::Result<Box<T>>
    {
        let delegate_handle = 0 as *const c_void;
        let delegate_handle_ref = &delegate_handle as *const *const c_void;

        let coreclr_library = UnixCoreClrHost::library()?;
        let coreclr_create_delegate: libl::Symbol<CoreClrCreateDelegateFn> = coreclr_library.get(b"coreclr_create_delegate")?;

        // Shutdown the CLR
        match coreclr_create_delegate(
            self.host_handle, self.domain_id,
            to_c_str(assembly_name), to_c_str(class_name), to_c_str(method_name),
            delegate_handle_ref)
        {
            // If healthy exit code, return unit
            0 => {
                let delegate_handle_box = mem::transmute::<Box<*const c_void>,Box<T>>(Box::from(delegate_handle));
                Ok(delegate_handle_box)
            },
            // Else error out
            _ => Err(Error::new(ErrorKind::Other, "Failed to create delegate"))
        }
    }
}

impl UnixCoreClrHost {
    /// Shuts down the CLR, returning a latched exit code?  Should follow up on what "latched exit code" is.
    pub fn shutdown_2(self: Self) -> io::Result<c_int> {
        let latched_exit_code = -1 as c_int;
        let latched_exit_code_ref = &latched_exit_code as *const c_int;

        unsafe {
            let coreclr_library = UnixCoreClrHost::library()?;
            let coreclr_shutdown_2: libl::Symbol<CoreClrShutdown2Fn> = coreclr_library.get(b"coreclr_shutdown_2")?;

            // Shutdown the CLR
            match coreclr_shutdown_2(self.host_handle, self.domain_id, latched_exit_code_ref) {
                // If healthy exit code, return the resulting exit code
                0 => Ok(latched_exit_code),
                // Else error out
                _ => Err(Error::new(ErrorKind::Other, "Failed to shutdown"))
            }
        }
    }
}