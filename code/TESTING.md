# 测试指南

## 测试文件说明

本项目包含以下测试文件：

- `processor_test.go` - 视频处理流程测试
- `handler_test.go` - OSS 事件处理测试

## 测试环境要求

1. **测试视频文件**：需要在 `testdata/test.mp4` 放置一个测试视频文件
2. **FFmpeg**：需要安装 FFmpeg（用于视频处理）
3. **Go 1.25+**：需要 Go 1.25 或更高版本

## 准备测试视频文件

### 方法 1：使用 FFmpeg 生成测试视频

```bash
cd code
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
  -c:v libx264 -pix_fmt yuv420p testdata/test.mp4
```

### 方法 2：使用现有视频文件

将你的测试视频文件重命名为 `test.mp4` 并放入 `testdata/` 目录。

## 运行测试

### 注意：main 包测试限制

由于 Go 的限制，`main` 包不能直接使用 `go test` 命令。有以下几种解决方案：

### 方案 1：在 IDE 中运行（推荐）

大多数 IDE（如 VS Code、GoLand）支持直接运行单个测试函数：

1. 打开测试文件（如 `processor_test.go`）
2. 点击测试函数上方的 "Run Test" 按钮
3. IDE 会自动处理 main 包的限制

### 方案 2：使用测试运行器

创建一个简单的测试运行器程序来执行测试：

```go
// test_runner.go
package main

import (
    "os"
    "testing"
)

func main() {
    // 设置测试环境
    os.Args = []string{"test"}
    
    // 运行测试
    m := testing.MainStart(testing.InternalTest, nil, nil, nil)
    m.Run()
}
```

### 方案 3：重构为库包（长期方案）

将核心逻辑从 `main` 包移到独立的库包（如 `video` 包），这样可以正常使用 `go test`。

## 测试内容

### TestIsVideoFile

测试视频文件类型检测功能，验证各种文件扩展名是否能正确识别。

### TestProcessVideo

测试完整的视频处理流程：
- 视频文件读取
- 关键帧提取
- 场景检测
- 音频提取
- 元数据生成

### TestHandleOSSEvent

测试 OSS 事件处理：
- 事件解析
- 文件类型过滤
- 处理流程

## 测试文件部署

所有测试相关文件已添加到 `.fcignore`，不会被部署到云端：

- `testdata/` - 测试数据目录
- `*_test.go` - 测试文件
- `run_tests.sh` - 测试脚本

## 故障排除

### 问题：测试视频文件不存在

**解决方案**：按照上述方法创建或添加测试视频文件。

### 问题：go test 报错 "cannot import main"

**原因**：Go 不允许直接测试 main 包。

**解决方案**：使用 IDE 运行测试，或重构代码为库包。

### 问题：FFmpeg 未找到

**解决方案**：安装 FFmpeg：

```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt-get install ffmpeg

# CentOS/RHEL
sudo yum install ffmpeg
```

## 下一步

1. 添加测试视频文件到 `testdata/test.mp4`
2. 在 IDE 中运行测试函数
3. 验证测试通过
4. 继续开发新功能


