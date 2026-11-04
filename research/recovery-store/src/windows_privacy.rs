//! User/SYSTEM-only DACLs for newly created local captures. No SID is logged.
use anyhow::{Result, ensure};
use std::{ffi::c_void, io, mem, os::windows::ffi::OsStrExt, path::Path, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL_SIZE_INFORMATION, AclSizeInformation,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetNamedSecurityInfoW, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, GetAce, GetAclInformation, GetSecurityDescriptorControl,
        GetTokenInformation, OWNER_SECURITY_INFORMATION, SECURITY_ATTRIBUTES, TOKEN_QUERY,
        TOKEN_USER, TokenIsAppContainer, TokenUser,
    },
    Storage::FileSystem::CreateDirectoryW,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

struct Local(*mut c_void);
fn require_trusted_owner(owner: &str, user: &str) -> Result<()> {
    // An owner can rewrite the DACL even without an explicit access rule.
    // Administrative compromise and other processes of the same user are out of scope.
    ensure!(
        owner == user || owner == "S-1-5-18" || owner == "S-1-5-32-544",
        "Unexpected capture owner"
    );
    Ok(())
}
fn require_sid_extent(ace_size: usize, revision: u8, subauthorities: u8) -> Result<()> {
    ensure!(
        revision == 1 && subauthorities <= 15,
        "Invalid access-rule SID"
    );
    // ACCESS_ALLOWED_ACE prefix = 8 bytes; SID header = 8 bytes.
    ensure!(
        ace_size >= 16 + 4 * usize::from(subauthorities),
        "Truncated access-rule SID"
    );
    Ok(())
}
impl Drop for Local {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
struct Token(HANDLE);
impl Drop for Token {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn wide(path: &Path) -> Result<Vec<u16>> {
    let mut name: Vec<u16> = path.as_os_str().encode_wide().collect();
    ensure!(!name.contains(&0), "NUL in local capture path");
    name.push(0);
    Ok(name)
}
unsafe fn local_text(memory: &Local) -> Result<String> {
    ensure!(!memory.0.is_null(), "Missing Windows SID text");
    let text = memory.0.cast::<u16>();
    let mut length = 0;
    while length < 256 && unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    ensure!(length < 256, "Windows SID text exceeds limit");
    Ok(String::from_utf16(unsafe {
        std::slice::from_raw_parts(text, length)
    })?)
}
unsafe fn sid_text(sid: *mut c_void) -> Result<String> {
    let mut raw = ptr::null_mut();
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut raw) };
    let memory = Local(raw.cast());
    ensure!(ok != 0, "Cannot encode Windows account SID");
    unsafe { local_text(&memory) }
}
fn container_token(token: HANDLE) -> Result<bool> {
    let mut flag = 0u32;
    let mut length = 0;
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenIsAppContainer,
            ptr::addr_of_mut!(flag).cast(),
            mem::size_of::<u32>() as u32,
            &mut length,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error().into());
    }
    ensure!(
        length == mem::size_of::<u32>() as u32,
        "Invalid process context metadata"
    );
    Ok(flag != 0)
}
fn current_user() -> Result<String> {
    unsafe {
        let mut raw = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw) == 0 {
            return Err(io::Error::last_os_error().into());
        }
        let token = Token(raw);
        ensure!(
            !container_token(token.0)?,
            "Private capture requires a normal interactive Windows process; AppContainer is unsupported"
        );
        let mut needed = 0;
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut needed);
        ensure!(
            (mem::size_of::<TOKEN_USER>() as u32..=16384).contains(&needed),
            "Invalid Windows token buffer size"
        );
        // TOKEN_USER contains pointers, so use pointer-aligned storage.
        let mut buffer = vec![0usize; (needed as usize).div_ceil(mem::size_of::<usize>())];
        if GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        ) == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        ensure!(
            needed as usize <= buffer.len() * mem::size_of::<usize>(),
            "Changed Windows token size"
        );
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let sid = sid_text(user.User.Sid)?;
        ensure!(sid != "S-1-5-18", "Interactive user context required");
        Ok(sid)
    }
}
fn create_with_descriptor(path: &Path, sddl: &str) -> Result<()> {
    let text: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let name = wide(path)?;
    unsafe {
        let mut raw = ptr::null_mut();
        let ok = ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            1,
            &mut raw,
            ptr::null_mut(),
        );
        let descriptor = Local(raw);
        ensure!(ok != 0, "Cannot create private Windows security descriptor");
        let attributes = SECURITY_ATTRIBUTES {
            nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        if CreateDirectoryW(name.as_ptr(), &attributes) == 0 {
            return Err(io::Error::last_os_error().into());
        }
    }
    Ok(())
}
pub(super) fn create(path: &Path) -> Result<()> {
    let user = current_user()?;
    create_with_descriptor(path, &format!("D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)"))?;
    verify(path, true)
}
/// Requires precisely the current user and SYSTEM. Directories are protected
/// against ACL inheritance; files must inherit exactly those two full-access ACEs.
pub(super) fn verify(path: &Path, directory: bool) -> Result<()> {
    let user = current_user()?;
    let name = wide(path)?;
    unsafe {
        let mut acl = ptr::null_mut();
        let mut owner = ptr::null_mut();
        let mut raw = ptr::null_mut();
        let status = GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut raw,
        );
        let descriptor = Local(raw);
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32).into());
        }
        ensure!(
            !acl.is_null() && !descriptor.0.is_null() && !owner.is_null(),
            "Missing capture DACL"
        );
        require_trusted_owner(&sid_text(owner)?, &user)?;
        let mut control = 0;
        let mut revision = 0;
        ensure!(
            GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) != 0,
            "Cannot inspect capture security descriptor"
        );
        ensure!(
            control & 0x4 != 0 && (!directory || control & 0x1000 != 0),
            "Unprotected capture DACL"
        );
        let mut info: ACL_SIZE_INFORMATION = mem::zeroed();
        ensure!(
            GetAclInformation(
                acl,
                ptr::addr_of_mut!(info).cast(),
                mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation
            ) != 0,
            "Cannot inspect capture ACL"
        );
        ensure!(
            info.AceCount == 2,
            "Capture permissions include unexpected principals"
        );
        let mut owner_seen = false;
        let mut system_seen = false;
        for index in 0..info.AceCount {
            let mut entry = ptr::null_mut();
            ensure!(
                GetAce(acl, index, &mut entry) != 0 && !entry.is_null(),
                "Cannot inspect capture access rule"
            );
            let header = &*entry.cast::<ACE_HEADER>();
            ensure!(
                header.AceType == 0
                    && header.AceSize as usize >= mem::size_of::<ACCESS_ALLOWED_ACE>(),
                "Unexpected capture access rule type"
            );
            ensure!(
                header.AceFlags == if directory { 3 } else { 0x10 },
                "Unexpected capture inheritance flags"
            );
            let allowed = &*entry.cast::<ACCESS_ALLOWED_ACE>();
            ensure!(allowed.Mask == 0x001f01ff, "Unexpected capture access mask");
            let sid_start = ptr::addr_of!(allowed.SidStart).cast::<u8>();
            require_sid_extent(header.AceSize as usize, *sid_start, *sid_start.add(1))?;
            let sid = sid_text(sid_start.cast_mut().cast())?;
            if sid == user && !owner_seen {
                owner_seen = true;
            } else if sid == "S-1-5-18" && !system_seen {
                system_seen = true;
            } else {
                anyhow::bail!("Capture permissions include unexpected principals");
            }
        }
        ensure!(
            owner_seen && system_seen,
            "Capture permission policy mismatch"
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn another_standard_account_cannot_own_private_storage() {
        assert!(require_trusted_owner("S-1-5-21-1-2-3-1002", "S-1-5-21-1-2-3-1001").is_err());
        assert!(require_trusted_owner("S-1-5-21-1-2-3-1001", "S-1-5-21-1-2-3-1001").is_ok());
        assert!(require_trusted_owner("S-1-5-18", "S-1-5-21-1-2-3-1001").is_ok());
        assert!(require_trusted_owner("S-1-5-32-544", "S-1-5-21-1-2-3-1001").is_ok());
    }
    #[test]
    fn truncated_or_invalid_sid_is_rejected_before_native_conversion() {
        assert!(require_sid_extent(12, 1, 1).is_err());
        assert!(require_sid_extent(20, 2, 1).is_err());
        assert!(require_sid_extent(80, 1, 16).is_err());
        assert!(require_sid_extent(20, 1, 1).is_ok());
    }
    #[test]
    fn permissive_directory_cannot_be_accepted_as_private_capture_storage() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("public-fixture");
        create_with_descriptor(&root, "D:P(A;OICI;FA;;;WD)").unwrap();
        assert!(
            verify(&root, true).is_err(),
            "Everyone access must not be accepted"
        );
    }
    #[test]
    fn private_capture_policy_matches_the_current_process_context() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("private-fixture");
        let mut raw = ptr::null_mut();
        assert_ne!(
            unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw) },
            0
        );
        let token = Token(raw);
        let restricted = container_token(token.0).unwrap();
        if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
            assert!(
                !restricted,
                "CI must exercise actual supported-context file creation"
            );
        }
        if restricted {
            assert!(create(&root).is_err());
            assert!(
                !root.exists(),
                "Unsupported context must be rejected before creation"
            );
            return;
        }
        create(&root).unwrap();
        let file = root.join("fixture.bin");
        std::fs::write(&file, b"synthetic").unwrap();
        verify(&file, false).unwrap();
    }
}
