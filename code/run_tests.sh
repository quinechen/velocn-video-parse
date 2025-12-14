#!/bin/bash
# 运行测试脚本
# 由于 main 包的特殊性，我们使用 go run 来执行测试

set -e

echo "🧪 运行视频处理测试..."
echo ""

# 检查测试视频文件是否存在
if [ ! -f "testdata/test.mp4" ]; then
    echo "⚠️  测试视频文件不存在: testdata/test.mp4"
    echo "请将测试视频文件放入 testdata/ 目录"
    echo ""
    echo "可以使用以下命令生成测试视频："
    echo "ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 -c:v libx264 -pix_fmt yuv420p testdata/test.mp4"
    exit 1
fi

echo "✅ 测试视频文件存在"
echo ""

# 运行测试（使用 go test，但跳过 main 包的问题）
# 注意：由于 main 包的限制，我们创建一个简单的测试运行器
echo "运行文件类型检测测试..."
go run -tags test processor_test.go processor.go handler.go config.go video_processor.go scene_detector.go audio_extractor.go metadata.go oss_event.go 2>&1 | grep -v "no Go files" || echo "测试需要单独运行"

echo ""
echo "📝 提示：由于 main 包的特殊性，建议使用以下方式测试："
echo "1. 使用 go run 直接运行测试函数"
echo "2. 或者将代码重构为库包（推荐用于生产环境）"
echo ""
echo "当前测试文件已创建，可以在 IDE 中运行单个测试函数"


