# 参数优化脚本快速指南

## 快速开始

```bash
# 1. 确保已编译二进制文件
make build-local

# 2. 运行优化脚本（推荐使用自适应搜索）
python3 lib-video-parse/scripts/optimize_params.py input.mov

# 3. 使用推荐参数处理视频
./dist/main process \
  --input input.mov \
  --output output \
  --sample-rate <优化后的值> \
  --threshold <优化后的值> \
  --min-scene-duration <优化后的值>
```

## 示例

假设你的视频是 `input.mov`，目标是找到约12个关键帧，且处理时间不超过视频时长的50%：

```bash
# 使用默认设置（目标12个关键帧，容差±2，最大时间50%）
python3 lib-video-parse/scripts/optimize_params.py input.mov

# 如果希望处理时间更短（不超过30%）
python3 lib-video-parse/scripts/optimize_params.py input.mov --max-time-ratio 0.3

# 如果希望关键帧数量更精确（目标12个，容差±1）
python3 lib-video-parse/scripts/optimize_params.py input.mov --target-keyframes 12 --tolerance 1
```

## 输出说明

脚本会输出：
1. **测试进度**：每个参数组合的测试结果
2. **最优参数**：推荐使用的参数组合
3. **推荐命令**：可直接复制使用的命令

结果也会保存到 `lib-video-parse/scripts/optimize_results.json` 文件中。

## 优化策略选择

- **adaptive（推荐）**：先粗后细，快速找到最优参数
- **grid_search**：全面测试，结果更可靠但更慢

## 常见问题

**Q: 处理时间仍然太长怎么办？**
A: 降低 `--max-time-ratio` 参数，例如 `--max-time-ratio 0.3`（30%）

**Q: 关键帧数量不符合预期怎么办？**
A: 调整 `--target-keyframes` 和 `--tolerance` 参数

**Q: 如何加快优化速度？**
A: 使用 `--strategy adaptive`（默认），或减少测试的参数范围（修改脚本）

## 详细文档

查看 `docs/parameter_optimization.md` 获取完整文档。

