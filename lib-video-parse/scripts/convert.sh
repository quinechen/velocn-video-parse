#!/bin/bash

# 视频转换脚本
# 用法: ./convert.sh [input.mp4] [output_dir] [options...]
# 示例: ./convert.sh input.mp4 output
#       ./convert.sh input.mp4 output --threshold 0.3 --sample-rate 2.0

# 注意：不使用 set -e，因为我们需要手动处理错误

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 脚本目录（脚本所在目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# lib-video-parse 目录（scripts 的父目录）
LIB_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
# 项目根目录（lib-video-parse 的父目录）
PROJECT_ROOT="$(cd "${LIB_DIR}/.." && pwd)"
BINARY_PATH="${LIB_DIR}/dist/main"

# 保存原始参数数量
ORIGINAL_ARG_COUNT=$#

# 默认值
INPUT_FILE="${1:-input.mp4}"
OUTPUT_DIR="${2:-output}"

# 检查二进制文件是否存在
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}错误: 二进制文件不存在: $BINARY_PATH${NC}"
    echo -e "${YELLOW}提示: 请先运行 'make build-local' 编译项目${NC}"
    exit 1
fi

# 检查二进制文件是否有执行权限
if [ ! -x "$BINARY_PATH" ]; then
    echo -e "${YELLOW}警告: 二进制文件没有执行权限，正在添加...${NC}"
    chmod +x "$BINARY_PATH"
fi

# 检查输入文件是否存在
if [ ! -f "$INPUT_FILE" ]; then
    echo -e "${RED}错误: 输入文件不存在: $INPUT_FILE${NC}"
    echo ""
    echo "用法: $0 [input.mp4] [output_dir] [options...]"
    echo ""
    echo "参数说明:"
    echo "  input.mp4    - 输入视频文件路径（默认: input.mp4）"
    echo "  output_dir   - 输出目录（默认: output）"
    echo ""
    echo "可选参数:"
    echo "  --threshold <value>          - 场景变化检测阈值 (0.0-1.0)，默认: 0.3"
    echo "  --min-scene-duration <sec>   - 最小场景持续时间（秒），默认: 1.0"
    echo "  --sample-rate <fps>          - 帧采样率（每秒采样多少帧），默认: 2.0"
    echo ""
    echo "示例:"
    echo "  $0 input.mp4 output"
    echo "  $0 input.mp4 output --threshold 0.3 --sample-rate 2.0"
    exit 1
fi

# 创建输出目录（如果不存在）
mkdir -p "$OUTPUT_DIR"

# 显示配置信息
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}视频转换工具${NC}"
echo -e "${GREEN}========================================${NC}"
echo "输入文件: $INPUT_FILE"
echo "输出目录: $OUTPUT_DIR"
echo "二进制文件: $BINARY_PATH"
echo ""

# 提取额外的参数（跳过前两个位置参数）
# 根据原始参数数量决定跳过多少个参数
if [ "$ORIGINAL_ARG_COUNT" -ge 2 ]; then
    shift 2
elif [ "$ORIGINAL_ARG_COUNT" -ge 1 ]; then
    shift 1
fi
EXTRA_ARGS=("$@")

# 运行二进制文件
echo -e "${GREEN}开始处理视频...${NC}"
echo ""

"$BINARY_PATH" process \
    --input "$INPUT_FILE" \
    --output "$OUTPUT_DIR" \
    "${EXTRA_ARGS[@]}"

# 检查执行结果
if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}✓ 转换完成！${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo "输出目录: $OUTPUT_DIR"
    echo ""
    echo "输出文件:"
    ls -lh "$OUTPUT_DIR" 2>/dev/null || echo "  (输出目录为空)"
else
    echo ""
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}✗ 转换失败${NC}"
    echo -e "${RED}========================================${NC}"
    exit 1
fi

