package video

import (
	"fmt"
	"image"
	"os"

	_ "image/jpeg"
)

// SceneDetector 场景检测器
type SceneDetector struct {
	Threshold        float64
	MinSceneDuration float64
}

// NewSceneDetector 创建新的场景检测器
func NewSceneDetector(threshold, minSceneDuration float64) *SceneDetector {
	return &SceneDetector{
		Threshold:        threshold,
		MinSceneDuration: minSceneDuration,
	}
}

// DetectScenes 检测场景变化
func (s *SceneDetector) DetectScenes(frames []FrameInfo, fps float64) ([]float64, error) {
	if len(frames) < 2 {
		return []float64{0.0}, nil
	}

	sceneChanges := []float64{0.0} // 第一个场景从 0 开始

	for i := 1; i < len(frames); i++ {
		prevFrame := frames[i-1]
		currFrame := frames[i]

		// 计算帧差异
		diff, err := s.CalculateFrameDifference(prevFrame.ImagePath, currFrame.ImagePath)
		if err != nil {
			// 如果计算失败，跳过这一帧
			continue
		}

		// 如果差异超过阈值，检测到场景变化
		if diff > s.Threshold {
			// 检查距离上一个场景变化的时间间隔
			timeSinceLastScene := currFrame.Time - sceneChanges[len(sceneChanges)-1]
			if timeSinceLastScene >= s.MinSceneDuration {
				sceneChanges = append(sceneChanges, currFrame.Time)
			}
		}
	}

	return sceneChanges, nil
}

// CalculateFrameDifference 计算两帧之间的差异度
// 返回 0.0-1.0 之间的值，1.0 表示完全不同的帧
func (s *SceneDetector) CalculateFrameDifference(imgPath1, imgPath2 string) (float64, error) {
	// 加载图像
	img1, err := loadImage(imgPath1)
	if err != nil {
		return 0, fmt.Errorf("加载图像1失败: %v", err)
	}

	img2, err := loadImage(imgPath2)
	if err != nil {
		return 0, fmt.Errorf("加载图像2失败: %v", err)
	}

	// 转换为灰度图
	gray1 := toGrayScale(img1)
	gray2 := toGrayScale(img2)

	// 计算直方图差异
	histDiff := s.calculateHistogramDifference(gray1, gray2)

	// 计算像素差异
	pixelDiff := s.calculatePixelDifference(gray1, gray2)

	// 组合差异（直方图差异权重 60%，像素差异权重 40%）
	return histDiff*0.6 + pixelDiff*0.4, nil
}

// loadImage 加载图像
func loadImage(path string) (image.Image, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	img, _, err := image.Decode(file)
	return img, err
}

// toGrayScale 转换为灰度图
func toGrayScale(img image.Image) *image.Gray {
	bounds := img.Bounds()
	gray := image.NewGray(bounds)

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			gray.Set(x, y, img.At(x, y))
		}
	}

	return gray
}

// calculateHistogramDifference 计算直方图差异
func (s *SceneDetector) calculateHistogramDifference(img1, img2 *image.Gray) float64 {
	hist1 := make([]int, 256)
	hist2 := make([]int, 256)

	bounds := img1.Bounds()
	totalPixels := bounds.Dx() * bounds.Dy()

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			gray1 := img1.GrayAt(x, y)
			gray2 := img2.GrayAt(x, y)
			hist1[gray1.Y]++
			hist2[gray2.Y]++
		}
	}

	// 归一化并计算差异
	diff := 0.0
	for i := 0; i < 256; i++ {
		h1 := float64(hist1[i]) / float64(totalPixels)
		h2 := float64(hist2[i]) / float64(totalPixels)
		diff += abs(h1 - h2)
	}

	return diff / 2.0 // 归一化到 0-1
}

// calculatePixelDifference 计算像素差异
func (s *SceneDetector) calculatePixelDifference(img1, img2 *image.Gray) float64 {
	bounds := img1.Bounds()
	totalPixels := bounds.Dx() * bounds.Dy()

	diffSum := 0.0
	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			gray1 := img1.GrayAt(x, y)
			gray2 := img2.GrayAt(x, y)
			diffSum += abs(float64(gray1.Y) - float64(gray2.Y))
		}
	}

	return diffSum / (float64(totalPixels) * 255.0)
}
