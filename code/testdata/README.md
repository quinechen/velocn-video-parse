# 测试数据目录

此目录用于存储测试视频文件，用于本地测试。

## 使用方法

1. 将测试视频文件 `test.mp4` 放入此目录
2. 运行测试：
   ```bash
   cd code
   go test -v ./...
   ```

## 注意事项

- 此目录已添加到 `.fcignore`，不会被部署到云端
- 测试文件应该是一个有效的视频文件（MP4 格式）
- 建议使用较小的测试视频文件（几秒到几分钟）以加快测试速度

## 测试文件要求

- 文件名：`test.mp4`
- 格式：MP4（H.264 编码）
- 大小：建议小于 10MB
- 时长：建议 5-30 秒

## 获取测试视频

你可以从以下来源获取测试视频：

1. 使用 FFmpeg 生成测试视频：
   ```bash
   ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 -c:v libx264 -pix_fmt yuv420p testdata/test.mp4
   ```

2. 从公开测试视频库下载（如 Big Buck Bunny 的片段）

3. 使用自己的测试视频文件


