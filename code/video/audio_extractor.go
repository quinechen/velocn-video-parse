package video

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

// AudioExtractor 音频提取器
type AudioExtractor struct {
	InputPath string
}

// NewAudioExtractor 创建新的音频提取器
func NewAudioExtractor(inputPath string) (*AudioExtractor, error) {
	if _, err := os.Stat(inputPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("视频文件不存在: %s", inputPath)
	}

	return &AudioExtractor{
		InputPath: inputPath,
	}, nil
}

// ExtractToFile 提取音频到文件
func (a *AudioExtractor) ExtractToFile(outputPath string) error {
	// 确保输出目录存在
	outputDir := filepath.Dir(outputPath)
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return fmt.Errorf("创建输出目录失败: %v", err)
	}

	// 使用 ffmpeg 提取音频
	cmd := exec.Command("ffmpeg",
		"-i", a.InputPath,
		"-vn",              // 不包含视频
		"-acodec", "aac",   // 使用 AAC 编码
		"-ab", "192k",      // 音频比特率
		"-ar", "44100",     // 采样率
		"-y",               // 覆盖已存在的文件
		outputPath,
	)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("提取音频失败: %v", err)
	}

	return nil
}

