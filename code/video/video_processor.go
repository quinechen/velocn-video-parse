package video

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
)

// VideoProcessor 视频处理器
type VideoProcessor struct {
	InputPath string
}

// NewVideoProcessor 创建新的视频处理器
func NewVideoProcessor(inputPath string) (*VideoProcessor, error) {
	if _, err := os.Stat(inputPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("视频文件不存在: %s", inputPath)
	}

	return &VideoProcessor{
		InputPath: inputPath,
	}, nil
}

// VideoInfo 视频信息
type VideoInfo struct {
	FPS    float64
	Width  int
	Height int
	Duration float64
}

// GetVideoInfo 获取视频信息
func (v *VideoProcessor) GetVideoInfo() (*VideoInfo, error) {
	// 使用 ffprobe 获取视频信息
	cmd := exec.Command("ffprobe",
		"-v", "error",
		"-select_streams", "v:0",
		"-show_entries", "stream=width,height,r_frame_rate",
		"-show_entries", "format=duration",
		"-of", "json",
		v.InputPath,
	)

	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("获取视频信息失败: %v", err)
	}

	var probeResult struct {
		Streams []struct {
			Width  string `json:"width"`
			Height string `json:"height"`
			RFrameRate string `json:"r_frame_rate"`
		} `json:"streams"`
		Format struct {
			Duration string `json:"duration"`
		} `json:"format"`
	}

	if err := json.Unmarshal(output, &probeResult); err != nil {
		return nil, fmt.Errorf("解析视频信息失败: %v", err)
	}

	if len(probeResult.Streams) == 0 {
		return nil, fmt.Errorf("未找到视频流")
	}

	stream := probeResult.Streams[0]

	width, _ := strconv.Atoi(stream.Width)
	height, _ := strconv.Atoi(stream.Height)

	// 解析帧率 (格式: "30/1" 或 "30000/1001")
	fps := 30.0 // 默认值
	if stream.RFrameRate != "" {
		parts := strings.Split(stream.RFrameRate, "/")
		if len(parts) == 2 {
			num, _ := strconv.ParseFloat(parts[0], 64)
			den, _ := strconv.ParseFloat(parts[1], 64)
			if den > 0 {
				fps = num / den
			}
		}
	}

	duration := 0.0
	if probeResult.Format.Duration != "" {
		duration, _ = strconv.ParseFloat(probeResult.Format.Duration, 64)
	}

	return &VideoInfo{
		FPS:      fps,
		Width:    width,
		Height:   height,
		Duration: duration,
	}, nil
}

// FrameInfo 帧信息
type FrameInfo struct {
	Time  float64
	ImagePath string
}

// ExtractFrames 提取视频帧
func (v *VideoProcessor) ExtractFrames(outputDir string, sampleRate float64) ([]FrameInfo, error) {
	// 创建输出目录
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return nil, fmt.Errorf("创建输出目录失败: %v", err)
	}

	// 获取视频信息（用于验证视频文件）
	_, err := v.GetVideoInfo()
	if err != nil {
		return nil, err
	}

	// 计算帧间隔（秒）
	frameInterval := 1.0 / sampleRate

	// 使用 ffmpeg 提取帧
	// 使用 -vf fps 参数按采样率提取帧
	framePattern := filepath.Join(outputDir, "frame_%06d.jpg")

	cmd := exec.Command("ffmpeg",
		"-i", v.InputPath,
		"-vf", fmt.Sprintf("fps=%f", sampleRate),
		"-q:v", "2", // 高质量 JPEG
		"-y", // 覆盖已存在的文件
		framePattern,
	)

	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("提取视频帧失败: %v", err)
	}

	// 读取提取的帧文件
	frames := []FrameInfo{}
	frameIndex := 1
	for {
		framePath := filepath.Join(outputDir, fmt.Sprintf("frame_%06d.jpg", frameIndex))
		if _, err := os.Stat(framePath); os.IsNotExist(err) {
			break
		}

		time := float64(frameIndex-1) * frameInterval
		frames = append(frames, FrameInfo{
			Time:      time,
			ImagePath: framePath,
		})

		frameIndex++
	}

	return frames, nil
}

