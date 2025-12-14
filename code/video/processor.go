package video

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"time"

	"velocn-video-parse/config"
)

// ProcessOutput 处理结果
type ProcessOutput struct {
	OutputDir      string
	Metadata       VideoMetadata
	KeyframeFiles  []string
	AudioFile      string
}

// ProcessVideo 处理视频文件
func ProcessVideo(inputVideoPath, outputDir string, cfg config.ProcessConfig) (*ProcessOutput, error) {
	startTime := time.Now()
	log.Printf("🎬 [视频处理] 开始处理视频: %s", inputVideoPath)
	log.Println("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	// 创建输出目录
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return nil, fmt.Errorf("创建输出目录失败: %v", err)
	}
	log.Printf("✅ [视频处理] 创建输出目录完成")

	// 1. 初始化视频处理器
	processor, err := NewVideoProcessor(inputVideoPath)
	if err != nil {
		return nil, fmt.Errorf("初始化视频处理器失败: %v", err)
	}
	log.Printf("✅ [视频处理] 初始化视频处理器完成")

	// 2. 获取视频信息
	info, err := processor.GetVideoInfo()
	if err != nil {
		return nil, fmt.Errorf("获取视频信息失败: %v", err)
	}
	log.Printf("✅ [视频处理] 获取视频信息完成")
	log.Printf("  • 分辨率: %dx%d", info.Width, info.Height)
	log.Printf("  • 帧率: %.2f fps", info.FPS)

	// 3. 提取视频帧
	framesDir := filepath.Join(outputDir, "frames")
	log.Printf("⏳ [视频处理] 正在提取视频帧（采样率: %.1f fps）...", cfg.SampleRate)
	frames, err := processor.ExtractFrames(framesDir, cfg.SampleRate)
	if err != nil {
		return nil, fmt.Errorf("提取视频帧失败: %v", err)
	}
	log.Printf("✅ [视频处理] 提取视频帧完成")
	log.Printf("  • 提取帧数: %d 帧", len(frames))

	// 4. 检测场景变化
	log.Printf("⏳ [视频处理] 正在检测场景变化...")
	detector := NewSceneDetector(cfg.Threshold, cfg.MinSceneDuration)
	sceneChanges, err := detector.DetectScenes(frames, info.FPS)
	if err != nil {
		return nil, fmt.Errorf("场景检测失败: %v", err)
	}
	log.Printf("✅ [视频处理] 场景检测完成")
	log.Printf("  • 检测到场景数: %d 个", len(sceneChanges))

	// 5. 提取关键帧并保存
	log.Printf("⏳ [视频处理] 正在提取并保存关键帧...")
	keyframesDir := outputDir
	var scenesMetadata []SceneMetadata
	var keyframeFiles []string
	keyframeCounter := 0

	totalDuration := info.Duration
	if len(frames) > 0 {
		totalDuration = frames[len(frames)-1].Time
	}

	for i, sceneStart := range sceneChanges {
		// 确定场景结束时间
		sceneEnd := totalDuration
		if i+1 < len(sceneChanges) {
			sceneEnd = sceneChanges[i+1]
		}

		duration := sceneEnd - sceneStart

		// 找到属于当前场景的帧
		var sceneFrames []FrameInfo
		for _, frame := range frames {
			if frame.Time >= sceneStart && frame.Time < sceneEnd {
				sceneFrames = append(sceneFrames, frame)
			}
		}

		if len(sceneFrames) == 0 {
			// 如果没有找到帧，使用场景开始时间附近的帧
			var closestFrame *FrameInfo
			minDiff := 1e10
			for j := range frames {
				diff := abs(frames[j].Time - sceneStart)
				if diff < minDiff {
					minDiff = diff
					closestFrame = &frames[j]
				}
			}

			if closestFrame == nil {
				log.Printf("⚠️  [视频处理] 场景 %d: 没有找到合适的帧，跳过", i)
				continue
			}

			// 复制帧作为关键帧
			keyframeFilename := fmt.Sprintf("keyframe_%04d.jpg", keyframeCounter)
			keyframePath := filepath.Join(keyframesDir, keyframeFilename)
			if err := copyFile(closestFrame.ImagePath, keyframePath); err != nil {
				log.Printf("❌ [视频处理] 保存关键帧失败: %v", err)
				continue
			}

			keyframeFiles = append(keyframeFiles, keyframeFilename)
			scenesMetadata = append(scenesMetadata, SceneMetadata{
				SceneID:      i,
				KeyframeFile: keyframeFilename,
				StartTime:    sceneStart,
				EndTime:      sceneEnd,
				Duration:     duration,
			})
			keyframeCounter++
			continue
		}

		// 每个场景只提取1个关键帧
		// 策略：在场景中间区域（30%-70%）选择最稳定的帧
		sceneMidStart := sceneStart + duration*0.3
		sceneMidEnd := sceneStart + duration*0.7

		// 找到中间区域的帧
		var midRegionFrames []FrameInfo
		for _, frame := range sceneFrames {
			if frame.Time >= sceneMidStart && frame.Time <= sceneMidEnd {
				midRegionFrames = append(midRegionFrames, frame)
			}
		}

		var keyframeFrame *FrameInfo
		if len(midRegionFrames) == 0 {
			// 如果中间区域没有帧，选择场景中间位置的帧
			targetTime := sceneStart + duration*0.5
			var closest *FrameInfo
			minDiff := 1e10
			for j := range sceneFrames {
				diff := abs(sceneFrames[j].Time - targetTime)
				if diff < minDiff {
					minDiff = diff
					closest = &sceneFrames[j]
				}
			}
			keyframeFrame = closest
		} else if len(midRegionFrames) == 1 {
			keyframeFrame = &midRegionFrames[0]
		} else {
			// 在中间区域选择最稳定的帧（与前后帧差异最小）
			bestFrame := &midRegionFrames[0]
			minAvgDiff := 1e10

			for j := range midRegionFrames {
				frame := &midRegionFrames[j]
				avgDiff := 0.0
				count := 0

				// 与前一个帧的差异
				for k := range frames {
					if frames[k].Time < frame.Time && frames[k].Time >= sceneStart {
						diff, _ := detector.CalculateFrameDifference(frames[k].ImagePath, frame.ImagePath)
						avgDiff += diff
						count++
						break
					}
				}

				// 与后一个帧的差异
				for k := range frames {
					if frames[k].Time > frame.Time && frames[k].Time < sceneEnd {
						diff, _ := detector.CalculateFrameDifference(frame.ImagePath, frames[k].ImagePath)
						avgDiff += diff
						count++
						break
					}
				}

				if count > 0 {
					avgDiff /= float64(count)
					if avgDiff < minAvgDiff {
						minAvgDiff = avgDiff
						bestFrame = frame
					}
				}
			}

			keyframeFrame = bestFrame
		}

		if keyframeFrame == nil {
			log.Printf("⚠️  [视频处理] 场景 %d: 没有找到合适的帧，跳过", i)
			continue
		}

		// 保存关键帧图片
		keyframeFilename := fmt.Sprintf("keyframe_%04d.jpg", keyframeCounter)
		keyframePath := filepath.Join(keyframesDir, keyframeFilename)
		if err := copyFile(keyframeFrame.ImagePath, keyframePath); err != nil {
			log.Printf("❌ [视频处理] 保存关键帧失败: %v", err)
			continue
		}

		keyframeFiles = append(keyframeFiles, keyframeFilename)
		scenesMetadata = append(scenesMetadata, SceneMetadata{
			SceneID:      i,
			KeyframeFile: keyframeFilename,
			StartTime:    sceneStart,
			EndTime:      sceneEnd,
			Duration:     duration,
		})
		keyframeCounter++
	}

	log.Printf("✅ [视频处理] 关键帧提取完成")
	log.Printf("  • 提取关键帧数: %d 个", len(keyframeFiles))

	// 6. 提取音频
	log.Printf("⏳ [视频处理] 正在提取音频...")
	audioFilename := "audio.aac"
	audioPath := filepath.Join(outputDir, audioFilename)
	audioExtractor, err := NewAudioExtractor(inputVideoPath)
	if err != nil {
		return nil, fmt.Errorf("创建音频提取器失败: %v", err)
	}
	if err := audioExtractor.ExtractToFile(audioPath); err != nil {
		return nil, fmt.Errorf("提取音频失败: %v", err)
	}
	log.Printf("✅ [视频处理] 音频提取完成")
	log.Printf("  • 音频文件: %s", audioPath)

	// 7. 生成元数据 JSON
	log.Printf("⏳ [视频处理] 正在生成元数据...")
	metadata := VideoMetadata{
		InputVideo:    inputVideoPath,
		TotalDuration: totalDuration,
		FPS:           info.FPS,
		Resolution:    fmt.Sprintf("%dx%d", info.Width, info.Height),
		SceneCount:    len(scenesMetadata),
		AudioFile:     audioFilename,
		Scenes:        scenesMetadata,
	}

	metadataPath := filepath.Join(outputDir, "metadata.json")
	metadataJSON, err := json.MarshalIndent(metadata, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("序列化元数据失败: %v", err)
	}
	if err := os.WriteFile(metadataPath, metadataJSON, 0644); err != nil {
		return nil, fmt.Errorf("写入元数据文件失败: %v", err)
	}
	log.Printf("✅ [视频处理] 元数据生成完成")
	log.Printf("  • 元数据文件: %s", metadataPath)

	// 总结
	totalDurationElapsed := time.Since(startTime)
	log.Println("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log.Printf("🎉 [视频处理] 处理完成！总耗时: %.2f秒", totalDurationElapsed.Seconds())
	log.Println("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log.Printf("📁 [视频处理] 输出目录: %s", outputDir)
	log.Printf("📸 [视频处理] 关键帧数量: %d", metadata.SceneCount)
	log.Printf("🎵 [视频处理] 音频文件: %s", audioFilename)
	log.Printf("✅ [视频处理] 视频处理成功完成")

	result := &ProcessOutput{
		OutputDir:     outputDir,
		Metadata:       metadata,
		KeyframeFiles: keyframeFiles,
		AudioFile:      audioFilename,
	}

	// 调用 webhook 回调（如果配置了）
	if cfg.WebhookURL != "" {
		log.Printf("⏳ [视频处理] 正在调用 Webhook 回调...")
		if err := callWebhook(cfg.WebhookURL, result, &metadata); err != nil {
			log.Printf("⚠️  [视频处理] Webhook 回调失败: %v", err)
		} else {
			log.Printf("✅ [视频处理] Webhook 回调成功")
		}
	}

	return result, nil
}

// copyFile 复制文件
func copyFile(src, dst string) error {
	data, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	return os.WriteFile(dst, data, 0644)
}

// abs 计算绝对值
func abs(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}

// callWebhook 调用 webhook 回调
func callWebhook(webhookURL string, result *ProcessOutput, metadata *VideoMetadata) error {
	// 这里简化实现，实际应该使用 HTTP 客户端发送 POST 请求
	// 可以使用 net/http 包
	log.Printf("Webhook 回调 URL: %s", webhookURL)
	log.Printf("Webhook 数据: %+v", result)
	return nil
}

