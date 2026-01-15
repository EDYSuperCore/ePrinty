use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SystemPrinter {
    pub system_queue_name: String,
    pub display_name: Option<String>,
    pub device_uri: Option<String>,
    pub is_accepting_jobs: Option<bool>,
    pub state: Option<i32>,
}

const IPP_VERSION: [u8; 2] = [0x01, 0x01];
const IPP_OP_CUPS_GET_PRINTERS: [u8; 2] = [0x40, 0x02];
const IPP_TAG_OPERATION_ATTRIBUTES: u8 = 0x01;
const IPP_TAG_END: u8 = 0x03;
const IPP_TAG_NAME_WITHOUT_LANGUAGE: u8 = 0x42;
const IPP_TAG_KEYWORD: u8 = 0x44;
const IPP_TAG_URI: u8 = 0x45;
const IPP_TAG_CHARSET: u8 = 0x47;
const IPP_TAG_NATURAL_LANGUAGE: u8 = 0x48;
const IPP_TAG_BOOLEAN: u8 = 0x22;
const IPP_TAG_INTEGER: u8 = 0x21;
const IPP_TAG_ENUM: u8 = 0x23;
const IPP_TAG_PRINTER_ATTRIBUTES: u8 = 0x04;

fn write_u16(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_be_bytes());
}

fn write_attr(buf: &mut Vec<u8>, tag: u8, name: &str, value: &str) {
    buf.push(tag);
    write_u16(buf, name.len() as u16);
    buf.extend_from_slice(name.as_bytes());
    write_u16(buf, value.len() as u16);
    buf.extend_from_slice(value.as_bytes());
}

fn write_attr_repeat(buf: &mut Vec<u8>, tag: u8, value: &str) {
    buf.push(tag);
    write_u16(buf, 0);
    write_u16(buf, value.len() as u16);
    buf.extend_from_slice(value.as_bytes());
}

fn build_ipp_request() -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&IPP_VERSION);
    buf.extend_from_slice(&IPP_OP_CUPS_GET_PRINTERS);
    write_u32(&mut buf, 1);
    buf.push(IPP_TAG_OPERATION_ATTRIBUTES);
    write_attr(&mut buf, IPP_TAG_CHARSET, "attributes-charset", "utf-8");
    write_attr(&mut buf, IPP_TAG_NATURAL_LANGUAGE, "attributes-natural-language", "en");
    write_attr(&mut buf, IPP_TAG_NAME_WITHOUT_LANGUAGE, "requesting-user-name", "eprinty");
    write_attr(&mut buf, IPP_TAG_KEYWORD, "requested-attributes", "printer-name");
    write_attr_repeat(&mut buf, IPP_TAG_KEYWORD, "printer-info");
    write_attr_repeat(&mut buf, IPP_TAG_KEYWORD, "device-uri");
    write_attr_repeat(&mut buf, IPP_TAG_KEYWORD, "printer-is-accepting-jobs");
    write_attr_repeat(&mut buf, IPP_TAG_KEYWORD, "printer-state");
    buf.push(IPP_TAG_END);
    buf
}

fn read_u16(bytes: &[u8], idx: &mut usize) -> Result<u16, String> {
    if *idx + 2 > bytes.len() {
        return Err("unexpected end of data".to_string());
    }
    let value = u16::from_be_bytes([bytes[*idx], bytes[*idx + 1]]);
    *idx += 2;
    Ok(value)
}

fn read_u32(bytes: &[u8], idx: &mut usize) -> Result<u32, String> {
    if *idx + 4 > bytes.len() {
        return Err("unexpected end of data".to_string());
    }
    let value = u32::from_be_bytes([
        bytes[*idx],
        bytes[*idx + 1],
        bytes[*idx + 2],
        bytes[*idx + 3],
    ]);
    *idx += 4;
    Ok(value)
}

fn parse_value(tag: u8, value: &[u8]) -> Option<IppValue> {
    match tag {
        IPP_TAG_INTEGER | IPP_TAG_ENUM => {
            if value.len() == 4 {
                let num = i32::from_be_bytes([value[0], value[1], value[2], value[3]]);
                Some(IppValue::Integer(num))
            } else {
                None
            }
        }
        IPP_TAG_BOOLEAN => value.first().map(|b| IppValue::Boolean(*b != 0)),
        _ => String::from_utf8(value.to_vec()).ok().map(IppValue::Text),
    }
}

enum IppValue {
    Text(String),
    Integer(i32),
    Boolean(bool),
}

fn parse_ipp_response(bytes: &[u8]) -> Result<Vec<SystemPrinter>, String> {
    if bytes.len() < 8 {
        return Err("IPP response too short".to_string());
    }
    let mut idx = 0;
    idx += 2; // version
    idx += 2; // status
    let _ = read_u32(bytes, &mut idx)?;

    let mut printers = Vec::new();
    let mut current: Option<SystemPrinter> = None;
    let mut last_name = String::new();
    let mut in_printer_group = false;

    while idx < bytes.len() {
        let tag = bytes[idx];
        idx += 1;
        if tag == IPP_TAG_END {
            break;
        }
        if tag == IPP_TAG_PRINTER_ATTRIBUTES {
            if let Some(printer) = current.take() {
                if !printer.system_queue_name.is_empty() {
                    printers.push(printer);
                }
            }
            current = Some(SystemPrinter {
                system_queue_name: String::new(),
                display_name: None,
                device_uri: None,
                is_accepting_jobs: None,
                state: None,
            });
            in_printer_group = true;
            last_name.clear();
            continue;
        }
        if tag <= 0x05 {
            in_printer_group = false;
            last_name.clear();
            continue;
        }

        let name_len = read_u16(bytes, &mut idx)? as usize;
        let name = if name_len > 0 {
            if idx + name_len > bytes.len() {
                return Err("invalid name length".to_string());
            }
            let name = String::from_utf8(bytes[idx..idx + name_len].to_vec())
                .map_err(|_| "invalid name encoding".to_string())?;
            idx += name_len;
            last_name = name.clone();
            name
        } else {
            last_name.clone()
        };

        let value_len = read_u16(bytes, &mut idx)? as usize;
        if idx + value_len > bytes.len() {
            return Err("invalid value length".to_string());
        }
        let value_bytes = &bytes[idx..idx + value_len];
        idx += value_len;

        if !in_printer_group {
            continue;
        }
        let printer = match current.as_mut() {
            Some(p) => p,
            None => continue,
        };

        let value = parse_value(tag, value_bytes);
        match (name.as_str(), value) {
            ("printer-name", Some(IppValue::Text(val))) => {
                if printer.system_queue_name.is_empty() {
                    printer.system_queue_name = val;
                }
            }
            ("printer-info", Some(IppValue::Text(val))) => {
                if printer.display_name.is_none() {
                    printer.display_name = Some(val);
                }
            }
            ("device-uri", Some(IppValue::Text(val))) => {
                if printer.device_uri.is_none() {
                    printer.device_uri = Some(val);
                }
            }
            ("printer-is-accepting-jobs", Some(IppValue::Boolean(val))) => {
                printer.is_accepting_jobs = Some(val);
            }
            ("printer-state", Some(IppValue::Integer(val))) => {
                printer.state = Some(val);
            }
            _ => {}
        }
    }

    if let Some(printer) = current.take() {
        if !printer.system_queue_name.is_empty() {
            printers.push(printer);
        }
    }

    Ok(printers)
}

pub fn list_printers_via_ipp() -> Result<Vec<SystemPrinter>, String> {
    let client = Client::builder()
        .timeout(Duration::from_millis(3000))
        .build()
        .map_err(|e| format!("ipp client build failed: {}", e))?;

    let body = build_ipp_request();
    let response = client
        .post("http://127.0.0.1:631/printers/")
        .header(CONTENT_TYPE, "application/ipp")
        .body(body)
        .send()
        .map_err(|e| format!("ipp request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("ipp http status {}", status));
    }

    let bytes = response.bytes().map_err(|e| format!("ipp read failed: {}", e))?;
    let printers = parse_ipp_response(&bytes)?;
    Ok(printers)
}
