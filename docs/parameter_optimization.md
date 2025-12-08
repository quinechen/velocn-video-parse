# 视频处理参数优化指南

## 概述

`optimize_params.py` 是一个自动化参数优化工具，用于找到最优的视频处理参数组合，平衡处理时间、关键帧数量和效果。

## 问题背景

视频处理涉及三个主要参数：
- **采样率** (`--sample-rate`): 每秒采样多少帧用于分析
  - 值越大：处理时间越长，但准确性更高
  - 值越小：处理时间越短，但可能遗漏快速场景切换
- **场景变化阈值** (`--threshold`): 场景变化检测的敏感度 (0.0-1.0)
  - 值越大：检测越敏感，更容易检测到场景变化
  - 值越小：检测越保守，只检测明显的场景变化
- **最小场景持续时间** (`--min-scene-duration`): 最小场景持续时间（秒）
  - 防止过于频繁的场景切换检测

**挑战**：
- 高采样率（如10.0 fps）导致处理时间比视频本身还长
- 需要找到平衡点：在保证效果（约12个关键帧）的前提下，最小化处理时间

## 使用方法

### 基本用法

```bash
# 使用默认参数优化
python lib-video-parse/scripts/optimize_params.py input.mov
```

### 高级用法

```bash
# 指定目标关键帧数量和容差
python lib-video-parse/scripts/optimize_params.py input.mov \
  --target-keyframes 12 \
  --tolerance 2

# 使用自适应搜索策略（更快，推荐）
python lib-video-parse/scripts/optimize_params.py input.mov \
  --strategy adaptive

# 使用网格搜索策略（更全面，但更慢）
python lib-video-parse/scripts/optimize_params.py input.mov \
  --strategy grid_search

# 指定最大处理时间比例（默认50%）
python lib-video-parse/scripts/optimize_params.py input.mov \
  --max-time-ratio 0.3  # 处理时间不超过视频时长的30%

# 指定二进制文件路径
python lib-video-parse/scripts/optimize_params.py input.mov \
  --binary ./dist/main
```

### 参数说明

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `video` | 视频文件路径（必需） | - |
| `--binary` | 二进制文件路径 | `./dist/main` |
| `--target-keyframes` | 目标关键帧数量 | `12` |
| `--tolerance` | 关键帧数量容差 | `±2` |
| `--max-time-ratio` | 最大处理时间与视频时长的比例 | `0.5` (50%) |
| `--strategy` | 优化策略 (`grid_search` 或 `adaptive`) | `adaptive` |
| `--output` | 结果输出文件 | `lib-video-parse/scripts/optimize_results.json` |

## 优化策略

### 1. 网格搜索 (Grid Search)

**特点**：
- 全面测试所有参数组合
- 结果更可靠
- 但测试时间较长

**参数范围**：
- 采样率: [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0]
- 阈值: [0.25, 0.3, 0.35, 0.4]
- 最小场景持续时间: [0.8, 1.0, 1.2, 1.5]

**总测试数**: 7 × 4 × 4 = 112 组参数

### 2. 自适应搜索 (Adaptive Search) - 推荐

**特点**：
- 先粗后细的两阶段搜索
- 测试时间较短
- 结果接近最优

**流程**：
1. **第一阶段（粗搜索）**：
   - 测试较少的参数组合，找到大致范围
   - 筛选出符合条件的结果

2. **第二阶段（精细搜索）**：
   - 在最佳结果附近进行精细搜索
   - 找到最优参数组合

## 输出说明

### 控制台输出

脚本会输出：
1. **视频信息**：视频时长、目标关键帧数量等
2. **测试进度**：每个参数组合的测试结果
3. **结果分析**：
   - 最优参数组合
   - 性能指标（处理时间、关键帧数量等）
   - Top 5 最佳结果
   - 推荐命令

### JSON 输出

结果会保存到 JSON 文件（默认: `lib-video-parse/scripts/optimize_results.json`），包含：
- `best`: 最优参数组合和性能指标
- `all_results`: 所有成功测试的结果

## 示例输出

```
📹 视频时长: 48.00秒
🎯 目标关键帧数量: 12 (±2)
⏱️  最大处理时间: 24.00秒 (50% 视频时长)

🔍 开始参数优化...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 第一阶段：粗搜索

测试: sample_rate=1.0, threshold=0.25, min_scene_duration=0.8
   ✓ 耗时: 12.34s | 关键帧: 8

测试: sample_rate=2.0, threshold=0.30, min_scene_duration=1.0
   ✓ 耗时: 18.56s | 关键帧: 12

...

📊 第二阶段：精细搜索

最佳结果: sample_rate=2.0, threshold=0.30, min_scene_duration=1.0
  关键帧: 12, 耗时: 18.56s

...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 结果分析
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✓ 成功测试: 45/48

🏆 最优参数组合:
   sample_rate: 2.00
   threshold: 0.30
   min_scene_duration: 1.00

📈 性能指标:
   • 处理时间: 18.56秒
   • 时间占比: 38.7% (视频时长: 48.00秒)
   • 关键帧数量: 12
   • 目标差异: 0 (目标: 12 ±2)

📋 Top 5 最佳结果:
⭐ 1. sample_rate=2.00, threshold=0.30, min_scene_duration=1.00
     关键帧: 12 | 耗时: 18.56s (38.7%) | 差异: 0
⭐ 2. sample_rate=2.50, threshold=0.30, min_scene_duration=1.00
     关键帧: 13 | 耗时: 22.34s (46.5%) | 差异: 1
...

💡 推荐命令:
./dist/main process \
  --input input.mov \
  --output output \
  --sample-rate 2.00 \
  --threshold 0.30 \
  --min-scene-duration 1.00

💾 结果已保存到: lib-video-parse/scripts/optimize_results.json
```

## 评分机制

结果按以下标准评分（分数越高越好）：

1. **关键帧数量得分**（权重60%）：
   - 越接近目标关键帧数量，得分越高
   - 公式: `max(0, 100 - |实际数量 - 目标数量| × 10)`

2. **时间得分**（权重40%）：
   - 处理时间越短，得分越高
   - 如果超过最大时间比例，会扣分
   - 公式: `100 × (1 - 时间比例 / 最大时间比例)`

3. **综合得分**：
   - `综合得分 = 关键帧得分 × 0.6 + 时间得分 × 0.4`

## 最佳实践

1. **首次优化**：
   ```bash
   python lib-video-parse/scripts/optimize_params.py input.mov --strategy adaptive
   ```
   - 使用自适应搜索，快速找到大致范围

2. **精细优化**：
   ```bash
   python lib-video-parse/scripts/optimize_params.py input.mov --strategy grid_search
   ```
   - 如果需要更精确的结果，使用网格搜索

3. **批量优化**：
   - 对不同类型的视频分别优化
   - 保存结果到不同的JSON文件
   - 根据视频类型选择不同的参数

4. **参数调整**：
   - 如果处理时间仍然太长，降低 `--max-time-ratio`
   - 如果关键帧数量不符合要求，调整 `--target-keyframes` 和 `--tolerance`

## 注意事项

1. **依赖要求**：
   - Python 3.6+
   - `ffprobe`（用于获取视频时长，可选）

2. **处理时间**：
   - 网格搜索可能需要较长时间（取决于视频长度和参数组合数）
   - 自适应搜索通常更快

3. **临时文件**：
   - 脚本会创建临时输出目录进行测试
   - 测试完成后会自动清理

4. **超时设置**：
   - 如果处理时间超过视频时长的2倍，会超时并跳过

## 故障排除

### 问题：找不到二进制文件

```bash
# 确保已编译
make build-local

# 或指定二进制文件路径
python lib-video-parse/scripts/optimize_params.py input.mov --binary ./dist/main
```

### 问题：处理超时

```bash
# 增加超时时间（修改脚本中的 timeout 参数）
# 或使用更低的采样率范围
```

### 问题：所有测试都失败

- 检查视频文件是否损坏
- 检查二进制文件是否正常工作
- 查看错误信息，可能是参数范围设置不当

## 总结

使用 `optimize_params.py` 可以：
- ✅ 自动找到最优参数组合
- ✅ 平衡处理时间和效果
- ✅ 避免手动尝试大量参数组合
- ✅ 生成可直接使用的命令

推荐工作流程：
1. 使用自适应搜索快速找到大致范围
2. 根据结果调整目标参数
3. 使用网格搜索进行精细优化（可选）
4. 使用推荐参数进行实际处理

