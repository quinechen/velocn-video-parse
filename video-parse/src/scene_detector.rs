use image::{DynamicImage, GrayImage};
use anyhow::Result;

/// 场景检测器，用于检测视频中的镜头切换点
pub struct SceneDetector {
    /// 场景变化阈值（像素差异百分比）
    threshold: f64,
    /// 最小场景持续时间（秒）
    min_scene_duration: f64,
}

impl SceneDetector {
    pub fn new(threshold: f64, min_scene_duration: f64) -> Self {
        Self {
            threshold,
            min_scene_duration,
        }
    }

    /// 计算两帧之间的差异度
    /// 返回 0.0-1.0 之间的值，1.0 表示完全不同的帧
    pub fn calculate_frame_difference(&self, frame1: &DynamicImage, frame2: &DynamicImage) -> f64 {
        // 将图像转换为灰度图并缩放到较小尺寸以提高性能
        let gray1 = frame1.to_luma8();
        let gray2 = frame2.to_luma8();
        
        // 计算直方图差异
        let hist_diff = self.calculate_histogram_difference(&gray1, &gray2);
        
        // 计算像素差异
        let pixel_diff = self.calculate_pixel_difference(&gray1, &gray2);
        
        // 组合两种差异度量
        hist_diff * 0.6 + pixel_diff * 0.4
    }

    /// 计算直方图差异
    fn calculate_histogram_difference(&self, img1: &GrayImage, img2: &GrayImage) -> f64 {
        let mut hist1 = [0u32; 256];
        let mut hist2 = [0u32; 256];
        
        for pixel in img1.pixels() {
            hist1[pixel[0] as usize] += 1;
        }
        
        for pixel in img2.pixels() {
            hist2[pixel[0] as usize] += 1;
        }
        
        // 归一化直方图
        let total_pixels = img1.width() * img1.height() as u32;
        let mut diff = 0.0;
        
        for i in 0..256 {
            let h1 = hist1[i] as f64 / total_pixels as f64;
            let h2 = hist2[i] as f64 / total_pixels as f64;
            diff += (h1 - h2).abs();
        }
        
        diff / 2.0 // 归一化到 0-1
    }

    /// 计算像素差异
    fn calculate_pixel_difference(&self, img1: &GrayImage, img2: &GrayImage) -> f64 {
        if img1.width() != img2.width() || img1.height() != img2.height() {
            return 1.0;
        }
        
        let mut diff_sum = 0u64;
        let total_pixels = img1.width() * img1.height();
        
        for (p1, p2) in img1.pixels().zip(img2.pixels()) {
            diff_sum += (p1[0] as i32 - p2[0] as i32).abs() as u64;
        }
        
        diff_sum as f64 / (total_pixels as f64 * 255.0)
    }

    /// 检测场景变化点
    /// 返回场景变化的时间戳（秒）
    pub fn detect_scenes(
        &self,
        frames: &[(f64, DynamicImage)],
        fps: f64,
    ) -> Result<Vec<f64>> {
        if frames.len() < 2 {
            return Ok(vec![0.0]);
        }

        let mut scene_changes = vec![0.0]; // 第一个场景从 0 开始
        let _min_frame_interval = (self.min_scene_duration * fps) as usize;

        for i in 1..frames.len() {
            let prev_idx = i - 1;
            let diff = self.calculate_frame_difference(&frames[prev_idx].1, &frames[i].1);
            
            // 检查是否超过阈值且满足最小时间间隔
            if diff > self.threshold {
                let last_change = *scene_changes.last().unwrap();
                let time_since_last = frames[i].0 - last_change;
                
                if time_since_last >= self.min_scene_duration {
                    scene_changes.push(frames[i].0);
                }
            }
        }

        Ok(scene_changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;

    #[test]
    fn test_frame_difference() {
        let detector = SceneDetector::new(0.3, 1.0);
        
        // 创建两个不同的图像
        let img1 = DynamicImage::ImageLuma8(
            ImageBuffer::from_fn(100, 100, |_, _| image::Luma([100u8]))
        );
        let img2 = DynamicImage::ImageLuma8(
            ImageBuffer::from_fn(100, 100, |_, _| image::Luma([200u8]))
        );
        
        let diff = detector.calculate_frame_difference(&img1, &img2);
        assert!(diff > 0.0);
    }
}