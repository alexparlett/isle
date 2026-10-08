//! The few calls of libpam the helper makes, with a conversation that answers its password
//! prompt once and passes on everything PAM says.

use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr,
};

use zeroize::Zeroizing;

const PAM_SUCCESS: c_int = 0;
const PAM_PERM_DENIED: c_int = 6;
const PAM_AUTH_ERR: c_int = 7;
const PAM_CRED_INSUFFICIENT: c_int = 8;
const PAM_USER_UNKNOWN: c_int = 10;
const PAM_MAXTRIES: c_int = 11;
const PAM_NEW_AUTHTOK_REQD: c_int = 12;
const PAM_ACCT_EXPIRED: c_int = 13;
const PAM_CONV_ERR: c_int = 19;
const PAM_BUF_ERR: c_int = 5;
const PAM_REFRESH_CRED: c_int = 0x0010;

const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_PROMPT_ECHO_ON: c_int = 2;
const PAM_ERROR_MSG: c_int = 3;
const PAM_TEXT_INFO: c_int = 4;

#[repr(C)]
struct PamMessage {
    msg_style: c_int,
    msg: *const c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut c_char,
    resp_retcode: c_int,
}

#[repr(C)]
struct PamConv {
    conv: Option<
        unsafe extern "C" fn(
            c_int,
            *mut *const PamMessage,
            *mut *mut PamResponse,
            *mut c_void,
        ) -> c_int,
    >,
    appdata_ptr: *mut c_void,
}

#[link(name = "pam")]
unsafe extern "C" {
    fn pam_start(
        service: *const c_char,
        user: *const c_char,
        conv: *const PamConv,
        handle: *mut *mut c_void,
    ) -> c_int;
    fn pam_start_confdir(
        service: *const c_char,
        user: *const c_char,
        conv: *const PamConv,
        confdir: *const c_char,
        handle: *mut *mut c_void,
    ) -> c_int;
    fn pam_authenticate(handle: *mut c_void, flags: c_int) -> c_int;
    fn pam_acct_mgmt(handle: *mut c_void, flags: c_int) -> c_int;
    fn pam_setcred(handle: *mut c_void, flags: c_int) -> c_int;
    fn pam_end(handle: *mut c_void, status: c_int) -> c_int;
    fn pam_strerror(handle: *mut c_void, error: c_int) -> *const c_char;
}

/// What PAM says that is not a prompt.
pub enum Message {
    Info,
    Error,
}

pub enum Failure {
    /// A wrong password, a locked-out account, or one that may not log in.
    Refused(String),
    /// PAM could not check at all.
    Broken(String),
}

/// One attempt's side of the conversation: the password for its first secret prompt, the
/// user's name for a visible one, and what PAM says passed on.
pub struct Conversation<F: FnMut(Message, &str)> {
    user: CString,
    password: Option<Zeroizing<String>>,
    said: F,
}

impl<F: FnMut(Message, &str)> Conversation<F> {
    pub fn new(user: CString, password: Zeroizing<String>, said: F) -> Self {
        Self {
            user,
            password: Some(password),
            said,
        }
    }

    /// The answer to one message, or none for a message that needs none; `Err` for a
    /// second password prompt, which this conversation cannot answer.
    fn answer(&mut self, style: c_int, text: &str) -> Result<Option<CString>, ()> {
        match style {
            PAM_PROMPT_ECHO_OFF => {
                let password = self.password.take().ok_or(())?;
                CString::new(password.as_bytes()).map(Some).map_err(|_| ())
            }
            PAM_PROMPT_ECHO_ON => Ok(Some(self.user.clone())),
            PAM_ERROR_MSG => {
                (self.said)(Message::Error, text);
                Ok(None)
            }
            PAM_TEXT_INFO => {
                (self.said)(Message::Info, text);
                Ok(None)
            }
            _ => Err(()),
        }
    }
}

unsafe extern "C" fn converse<F: FnMut(Message, &str)>(
    count: c_int,
    messages: *mut *const PamMessage,
    responses: *mut *mut PamResponse,
    data: *mut c_void,
) -> c_int {
    let run = std::panic::AssertUnwindSafe(|| unsafe {
        if count <= 0 || messages.is_null() || responses.is_null() || data.is_null() {
            return PAM_CONV_ERR;
        }
        let conversation = &mut *(data as *mut Conversation<F>);
        let out =
            libc::calloc(count as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse;
        if out.is_null() {
            return PAM_BUF_ERR;
        }
        for i in 0..count as usize {
            let message = &**messages.add(i);
            let text = if message.msg.is_null() {
                String::new()
            } else {
                CStr::from_ptr(message.msg).to_string_lossy().into_owned()
            };
            match conversation.answer(message.msg_style, &text) {
                Ok(Some(answer)) => (*out.add(i)).resp = libc::strdup(answer.as_ptr()),
                Ok(None) => {}
                Err(()) => {
                    for j in 0..i {
                        let resp = (*out.add(j)).resp;
                        if !resp.is_null() {
                            libc::explicit_bzero(resp as *mut c_void, libc::strlen(resp));
                            libc::free(resp as *mut c_void);
                        }
                    }
                    libc::free(out as *mut c_void);
                    return PAM_CONV_ERR;
                }
            }
        }
        *responses = out;
        PAM_SUCCESS
    });
    std::panic::catch_unwind(run).unwrap_or(PAM_CONV_ERR)
}

fn error_text(handle: *mut c_void, code: c_int) -> String {
    unsafe {
        let text = pam_strerror(handle, code);
        if text.is_null() {
            format!("PAM error {code}")
        } else {
            CStr::from_ptr(text).to_string_lossy().into_owned()
        }
    }
}

/// Authenticates `user` through `service` and checks the account may log in.
pub fn check<F: FnMut(Message, &str)>(
    service: &CStr,
    user: &CStr,
    confdir: Option<&CStr>,
    conversation: &mut Conversation<F>,
) -> Result<(), Failure> {
    let conv = PamConv {
        conv: Some(converse::<F>),
        appdata_ptr: conversation as *mut Conversation<F> as *mut c_void,
    };
    let mut handle: *mut c_void = ptr::null_mut();
    let started = unsafe {
        match confdir {
            Some(dir) => pam_start_confdir(
                service.as_ptr(),
                user.as_ptr(),
                &conv,
                dir.as_ptr(),
                &mut handle,
            ),
            None => pam_start(service.as_ptr(), user.as_ptr(), &conv, &mut handle),
        }
    };
    if started != PAM_SUCCESS || handle.is_null() {
        return Err(Failure::Broken(format!(
            "PAM did not start: error {started}"
        )));
    }
    let mut code = unsafe { pam_authenticate(handle, 0) };
    if code == PAM_SUCCESS {
        code = unsafe { pam_acct_mgmt(handle, 0) };
    }
    let result = match code {
        PAM_SUCCESS => {
            unsafe { pam_setcred(handle, PAM_REFRESH_CRED) };
            Ok(())
        }
        PAM_AUTH_ERR
        | PAM_PERM_DENIED
        | PAM_CRED_INSUFFICIENT
        | PAM_USER_UNKNOWN
        | PAM_MAXTRIES
        | PAM_ACCT_EXPIRED
        | PAM_NEW_AUTHTOK_REQD => Err(Failure::Refused(error_text(handle, code))),
        other => Err(Failure::Broken(error_text(handle, other))),
    };
    unsafe { pam_end(handle, code) };
    result
}
