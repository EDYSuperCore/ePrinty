# 本地测试服务器设置

## 快速启动本地服务器

### 方法 1: Python (推荐)

```bash
# Python 3
python -m http.server 8000

# Python 2
python -m SimpleHTTPServer 8000
```

访问: http://localhost:8000/test_config.json

### 方法 2: Node.js

```bash
# 安装 http-server
npm install -g http-server

# 启动服务器
http-server -p 8000
```

### 方法 3: PHP

```bash
php -S localhost:8000
```

## 修改应用配置

编辑 `src-tauri/src/main.rs` 第 34 行：

```rust
let url = "http://localhost:8000/test_config.json";
```

## 测试步骤

1. 将 `test_config.json` 放在服务器根目录
2. 启动本地服务器（使用上述方法之一）
3. 修改 Rust 代码中的 URL
4. 运行 `npm run tauri dev`
5. 应用会从本地服务器加载配置

## 注意事项

⚠️ **HTTP vs HTTPS**

- Tauri 默认允许 HTTP 和 HTTPS 请求
- 生产环境建议使用 HTTPS
- 本地开发可以使用 HTTP

## CORS 问题

如果遇到 CORS 错误，本地服务器可能需要配置 CORS 头：

### Python 示例

```python
from http.server import HTTPServer, SimpleHTTPRequestHandler
import json

class CORSHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET')
        self.send_header('Content-Type', 'application/json')
        super().end_headers()

httpd = HTTPServer(('localhost', 8000), CORSHandler)
httpd.serve_forever()
```

保存为 `server.py`，运行 `python server.py`

