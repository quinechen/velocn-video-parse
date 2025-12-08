#!/usr/bin/env python3
"""
视频处理参数优化脚本

用于找到最优的视频处理参数组合，平衡处理时间、关键帧数量和效果。

目标：
- 找到合适的采样率（sample-rate），避免处理时间过长
- 找到合适的最小场景持续时间（min-scene-duration）
- 找到合适的场景变化阈值（threshold）
- 在保证效果（约12个关键帧）的前提下，最小化处理时间
"""

import subprocess
import json
import os
import time
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass
import argparse


@dataclass
class TestResult:
    """测试结果"""
    sample_rate: float
    threshold: float
    min_scene_duration: float
    processing_time: float
    keyframe_count: int
    video_duration: float
    success: bool
    error: Optional[str] = None
    time_ratio: float = 0.0  # 处理时间与视频时长的比例
    keyframe_diff: int = 0  # 关键帧数量与目标的差异
    score: float = 0.0  # 综合得分


class ParameterOptimizer:
    """参数优化器"""
    
    def __init__(self, video_path: str, binary_path: str = None, target_keyframes: int = 12, 
                 tolerance: int = 2, max_time_ratio: float = 0.5):
        """
        初始化优化器
        
        Args:
            video_path: 视频文件路径
            binary_path: 二进制文件路径（默认: ./dist/main）
            target_keyframes: 目标关键帧数量（默认: 12）
            tolerance: 关键帧数量容差（默认: ±2）
            max_time_ratio: 最大处理时间与视频时长的比例（默认: 0.5，即处理时间不超过视频时长的50%）
        """
        self.video_path = Path(video_path)
        if not self.video_path.exists():
            raise FileNotFoundError(f"视频文件不存在: {video_path}")
        
        if binary_path:
            self.binary_path = Path(binary_path)
        else:
            # scripts 目录 -> lib-video-parse 目录 -> dist/main
            self.binary_path = Path(__file__).parent.parent / "dist" / "main"
        
        if not self.binary_path.exists():
            raise FileNotFoundError(f"二进制文件不存在: {self.binary_path}")
        
        self.target_keyframes = target_keyframes
        self.tolerance = tolerance
        self.max_time_ratio = max_time_ratio
        
        # 获取视频时长
        self.video_duration = self._get_video_duration()
        print(f"📹 视频时长: {self.video_duration:.2f}秒")
        print(f"🎯 目标关键帧数量: {self.target_keyframes} (±{self.tolerance})")
        print(f"⏱️  最大处理时间: {self.video_duration * self.max_time_ratio:.2f}秒 ({self.max_time_ratio * 100:.0f}% 视频时长)")
        print()
    
    def _get_video_duration(self) -> float:
        """获取视频时长（使用ffprobe）"""
        try:
            cmd = [
                "ffprobe",
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                str(self.video_path)
            ]
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            return float(result.stdout.strip())
        except (subprocess.CalledProcessError, ValueError, FileNotFoundError):
            print("⚠️  无法获取视频时长，使用默认值60秒")
            return 60.0
    
    def _run_processing(self, sample_rate: float, threshold: float, 
                       min_scene_duration: float, output_dir: Path) -> Tuple[bool, float, int, Optional[str]]:
        """
        运行视频处理
        
        Returns:
            (success, processing_time, keyframe_count, error_message)
        """
        # 确保输出目录存在
        output_dir.mkdir(parents=True, exist_ok=True)
        
        # 构建命令
        cmd = [
            str(self.binary_path),
            "process",
            "--input", str(self.video_path),
            "--output", str(output_dir),
            "--sample-rate", str(sample_rate),
            "--threshold", str(threshold),
            "--min-scene-duration", str(min_scene_duration),
        ]
        
        # 记录开始时间
        start_time = time.time()
        
        try:
            # 运行命令，捕获输出
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=self.video_duration * 2,  # 超时时间：视频时长的2倍
            )
            
            processing_time = time.time() - start_time
            
            if result.returncode != 0:
                return False, processing_time, 0, result.stderr
            
            # 读取元数据文件获取关键帧数量
            metadata_path = output_dir / "metadata.json"
            if metadata_path.exists():
                with open(metadata_path, 'r', encoding='utf-8') as f:
                    metadata = json.load(f)
                    keyframe_count = metadata.get('scene_count', 0)
                    return True, processing_time, keyframe_count, None
            else:
                # 如果没有元数据文件，尝试统计关键帧文件
                keyframe_files = list(output_dir.glob("keyframe_*.jpg"))
                return True, processing_time, len(keyframe_files), None
                
        except subprocess.TimeoutExpired:
            processing_time = time.time() - start_time
            return False, processing_time, 0, "处理超时"
        except Exception as e:
            processing_time = time.time() - start_time
            return False, processing_time, 0, str(e)
    
    def test_parameters(self, sample_rate: float, threshold: float, 
                      min_scene_duration: float) -> TestResult:
        """测试一组参数"""
        output_dir = Path(f"optimize_test_{int(time.time() * 1000)}")
        
        try:
            success, processing_time, keyframe_count, error = self._run_processing(
                sample_rate, threshold, min_scene_duration, output_dir
            )
            
            return TestResult(
                sample_rate=sample_rate,
                threshold=threshold,
                min_scene_duration=min_scene_duration,
                processing_time=processing_time,
                keyframe_count=keyframe_count,
                video_duration=self.video_duration,
                success=success,
                error=error
            )
        finally:
            # 清理临时输出目录
            if output_dir.exists():
                import shutil
                shutil.rmtree(output_dir, ignore_errors=True)
    
    def optimize(self, strategy: str = "grid_search") -> List[TestResult]:
        """
        优化参数
        
        Args:
            strategy: 优化策略 ("grid_search" 或 "adaptive")
        
        Returns:
            测试结果列表
        """
        print("🔍 开始参数优化...")
        print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        
        if strategy == "grid_search":
            return self._grid_search()
        elif strategy == "adaptive":
            return self._adaptive_search()
        else:
            raise ValueError(f"未知的优化策略: {strategy}")
    
    def _grid_search(self) -> List[TestResult]:
        """网格搜索策略"""
        results = []
        
        # 定义参数范围
        # 采样率：从低到高，重点关注低采样率（因为高采样率太慢）
        sample_rates = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0]
        
        # 阈值：常用范围
        thresholds = [0.25, 0.3, 0.35, 0.4]
        
        # 最小场景持续时间：常用范围
        min_scene_durations = [0.8, 1.0, 1.2, 1.5]
        
        total_tests = len(sample_rates) * len(thresholds) * len(min_scene_durations)
        current_test = 0
        
        print(f"📊 网格搜索: {total_tests} 组参数组合")
        print()
        
        for sample_rate in sample_rates:
            for threshold in thresholds:
                for min_scene_duration in min_scene_durations:
                    current_test += 1
                    
                    print(f"[{current_test}/{total_tests}] 测试参数: "
                          f"sample_rate={sample_rate:.1f}, "
                          f"threshold={threshold:.2f}, "
                          f"min_scene_duration={min_scene_duration:.1f}")
                    
                    result = self.test_parameters(sample_rate, threshold, min_scene_duration)
                    results.append(result)
                    
                    if result.success:
                        time_ratio = result.processing_time / result.video_duration
                        keyframe_diff = abs(result.keyframe_count - self.target_keyframes)
                        
                        status = "✓"
                        if keyframe_diff <= self.tolerance and time_ratio <= self.max_time_ratio:
                            status = "⭐"  # 优秀
                        
                        print(f"   {status} 耗时: {result.processing_time:.2f}s "
                              f"({time_ratio*100:.1f}% 视频时长) | "
                              f"关键帧: {result.keyframe_count} | "
                              f"差异: {keyframe_diff}")
                    else:
                        print(f"   ✗ 失败: {result.error}")
                    
                    print()
        
        return results
    
    def _adaptive_search(self) -> List[TestResult]:
        """自适应搜索策略（先粗后细）"""
        results = []
        
        # 第一阶段：粗搜索，找到大致范围
        print("📊 第一阶段：粗搜索")
        print()
        
        coarse_sample_rates = [1.0, 2.0, 3.0, 5.0]
        coarse_thresholds = [0.25, 0.3, 0.35]
        coarse_min_scene_durations = [0.8, 1.0, 1.5]
        
        best_results = []
        
        for sample_rate in coarse_sample_rates:
            for threshold in coarse_thresholds:
                for min_scene_duration in coarse_min_scene_durations:
                    print(f"测试: sample_rate={sample_rate:.1f}, "
                          f"threshold={threshold:.2f}, "
                          f"min_scene_duration={min_scene_duration:.1f}")
                    
                    result = self.test_parameters(sample_rate, threshold, min_scene_duration)
                    results.append(result)
                    
                    if result.success:
                        time_ratio = result.processing_time / result.video_duration
                        keyframe_diff = abs(result.keyframe_count - self.target_keyframes)
                        
                        # 筛选出符合条件的结果
                        if keyframe_diff <= self.tolerance * 2 and time_ratio <= self.max_time_ratio * 1.5:
                            best_results.append(result)
                            print(f"   ✓ 耗时: {result.processing_time:.2f}s | "
                                  f"关键帧: {result.keyframe_count}")
                        else:
                            print(f"   - 耗时: {result.processing_time:.2f}s | "
                                  f"关键帧: {result.keyframe_count} (不符合条件)")
                    else:
                        print(f"   ✗ 失败: {result.error}")
                    print()
        
        if not best_results:
            print("⚠️  第一阶段未找到符合条件的结果，返回所有结果")
            return results
        
        # 第二阶段：在最佳结果附近精细搜索
        print("📊 第二阶段：精细搜索")
        print()
        
        # 找到最佳结果（关键帧数量最接近目标，且处理时间最短）
        best_result = min(
            best_results,
            key=lambda r: (
                abs(r.keyframe_count - self.target_keyframes),
                r.processing_time
            )
        )
        
        print(f"最佳结果: sample_rate={best_result.sample_rate:.1f}, "
              f"threshold={best_result.threshold:.2f}, "
              f"min_scene_duration={best_result.min_scene_duration:.1f}")
        print(f"  关键帧: {best_result.keyframe_count}, "
              f"耗时: {best_result.processing_time:.2f}s")
        print()
        
        # 在最佳结果附近搜索
        fine_sample_rates = [
            max(0.5, best_result.sample_rate - 0.5),
            best_result.sample_rate,
            min(10.0, best_result.sample_rate + 0.5),
        ]
        fine_thresholds = [
            max(0.2, best_result.threshold - 0.05),
            best_result.threshold,
            min(0.5, best_result.threshold + 0.05),
        ]
        fine_min_scene_durations = [
            max(0.5, best_result.min_scene_duration - 0.2),
            best_result.min_scene_duration,
            min(2.0, best_result.min_scene_duration + 0.2),
        ]
        
        for sample_rate in fine_sample_rates:
            for threshold in fine_thresholds:
                for min_scene_duration in fine_min_scene_durations:
                    # 跳过已经测试过的组合
                    if (sample_rate == best_result.sample_rate and
                        threshold == best_result.threshold and
                        min_scene_duration == best_result.min_scene_duration):
                        continue
                    
                    print(f"精细测试: sample_rate={sample_rate:.1f}, "
                          f"threshold={threshold:.2f}, "
                          f"min_scene_duration={min_scene_duration:.1f}")
                    
                    result = self.test_parameters(sample_rate, threshold, min_scene_duration)
                    results.append(result)
                    
                    if result.success:
                        print(f"   ✓ 耗时: {result.processing_time:.2f}s | "
                              f"关键帧: {result.keyframe_count}")
                    else:
                        print(f"   ✗ 失败: {result.error}")
                    print()
        
        return results
    
    def analyze_results(self, results: List[TestResult]) -> Dict:
        """分析结果并找到最优参数"""
        print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        print("📊 结果分析")
        print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        
        # 过滤成功的结果
        successful_results = [r for r in results if r.success]
        
        if not successful_results:
            print("❌ 没有成功的结果")
            return {}
        
        print(f"✓ 成功测试: {len(successful_results)}/{len(results)}")
        print()
        
        # 计算各项指标
        for result in successful_results:
            result.time_ratio = result.processing_time / result.video_duration
            result.keyframe_diff = abs(result.keyframe_count - self.target_keyframes)
            result.score = self._calculate_score(result)
        
        # 按分数排序
        successful_results.sort(key=lambda r: r.score, reverse=True)
        
        # 找到最优结果（关键帧数量符合要求，且处理时间最短）
        optimal_results = [
            r for r in successful_results
            if r.keyframe_diff <= self.tolerance and r.time_ratio <= self.max_time_ratio
        ]
        
        if optimal_results:
            optimal_results.sort(key=lambda r: (r.keyframe_diff, r.processing_time))
            best_result = optimal_results[0]
        else:
            # 如果没有完全符合条件的结果，选择最接近的
            successful_results.sort(key=lambda r: (
                r.keyframe_diff,
                r.time_ratio if r.time_ratio <= self.max_time_ratio * 1.5 else float('inf')
            ))
            best_result = successful_results[0]
        
        # 显示最优结果
        print("🏆 最优参数组合:")
        print(f"   sample_rate: {best_result.sample_rate:.2f}")
        print(f"   threshold: {best_result.threshold:.2f}")
        print(f"   min_scene_duration: {best_result.min_scene_duration:.2f}")
        print()
        print("📈 性能指标:")
        print(f"   • 处理时间: {best_result.processing_time:.2f}秒")
        print(f"   • 时间占比: {best_result.time_ratio*100:.1f}% (视频时长: {best_result.video_duration:.2f}秒)")
        print(f"   • 关键帧数量: {best_result.keyframe_count}")
        print(f"   • 目标差异: {best_result.keyframe_diff} (目标: {self.target_keyframes} ±{self.tolerance})")
        print()
        
        # 显示前5个最佳结果
        print("📋 Top 5 最佳结果:")
        for i, result in enumerate(successful_results[:5], 1):
            status = "⭐" if result.keyframe_diff <= self.tolerance and result.time_ratio <= self.max_time_ratio else "  "
            print(f"{status} {i}. sample_rate={result.sample_rate:.2f}, "
                  f"threshold={result.threshold:.2f}, "
                  f"min_scene_duration={result.min_scene_duration:.2f}")
            print(f"     关键帧: {result.keyframe_count} | "
                  f"耗时: {result.processing_time:.2f}s ({result.time_ratio*100:.1f}%) | "
                  f"差异: {result.keyframe_diff}")
        
        print()
        
        # 生成命令
        print("💡 推荐命令:")
        print(f"./dist/main process \\")
        print(f"  --input {self.video_path} \\")
        print(f"  --output output \\")
        print(f"  --sample-rate {best_result.sample_rate:.2f} \\")
        print(f"  --threshold {best_result.threshold:.2f} \\")
        print(f"  --min-scene-duration {best_result.min_scene_duration:.2f}")
        print()
        
        return {
            "best": {
                "sample_rate": best_result.sample_rate,
                "threshold": best_result.threshold,
                "min_scene_duration": best_result.min_scene_duration,
                "processing_time": best_result.processing_time,
                "keyframe_count": best_result.keyframe_count,
                "time_ratio": best_result.time_ratio,
            },
            "all_results": [
                {
                    "sample_rate": r.sample_rate,
                    "threshold": r.threshold,
                    "min_scene_duration": r.min_scene_duration,
                    "processing_time": r.processing_time,
                    "keyframe_count": r.keyframe_count,
                    "time_ratio": r.time_ratio,
                    "keyframe_diff": r.keyframe_diff,
                }
                for r in successful_results
            ]
        }
    
    def _calculate_score(self, result: TestResult) -> float:
        """计算结果分数（越高越好）"""
        # 关键帧数量得分（越接近目标越好）
        keyframe_score = max(0, 100 - abs(result.keyframe_count - self.target_keyframes) * 10)
        
        # 时间得分（时间越短越好，但不能超过最大时间）
        if result.time_ratio <= self.max_time_ratio:
            time_score = 100 * (1 - result.time_ratio / self.max_time_ratio)
        else:
            time_score = max(0, 100 - (result.time_ratio - self.max_time_ratio) * 200)
        
        # 综合得分（关键帧数量权重60%，时间权重40%）
        return keyframe_score * 0.6 + time_score * 0.4


def main():
    parser = argparse.ArgumentParser(
        description="视频处理参数优化工具",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 使用默认参数优化
  python scripts/optimize_params.py input.mov

  # 指定目标关键帧数量和容差
  python scripts/optimize_params.py input.mov --target-keyframes 12 --tolerance 2

  # 使用自适应搜索策略（更快）
  python scripts/optimize_params.py input.mov --strategy adaptive

  # 指定二进制文件路径
  python scripts/optimize_params.py input.mov --binary ./dist/main
        """
    )
    
    parser.add_argument("video", help="视频文件路径")
    parser.add_argument("--binary", default=None, help="二进制文件路径（默认: 项目根目录下的 dist/main）")
    parser.add_argument("--target-keyframes", type=int, default=12, help="目标关键帧数量（默认: 12）")
    parser.add_argument("--tolerance", type=int, default=2, help="关键帧数量容差（默认: ±2）")
    parser.add_argument("--max-time-ratio", type=float, default=0.5, 
                       help="最大处理时间与视频时长的比例（默认: 0.5，即50%%）")
    parser.add_argument("--strategy", choices=["grid_search", "adaptive"], default="adaptive",
                       help="优化策略（默认: adaptive）")
    parser.add_argument("--output", default=None,
                       help="结果输出文件（默认: scripts/optimize_results.json）")
    
    args = parser.parse_args()
    
    # 设置默认输出路径
    if args.output is None:
        script_dir = Path(__file__).parent
        args.output = str(script_dir / "optimize_results.json")
    
    try:
        optimizer = ParameterOptimizer(
            video_path=args.video,
            binary_path=args.binary,
            target_keyframes=args.target_keyframes,
            tolerance=args.tolerance,
            max_time_ratio=args.max_time_ratio,
        )
        
        results = optimizer.optimize(strategy=args.strategy)
        analysis = optimizer.analyze_results(results)
        
        # 保存结果到JSON文件
        if analysis:
            with open(args.output, 'w', encoding='utf-8') as f:
                json.dump(analysis, f, indent=2, ensure_ascii=False)
            print(f"💾 结果已保存到: {args.output}")
        
        return 0
        
    except Exception as e:
        print(f"❌ 错误: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())

