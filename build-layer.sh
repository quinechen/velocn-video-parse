#!/bin/bash
# 构建函数计算 FFmpeg 层的脚本

set -e

LAYER_NAME="ffmpeg-layer"
LAYER_DIR="./layers"
LAYER_ARCHIVE="${LAYER_DIR}/${LAYER_NAME}.tar.gz"

echo "开始构建 FFmpeg 层..."

# 创建层目录
mkdir -p "${LAYER_DIR}"

# 构建 Docker 镜像
echo "构建 Docker 镜像..."
docker build -f Dockerfile.layer -t "${LAYER_NAME}:latest" .

# 运行容器并导出层文件
echo "导出层文件..."
docker run --rm "${LAYER_NAME}:latest" tar czf - -C /opt . > "${LAYER_ARCHIVE}"

# 验证层文件
if [ -f "${LAYER_ARCHIVE}" ]; then
    SIZE=$(du -h "${LAYER_ARCHIVE}" | cut -f1)
    echo "✓ 层文件构建成功: ${LAYER_ARCHIVE} (大小: ${SIZE})"
    
    # 显示层内容预览
    echo ""
    echo "层内容预览:"
    tar -tzf "${LAYER_ARCHIVE}" | head -20
    echo "..."
    echo ""
    echo "总文件数: $(tar -tzf "${LAYER_ARCHIVE}" | wc -l)"
else
    echo "✗ 层文件构建失败"
    exit 1
fi

echo ""
echo "构建完成！"
echo "层文件位置: ${LAYER_ARCHIVE}"
echo ""
echo "接下来可以在 s.yaml 中配置使用此层:"
echo "  layers:"
echo "    - layerName: ${LAYER_NAME}"
echo "      code: ${LAYER_ARCHIVE}"
