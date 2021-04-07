/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

extern crate ini;
extern crate winapi;

use ini::Ini;
use libc::time;
use serde::Serialize;
use serde_json::ser::to_writer;
use std::convert::TryInto;
use std::ffi::OsString;
use std::fs::{read_to_string, File};
use std::io::{BufRead, BufReader, Write};
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::slice::from_raw_parts;
use uuid::Uuid;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, MAX_PATH, PBOOL, PDWORD, TRUE};
use winapi::shared::winerror::{E_UNEXPECTED, S_OK};
use winapi::um::combaseapi::CoTaskMemFree;
use winapi::um::knownfolders::FOLDERID_RoamingAppData;
use winapi::um::processthreadsapi::{GetProcessId, GetThreadId, TerminateProcess};
use winapi::um::psapi::K32GetModuleFileNameExW;
use winapi::um::shlobj::SHGetKnownFolderPath;
use winapi::um::winbase::VerifyVersionInfoW;
use winapi::um::winnt::{
    VerSetConditionMask, CONTEXT, DWORDLONG, EXCEPTION_POINTERS, EXCEPTION_RECORD, HANDLE, HRESULT,
    LPOSVERSIONINFOEXW, OSVERSIONINFOEXW, PCWSTR, PEXCEPTION_POINTERS, PVOID, PWSTR,
    VER_GREATER_EQUAL, VER_MAJORVERSION, VER_MINORVERSION, VER_SERVICEPACKMAJOR,
    VER_SERVICEPACKMINOR,
};
use winapi::STRUCT;

#[no_mangle]
pub extern "C" fn OutOfProcessExceptionEventCallback(
    _context: PVOID,
    exception_information: PWER_RUNTIME_EXCEPTION_INFORMATION,
    b_ownership_claimed: PBOOL,
    _wsz_event_name: PWSTR,
    _pch_size: PDWORD,
    _dw_signature_count: PDWORD,
) -> HRESULT {
    match out_of_process_exception_event_callback(exception_information) {
        Ok(_) => {
            unsafe {
                // Inform WER that we claim ownership of this crash
                *b_ownership_claimed = TRUE;
                // Make sure that Firefox shuts down
                TerminateProcess((*exception_information).hProcess, 1);
            }
            S_OK
        }
        Err(_) => E_UNEXPECTED,
    }
}

#[no_mangle]
pub extern "C" fn OutOfProcessExceptionEventSignatureCallback(
    _context: PVOID,
    _exception_information: PWER_RUNTIME_EXCEPTION_INFORMATION,
    _w_index: DWORD,
    _wsz_name: PWSTR,
    _ch_name: PDWORD,
    _wsz_value: PWSTR,
    _ch_value: PDWORD,
) -> HRESULT {
    S_OK
}

#[no_mangle]
pub extern "C" fn OutOfProcessExceptionEventDebuggerLaunchCallback(
    _context: PVOID,
    _exception_information: PWER_RUNTIME_EXCEPTION_INFORMATION,
    b_is_custom_debugger: PBOOL,
    _wsz_debugger_launch: PWSTR,
    _ch_debugger_launch: PDWORD,
    _b_is_debugger_autolaunch: PBOOL,
) -> HRESULT {
    unsafe {
        *b_is_custom_debugger = FALSE;
    }

    S_OK
}

fn out_of_process_exception_event_callback(
    exception_information: PWER_RUNTIME_EXCEPTION_INFORMATION,
) -> Result<(), ()> {
    let process = unsafe { (*exception_information).hProcess };

    let application_path = get_application_path(process)?;
    let mut install_path = application_path.clone();
    install_path.pop();
    let application_data = ApplicationData::load_from_disk(install_path.as_ref())?;
    let release_channel = get_release_channel(install_path.as_ref())?;
    let crash_reports_dir = get_crash_reports_dir(&application_data)?;
    let install_time = get_install_time(&crash_reports_dir, &application_data.build_id)?;
    let crash_report = CrashReport::new(
        &crash_reports_dir,
        &release_channel,
        &application_data,
        &install_time,
    );
    crash_report.write_minidump(exception_information)?;
    crash_report.write_extra_file()?;
    crash_report.write_event_file()?;

    Ok(())
}

fn get_crash_reports_dir(application_data: &ApplicationData) -> Result<PathBuf, ()> {
    let mut psz_path: PWSTR = null_mut();
    unsafe {
        let res = SHGetKnownFolderPath(
            &FOLDERID_RoamingAppData as *const _,
            0,
            null_mut(),
            &mut psz_path as *mut _,
        );

        if res == S_OK {
            let mut len = 0;
            while psz_path.offset(len).read() != 0 {
                len += 1;
            }
            let str = OsString::from_wide(from_raw_parts(psz_path, len as usize));
            CoTaskMemFree(psz_path as _);
            let mut path = PathBuf::from(str);
            if let Some(vendor) = &application_data.vendor {
                path.push(vendor);
            }
            path.push(&application_data.name);
            path.push("Crash Reports");
            Ok(path)
        } else {
            Err(())
        }
    }
}

fn get_application_path(process: HANDLE) -> Result<PathBuf, ()> {
    let mut path: [u16; MAX_PATH + 1] = [0; MAX_PATH + 1];
    unsafe {
        let res = K32GetModuleFileNameExW(
            process,
            null_mut(),
            (&mut path).as_mut_ptr(),
            (MAX_PATH + 1) as u32,
        );

        if res == 0 {
            return Err(());
        }

        let application_path = PathBuf::from(OsString::from_wide(&path[0..res as usize]));
        Ok(application_path)
    }
}

fn get_release_channel(install_path: &Path) -> Result<String, ()> {
    let channel_prefs =
        File::open(install_path.join("defaults/pref/channel-prefs.js")).map_err(|_e| ())?;
    let lines = BufReader::new(channel_prefs).lines();
    let line = lines
        .filter_map(Result::ok)
        .find(|line| line.contains("app.update.channel"))
        .ok_or(())?;
    line.split("\"").nth(3).map(|s| s.to_string()).ok_or(())
}

fn get_install_time(crash_reports_path: &Path, build_id: &str) -> Result<String, ()> {
    let file_name = "InstallTime".to_owned() + build_id;
    let file_path = crash_reports_path.join(file_name);
    read_to_string(file_path).map_err(|_e| ())
}

#[derive(Debug)]
struct ApplicationData {
    vendor: Option<String>,
    name: String,
    version: String,
    build_id: String,
    product_id: String,
    server_url: String,
}

impl ApplicationData {
    fn load_from_disk(install_path: &Path) -> Result<ApplicationData, ()> {
        let ini_path = ApplicationData::get_path(install_path);
        let conf = Ini::load_from_file(ini_path).map_err(|_e| ())?;

        // Parse the "App" section
        let app_section = conf.section(Some("App")).ok_or(())?;
        let vendor = app_section.get("Vendor").map(|s| s.to_owned());
        let name = app_section.get("Name").ok_or(())?.to_owned();
        let version = app_section.get("Version").ok_or(())?.to_owned();
        let build_id = app_section.get("BuildID").ok_or(())?.to_owned();
        let product_id = app_section.get("ID").ok_or(())?.to_owned();

        // Parse the "Crash Reporter" section
        let crash_reporter_section = conf.section(Some("Crash Reporter")).ok_or(())?;
        let server_url = crash_reporter_section
            .get("ServerURL")
            .ok_or(())?
            .to_owned();

        // InstallTime<build_id>

        Ok(ApplicationData {
            vendor,
            name,
            version,
            build_id,
            product_id,
            server_url,
        })
    }

    fn get_path(install_path: &Path) -> PathBuf {
        install_path.join("application.ini")
    }
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct Annotations<'a> {
    CrashTime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    Vendor: Option<&'a str>,
    ProductName: &'a str,
    Version: &'a str,
    BuildID: &'a str,
    ProductID: &'a str,
    ServerURL: &'a str,
    ReleaseChannel: &'a str,
    InstallTime: &'a str,
}

impl Annotations<'_> {
    fn from_application_data<'a>(
        application_data: &'a ApplicationData,
        release_channel: &'a str,
        install_time: &'a str,
        crash_time: i64,
    ) -> Annotations<'a> {
        Annotations {
            CrashTime: crash_time.to_string(),
            Vendor: application_data.vendor.as_deref(),
            ProductName: &application_data.name,
            Version: &application_data.version,
            BuildID: &application_data.build_id,
            ProductID: &application_data.product_id,
            ServerURL: &application_data.server_url,
            ReleaseChannel: release_channel,
            InstallTime: install_time,
        }
    }
}

struct CrashReport<'a> {
    uuid: String,
    crash_reports_path: PathBuf,
    release_channel: &'a str,
    annotations: Annotations<'a>,
    crash_time: i64,
}

impl CrashReport<'_> {
    fn new<'a>(
        crash_reports_path: &Path,
        release_channel: &'a str,
        application_data: &'a ApplicationData,
        install_time: &'a str,
    ) -> CrashReport<'a> {
        let uuid = Uuid::new_v4()
            .to_hyphenated()
            .encode_lower(&mut Uuid::encode_buffer())
            .to_owned();
        let crash_reports_path = PathBuf::from(crash_reports_path);
        let crash_time: i64 = unsafe { time(null_mut()).into() };
        let annotations = Annotations::from_application_data(
            application_data,
            release_channel,
            install_time,
            crash_time,
        );
        CrashReport {
            uuid,
            crash_reports_path,
            release_channel,
            annotations,
            crash_time,
        }
    }

    fn is_nightly(&self) -> bool {
        self.release_channel == "nightly" || self.release_channel == "default"
    }

    fn get_minidump_type(&self) -> MINIDUMP_TYPE {
        let mut minidump_type = MiniDumpWithFullMemoryInfo | MiniDumpWithUnloadedModules;
        if self.is_nightly() {
            // This is Nightly only because this doubles the size of minidumps based
            // on the experimental data.
            minidump_type = minidump_type | MiniDumpWithProcessThreadData;

            // dbghelp.dll on Win7 can't handle overlapping memory regions so we only
            // enable this feature on Win8 or later.
            if is_windows8_or_later() {
                // This allows us to examine heap objects referenced from stack objects
                // at the cost of further doubling the size of minidumps.
                minidump_type = minidump_type | MiniDumpWithIndirectlyReferencedMemory
            }
        }
        minidump_type
    }

    fn get_pending_path(&self) -> PathBuf {
        self.crash_reports_path.join("pending")
    }

    fn get_events_path(&self) -> PathBuf {
        self.crash_reports_path.join("events")
    }

    fn get_minidump_path(&self) -> PathBuf {
        self.get_pending_path().join(self.uuid.to_string() + ".dmp")
    }

    fn get_extra_file_path(&self) -> PathBuf {
        self.get_pending_path()
            .join(self.uuid.to_string() + ".extra")
    }

    fn get_event_file_path(&self) -> PathBuf {
        self.get_events_path().join(self.uuid.to_string())
    }

    fn write_minidump(
        &self,
        exception_information: PWER_RUNTIME_EXCEPTION_INFORMATION,
    ) -> Result<(), ()> {
        let minidump_path = self.get_minidump_path();
        let minidump_file = File::create(minidump_path).map_err(|_e| ())?;
        let minidump_type: MINIDUMP_TYPE = self.get_minidump_type();

        unsafe {
            let mut exception_pointers = EXCEPTION_POINTERS {
                ExceptionRecord: &mut ((*exception_information).exceptionRecord),
                ContextRecord: &mut ((*exception_information).context),
            };

            let mut exception = MINIDUMP_EXCEPTION_INFORMATION {
                ThreadId: GetThreadId((*exception_information).hThread),
                ExceptionPointers: &mut exception_pointers,
                ClientPointers: FALSE,
            };

            let res = MiniDumpWriteDump(
                (*exception_information).hProcess,
                GetProcessId((*exception_information).hProcess),
                minidump_file.as_raw_handle() as _,
                minidump_type,
                &mut exception,
                /* userStream */ null(),
                /* callback */ null(),
            );

            match res {
                FALSE => Err(()),
                _ => Ok(()),
            }
        }
    }

    fn write_extra_file(&self) -> Result<(), ()> {
        let extra_file = File::create(self.get_extra_file_path()).map_err(|_e| ())?;
        to_writer(extra_file, &self.annotations).map_err(|_e| ())
    }

    fn write_event_file(&self) -> Result<(), ()> {
        let mut event_file = File::create(self.get_event_file_path()).map_err(|_e| ())?;
        writeln!(event_file, "crash.main.3").map_err(|_e| ())?;
        writeln!(event_file, "{}", self.crash_time).map_err(|_e| ())?;
        writeln!(event_file, "{}", self.uuid).map_err(|_e| ())?;
        to_writer(event_file, &self.annotations).map_err(|_e| ())
    }
}

fn is_windows8_or_later() -> bool {
    let mut info = OSVERSIONINFOEXW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOEXW>().try_into().unwrap(),
        dwMajorVersion: 6,
        dwMinorVersion: 2,
        ..Default::default()
    };

    unsafe {
        let mut mask: DWORDLONG = 0;
        mask = VerSetConditionMask(mask, VER_MAJORVERSION, VER_GREATER_EQUAL);
        mask = VerSetConditionMask(mask, VER_MINORVERSION, VER_GREATER_EQUAL);
        mask = VerSetConditionMask(mask, VER_SERVICEPACKMAJOR, VER_GREATER_EQUAL);
        mask = VerSetConditionMask(mask, VER_SERVICEPACKMINOR, VER_GREATER_EQUAL);

        let res = VerifyVersionInfoW(
            &mut info as LPOSVERSIONINFOEXW,
            VER_MAJORVERSION | VER_MINORVERSION | VER_SERVICEPACKMAJOR | VER_SERVICEPACKMINOR,
            mask,
        );

        res != FALSE
    }
}

/******************************************************************************
 * The stuff below should be migrated to the winapi crate, see bug 1696414    *
 ******************************************************************************/

// we can't use winapi's ENUM macro directly because it doesn't support
// attributes, so let's define this one here until we migrate this code
macro_rules! ENUM {
    {enum $name:ident { $($variant:ident = $value:expr,)+ }} => {
        #[allow(non_camel_case_types)] pub type $name = u32;
        $(#[allow(non_upper_case_globals)] pub const $variant: $name = $value;)+
    };
}

// winapi doesn't export the FN macro, so we duplicate it here
macro_rules! FN {
    (stdcall $func:ident($($t:ty,)*) -> $ret:ty) => (
        #[allow(non_camel_case_types)] pub type $func = Option<unsafe extern "system" fn($($t,)*) -> $ret>;
    );
    (stdcall $func:ident($($p:ident: $t:ty,)*) -> $ret:ty) => (
        #[allow(non_camel_case_types)] pub type $func = Option<unsafe extern "system" fn($($p: $t,)*) -> $ret>;
    );
}

// From um/WerApi.h

STRUCT! {#[allow(non_snake_case)] struct WER_RUNTIME_EXCEPTION_INFORMATION
{
    dwSize: DWORD,
    hProcess: HANDLE,
    hThread: HANDLE,
    exceptionRecord: EXCEPTION_RECORD,
    context: CONTEXT,
    pwszReportId: PCWSTR,
    bIsFatal: BOOL,
    dwReserved: DWORD,
}}

#[allow(non_camel_case_types)]
pub type PWER_RUNTIME_EXCEPTION_INFORMATION = *mut WER_RUNTIME_EXCEPTION_INFORMATION;

// From minidumpapiset.hProcess

STRUCT! {#[allow(non_snake_case)] #[repr(packed(4))] struct MINIDUMP_EXCEPTION_INFORMATION {
    ThreadId: DWORD,
    ExceptionPointers: PEXCEPTION_POINTERS,
    ClientPointers: BOOL,
}}

#[allow(non_camel_case_types)]
pub type PMINIDUMP_EXCEPTION_INFORMATION = *mut MINIDUMP_EXCEPTION_INFORMATION;

ENUM! { enum MINIDUMP_TYPE {
    MiniDumpNormal                         = 0x00000000,
    MiniDumpWithDataSegs                   = 0x00000001,
    MiniDumpWithFullMemory                 = 0x00000002,
    MiniDumpWithHandleData                 = 0x00000004,
    MiniDumpFilterMemory                   = 0x00000008,
    MiniDumpScanMemory                     = 0x00000010,
    MiniDumpWithUnloadedModules            = 0x00000020,
    MiniDumpWithIndirectlyReferencedMemory = 0x00000040,
    MiniDumpFilterModulePaths              = 0x00000080,
    MiniDumpWithProcessThreadData          = 0x00000100,
    MiniDumpWithPrivateReadWriteMemory     = 0x00000200,
    MiniDumpWithoutOptionalData            = 0x00000400,
    MiniDumpWithFullMemoryInfo             = 0x00000800,
    MiniDumpWithThreadInfo                 = 0x00001000,
    MiniDumpWithCodeSegs                   = 0x00002000,
    MiniDumpWithoutAuxiliaryState          = 0x00004000,
    MiniDumpWithFullAuxiliaryState         = 0x00008000,
    MiniDumpWithPrivateWriteCopyMemory     = 0x00010000,
    MiniDumpIgnoreInaccessibleMemory       = 0x00020000,
    MiniDumpWithTokenInformation           = 0x00040000,
    MiniDumpWithModuleHeaders              = 0x00080000,
    MiniDumpFilterTriage                   = 0x00100000,
    MiniDumpWithAvxXStateContext           = 0x00200000,
    MiniDumpWithIptTrace                   = 0x00400000,
    MiniDumpScanInaccessiblePartialPages   = 0x00800000,
    MiniDumpValidTypeFlags                 = 0x00ffffff,
}}

// We don't actually need the following three structs so we use placeholders
STRUCT! {#[allow(non_snake_case)] struct MINIDUMP_CALLBACK_INPUT {
    dummy: u32,
}}

#[allow(non_camel_case_types)]
pub type PMINIDUMP_CALLBACK_INPUT = *const MINIDUMP_CALLBACK_INPUT;

STRUCT! {#[allow(non_snake_case)] struct MINIDUMP_USER_STREAM_INFORMATION {
    dummy: u32,
}}

#[allow(non_camel_case_types)]
pub type PMINIDUMP_USER_STREAM_INFORMATION = *const MINIDUMP_USER_STREAM_INFORMATION;

STRUCT! {#[allow(non_snake_case)] struct MINIDUMP_CALLBACK_OUTPUT {
    dummy: u32,
}}

#[allow(non_camel_case_types)]
pub type PMINIDUMP_CALLBACK_OUTPUT = *const MINIDUMP_CALLBACK_OUTPUT;

// MiniDumpWriteDump() function and structs
FN! {stdcall MINIDUMP_CALLBACK_ROUTINE(
CallbackParam: PVOID,
CallbackInput: PMINIDUMP_CALLBACK_INPUT,
CallbackOutput: PMINIDUMP_CALLBACK_OUTPUT,
) -> BOOL}

STRUCT! {#[allow(non_snake_case)] #[repr(packed(4))] struct MINIDUMP_CALLBACK_INFORMATION {
    CallbackRoutine: MINIDUMP_CALLBACK_ROUTINE,
    CallbackParam: PVOID,
}}

#[allow(non_camel_case_types)]
pub type PMINIDUMP_CALLBACK_INFORMATION = *const MINIDUMP_CALLBACK_INFORMATION;

extern "system" {
    pub fn MiniDumpWriteDump(
        hProcess: HANDLE,
        ProcessId: DWORD,
        hFile: HANDLE,
        DumpType: MINIDUMP_TYPE,
        Exceptionparam: PMINIDUMP_EXCEPTION_INFORMATION,
        UserStreamParam: PMINIDUMP_USER_STREAM_INFORMATION,
        CallbackParam: PMINIDUMP_CALLBACK_INFORMATION,
    ) -> BOOL;
}
