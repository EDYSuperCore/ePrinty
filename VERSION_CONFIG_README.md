# 版本配置文件说明

## 配置文件位置

将 `version_config.json` 文件放置在远程服务器，默认访问地址为：
```
https://p.edianyun.icu/version_config.json
```

## 配置文件结构

参考 `version_config.json.example` 文件，包含以下字段：

### 必需字段

- `app_name`: 应用名称
- `app_version`: 应用版本号（格式：x.y.z，如 "1.1.0"）
- `build_number`: 构建号（数字，用于区分同一版本的不同构建）
- `release_date`: 发布日期（格式：YYYY-MM-DD）
- `force_update`: 是否强制更新（布尔值）

### 可选字段

- `update_url`: 更新文件下载地址（可选，如果有则提供下载功能）
- `update_type`: 更新类型（"manual" 手动更新 或 "auto" 自动更新）
- `update_description`: 更新内容描述（多行文本）
- `changelog`: 更新日志列表（数组）
- `min_supported_version`: 最小支持版本（低于此版本的建议更新）
- `download_size`: 下载文件大小（如 "5.2 MB"）
- `checksum`: 文件校验和（可选）
  - `algorithm`: 算法（如 "sha256"）
  - `value`: 校验和值
- `printer_config`: 打印机配置信息（可选）
  - `version`: 配置版本
  - `url`: 配置 URL

## 使用说明

1. **部署配置文件**
   - 将 `version_config.json.example` 复制为 `version_config.json`
   - 修改其中的版本号、更新内容等信息
   - 上传到服务器，确保可以通过 HTTPS 访问

2. **更新版本号**
   - 修改 `app_version` 字段为新版本号
   - 更新 `build_number` 如果重新构建
   - 更新 `release_date` 为当前日期

3. **配置更新文件**
   - 如果提供更新文件，设置 `update_url` 为新版本的下载链接
   - 可以设置 `force_update: true` 来强制用户更新

4. **添加更新日志**
   - 在 `changelog` 数组中添加版本更新记录
   - 包含版本号、日期和更新内容列表

## 版本检查流程

1. 应用启动时自动检查远程版本配置
2. 比较本地版本和远程版本
3. 如果发现新版本，显示更新提示对话框
4. 用户可以选择下载并更新，或稍后更新

## 注意事项

- 确保服务器支持 HTTPS
- 配置文件必须是有效的 JSON 格式
- 版本号格式建议使用语义化版本（Semantic Versioning）
- 更新文件应该是可直接安装的 .exe 文件


