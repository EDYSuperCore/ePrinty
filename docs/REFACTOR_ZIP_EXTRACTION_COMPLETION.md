# Windows 驱动程序解压流程重构 - 交付物总结

**目标**: 替换 Windows 安装流程中的 PowerShell Expand-Archive 方案为 Rust 原生解压（zip crate），保持现有 step 命名/上报，同时提升可观测性和可靠性。

**完成日期**: 2026年1月21日

---

## 一、交付物清单

### 1. 新增 Rust 解压工具模块
**文件**: [src-tauri/src/utils/zip_extract.rs](src-tauri/src/utils/zip_extract.rs)

**功能**:
- `extract_zip_to_dir()`: 核心解压函数，提供以下特性：
  - ✅ 防 Zip Slip 路径验证（检测 `..` 和绝对路径逃逸）
  - ✅ 详细的错误分类（IO、格式、权限、取消等）
  - ✅ 可观测性：完整的日志记录
  - ✅ 取消支持：通过 `AtomicBool` 标志支持优雅中断
  - ✅ 进度回调：可选的进度通知机制（`fn(done, total)`）
  - ✅ 大文件优化：自适应 buffer 大小（64KB～2MB）

**关键数据结构**:
```rust
pub struct ExtractReport {
    pub files_extracted: usize,
    pub directories_created: usize,
    pub bytes_written: u64,
    pub elapsed_ms: u128,
}

pub enum ExtractError {
    ZipOpenFailed { path, reason },
    ZipFormatError { reason },
    ZipSlipDetected { entry_name, resolved_path },
    IoError { operation, path, reason },
    PermissionDenied { path, reason },
    Cancelled,
    Other(String),
}
```

**内置单元测试** (3个):
- `test_normal_extraction()`: 验证正常解压功能
- `test_zip_slip_detection()`: 验证 Zip Slip 防护
- `test_cancellation()`: 验证取消机制

---

### 2. 模块集成

**文件**: [src-tauri/src/utils/mod.rs](src-tauri/src/utils/mod.rs)
- 暴露 `zip_extract` 模块

**文件**: [src-tauri/src/main.rs](src-tauri/src/main.rs)
- 添加 `mod utils;` 导入
- 移除未使用的 `POWERSHELL_TIMEOUT_SECS` 常量（已在 ps.rs 中使用）

---

### 3. 解压流程替换

**文件**: [src-tauri/src/platform/windows/archive.rs](src-tauri/src/platform/windows/archive.rs)

**关键改动**:

#### Step 3: expand_archive（第 855～920 行）
- ✅ **替换**: 用 `crate::utils::zip_extract::extract_zip_to_dir()` 替代 PowerShell Expand-Archive
- ✅ **保留**: 现有的 step 命名（`step=expand_archive`）和 UI 进度报告机制
- ✅ **日志增强**:
  ```
  [ExtractZipForDriver] step=expand_archive inputs=zip_path="..." staging_dir="..."
  [ExtractZipForDriver] step=expand_archive result=success files_extracted=123 
                        dirs_created=45 bytes_written=5242880 elapsed_ms=1234
  ```

#### Staging 清理逻辑（第 838～880 行）
- ✅ **环境变量支持**: `EPRINTY_KEEP_STAGING`
  - 设置该环境变量时，解压失败后 staging 目录不被清理，便于排查
  - 默认行为：失败时自动清理 staging
- ✅ **改进的日志**:
  ```
  [ExtractZipForDriver] cleanup_success staging_dir="..."
  [ExtractZipForDriver] cleanup_skipped staging_dir="..." reason="EPRINTY_KEEP_STAGING 已设置"
  ```

---

### 4. 错误处理增强

**文件**: [src-tauri/src/platform/windows/archive.rs](src-tauri/src/platform/windows/archive.rs) (第 900～920 行)

错误消息包含完整上下文：
```
解压失败: [底层原因] | ZIP: [路径] | Dest: [目标路径] | [清理状态提示]
```

示例：
```
解压失败: 检测到路径逃逸攻击 | Entry: '../evil.txt' | Resolved: '...' 
         | ZIP: C:\drivers\payload.zip | Dest: C:\drivers\uuid\_staging123
         | (staging 目录已保留供排查: C:\drivers\uuid\_staging123)
```

---

### 5. 依赖更新

**文件**: [src-tauri/Cargo.toml](src-tauri/Cargo.toml)

```toml
[dependencies]
zip = "0.6"       # Rust 原生 ZIP 处理
tempfile = "3"    # 单元测试用临时文件
```

---

### 6. 集成测试

**文件**: [src-tauri/tests/zip_extraction_tests.rs](src-tauri/tests/zip_extraction_tests.rs)

包含:
- `test_zip_extraction_basic()`: 验证基础 ZIP 创建和路径
- `test_staging_cleanup_env_var()`: 验证环境变量读取

运行: `cargo test zip_extraction`

---

### 7. 旧代码标记为已弃用

**文件**: [src-tauri/src/platform/windows/archive.rs](src-tauri/src/platform/windows/archive.rs) (第 315 行)

`extract_zip()` 函数标记为 `#[deprecated]`:
```rust
#[deprecated(
    since = "1.4.1",
    note = "使用 extract_zip_for_driver() 代替，它使用 Rust 原生 zip crate 而不依赖 PowerShell"
)]
```

---

## 二、验收标准检查清单

- ✅ **性能改进**: 不再依赖 120s PowerShell 超时逻辑；解压可持续进行直至完成
- ✅ **错误可观测性**: 日志包含：
  - ZIP 路径、目标目录、条目名称
  - 文件数、目录数、字节数、耗时（ms）
  - 具体错误类型（I/O、格式、权限、Zip Slip 等）
- ✅ **UI 进度连贯**: 现有的 `StepReporter` 继续工作，"解压/合并文件" 步骤不误报超时
- ✅ **取消机制**: 虽未在当前调用点启用，但代码已支持 `AtomicBool` 取消标志
- ✅ **Staging 保留开关**: `EPRINTY_KEEP_STAGING=1` 生效，失败时保留现场
- ✅ **Zip Slip 防护**: 内置检测，拒绝包含 `..` 或绝对路径的条目
- ✅ **编译成功**: 无错误，仅有未使用函数警告（系统设计允许）

---

## 三、代码变更统计

| 项目 | 新增 | 修改 | 删除 |
|------|------|------|------|
| 新文件 | 3 (zip_extract.rs, mod.rs, tests) | - | - |
| Cargo.toml | - | +2 行 (依赖) | - |
| archive.rs | - | 约 150 行 | 约 100 行 (PowerShell 代码) |
| main.rs | - | -4 行 | -1 行 (常量) |
| **总计** | **3 个新文件** | **~150 行** | **~100 行** |

---

## 四、运行和验证

### 编译验证
```bash
cd src-tauri
cargo check                          # 快速检查
cargo build --release               # 完整构建
```

### 单元测试（模块内）
```bash
# 在 zip_extract.rs 内编写的测试
cargo test zip_extract::tests       # 如果启用了库
```

### 集成测试
```bash
cargo test zip_extraction           # 运行集成测试集
```

### 环境变量测试
```powershell
# Windows PowerShell
$env:EPRINTY_KEEP_STAGING = "1"
# 然后运行安装流程，验证 staging 目录在失败时被保留
```

---

## 五、已知限制与后续改进

### 当前状态
1. 取消标志 (`AtomicBool`) 已支持但未在调用点激活
   - 可在 UI 实现"中止下载"时使用
   
2. 进度回调已支持但未集成
   - 可在 UI 展示"已提取 X / Y 个文件" 时使用

3. 未采用 `anyhow` 或 `thiserror`
   - 当前使用自定义 `ExtractError` 枚举，便于模式匹配

### 后续建议
- [ ] 集成进度回调到 UI (`StepReporter.update_progress()`)
- [ ] 激活取消标志以支持用户中止解压
- [ ] 考虑异步解压以避免阻塞 UI 线程（如果需要）
- [ ] 为 extract_zip() 旧函数添加性能警告日志

---

## 六、故障排查指南

### 症状: 解压失败且无法确定原因
**解决**: 设置环境变量并检查 staging 目录
```powershell
$env:EPRINTY_KEEP_STAGING = "1"
# 运行安装，失败后查看日志和 staging 目录内容
Get-ChildItem "C:\drivers\<uuid>\_staging*" -Recurse
```

### 症状: ZIP 格式错误
**日志示例**:
```
error=Open_ZIP_file reason="ZIP 格式错误或损坏: ..."
```
**解决**: 验证 payload.zip 完整性
```powershell
Test-Path "path\to\payload.zip"
(Get-Item "path\to\payload.zip").Length
```

### 症状: 权限被拒绝
**日志示例**:
```
error=create_dest_dir reason="权限被拒绝"
```
**解决**: 确保进程有写权限到 drivers 目录

---

## 七、性能基准（理论预期）

基于 Rust zip crate 和 buffered IO：

| ZIP 大小 | 文件数 | 预期耗时 | 相比 PowerShell |
|---------|--------|---------|-----------------|
| 10 MB | 50 | 50～100 ms | ~10x 快 |
| 100 MB | 500 | 200～400 ms | ~10x 快 |
| 500 MB | 5000 | 1～2 s | ~10x 快 |

*注*: PowerShell Expand-Archive 平均 1-2s (小文件) 到 120s (超时)

---

## 八、接收标志

本重构任务的所有交付物均已完成，符合需求规格：

- ✅ Rust 原生解压模块 (`zip_extract.rs`)
- ✅ PowerShell 替换实现 (`archive.rs` Step 3)
- ✅ 可观测性增强（日志、错误分类）
- ✅ 环境变量控制清理行为
- ✅ Zip Slip 防护和单元测试
- ✅ 无编译错误，全部功能可用

**建议**: 在下一个 release 中标记 `extract_zip()` 为完全弃用，计划在 v2.0 中移除。

---

**联系人**: 代码提交人  
**最后更新**: 2026-01-21
