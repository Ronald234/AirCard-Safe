use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, bail};
use libloading::{Library, Symbol};

// Minimal advapi32 registry bindings used to locate the Microsoft Store
// ("Apple Devices" / Store iTunes) package that ships the device DLLs.
type HKey = *mut std::ffi::c_void;
// Windows defines predefined handles by sign-extending a signed LONG.
const HKEY_CURRENT_USER: HKey = 0x8000_0001u32 as i32 as isize as HKey;
const KEY_READ: u32 = 0x2_0019;
const ERROR_SUCCESS: i32 = 0;
const REG_SZ: u32 = 1;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hKey: HKey,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut HKey,
    ) -> i32;
    fn RegEnumKeyExW(
        hKey: HKey,
        dwIndex: u32,
        lpName: *mut u16,
        lpcchName: *mut u32,
        lpReserved: *mut u32,
        lpClass: *mut u16,
        lpcchClass: *mut u32,
        lpftLastWriteTime: *mut std::ffi::c_void,
    ) -> i32;
    fn RegQueryValueExW(
        hKey: HKey,
        lpValueName: *const u16,
        lpReserved: *mut u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    fn RegCloseKey(hKey: HKey) -> i32;
}

const APPLE_SUPPORT_DIRS: &[&str] = &[
    r"C:\Program Files\Common Files\Apple\Mobile Device Support",
    r"C:\Program Files (x86)\Common Files\Apple\Mobile Device Support",
];

const APPMODEL_PACKAGES_KEY: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";
const APPLE_PACKAGE_PREFIXES: &[&str] = &["AppleInc.AppleDevices", "AppleInc.iTunes"];

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Read a `REG_SZ` value from an open registry key.
///
/// # Safety
/// `key` must be a valid handle opened with at least `KEY_QUERY_VALUE` access.
unsafe fn reg_read_string(key: HKey, value_name: &str) -> Option<String> {
    let name = to_wide(value_name);
    let mut value_type: u32 = 0;
    let mut size_bytes: u32 = 0;

    let status = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            ptr::null_mut(),
            &mut size_bytes,
        )
    };
    if status != ERROR_SUCCESS
        || value_type != REG_SZ
        || !(2..=65536).contains(&size_bytes)
        || size_bytes % 2 != 0
    {
        return None;
    }

    let mut buffer = vec![0u8; size_bytes as usize];
    let status = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            buffer.as_mut_ptr(),
            &mut size_bytes,
        )
    };
    if status != ERROR_SUCCESS
        || value_type != REG_SZ
        || size_bytes as usize > buffer.len()
        || size_bytes % 2 != 0
    {
        return None;
    }

    let (pairs, _) = buffer[..size_bytes as usize].as_chunks::<2>();
    let u16_units: Vec<u16> = pairs
        .iter()
        .map(|&[lo, hi]| u16::from_le_bytes([lo, hi]))
        .collect();
    let text = String::from_utf16(&u16_units).ok()?;
    let trimmed = text.trim_end_matches('\0');
    if trimmed.is_empty() || trimmed.contains('\0') {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Discover install folders of registered MSIX packages whose name starts with
/// one of `prefixes`, via `PackageRootFolder` in the per-user AppModel repo.
fn msix_package_roots(prefixes: &[&str]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    unsafe {
        let mut repo_key: HKey = ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            to_wide(APPMODEL_PACKAGES_KEY).as_ptr(),
            0,
            KEY_READ,
            &mut repo_key,
        ) != ERROR_SUCCESS
        {
            return roots;
        }

        let mut index: u32 = 0;
        while index < 16384 {
            let mut name_buf = [0u16; 512];
            let mut name_len = name_buf.len() as u32;
            let status = RegEnumKeyExW(
                repo_key,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if status != ERROR_SUCCESS {
                break;
            }
            index += 1;

            let package_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            if !prefixes
                .iter()
                .any(|p| package_name.starts_with(&format!("{p}_")))
            {
                continue;
            }

            let mut pkg_key: HKey = ptr::null_mut();
            if RegOpenKeyExW(
                repo_key,
                to_wide(&package_name).as_ptr(),
                0,
                KEY_READ,
                &mut pkg_key,
            ) == ERROR_SUCCESS
            {
                if let Some(folder) = reg_read_string(pkg_key, "PackageRootFolder") {
                    roots.push(PathBuf::from(folder));
                }
                RegCloseKey(pkg_key);
            }
        }

        RegCloseKey(repo_key);
    }
    roots
}

pub type CFTypeRef = *const std::ffi::c_void;
pub type CFStringRef = *const std::ffi::c_void;
pub type CFDataRef = *const std::ffi::c_void;
pub type CFDictionaryRef = *const std::ffi::c_void;
pub type CFArrayRef = *const std::ffi::c_void;
pub type CFPropertyListRef = *const std::ffi::c_void;
pub type CFAllocatorRef = *const std::ffi::c_void;
pub type CFIndex = isize;
pub type CFStringEncoding = u32;

pub const K_CFSTRING_ENCODING_UTF8: CFStringEncoding = 0x08000100;
pub const K_CFPROPERTY_LIST_BINARY_FORMAT_V1_0: isize = 200;

pub type ATHostConnectionRef = *mut std::ffi::c_void;

pub type AMDeviceRef = *const std::ffi::c_void;
pub type AMDeviceNotificationRef = *const std::ffi::c_void;
pub type AMDServiceConnectionRef = *mut std::ffi::c_void;
pub type AFCConnectionRef = *mut std::ffi::c_void;
pub type AFCKeyValueRef = *mut std::ffi::c_void;
pub type AFCFileRef = u64;
pub type AFCDirectoryRef = *mut std::ffi::c_void;

#[repr(C)]
pub struct AMDeviceNotificationCallbackInfo {
    pub device: AMDeviceRef,
    pub message: u32,
}

pub type AMDeviceNotificationCallback =
    extern "C" fn(*const AMDeviceNotificationCallbackInfo, *mut std::ffi::c_void);

#[allow(dead_code)]
pub struct AppleLibraries {
    _cf_lib: Library,
    _md_lib: Library,
    #[cfg(test)]
    _ath_lib: Library,

    // CoreFoundation functions
    pub cf_string_create: unsafe extern "C" fn(
        CFAllocatorRef,
        *const std::ffi::c_char,
        CFStringEncoding,
    ) -> CFStringRef,
    pub cf_string_get_length: unsafe extern "C" fn(CFStringRef) -> CFIndex,
    pub cf_string_get_max_size: unsafe extern "C" fn(CFIndex, CFStringEncoding) -> CFIndex,
    pub cf_string_get_c_string:
        unsafe extern "C" fn(CFStringRef, *mut std::ffi::c_char, CFIndex, CFStringEncoding) -> u8,
    pub cf_data_create: unsafe extern "C" fn(CFAllocatorRef, *const u8, CFIndex) -> CFDataRef,
    pub cf_data_get_byte_ptr: unsafe extern "C" fn(CFDataRef) -> *const u8,
    pub cf_data_get_length: unsafe extern "C" fn(CFDataRef) -> CFIndex,
    pub cf_property_list_create_with_data: unsafe extern "C" fn(
        CFAllocatorRef,
        CFDataRef,
        usize,
        *mut isize,
        *mut CFTypeRef,
    ) -> CFPropertyListRef,
    pub cf_property_list_create_data: unsafe extern "C" fn(
        CFAllocatorRef,
        CFPropertyListRef,
        isize,
        usize,
        *mut CFTypeRef,
    ) -> CFDataRef,
    pub cf_release: unsafe extern "C" fn(CFTypeRef),
    pub cf_retain: unsafe extern "C" fn(CFTypeRef) -> CFTypeRef,
    pub cf_equal: unsafe extern "C" fn(CFTypeRef, CFTypeRef) -> u32,
    pub cf_run_loop_get_main: unsafe extern "C" fn() -> *const std::ffi::c_void,
    pub cf_run_loop_run_in_mode: unsafe extern "C" fn(CFStringRef, f64, u8) -> i32,
    pub cf_run_loop_stop: unsafe extern "C" fn(*const std::ffi::c_void),

    // MobileDevice functions
    pub am_device_create_from_properties: unsafe extern "C" fn(CFDictionaryRef) -> AMDeviceRef,
    pub am_device_notification_subscribe: unsafe extern "C" fn(
        AMDeviceNotificationCallback,
        i32,
        u32,
        *mut std::ffi::c_void,
        *mut AMDeviceNotificationRef,
        CFDictionaryRef,
    ) -> i32,
    pub am_device_notification_unsubscribe: unsafe extern "C" fn(AMDeviceNotificationRef) -> i32,
    pub am_device_copy_device_identifier: unsafe extern "C" fn(AMDeviceRef) -> CFStringRef,
    pub am_device_copy_value:
        unsafe extern "C" fn(AMDeviceRef, CFStringRef, CFStringRef) -> CFTypeRef,
    pub am_device_connect: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_disconnect: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_is_paired: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_pair: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_validate_pairing: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_start_session: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_stop_session: unsafe extern "C" fn(AMDeviceRef) -> i32,
    pub am_device_secure_start_service: unsafe extern "C" fn(
        AMDeviceRef,
        CFStringRef,
        CFDictionaryRef,
        *mut AMDServiceConnectionRef,
    ) -> i32,
    pub amd_service_connection_get_socket: unsafe extern "C" fn(AMDServiceConnectionRef) -> i32,
    pub amd_service_connection_get_secure_io_context:
        unsafe extern "C" fn(AMDServiceConnectionRef) -> *mut std::ffi::c_void,
    pub amd_service_connection_invalidate: unsafe extern "C" fn(AMDServiceConnectionRef) -> i32,
    pub amd_service_connection_send:
        unsafe extern "C" fn(AMDServiceConnectionRef, *const u8, usize) -> i32,
    pub amd_service_connection_receive:
        unsafe extern "C" fn(AMDServiceConnectionRef, *mut u8, usize) -> i32,
    pub amd_service_connection_send_message:
        unsafe extern "C" fn(AMDServiceConnectionRef, CFTypeRef, isize) -> i32,
    pub amd_service_connection_receive_message:
        unsafe extern "C" fn(AMDServiceConnectionRef, *mut CFTypeRef, *mut isize) -> i32,

    // AFC functions
    pub afc_connection_open: unsafe extern "C" fn(i32, u32, *mut AFCConnectionRef) -> i32,
    pub afc_connection_close: unsafe extern "C" fn(AFCConnectionRef) -> i32,
    pub afc_connection_set_secure_context:
        unsafe extern "C" fn(AFCConnectionRef, *mut std::ffi::c_void) -> i32,
    pub afc_connection_set_dispose_secure_context:
        unsafe extern "C" fn(AFCConnectionRef, i32) -> i32,
    pub afc_connection_set_io_timeout: unsafe extern "C" fn(AFCConnectionRef, u32) -> i32,
    pub afc_file_info_open:
        unsafe extern "C" fn(AFCConnectionRef, *const std::ffi::c_char, *mut AFCKeyValueRef) -> i32,
    pub afc_key_value_read: unsafe extern "C" fn(
        AFCKeyValueRef,
        *mut *const std::ffi::c_char,
        *mut *const std::ffi::c_char,
    ) -> i32,
    pub afc_key_value_close: unsafe extern "C" fn(AFCKeyValueRef) -> i32,
    pub afc_file_ref_open: unsafe extern "C" fn(
        AFCConnectionRef,
        *const std::ffi::c_char,
        u64,
        *mut AFCFileRef,
    ) -> i32,
    pub afc_file_ref_read:
        unsafe extern "C" fn(AFCConnectionRef, AFCFileRef, *mut u8, *mut isize) -> i32,
    pub afc_file_ref_write:
        unsafe extern "C" fn(AFCConnectionRef, AFCFileRef, *const u8, isize) -> i32,
    pub afc_file_ref_close: unsafe extern "C" fn(AFCConnectionRef, AFCFileRef) -> i32,
    pub afc_directory_open: unsafe extern "C" fn(
        AFCConnectionRef,
        *const std::ffi::c_char,
        *mut AFCDirectoryRef,
    ) -> i32,
    pub afc_directory_read: unsafe extern "C" fn(
        AFCConnectionRef,
        AFCDirectoryRef,
        *mut *const std::ffi::c_char,
    ) -> i32,
    pub afc_directory_close: unsafe extern "C" fn(AFCConnectionRef, AFCDirectoryRef) -> i32,
    pub afc_directory_create:
        unsafe extern "C" fn(AFCConnectionRef, *const std::ffi::c_char) -> i32,
    pub afc_remove_path: unsafe extern "C" fn(AFCConnectionRef, *const std::ffi::c_char) -> i32,

    // AirTrafficHost functions
    #[cfg(test)]
    pub at_host_connection_create: unsafe extern "C" fn(CFStringRef) -> ATHostConnectionRef,
    #[cfg(test)]
    pub at_host_connection_release: unsafe extern "C" fn(ATHostConnectionRef),
    #[cfg(test)]
    pub at_host_connection_send_host_info:
        unsafe extern "C" fn(ATHostConnectionRef, CFDictionaryRef),
    #[cfg(test)]
    pub at_host_connection_send_sync_request:
        unsafe extern "C" fn(ATHostConnectionRef, CFArrayRef, CFDictionaryRef, CFDictionaryRef),
    #[cfg(test)]
    pub at_host_connection_send_metadata_sync_finished:
        unsafe extern "C" fn(ATHostConnectionRef, CFDictionaryRef, CFDictionaryRef),
    #[cfg(test)]
    pub at_host_connection_send_asset_completed:
        unsafe extern "C" fn(ATHostConnectionRef, CFStringRef, CFStringRef, CFStringRef),
    #[cfg(test)]
    pub at_host_connection_read_message:
        unsafe extern "C" fn(ATHostConnectionRef) -> CFDictionaryRef,
    #[cfg(test)]
    pub at_cf_message_get_name: unsafe extern "C" fn(CFDictionaryRef) -> CFStringRef,
    #[cfg(test)]
    pub at_cf_message_get_param: unsafe extern "C" fn(CFDictionaryRef, CFStringRef) -> CFTypeRef,
}

static LIBRARIES: OnceLock<Arc<AppleLibraries>> = OnceLock::new();

pub fn locate_support_dir() -> Option<PathBuf> {
    APPLE_SUPPORT_DIRS
        .iter()
        .map(PathBuf::from)
        .chain(
            msix_package_roots(APPLE_PACKAGE_PREFIXES)
                .into_iter()
                .filter(|root| is_apple_store_root(root))
                .filter_map(|root| root.canonicalize().ok())
                .filter(|root| is_apple_store_root(root))
                .flat_map(|root| [root.join("AMDS64"), root])
                .filter_map(|candidate| candidate.canonicalize().ok())
                .filter(|candidate| {
                    is_apple_store_root(candidate)
                        || candidate.file_name().is_some_and(|name| name == "AMDS64")
                            && candidate.parent().is_some_and(is_apple_store_root)
                }),
        )
        .find(|dir| has_required_dlls(dir))
}

fn has_required_dlls(dir: &Path) -> bool {
    dir.join("CoreFoundation.dll").is_file() && dir.join("MobileDevice.dll").is_file()
}

pub fn get_apple_libraries() -> Result<Arc<AppleLibraries>> {
    if let Some(libs) = LIBRARIES.get() {
        return Ok(Arc::clone(libs));
    }

    let dir = locate_support_dir().context(
        "Apple Mobile Device Support was not found. Install Apple Devices or the 64-bit iTunes package from Apple.",
    )?;

    let cf_path = dir.join("CoreFoundation.dll");
    let md_path = dir.join("MobileDevice.dll");
    #[cfg(test)]
    let ath_path = dir.join("AirTrafficHost.dll");

    unsafe {
        let cf_lib = load_support_library(&cf_path).context("Failed to load CoreFoundation.dll from the installed Apple runtime. If Store package access is denied, install Apple's standalone 64-bit Mobile Device Support runtime.")?;
        let md_lib = load_support_library(&md_path)
            .context("Failed to load MobileDevice.dll from the installed Apple runtime")?;
        #[cfg(test)]
        let ath_lib =
            load_support_library(&ath_path).context("Failed to load AirTrafficHost.dll")?;

        macro_rules! load_sym {
            ($lib:expr, $name:expr) => {{
                let symbol: Symbol<_> = $lib
                    .get($name.as_bytes())
                    .context(concat!("Missing symbol: ", $name))?;
                *symbol
            }};
        }

        let cf_string_create = load_sym!(cf_lib, "CFStringCreateWithCString");
        let cf_string_get_length = load_sym!(cf_lib, "CFStringGetLength");
        let cf_string_get_max_size = load_sym!(cf_lib, "CFStringGetMaximumSizeForEncoding");
        let cf_string_get_c_string = load_sym!(cf_lib, "CFStringGetCString");
        let cf_data_create = load_sym!(cf_lib, "CFDataCreate");
        let cf_data_get_byte_ptr = load_sym!(cf_lib, "CFDataGetBytePtr");
        let cf_data_get_length = load_sym!(cf_lib, "CFDataGetLength");
        let cf_property_list_create_with_data = load_sym!(cf_lib, "CFPropertyListCreateWithData");
        let cf_property_list_create_data = load_sym!(cf_lib, "CFPropertyListCreateData");
        let cf_release = load_sym!(cf_lib, "CFRelease");
        let cf_retain = load_sym!(cf_lib, "CFRetain");
        let cf_equal = load_sym!(cf_lib, "CFEqual");
        let cf_run_loop_get_main = load_sym!(cf_lib, "CFRunLoopGetMain");
        let cf_run_loop_run_in_mode = load_sym!(cf_lib, "CFRunLoopRunInMode");
        let cf_run_loop_stop = load_sym!(cf_lib, "CFRunLoopStop");

        let am_device_create_from_properties = load_sym!(md_lib, "AMDeviceCreateFromProperties");
        let am_device_notification_subscribe =
            load_sym!(md_lib, "AMDeviceNotificationSubscribeWithOptions");
        let am_device_notification_unsubscribe =
            load_sym!(md_lib, "AMDeviceNotificationUnsubscribe");
        let am_device_copy_device_identifier = load_sym!(md_lib, "AMDeviceCopyDeviceIdentifier");
        let am_device_copy_value = load_sym!(md_lib, "AMDeviceCopyValue");
        let am_device_connect = load_sym!(md_lib, "AMDeviceConnect");
        let am_device_disconnect = load_sym!(md_lib, "AMDeviceDisconnect");
        let am_device_is_paired = load_sym!(md_lib, "AMDeviceIsPaired");
        let am_device_pair = load_sym!(md_lib, "AMDevicePair");
        let am_device_validate_pairing = load_sym!(md_lib, "AMDeviceValidatePairing");
        let am_device_start_session = load_sym!(md_lib, "AMDeviceStartSession");
        let am_device_stop_session = load_sym!(md_lib, "AMDeviceStopSession");
        let am_device_secure_start_service = load_sym!(md_lib, "AMDeviceSecureStartService");
        let amd_service_connection_get_socket = load_sym!(md_lib, "AMDServiceConnectionGetSocket");
        let amd_service_connection_get_secure_io_context =
            load_sym!(md_lib, "AMDServiceConnectionGetSecureIOContext");
        let amd_service_connection_invalidate = load_sym!(md_lib, "AMDServiceConnectionInvalidate");
        let amd_service_connection_send = load_sym!(md_lib, "AMDServiceConnectionSend");
        let amd_service_connection_receive = load_sym!(md_lib, "AMDServiceConnectionReceive");
        let amd_service_connection_send_message =
            load_sym!(md_lib, "AMDServiceConnectionSendMessage");
        let amd_service_connection_receive_message =
            load_sym!(md_lib, "AMDServiceConnectionReceiveMessage");

        let afc_connection_open = load_sym!(md_lib, "AFCConnectionOpen");
        let afc_connection_close = load_sym!(md_lib, "AFCConnectionClose");
        let afc_connection_set_secure_context = load_sym!(md_lib, "AFCConnectionSetSecureContext");
        let afc_connection_set_dispose_secure_context =
            load_sym!(md_lib, "AFCConnectionSetDisposeSecureContextOnInvalidate");
        let afc_connection_set_io_timeout = load_sym!(md_lib, "AFCConnectionSetIOTimeout");
        let afc_file_info_open = load_sym!(md_lib, "AFCFileInfoOpen");
        let afc_key_value_read = load_sym!(md_lib, "AFCKeyValueRead");
        let afc_key_value_close = load_sym!(md_lib, "AFCKeyValueClose");
        let afc_file_ref_open = load_sym!(md_lib, "AFCFileRefOpen");
        let afc_file_ref_read = load_sym!(md_lib, "AFCFileRefRead");
        let afc_file_ref_write = load_sym!(md_lib, "AFCFileRefWrite");
        let afc_file_ref_close = load_sym!(md_lib, "AFCFileRefClose");
        let afc_directory_open = load_sym!(md_lib, "AFCDirectoryOpen");
        let afc_directory_read = load_sym!(md_lib, "AFCDirectoryRead");
        let afc_directory_close = load_sym!(md_lib, "AFCDirectoryClose");
        let afc_directory_create = load_sym!(md_lib, "AFCDirectoryCreate");
        let afc_remove_path = load_sym!(md_lib, "AFCRemovePath");

        #[cfg(test)]
        let at_host_connection_create = load_sym!(ath_lib, "ATHostConnectionCreate");
        #[cfg(test)]
        let at_host_connection_release = load_sym!(ath_lib, "ATHostConnectionRelease");
        #[cfg(test)]
        let at_host_connection_send_host_info = load_sym!(ath_lib, "ATHostConnectionSendHostInfo");
        #[cfg(test)]
        let at_host_connection_send_sync_request =
            load_sym!(ath_lib, "ATHostConnectionSendSyncRequest");
        #[cfg(test)]
        let at_host_connection_send_metadata_sync_finished =
            load_sym!(ath_lib, "ATHostConnectionSendMetadataSyncFinished");
        #[cfg(test)]
        let at_host_connection_send_asset_completed =
            load_sym!(ath_lib, "ATHostConnectionSendAssetCompleted");
        #[cfg(test)]
        let at_host_connection_read_message = load_sym!(ath_lib, "ATHostConnectionReadMessage");
        #[cfg(test)]
        let at_cf_message_get_name = load_sym!(ath_lib, "ATCFMessageGetName");
        #[cfg(test)]
        let at_cf_message_get_param = load_sym!(ath_lib, "ATCFMessageGetParam");

        let apple_libs = Arc::new(AppleLibraries {
            _cf_lib: cf_lib,
            _md_lib: md_lib,
            #[cfg(test)]
            _ath_lib: ath_lib,

            cf_string_create,
            cf_string_get_length,
            cf_string_get_max_size,
            cf_string_get_c_string,
            cf_data_create,
            cf_data_get_byte_ptr,
            cf_data_get_length,
            cf_property_list_create_with_data,
            cf_property_list_create_data,
            cf_release,
            cf_retain,
            cf_equal,
            cf_run_loop_get_main,
            cf_run_loop_run_in_mode,
            cf_run_loop_stop,

            am_device_create_from_properties,
            am_device_notification_subscribe,
            am_device_notification_unsubscribe,
            am_device_copy_device_identifier,
            am_device_copy_value,
            am_device_connect,
            am_device_disconnect,
            am_device_is_paired,
            am_device_pair,
            am_device_validate_pairing,
            am_device_start_session,
            am_device_stop_session,
            am_device_secure_start_service,
            amd_service_connection_get_socket,
            amd_service_connection_get_secure_io_context,
            amd_service_connection_invalidate,
            amd_service_connection_send,
            amd_service_connection_receive,
            amd_service_connection_send_message,
            amd_service_connection_receive_message,

            afc_connection_open,
            afc_connection_close,
            afc_connection_set_secure_context,
            afc_connection_set_dispose_secure_context,
            afc_connection_set_io_timeout,
            afc_file_info_open,
            afc_key_value_read,
            afc_key_value_close,
            afc_file_ref_open,
            afc_file_ref_read,
            afc_file_ref_write,
            afc_file_ref_close,
            afc_directory_open,
            afc_directory_read,
            afc_directory_close,
            afc_directory_create,
            afc_remove_path,

            #[cfg(test)]
            at_host_connection_create,
            #[cfg(test)]
            at_host_connection_release,
            #[cfg(test)]
            at_host_connection_send_host_info,
            #[cfg(test)]
            at_host_connection_send_sync_request,
            #[cfg(test)]
            at_host_connection_send_metadata_sync_finished,
            #[cfg(test)]
            at_host_connection_send_asset_completed,
            #[cfg(test)]
            at_host_connection_read_message,
            #[cfg(test)]
            at_cf_message_get_name,
            #[cfg(test)]
            at_cf_message_get_param,
        });

        let _ = LIBRARIES.set(Arc::clone(&apple_libs));
        Ok(apple_libs)
    }
}

pub struct CFStringGuard {
    pub raw: CFStringRef,
    libs: Arc<AppleLibraries>,
}

impl Drop for CFStringGuard {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                (self.libs.cf_release)(self.raw);
            }
        }
    }
}

pub struct CFTypeGuard {
    pub raw: CFTypeRef,
    libs: Arc<AppleLibraries>,
}

impl Drop for CFTypeGuard {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                (self.libs.cf_release)(self.raw);
            }
        }
    }
}

impl AppleLibraries {
    pub fn create_cf_string(&self, s: &str) -> Result<CFStringGuard> {
        let c_str = CString::new(s).context("String contains null byte")?;
        let raw = unsafe {
            (self.cf_string_create)(ptr::null(), c_str.as_ptr(), K_CFSTRING_ENCODING_UTF8)
        };
        if raw.is_null() {
            bail!("Failed to create CFString for {}", s);
        }
        Ok(CFStringGuard {
            raw,
            libs: get_apple_libraries()?,
        })
    }

    pub fn to_rust_string(&self, cf_str: CFStringRef) -> String {
        if cf_str.is_null() {
            return String::new();
        }
        unsafe {
            let length = (self.cf_string_get_length)(cf_str);
            let max_bytes = (self.cf_string_get_max_size)(length, K_CFSTRING_ENCODING_UTF8) + 1;
            let mut buffer: Vec<u8> = vec![0; max_bytes as usize];
            if (self.cf_string_get_c_string)(
                cf_str,
                buffer.as_mut_ptr() as *mut std::ffi::c_char,
                max_bytes,
                K_CFSTRING_ENCODING_UTF8,
            ) != 0
            {
                if let Ok(c_str) =
                    CStr::from_ptr(buffer.as_ptr() as *const std::ffi::c_char).to_str()
                {
                    return c_str.to_owned();
                }
            }
        }
        String::new()
    }

    pub fn create_cf_plist_from_bytes(&self, bytes: &[u8]) -> Result<CFTypeGuard> {
        unsafe {
            let cf_data = (self.cf_data_create)(ptr::null(), bytes.as_ptr(), bytes.len() as isize);
            if cf_data.is_null() {
                bail!("Failed to create CFData");
            }
            let plist = (self.cf_property_list_create_with_data)(
                ptr::null(),
                cf_data,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            (self.cf_release)(cf_data);
            if plist.is_null() {
                bail!("Failed to parse plist data into CFPropertyList");
            }
            Ok(CFTypeGuard {
                raw: plist,
                libs: get_apple_libraries()?,
            })
        }
    }

    pub fn cf_plist_to_bytes(&self, plist: CFPropertyListRef) -> Result<Vec<u8>> {
        if plist.is_null() {
            bail!("Null CFPropertyList");
        }
        unsafe {
            let cf_data = (self.cf_property_list_create_data)(
                ptr::null(),
                plist,
                K_CFPROPERTY_LIST_BINARY_FORMAT_V1_0,
                0,
                ptr::null_mut(),
            );
            if cf_data.is_null() {
                bail!("Failed to serialize CFPropertyList to binary data");
            }
            let ptr = (self.cf_data_get_byte_ptr)(cf_data);
            let len = (self.cf_data_get_length)(cf_data) as usize;
            let slice = std::slice::from_raw_parts(ptr, len);
            let result = slice.to_vec();
            (self.cf_release)(cf_data);
            Ok(result)
        }
    }
}

pub fn verify_support() -> Result<String> {
    let _libs = get_apple_libraries()?;
    let dir = locate_support_dir().unwrap_or_default();
    Ok(format!(
        "Apple Mobile Device Support ready: {}",
        dir.display()
    ))
}

fn is_apple_store_root(root: &Path) -> bool {
    let text = root.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    let root = Path::new(text);
    let Some(name) = root.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    root.parent()
        .is_some_and(|parent| parent == Path::new(r"C:\Program Files\WindowsApps"))
        && APPLE_PACKAGE_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(&format!("{prefix}_")))
        && name.ends_with("_x64__nzyj5cx40ttqa")
        && !root
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
}

unsafe fn load_support_library(path: &Path) -> Result<Library> {
    use libloading::os::windows::{
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32, Library as WindowsLibrary,
    };
    anyhow::ensure!(
        path.is_absolute(),
        "Apple runtime DLL path must be absolute"
    );
    // Never search the working directory, PATH or an application-owned DLL cache.
    let library = unsafe {
        WindowsLibrary::load_with_flags(
            path,
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    }?;
    Ok(library.into())
}

#[cfg(test)]
mod support_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn predefined_registry_handle_sign_extends_on_64_bit_windows() {
        assert_eq!(HKEY_CURRENT_USER as usize, (-2147483647isize) as usize);
    }

    #[test]
    fn accepts_only_protected_apple_store_package_roots() {
        assert!(is_apple_store_root(Path::new(
            r"C:\Program Files\WindowsApps\AppleInc.AppleDevices_1.0.0.0_x64__nzyj5cx40ttqa"
        )));
        assert!(is_apple_store_root(Path::new(
            r"C:\Program Files\WindowsApps\AppleInc.iTunes_1.0.0.0_x64__nzyj5cx40ttqa"
        )));
        for path in [
            r"C:\Users\Public\AppleInc.AppleDevices_1.0.0.0_x64__nzyj5cx40ttqa",
            r"C:\Program Files\WindowsApps\AppleInc.AppleDevicesEvil_1.0.0.0_x64__nzyj5cx40ttqa",
            r"C:\Program Files\WindowsApps\AppleInc.AppleDevices_1.0.0.0_x64__otherpublisher",
            r"C:\Program Files\WindowsApps\AppleInc.AppleDevices_1.0.0.0_x64__nzyj5cx40ttqa\..\Untrusted",
        ] {
            assert!(!is_apple_store_root(Path::new(path)));
        }
    }
}
