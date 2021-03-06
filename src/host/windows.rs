//! Hosting the CLR from Rust using the Windows hosting API (ICLRRuntimeHost2 from MSCOREE.IDL)

use com::IUnknown;
use winapi::minwindef::DWORD;
use winapi::winerror::HRESULT;
use winapi::winnt::LPCWSTR;
use std::os::raw::c_void;

iid!(IID_ICLRRUNTIMEHOST = 0x90F1A06C, 0x7712, 0x4762, 0x86, 0xB5, 0x7A, 0x5E, 0xBA, 0x6B, 0xDB, 0x02);
com_interface! {
    interface ICLRRuntimeHost : IUnknown {
        iid: IID_ICLRRUNTIMEHOST,
        vtable: IClrRuntimeHostVtbl,

        /// Initializes the CLR into a process
        fn start() -> HRESULT;

        /// Stops the execution of code by the runtime
        fn stop() -> HRESULT;

        /// Gets the ID of the currently executing AppDomain
        fn get_current_app_domain_id(app_domain_id: *mut DWORD) -> HRESULT;
    }
}

iid!(IID_ICLRRUNTIMEHOST2 = 0x712AB73F, 0x2C22, 0x4807, 0xAD, 0x7E, 0xF5, 0x01, 0xD7, 0xB7, 0x2C, 0x2D);
com_interface! {
    interface ICLRRuntimeHost2 : ICLRRuntimeHost, IUnknown {
        iid: IID_ICLRRUNTIMEHOST2,
        vtable: IClrRuntimeHost2Vtbl,

        /// Executes the specified assembly in the runtime
        fn execute_assembly(
            app_domain_id: DWORD,
            assembly_path: LPCWSTR,
            argc: i32,
            argv: *const LPCWSTR,
            return_value: DWORD) -> HRESULT;
        
        /// Creates a function delegate for a call into the runtime
        fn create_delegate(
            app_domain_id: DWORD,
            assembly_name: LPCWSTR,     
            class_name: LPCWSTR,     
            method_name: LPCWSTR,
            fn_ptr: *mut *const c_void) -> HRESULT;
    }
}