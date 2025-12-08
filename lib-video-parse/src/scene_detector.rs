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

    /// 计算两帧之间的差异度（高级算法）
    /// 返回 0.0-1.0 之间的值，1.0 表示完全不同的帧
    /// 
    /// 使用多种方法组合：
    /// 1. 区域分割分析：区分中心区域（主体）和边缘区域（背景）
    /// 2. 边缘检测：使用Sobel算子检测边缘变化
    /// 3. HSV颜色空间分析：比较色调和饱和度的变化
    /// 4. 梯度分析：比较图像梯度分布
    /// 5. 传统方法：直方图和像素差异
    pub fn calculate_frame_difference(&self, frame1: &DynamicImage, frame2: &DynamicImage) -> f64 {
        // 将图像转换为灰度图
        let gray1 = frame1.to_luma8();
        let gray2 = frame2.to_luma8();
        
        // 1. 区域分割分析（权重 25%）
        // 区分中心区域（主体）和边缘区域（背景）
        let region_diff = self.calculate_region_difference(&gray1, &gray2);
        
        // 2. 边缘检测差异（权重 25%）
        // 使用Sobel算子检测边缘，比较边缘信息变化
        let edge_diff = self.calculate_edge_difference(&gray1, &gray2);
        
        // 3. HSV颜色空间分析（权重 20%）
        // 比较色调和饱和度的变化，对背景变化敏感
        let hsv_diff = self.calculate_hsv_difference(frame1, frame2);
        
        // 4. 梯度分析（权重 15%）
        // 比较图像梯度分布的变化
        let gradient_diff = self.calculate_gradient_difference(&gray1, &gray2);
        
        // 5. 传统方法（权重 15%）
        // 直方图差异和像素差异的组合
        let hist_diff = self.calculate_histogram_difference(&gray1, &gray2);
        let pixel_diff = self.calculate_pixel_difference(&gray1, &gray2);
        let traditional_diff = hist_diff * 0.6 + pixel_diff * 0.4;
        
        // 加权组合所有差异度量
        region_diff * 0.25 + edge_diff * 0.25 + hsv_diff * 0.20 + gradient_diff * 0.15 + traditional_diff * 0.15
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

    /// 计算区域差异（区分中心区域和边缘区域）
    /// 用于检测主体不变但背景变化的情况
    fn calculate_region_difference(&self, img1: &GrayImage, img2: &GrayImage) -> f64 {
        if img1.width() != img2.width() || img1.height() != img2.height() {
            return 1.0;
        }

        let width = img1.width();
        let height = img1.height();
        
        // 定义中心区域（占图像面积的40%）
        // 中心区域：从 30% 到 70% 的位置
        let center_x_start = (width as f64 * 0.3) as u32;
        let center_x_end = (width as f64 * 0.7) as u32;
        let center_y_start = (height as f64 * 0.3) as u32;
        let center_y_end = (height as f64 * 0.7) as u32;
        
        let mut center_diff_sum = 0u64;
        let mut edge_diff_sum = 0u64;
        let mut center_pixels = 0u32;
        let mut edge_pixels = 0u32;
        
        for y in 0..height {
            for x in 0..width {
                let p1 = img1.get_pixel(x, y)[0];
                let p2 = img2.get_pixel(x, y)[0];
                let diff = (p1 as i32 - p2 as i32).abs() as u64;
                
                // 判断是否在中心区域
                if x >= center_x_start && x < center_x_end && 
                   y >= center_y_start && y < center_y_end {
                    center_diff_sum += diff;
                    center_pixels += 1;
                } else {
                    edge_diff_sum += diff;
                    edge_pixels += 1;
                }
            }
        }
        
        // 计算中心区域和边缘区域的差异
        let center_diff = if center_pixels > 0 {
            center_diff_sum as f64 / (center_pixels as f64 * 255.0)
        } else {
            0.0
        };
        
        let edge_diff = if edge_pixels > 0 {
            edge_diff_sum as f64 / (edge_pixels as f64 * 255.0)
        } else {
            0.0
        };
        
        // 如果边缘区域变化大但中心区域变化小，说明可能是背景变化
        // 给予更高的权重
        if edge_diff > center_diff * 1.5 {
            // 背景变化明显，给予更高的差异分数
            edge_diff * 0.7 + center_diff * 0.3
        } else {
            // 整体变化
            (edge_diff + center_diff) / 2.0
        }
    }

    /// 计算边缘差异（使用Sobel算子）
    /// 边缘变化更能反映场景切换
    fn calculate_edge_difference(&self, img1: &GrayImage, img2: &GrayImage) -> f64 {
        if img1.width() != img2.width() || img1.height() != img2.height() {
            return 1.0;
        }

        let width = img1.width();
        let height = img1.height();
        
        // Sobel算子（简化版，只计算x和y方向的梯度）
        let sobel_x: [[i32; 3]; 3] = [
            [-1, 0, 1],
            [-2, 0, 2],
            [-1, 0, 1],
        ];
        
        let sobel_y: [[i32; 3]; 3] = [
            [-1, -2, -1],
            [ 0,  0,  0],
            [ 1,  2,  1],
        ];
        
        // 计算边缘强度
        let mut edge1_sum = 0u64;
        let mut edge2_sum = 0u64;
        let mut edge_diff_sum = 0u64;
        let mut edge_pixels = 0u32;
        
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                // 计算img1的边缘强度
                let mut gx1 = 0i32;
                let mut gy1 = 0i32;
                for ky in 0..3 {
                    for kx in 0..3 {
                        let px = (x + kx) as i32 - 1;
                        let py = (y + ky) as i32 - 1;
                        let pixel = img1.get_pixel(px as u32, py as u32)[0] as i32;
                        gx1 += pixel * sobel_x[ky as usize][kx as usize];
                        gy1 += pixel * sobel_y[ky as usize][kx as usize];
                    }
                }
                let edge1 = ((gx1 * gx1 + gy1 * gy1) as f64).sqrt() as u32;
                
                // 计算img2的边缘强度
                let mut gx2 = 0i32;
                let mut gy2 = 0i32;
                for ky in 0..3 {
                    for kx in 0..3 {
                        let px = (x + kx) as i32 - 1;
                        let py = (y + ky) as i32 - 1;
                        let pixel = img2.get_pixel(px as u32, py as u32)[0] as i32;
                        gx2 += pixel * sobel_x[ky as usize][kx as usize];
                        gy2 += pixel * sobel_y[ky as usize][kx as usize];
                    }
                }
                let edge2 = ((gx2 * gx2 + gy2 * gy2) as f64).sqrt() as u32;
                
                edge1_sum += edge1 as u64;
                edge2_sum += edge2 as u64;
                edge_diff_sum += (edge1 as i32 - edge2 as i32).abs() as u64;
                edge_pixels += 1;
            }
        }
        
        if edge_pixels == 0 {
            return 0.0;
        }
        
        // 归一化边缘差异
        let max_edge = (edge1_sum.max(edge2_sum) as f64 / edge_pixels as f64).max(1.0);
        edge_diff_sum as f64 / (edge_pixels as f64 * max_edge)
    }

    /// 计算HSV颜色空间差异
    /// 对色调和饱和度的变化敏感，能检测背景颜色变化
    fn calculate_hsv_difference(&self, img1: &DynamicImage, img2: &DynamicImage) -> f64 {
        let rgb1 = img1.to_rgb8();
        let rgb2 = img2.to_rgb8();
        
        if rgb1.width() != rgb2.width() || rgb1.height() != rgb2.height() {
            return 1.0;
        }
        
        let width = rgb1.width();
        let height = rgb1.height();
        let mut hue_diff_sum = 0.0;
        let mut sat_diff_sum = 0.0;
        let mut val_diff_sum = 0.0;
        let total_pixels = width * height;
        
        // 采样计算（每4个像素采样一次，提高性能）
        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let p1 = rgb1.get_pixel(x, y);
                let p2 = rgb2.get_pixel(x, y);
                
                // RGB转HSV
                let (h1, s1, v1) = self.rgb_to_hsv(p1[0], p1[1], p1[2]);
                let (h2, s2, v2) = self.rgb_to_hsv(p2[0], p2[1], p2[2]);
                
                // 计算色调差异（考虑色环的循环性）
                let mut hue_diff = (h1 - h2).abs();
                if hue_diff > 180.0 {
                    hue_diff = 360.0 - hue_diff;
                }
                hue_diff_sum += hue_diff / 180.0; // 归一化到0-1
                
                // 计算饱和度和亮度差异
                sat_diff_sum += (s1 - s2).abs();
                val_diff_sum += (v1 - v2).abs();
            }
        }
        
        let sample_count = ((width / 2) * (height / 2)) as f64;
        let hue_diff = hue_diff_sum / sample_count;
        let sat_diff = sat_diff_sum / sample_count;
        let val_diff = val_diff_sum / sample_count;
        
        // 色调和饱和度变化更能反映场景切换
        hue_diff * 0.5 + sat_diff * 0.3 + val_diff * 0.2
    }

    /// RGB转HSV辅助函数
    fn rgb_to_hsv(&self, r: u8, g: u8, b: u8) -> (f64, f64, f64) {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        
        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;
        
        // 色调
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };
        
        let h = if h < 0.0 { h + 360.0 } else { h };
        
        // 饱和度
        let s = if max == 0.0 { 0.0 } else { delta / max };
        
        // 亮度
        let v = max;
        
        (h, s, v)
    }

    /// 计算梯度差异
    /// 比较图像梯度分布的变化
    fn calculate_gradient_difference(&self, img1: &GrayImage, img2: &GrayImage) -> f64 {
        if img1.width() != img2.width() || img1.height() != img2.height() {
            return 1.0;
        }

        let width = img1.width();
        let height = img1.height();
        
        let mut gradient_diff_sum = 0u64;
        let mut gradient_pixels = 0u32;
        
        // 计算梯度（使用简单的差分）
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                // img1的梯度
                let gx1 = img1.get_pixel(x + 1, y)[0] as i32 - img1.get_pixel(x - 1, y)[0] as i32;
                let gy1 = img1.get_pixel(x, y + 1)[0] as i32 - img1.get_pixel(x, y - 1)[0] as i32;
                let grad1 = ((gx1 * gx1 + gy1 * gy1) as f64).sqrt() as u32;
                
                // img2的梯度
                let gx2 = img2.get_pixel(x + 1, y)[0] as i32 - img2.get_pixel(x - 1, y)[0] as i32;
                let gy2 = img2.get_pixel(x, y + 1)[0] as i32 - img2.get_pixel(x, y - 1)[0] as i32;
                let grad2 = ((gx2 * gx2 + gy2 * gy2) as f64).sqrt() as u32;
                
                gradient_diff_sum += (grad1 as i32 - grad2 as i32).abs() as u64;
                gradient_pixels += 1;
            }
        }
        
        if gradient_pixels == 0 {
            return 0.0;
        }
        
        // 归一化
        gradient_diff_sum as f64 / (gradient_pixels as f64 * 255.0 * 2.0_f64.sqrt())
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