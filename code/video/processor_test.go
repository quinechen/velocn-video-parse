package video

import (
	"os"
	"path/filepath"
	"testing"
)

// TestProcessVideo 测试视频处理流程
func TestProcessVideo(t *testing.T) {
	// 检查测试视频文件是否存在
	testVideoPath := filepath.Join("testdata", "test.mp4")
	if _, err := os.Stat(testVideoPath); os.IsNotExist(err) {
		t.Skipf("测试视频文件不存在: %s，跳过测试", testVideoPath)
		return
	}

	// 创建临时输出目录
	outputDir := filepath.Join(os.TempDir(), "video-parse-test", "output")
	defer os.RemoveAll(filepath.Dir(outputDir))

	// 配置处理参数
	cfg := ProcessConfig{
		Threshold:        0.35,
		MinSceneDuration: 0.8,
		SampleRate:       0.5,
	}

	// 执行视频处理
	t.Logf("开始处理测试视频: %s", testVideoPath)
	result, err := ProcessVideo(testVideoPath, outputDir, config)
	if err != nil {
		t.Fatalf("处理视频失败: %v", err)
	}

	// 验证处理结果
	if result == nil {
		t.Fatal("处理结果为空")
	}

	// 验证元数据
	if result.Metadata.SceneCount <= 0 {
		t.Errorf("期望检测到至少 1 个场景，实际: %d", result.Metadata.SceneCount)
	}

	// 验证关键帧
	if len(result.KeyframeFiles) == 0 {
		t.Error("期望提取至少 1 个关键帧，实际: 0")
	}

	// 验证关键帧文件是否存在
	for i, keyframe := range result.KeyframeFiles {
		if _, err := os.Stat(keyframe); os.IsNotExist(err) {
			t.Errorf("关键帧文件不存在: %s (索引: %d)", keyframe, i)
		}
	}

	// 验证音频文件（如果存在）
	if result.AudioFile != "" {
		if _, err := os.Stat(result.AudioFile); os.IsNotExist(err) {
			t.Errorf("音频文件不存在: %s", result.AudioFile)
		}
	}

	t.Logf("✅ 测试通过:")
	t.Logf("  • 场景数: %d", result.Metadata.SceneCount)
	t.Logf("  • 关键帧数: %d", len(result.KeyframeFiles))
	t.Logf("  • 音频文件: %s", result.AudioFile)
	t.Logf("  • 输出目录: %s", result.OutputDir)
}

// TestProcessVideoWithCustomConfig 测试自定义配置的视频处理
func TestProcessVideoWithCustomConfig(t *testing.T) {
	// 检查测试视频文件是否存在
	testVideoPath := filepath.Join("testdata", "test.mp4")
	if _, err := os.Stat(testVideoPath); os.IsNotExist(err) {
		t.Skipf("测试视频文件不存在: %s，跳过测试", testVideoPath)
		return
	}

	// 创建临时输出目录
	outputDir := filepath.Join(os.TempDir(), "video-parse-test-custom", "output")
	defer os.RemoveAll(filepath.Dir(outputDir))

	// 自定义配置（更严格的阈值，更低的采样率）
	cfg := ProcessConfig{
		Threshold:        0.5,  // 更高的阈值，更少的场景
		MinSceneDuration: 1.0,  // 更长的最小场景时长
		SampleRate:       0.25, // 更低的采样率
	}

	// 执行视频处理
	t.Logf("开始处理测试视频（自定义配置）: %s", testVideoPath)
	result, err := ProcessVideo(testVideoPath, outputDir, config)
	if err != nil {
		t.Fatalf("处理视频失败: %v", err)
	}

	// 验证处理结果
	if result == nil {
		t.Fatal("处理结果为空")
	}

	t.Logf("✅ 自定义配置测试通过:")
	t.Logf("  • 场景数: %d", result.Metadata.SceneCount)
	t.Logf("  • 关键帧数: %d", len(result.KeyframeFiles))
}

// TestIsVideoFile 测试视频文件类型检测
func TestIsVideoFile(t *testing.T) {
	tests := []struct {
		filename string
		expected bool
	}{
		{"test.mp4", true},
		{"test.avi", true},
		{"test.mov", true},
		{"test.mkv", true},
		{"test.flv", true},
		{"test.wmv", true},
		{"test.webm", true},
		{"test.m4v", true},
		{"test.3gp", true},
		{"test.ogv", true},
		{"test.ts", true},
		{"test.jpg", false},
		{"test.png", false},
		{"test.gif", false},
		{"test.txt", false},
		{"test.pdf", false},
		{"test", false},
		{"test.MP4", true}, // 测试大小写不敏感
		{"test.AVI", true},
	}

	for _, tt := range tests {
		t.Run(tt.filename, func(t *testing.T) {
			result := isVideoFile(tt.filename)
			if result != tt.expected {
				t.Errorf("isVideoFile(%q) = %v, 期望 %v", tt.filename, result, tt.expected)
			}
		})
	}
}
