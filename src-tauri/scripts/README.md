# 打印机安装脚本目录

## 文件说明

### prnport.vbs

用于添加和删除打印机端口的 Windows VBScript 脚本。

## 使用方法

请将 `prnport.vbs` 文件放在此目录（`src-tauri/scripts/`）下。

## 脚本调用方式

应用程序会使用以下命令调用脚本：

```bash
cscript //NoLogo //B scripts/prnport.vbs -a -r IP_192_168_20_65 -h 192.168.20.65
```

参数说明：
- `-a`: 添加端口
- `-r`: 端口名称（格式：IP_192_168_20_65）
- `-h`: 打印机 IP 地址

## 注意事项

1. 脚本文件必须放在 `src-tauri/scripts/` 目录下
2. 构建时脚本会被包含在应用资源中
3. 确保脚本具有执行权限

