use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

#[derive(Debug, Clone)]
pub struct SystemPrinter {
    pub system_queue_name: String,
    pub display_name: Option<String>,
    pub device_uri: Option<String>,
    pub is_accepting_jobs: Option<bool>,
    pub state: Option<i32>,
}

#[repr(C)]
struct cups_option_t {
    name: *const c_char,
    value: *const c_char,
}

#[repr(C)]
struct cups_dest_t {
    name: *const c_char,
    instance: *const c_char,
    is_default: c_int,
    num_options: c_int,
    options: *mut cups_option_t,
}

extern "C" {
    fn cupsGetDests(dests: *mut *mut cups_dest_t) -> c_int;
    fn cupsFreeDests(num_dests: c_int, dests: *mut cups_dest_t);
    fn cupsGetOption(
        name: *const c_char,
        num_options: c_int,
        options: *const cups_option_t,
    ) -> *const c_char;
}

fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

fn parse_bool(value: &str) -> Option<bool> {
    let lower = value.trim().to_lowercase();
    match lower.as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok()
}

unsafe fn get_option(
    name: &str,
    num_options: c_int,
    options: *mut cups_option_t,
) -> Option<String> {
    if options.is_null() || num_options <= 0 {
        return None;
    }
    let name_c = CString::new(name).ok()?;
    let value_ptr = cupsGetOption(name_c.as_ptr(), num_options, options as *const cups_option_t);
    cstr_to_string(value_ptr)
}

pub fn list_printers_via_cups() -> Result<Vec<SystemPrinter>, String> {
    let mut dests_ptr: *mut cups_dest_t = std::ptr::null_mut();
    let num_dests = unsafe { cupsGetDests(&mut dests_ptr) };
    if num_dests < 0 {
        return Err(format!("cupsGetDests failed count={}", num_dests));
    }
    if dests_ptr.is_null() || num_dests == 0 {
        return Ok(Vec::new());
    }

    let dests = unsafe { std::slice::from_raw_parts(dests_ptr, num_dests as usize) };
    let mut printers = Vec::new();

    for dest in dests {
        let name = cstr_to_string(dest.name).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let display_name = unsafe { get_option("printer-info", dest.num_options, dest.options) }
            .or_else(|| Some(name.clone()));
        let device_uri = unsafe { get_option("device-uri", dest.num_options, dest.options) };
        let is_accepting_jobs = unsafe {
            get_option("printer-is-accepting-jobs", dest.num_options, dest.options)
                .and_then(|v| parse_bool(&v))
        };
        let state = unsafe {
            get_option("printer-state", dest.num_options, dest.options).and_then(|v| parse_i32(&v))
        };

        printers.push(SystemPrinter {
            system_queue_name: name,
            display_name,
            device_uri,
            is_accepting_jobs,
            state,
        });
    }

    unsafe { cupsFreeDests(num_dests, dests_ptr) };
    Ok(printers)
}
