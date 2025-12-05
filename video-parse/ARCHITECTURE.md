# 视频拉片工具架构设计

## 方案概述

本方案实现了一个完整的视频分析系统，能够自动检测视频中的场景变化（镜头切换），提取关键帧，并生成详细的元数据。系统采用模块化设计，使用 Rust 语言开发，利用 FFmpeg 进行底层视频处理。

## 核心功能

1. **视频解码与帧提取**：从视频文件中解码并提取帧
2. **场景检测**：分析帧序列，检测镜头切换点
3. **关键帧提取**：保存每个场景的代表帧
4. **音频提取**：从视频中分离音频流
5. **元数据生成**：生成包含场景信息的 JSON 文件

## 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                      CLI 接口层                          │
│                  (main.rs - Args)                       │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                    业务逻辑层                            │
│              (main.rs - process_video)                  │
└─────┬───────────┬───────────┬───────────┬──────────────┘
      │           │           │           │
      ▼           ▼           ▼           ▼
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│ 视频处理  │ │ 场景检测  │ │ 音频提取  │ │ 元数据   │
│ Processor│ │ Detector │ │ Extractor│ │ Metadata │
└──────────┘ └──────────┘ └──────────┘ └──────────┘
      │           │           │           │
      ▼           ▼           ▼           ▼
┌─────────────────────────────────────────────────────────┐
│                    FFmpeg 底层库                        │
│              (ffmpeg-next, image)                       │
└─────────────────────────────────────────────────────────┘
```

## 模块设计

### 1. VideoProcessor (视频处理器)

**职责**：
- 打开视频文件
- 获取视频基本信息（分辨率、帧率等）
- 解码视频帧
- 将 FFmpeg 帧转换为 Rust Image 格式

**关键设计**：
- 支持帧采样，可配置采样率以减少计算量
- 使用 FFmpeg 的软件缩放器转换为 RGB24 格式
- 维护帧的时间戳信息

**API**：
```rust
impl VideoProcessor {
    fn new(input_path: &Path) -> Result<Self>
    fn get_video_info(&self) -> Result<(fps, width, height)>
    fn extract_frames(&self, sample_rate: Option<f64>) -> Result<Vec<(timestamp, image)>>
}
```

### 2. SceneDetector (场景检测器)

**职责**：
- 计算相邻帧之间的差异
- 检测场景变化点
- 过滤误检（最小场景持续时间）

**算法设计**：

1. **帧差异计算**：
   - 直方图差异：计算两帧的灰度直方图，比较分布差异
   - 像素差异：逐像素比较，计算平均差异
   - 组合差异：`差异度 = 0.6 * 直方图差异 + 0.4 * 像素差异`

2. **场景变化检测**：
   - 当帧差异超过阈值时，判定为场景切换
   - 应用最小场景持续时间过滤，避免快速切换导致的误检

**关键参数**：
- `threshold`: 场景变化阈值（0.0-1.0）
- `min_scene_duration`: 最小场景持续时间（秒）

**API**：
```rust
impl SceneDetector {
    fn new(threshold: f64, min_scene_duration: f64) -> Self
    fn calculate_frame_difference(&self, frame1: &Image, frame2: &Image) -> f64
    fn detect_scenes(&self, frames: &[(f64, Image)], fps: f64) -> Result<Vec<f64>>
}
```

### 3. AudioExtractor (音频提取器)

**职责**：
- 从视频文件中提取音频流
- 保存为音频文件（AAC 格式）

**设计**：
- 使用 FFmpeg 命令行工具进行音频提取
- 优先尝试直接复制音频流（无损）
- 失败时回退到重新编码为 AAC

**API**：
```rust
impl AudioExtractor {
    fn new(input_path: &Path) -> Result<Self>
    fn extract_to_file(&self, output_path: &Path) -> Result<()>
}
```

### 4. Metadata (元数据结构)

**职责**：
- 定义场景和视频的元数据结构
- 支持 JSON 序列化

**数据结构**：
```rust
struct SceneMetadata {
    scene_id: usize,
    keyframe_file: String,
    start_time: f64,
    end_time: f64,
    duration: f64,
}

struct VideoMetadata {
    input_video: String,
    total_duration: f64,
    fps: f64,
    resolution: String,
    scene_count: usize,
    audio_file: String,
    scenes: Vec<SceneMetadata>,
}
```

## 处理流程

```
1. 初始化
   ├─ 解析命令行参数
   ├─ 创建输出目录
   └─ 初始化 FFmpeg

2. 视频分析
   ├─ 打开视频文件
   ├─ 获取视频信息（分辨率、帧率）
   ├─ 提取视频帧（按采样率）
   └─ 检测场景变化点

3. 关键帧提取
   ├─ 遍历场景变化点
   ├─ 提取每个场景的关键帧
   └─ 保存为 JPG 图片

4. 音频提取
   └─ 使用 FFmpeg 提取音频流

5. 元数据生成
   ├─ 构建场景元数据
   ├─ 构建视频元数据
   └─ 序列化为 JSON

6. 输出
   └─ 保存所有文件到输出目录
```

## 性能优化策略

1. **帧采样**：
   - 默认采样率 2 fps，可配置
   - 减少需要处理的帧数，提高速度
   - 对于大多数视频，2 fps 足够检测场景变化

2. **图像处理优化**：
   - 场景检测时使用灰度图
   - 直方图计算使用归一化，避免分辨率影响

3. **内存管理**：
   - 流式处理帧，不一次性加载所有帧
   - 及时释放不需要的图像数据

## 扩展性设计

### 可扩展的场景检测算法

当前使用基于帧差异的检测方法，未来可以扩展：

1. **基于颜色的检测**：分析颜色分布变化
2. **基于运动向量的检测**：分析相机运动
3. **机器学习方法**：使用训练好的模型检测场景变化
4. **音频辅助检测**：结合音频变化检测场景切换

### 可扩展的输出格式

- 支持多种图片格式（PNG、WebP 等）
- 支持多种音频格式（MP3、OGG 等）
- 支持多种元数据格式（XML、YAML 等）

## 错误处理

- 使用 `anyhow` 进行统一的错误处理
- 提供清晰的错误信息
- 关键步骤都有错误检查和上下文信息

## 测试策略

- 单元测试：测试各个模块的核心功能
- 集成测试：测试完整的处理流程
- 性能测试：测试不同大小视频的处理时间

## 依赖说明

- `ffmpeg-next`: FFmpeg Rust 绑定，用于视频解码
- `image`: 图像处理库
- `serde` + `serde_json`: JSON 序列化
- `clap`: 命令行参数解析
- `anyhow`: 错误处理

## 系统要求

- Rust 1.70+
- FFmpeg 开发库（libavcodec, libavformat, libavutil 等）
- FFmpeg 命令行工具（用于音频提取）