// StepReporter：确保每个步骤都有明确的终态（success/failed/skipped）
// 使用 RAII 模式，Drop 时自动检查是否已发送终态事件

use serde_json;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// StepReporter：管理单个步骤的生命周期
pub struct StepReporter {
    app: Arc<AppHandle>,
    job_id: String,
    printer_name: String,
    step_id: String,
    terminated: bool, // 是否已发送终态事件
}

impl StepReporter {
    /// 开始一个新步骤（发送 running 事件）
    pub fn start(
        app: Arc<AppHandle>,
        job_id: String,
        printer_name: String,
        step_id: String,
        message: String,
    ) -> Self {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let event = crate::InstallProgressEvent {
            job_id: job_id.clone(),
            printer_name: printer_name.clone(),
            step_id: step_id.clone(),
            state: "running".to_string(),
            message: message.clone(),
            ts_ms,
            progress: None,
            error: None,
            meta: None,
            legacy_phase: None,
        };

        let _ = app.emit_all("install_progress", &event);
        eprintln!(
            "[StepReporter] step={} state=running message=\"{}\"",
            step_id, message
        );

        Self {
            app,
            job_id,
            printer_name,
            step_id,
            terminated: false,
        }
    }

    /// 更新进度（在 running 状态下）
    pub fn update_progress(
        &self,
        current: Option<u64>,
        total: Option<u64>,
        unit: Option<String>,
        percent: Option<f64>,
        message: Option<String>,
    ) {
        if self.terminated {
            return;
        }

        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let progress = Some(crate::ProgressPayload {
            current,
            total,
            unit,
            percent,
        });

        let event = crate::InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: self.step_id.clone(),
            state: "running".to_string(),
            message: message.unwrap_or_else(|| "进行中...".to_string()),
            ts_ms,
            progress,
            error: None,
            meta: None,
            legacy_phase: None,
        };

        let _ = self.app.emit_all("install_progress", &event);
    }

    /// 标记为成功
    pub fn success(mut self, message: String, meta: Option<serde_json::Value>) -> Result<(), String> {
        self.terminated = true;
        self.emit_terminal_state("success", message, None, meta)
    }

    /// 标记为跳过
    pub fn skipped(mut self, message: String, meta: Option<serde_json::Value>) -> Result<(), String> {
        self.terminated = true;
        self.emit_terminal_state("skipped", message, None, meta)
    }

    /// 标记为失败
    pub fn failed(
        mut self,
        code: String,
        detail: String,
        stdout: Option<String>,
        stderr: Option<String>,
        meta: Option<serde_json::Value>,
    ) -> Result<(), String> {
        self.terminated = true;
        let error = Some(crate::ErrorPayload {
            code,
            detail: detail.clone(),
            stderr,
            stdout,
        });
        self.emit_terminal_state("failed", detail, error, meta)
    }

    /// 发送终态事件
    fn emit_terminal_state(
        &self,
        state: &str,
        message: String,
        error: Option<crate::ErrorPayload>,
        meta: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let event = crate::InstallProgressEvent {
            job_id: self.job_id.clone(),
            printer_name: self.printer_name.clone(),
            step_id: self.step_id.clone(),
            state: state.to_string(),
            message,
            ts_ms,
            progress: None,
            error,
            meta,
            legacy_phase: None,
        };

        match self.app.emit_all("install_progress", &event) {
            Ok(_) => {
                eprintln!(
                    "[StepReporter] step={} state={} message=\"{}\"",
                    self.step_id, state, event.message
                );
                Ok(())
            }
            Err(e) => {
                eprintln!(
                    "[StepReporter] step={} state={} emit failed: {}",
                    self.step_id, state, e
                );
                Err(format!("emit failed: {}", e))
            }
        }
    }
}

impl Drop for StepReporter {
    /// Drop 时检查是否已发送终态事件，如果没有则自动发送 failed
    fn drop(&mut self) {
        if !self.terminated {
            eprintln!(
                "[StepReporter] WARNING: step={} was dropped without terminal state, auto-emitting failed",
                self.step_id
            );
            let _ = self.emit_terminal_state(
                "failed",
                format!("步骤未正常结束（code=STEP_DANGLING）"),
                Some(crate::ErrorPayload {
                    code: "STEP_DANGLING".to_string(),
                    detail: format!("步骤 {} 未正常结束，可能函数提前返回", self.step_id),
                    stderr: None,
                    stdout: None,
                }),
                None,
            );
        }
    }
}
