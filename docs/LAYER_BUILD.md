# 函数计算层构建指南

本文档说明如何构建包含 FFmpeg 及相关运行时库的函数计算层。

## 概述

函数计算层（Layer）允许我们将公共依赖（如 FFmpeg）打包成独立的层，在多个函数之间共享，减少函数包大小并提高部署效率。

## 依赖说明

Rust 程序 `video-parse` 需要以下运行时依赖：

1. **FFmpeg 命令行工具**：用于音频提取
2. **FFmpeg 运行时库**：
   - libavcodec（视频/音频编解码）
   - libavformat（容器格式处理）
   - libavutil（工具函数）
   - libavfilter（滤镜处理）
   - libavdevice（设备输入/输出）
   - libswscale（图像缩放）
   - libswresample（音频重采样）

## 构建步骤

### 方法一：使用构建脚本（推荐）

```bash
# 赋予执行权限
chmod +x build-layer.sh

# 执行构建
./build-layer.sh
```

构建完成后，层文件会保存在 `./layers/ffmpeg-layer.tar.gz`

### 方法二：手动构建

```bash
# 1. 构建 Docker 镜像
docker build -f Dockerfile.layer -t ffmpeg-layer:latest .

# 2. 创建层目录
mkdir -p layers

# 3. 导出层文件
docker run --rm ffmpeg-layer:latest tar czf - -C /opt . > layers/ffmpeg-layer.tar.gz

# 4. 验证层文件
tar -tzf layers/ffmpeg-layer.tar.gz | head -20
```

## 在 s.yaml 中配置层

构建完成后，在 `s.yaml` 中添加层配置：

```yaml
resources:
  hello_world:
    component: fc3 
    actions:    
      pre-${regex('deploy|local')}:
        - run: make video-parse
    props:
      region: ${vars.region} 
      functionName: "velocn-video-parse-function"
      runtime: "custom"
      # ... 其他配置 ...
      layers:
        - layerName: ffmpeg-layer
          code: ./layers/ffmpeg-layer.tar.gz
      code: ./code/target
      customRuntimeConfig:
        command:
          - '/code/main'
          - 'serve'
        port: 9000
```

## 层结构说明

函数计算层会将 `/opt` 目录的内容添加到运行时环境：

```
/opt/
├── bin/
│   ├── ffmpeg          # FFmpeg 命令行工具
│   └── ffprobe         # FFprobe 工具（可选）
└── lib/
    ├── libavcodec.so.*
    ├── libavformat.so.*
    ├── libavutil.so.*
    └── ... (其他 FFmpeg 库文件)
```

函数计算会自动设置：
- `PATH=/opt/bin:$PATH` - 使 ffmpeg 命令可用
- `LD_LIBRARY_PATH=/opt/lib:$LD_LIBRARY_PATH` - 使库文件可被加载

## 验证层内容

构建完成后，可以验证层文件内容：

```bash
# 查看层文件大小
ls -lh layers/ffmpeg-layer.tar.gz

# 列出层中的所有文件
tar -tzf layers/ffmpeg-layer.tar.gz

# 统计文件数量
tar -tzf layers/ffmpeg-layer.tar.gz | wc -l

# 查看二进制文件
tar -xzf layers/ffmpeg-layer.tar.gz -C /tmp/test-layer
/tmp/test-layer/bin/ffmpeg -version
```

## 注意事项

1. **层大小限制**：函数计算的层有大小限制（通常为 50MB 压缩后），FFmpeg 层可能接近或超过此限制，需要优化
2. **架构匹配**：确保 Docker 镜像的架构与函数计算运行环境一致（通常是 x86_64）
3. **库版本兼容性**：确保层中的 FFmpeg 版本与 Rust 程序编译时使用的版本兼容
4. **更新层**：如果更新了 FFmpeg 版本或依赖，需要重新构建层

## 优化建议

如果层文件过大，可以考虑：

1. **只包含必要的编解码器**：移除不需要的编解码器支持
2. **使用静态链接**：编译静态链接的 FFmpeg（会增加层大小但减少依赖）
3. **分离层**：将 FFmpeg 和 Rust 运行时库分成不同的层
4. **使用更小的基础镜像**：使用 Alpine Linux 等更小的基础镜像

## 故障排查

### 问题：层构建失败

- 检查 Docker 是否正常运行：`docker ps`
- 检查网络连接（需要下载 Debian 包）
- 查看 Docker 构建日志中的错误信息

### 问题：函数运行时找不到 ffmpeg

- 确认层已正确配置在 s.yaml 中
- 检查层文件路径是否正确
- 验证层文件是否包含 `/opt/bin/ffmpeg`

### 问题：库加载失败

- 确认所有依赖库都已包含在层中
- 检查 `LD_LIBRARY_PATH` 是否正确设置
- 使用 `ldd` 检查二进制文件的依赖关系

## 参考文档

- [阿里云函数计算 - 使用 Dockerfile 构建层](https://help.aliyun.com/zh/functioncompute/fc-3-0/user-guide/use-a-dockerfile-to-build-a-layer-1)
- [FFmpeg 官方文档](https://ffmpeg.org/documentation.html)
