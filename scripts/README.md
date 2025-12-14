# Scripts 目录说明

本目录包含项目构建和部署相关的脚本文件。

## 脚本结构

### 核心脚本

- **`common.sh`** - 通用函数库，包含颜色输出、日志函数、错误处理等
- **`install-deps.sh`** - 安装 FFmpeg 和编译依赖
- **`build.sh`** - 本地编译 Rust 二进制文件
- **`dev.sh`** - 启动本地开发服务器
- **`docker.sh`** - 构建 Docker 镜像
- **`docker-push.sh`** - 推送镜像到容器镜像服务
- **`deploy.sh`** - 部署到阿里云函数计算（构建+推送+部署）

### 辅助脚本

- **`prepare-config.sh`** - 准备配置文件（内部使用）

## 使用方法

所有脚本都通过 Makefile 调用，不需要直接执行：

```bash
# 安装依赖
make install-deps

# 本地编译
make build

# 启动开发服务器
make dev

# 构建 Docker 镜像
make docker

# 推送镜像
make docker-push

# 部署到函数计算
make deploy
```

## 脚本特性

### 错误处理

- 使用 `set -euo pipefail` 确保脚本在错误时立即退出
- 统一的错误处理和日志输出
- 清晰的错误提示信息

### 日志输出

- 彩色日志输出（INFO、SUCCESS、WARN、ERROR）
- 步骤分隔线，便于阅读
- 详细的执行信息

### 环境检测

- 自动检测操作系统（macOS/Ubuntu/Debian）
- 检查必要的命令和依赖
- 提供友好的错误提示

### 路径处理

- 自动获取项目根目录
- 正确处理相对路径和绝对路径
- 支持从任意目录执行脚本

## 开发指南

### 添加新脚本

1. 在 `scripts/` 目录创建新的 `.sh` 文件
2. 在文件开头添加：
   ```bash
   #!/bin/bash
   set -euo pipefail
   
   # 加载通用函数
   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
   source "${SCRIPT_DIR}/common.sh"
   ```
3. 使用 `common.sh` 中的函数进行日志输出和错误处理
4. 在 `Makefile` 中添加对应的目标

### 使用通用函数

```bash
# 日志输出
log_info "这是一条信息"
log_success "操作成功"
log_warn "这是一条警告"
log_error "这是一条错误"

# 步骤分隔
log_step "执行某个步骤"

# 检查命令
check_command docker

# 错误退出
error_exit "错误信息"

# 切换到项目根目录
cd_project_root
```

## 注意事项

1. 所有脚本都使用 `set -euo pipefail` 确保严格模式
2. 脚本会自动切换到项目根目录
3. 使用 `common.sh` 中的配置变量，不要硬编码路径
4. 错误信息要清晰，便于用户排查问题
