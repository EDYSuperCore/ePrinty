use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

use crate::{ErrorPayload, InstallProgressEvent, ProgressPayload};

const ALLOWED_STEP_IDS: &[&str] = &[
    "job.init",
    "job.done",
    "job.failed",
    "device.probe",
    "driver.download",
    "driver.verify",
    "driver.extract",
    "driver.stageDriver",
    "driver.registerDriver",
    "device.ensurePort",
    "device.ensureQueue",
    "device.finalVerify",
];

const ALLOWED_STATES: &[&str] = &["pending", "running", "success", "failed", "skipped"];

static INSTALL_MODE_REGISTRY: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn now_ts_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn registry() -> &'static Mutex<HashMap<String, String>> {
    INSTALL_MODE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_install_mode(job_id: &str, install_mode: &str) {
    if job_id.trim().is_empty() || install_mode.trim().is_empty() {
        return;
    }
    if let Ok(mut map) = registry().lock() {
        map.insert(job_id.to_string(), install_mode.to_string());
    }
}

fn resolve_install_mode(job_id: &str, explicit: Option<String>) -> String {
    if let Some(mode) = explicit {
        if !mode.trim().is_empty() {
            return mode;
        }
    }
    if let Ok(map) = registry().lock() {
        if let Some(mode) = map.get(job_id) {
            return mode.clone();
        }
    }
    "auto".to_string()
}

fn validate_step_id(step_id: &str) -> Result<(), String> {
    if ALLOWED_STEP_IDS.contains(&step_id) {
        Ok(())
    } else {
        Err(format!("invalid step_id \"{}\"", step_id))
    }
}

fn validate_state(state: &str) -> Result<(), String> {
    if ALLOWED_STATES.contains(&state) {
        Ok(())
    } else {
        Err(format!("invalid state \"{}\"", state))
    }
}

pub fn emit_install_progress(app: &AppHandle, mut event: InstallProgressEvent) -> Result<(), String> {
    if event.ts_ms <= 0 {
        event.ts_ms = now_ts_ms();
    }
    validate_step_id(&event.step_id)?;
    validate_state(&event.state)?;

    let resolved_install_mode = resolve_install_mode(&event.job_id, event.install_mode.take());
    event.install_mode = Some(resolved_install_mode.clone());

    if event.step_id == "job.init" {
        register_install_mode(&event.job_id, &resolved_install_mode);
    }

    app.emit_all("install_progress", &event)
        .map_err(|e| format!("emit install_progress failed: {}", e))
}

#[derive(Debug, Clone)]
pub struct StepReporter {
    app: AppHandle,
    job_id: String,
    printer_name: String,
    install_mode: String,
    default_meta: Option<Value>,
}

impl StepReporter {
    pub fn new(
        app: AppHandle,
        job_id: String,
        printer_name: String,
        install_mode: Option<String>,
        default_meta: Option<Value>,
    ) -> Self {
        let resolved_install_mode = resolve_install_mode(&job_id, install_mode);
        Self {
            app,
            job_id,
            printer_name,
            install_mode: resolved_install_mode,
            default_meta,
        }
    }

    pub fn set_default_meta(&mut self, meta: Option<Value>) {
        self.default_meta = meta;
    }

    pub fn emit_job_init(&self, message: &str) -> Result<(), String> {
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: "job.init".to_string(),
            state: "running".to_string(),
            message: message.to_string(),
            ts_ms: now_ts_ms(),
            progress: None,
            error: None,
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }

    pub fn emit_step_running(
        &self,
        step_id: &str,
        message: &str,
        progress: Option<ProgressPayload>,
    ) -> Result<(), String> {
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: step_id.to_string(),
            state: "running".to_string(),
            message: message.to_string(),
            ts_ms: now_ts_ms(),
            progress,
            error: None,
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }

    pub fn emit_step_success(&self, step_id: &str, message: Option<&str>) -> Result<(), String> {
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: step_id.to_string(),
            state: "success".to_string(),
            message: message.unwrap_or("完成").to_string(),
            ts_ms: now_ts_ms(),
            progress: None,
            error: None,
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }

    pub fn emit_step_skipped(&self, step_id: &str, message: Option<&str>) -> Result<(), String> {
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: step_id.to_string(),
            state: "skipped".to_string(),
            message: message.unwrap_or("已跳过").to_string(),
            ts_ms: now_ts_ms(),
            progress: None,
            error: None,
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }

    pub fn emit_step_fail(
        &self,
        step_id: &str,
        error_code: &str,
        message: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        let error_detail = detail.unwrap_or(message);
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: step_id.to_string(),
            state: "failed".to_string(),
            message: message.to_string(),
            ts_ms: now_ts_ms(),
            progress: None,
            error: Some(ErrorPayload {
                code: error_code.to_string(),
                detail: error_detail.to_string(),
                stdout: None,
                stderr: None,
            }),
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }

    pub fn emit_job_done(&self, success: bool, message: Option<&str>) -> Result<(), String> {
        let (state, default_message) = if success {
            ("success", "安装完成")
        } else {
            ("failed", "安装失败")
        };
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: "job.done".to_string(),
            state: state.to_string(),
            message: message.unwrap_or(default_message).to_string(),
            ts_ms: now_ts_ms(),
            progress: None,
            error: None,
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }

    pub fn emit_job_failed(
        &self,
        error_code: &str,
        message: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        let error_detail = detail.unwrap_or(message);
        let event = InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: "job.failed".to_string(),
            state: "failed".to_string(),
            message: message.to_string(),
            ts_ms: now_ts_ms(),
            progress: None,
            error: Some(ErrorPayload {
                code: error_code.to_string(),
                detail: error_detail.to_string(),
                stdout: None,
                stderr: None,
            }),
            meta: self.default_meta.clone(),
            legacy_phase: None,
            install_mode: Some(self.install_mode.clone()),
        };
        emit_install_progress(&self.app, event)
    }
}
