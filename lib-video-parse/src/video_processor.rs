use ffmpeg_next as ffmpeg;
use image::DynamicImage;
use anyhow::{Context, Result};
use std::path::Path;
use std::time::Instant;
use std::io::{self, Write};

/// 视频处理器，负责解码视频并提取帧
pub struct VideoProcessor {
    input_path: String,
}

impl VideoProcessor {
    pub fn new(input_path: impl AsRef<Path>) -> Result<Self> {
        ffmpeg::init().context("初始化 FFmpeg 失败")?;
        
        // 设置 FFmpeg 日志级别为 ERROR，抑制警告和信息消息
        // 日志级别：panic, fatal, error, warning, info, verbose, debug, trace
        // 设置为 error 级别，只显示错误和致命错误
        unsafe {
            ffmpeg::sys::av_log_set_level(ffmpeg::sys::AV_LOG_ERROR as i32);
        }
        
        Ok(Self {
            input_path: input_path.as_ref().to_string_lossy().to_string(),
        })
    }

    /// 获取视频信息
    pub fn get_video_info(&self) -> Result<(f64, u32, u32)> {
        let ictx = ffmpeg::format::input(&self.input_path)
            .context("无法打开视频文件")?;
        
        let video_stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .context("未找到视频流")?;
        
        let decoder_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .context("无法创建解码器上下文")?;
        
        let decoder = decoder_context.decoder()
            .video()
            .context("无法创建视频解码器")?;
        
        let fps = video_stream.avg_frame_rate();
        let fps_value = if fps.denominator() > 0 {
            fps.numerator() as f64 / fps.denominator() as f64
        } else {
            30.0 // 默认值
        };
        
        Ok((fps_value, decoder.width(), decoder.height()))
    }

    /// 提取视频帧（优化版：使用 seek 跳转，大幅加速）
    /// 返回 (时间戳(秒), 图像) 的向量
    pub fn extract_frames(&self, sample_rate: Option<f64>) -> Result<Vec<(f64, DynamicImage)>> {
        // 先获取视频信息
        let (fps_value, _width, _height) = self.get_video_info()?;
        
        // 打开视频文件
        let mut ictx = ffmpeg::format::input(&self.input_path)
            .context("无法打开视频文件")?;
        
        let video_stream_index = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .context("未找到视频流")?
            .index();
        
        // 保存 video_stream_index 和 time_base，避免借用问题
        let time_base = ictx.stream(video_stream_index).unwrap().time_base();
        
        // 获取视频时长（秒）
        let duration = ictx.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64;
        
        // 采样率：如果指定了，使用指定的；否则使用 fps
        let sample_rate = sample_rate.unwrap_or(fps_value);
        
        // 计算需要提取的时间点
        let frame_interval = 1.0 / sample_rate; // 每帧之间的时间间隔（秒）
        let num_frames = (duration / frame_interval).ceil() as usize;
        
        // 创建解码器上下文的辅助函数（避免重复代码）
        let create_decoder_context = || -> Result<ffmpeg::codec::context::Context> {
            Ok(ffmpeg::codec::context::Context::from_parameters(
                ictx.stream(video_stream_index).unwrap().parameters()
            ).context("无法创建解码器上下文")?)
        };
        
        let decoder_context = create_decoder_context()?;
        
        // 禁用硬件加速，直接使用软件解码
        // 硬件加速在某些情况下不稳定，特别是使用 seek 的场景
        
        let mut decoder = decoder_context.decoder()
            .video()
            .context("无法创建视频解码器")?;
        
        // 创建缩放器（软件解码）
        let input_format = decoder.format();
        
        let mut scaler = ffmpeg::software::scaling::Context::get(
            input_format,
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        ).context("无法创建缩放器")?;
        
        let mut frames = Vec::new();
        
        // 进度跟踪
        let extract_start_time = Instant::now();
        let progress_interval = (num_frames / 20).max(1); // 每5%显示一次进度条
        let log_interval = (num_frames / 10).max(1); // 每10%输出一次详细日志
        let mut last_log_time = Instant::now();
        let mut last_log_frame = 0;
        
        println!("   📊 帧提取参数:");
        println!("      • 预计提取帧数: {} 帧", num_frames);
        println!("      • 视频时长: {:.2}秒", duration);
        println!("      • 采样间隔: {:.3}秒", frame_interval);
        println!("   🚀 开始提取视频帧...");
        
        // 对每个需要提取的时间点进行 seek 和解码
        for i in 0..num_frames {
            let target_time = i as f64 * frame_interval;
            
            // 如果超过视频时长，停止
            if target_time >= duration {
                break;
            }
            
            // 将时间转换为时间戳（基于 AV_TIME_BASE）
            let timestamp = (target_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;
            
            // Seek 到目标时间点（向后查找最近的 keyframe）
            unsafe {
                let ret = ffmpeg::sys::av_seek_frame(
                    ictx.as_mut_ptr(),
                    -1, // 对所有流 seek
                    timestamp,
                    ffmpeg::sys::AVSEEK_FLAG_BACKWARD as i32, // 向后查找最近的 keyframe
                );
                if ret < 0 {
                    // Seek 失败，跳过这个时间点
                    continue;
                }
            }
            
            // 刷新解码器缓冲区
            decoder.flush();
            
            // 读取并解码帧，直到找到目标时间点附近的帧
            let mut found_frame = false;
            let mut best_frame: Option<(f64, DynamicImage)> = None;
            let mut best_time_diff = f64::MAX;
            
            // 读取一些数据包来找到最接近目标时间的帧
            let mut packets_read = 0;
            const MAX_PACKETS_TO_READ: usize = 50; // 最多读取50个数据包来找到目标帧
            
            for (stream, packet) in ictx.packets() {
                if stream.index() != video_stream_index {
                    continue;
                }
                
                packets_read += 1;
                if packets_read > MAX_PACKETS_TO_READ {
                    break; // 避免无限循环
                }
                
                let send_result = decoder.send_packet(&packet);
                if send_result.is_err() {
                    // 发送数据包失败，跳过这个数据包
                    continue;
                }
                
                let mut decoded = ffmpeg::frame::Video::empty();
                
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let frame_time = decoded.timestamp()
                        .map(|ts| {
                            let tb_num = time_base.numerator() as f64;
                            let tb_den = time_base.denominator() as f64;
                            ts as f64 * tb_num / tb_den
                        })
                        .unwrap_or(0.0);
                    
                    let time_diff = (frame_time - target_time).abs();
                    
                    // 如果找到更接近目标时间的帧，保存它
                    if time_diff < best_time_diff {
                        best_time_diff = time_diff;
                        
                        // 如果时间差小于一个采样间隔的一半，认为找到了合适的帧
                        if time_diff <= frame_interval / 2.0 {
                            // 解码并转换帧（软件解码）
                            let mut rgb_frame = ffmpeg::frame::Video::empty();
                            if scaler.run(&decoded, &mut rgb_frame).is_ok() {
                                if let Ok(img) = self.frame_to_image(&rgb_frame) {
                                    best_frame = Some((frame_time, img));
                                    found_frame = true;
                                }
                            }
                        }
                    }
                    
                    // 如果已经超过目标时间太多，停止搜索
                    if frame_time > target_time + frame_interval {
                        break;
                    }
                }
                
                // 如果找到了合适的帧，停止读取更多数据包
                if found_frame {
                    break;
                }
            }
            
            // 如果找到了帧，添加到结果中
            if let Some((time, img)) = best_frame {
                frames.push((time, img));
            }
            
            // 显示进度条（每5%更新一次）
            if (i + 1) % progress_interval == 0 || i == num_frames - 1 {
                let progress = ((i + 1) as f64 / num_frames as f64 * 100.0) as u32;
                let elapsed = extract_start_time.elapsed();
                let elapsed_secs = elapsed.as_secs_f64();
                let fps = (i + 1) as f64 / elapsed_secs.max(0.001);
                let remaining_frames = num_frames - (i + 1);
                let estimated_remaining = if fps > 0.0 {
                    remaining_frames as f64 / fps
                } else {
                    0.0
                };
                
                // 计算进度条
                let bar_width = 30;
                let filled = (progress as f64 / 100.0 * bar_width as f64) as usize;
                let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);
                
                print!("\r   📈 进度: [{}] {}% ({}/{}) | 已用: {:.1}s | 速度: {:.1} 帧/s | 剩余: {:.1}s     ", 
                    bar, progress, i + 1, num_frames, elapsed_secs, fps, estimated_remaining);
                io::stdout().flush().ok();
            }
            
            // 输出详细日志（每10%输出一次）
            if (i + 1) % log_interval == 0 || i == num_frames - 1 {
                let progress = ((i + 1) as f64 / num_frames as f64 * 100.0) as u32;
                let elapsed = extract_start_time.elapsed();
                let elapsed_secs = elapsed.as_secs_f64();
                let avg_fps = (i + 1) as f64 / elapsed_secs.max(0.001);
                
                // 计算最近一段时间的速度
                let frames_since_last_log = (i + 1) - last_log_frame;
                let time_since_last_log = last_log_time.elapsed().as_secs_f64();
                let recent_fps = if time_since_last_log > 0.0 && frames_since_last_log > 0 {
                    frames_since_last_log as f64 / time_since_last_log
                } else {
                    avg_fps
                };
                
                // 输出详细日志（换行输出，不影响进度条）
                println!("\n   📝 进度日志: {}% ({}/{}) | 已用: {:.1}s | 平均速度: {:.1} 帧/s | 当前速度: {:.1} 帧/s", 
                    progress, i + 1, num_frames, elapsed_secs, avg_fps, recent_fps);
                
                last_log_frame = i + 1;
                last_log_time = Instant::now();
            }
        }
        
        println!(); // 换行，结束进度显示
        
        // 输出提取完成总结
        let total_elapsed = extract_start_time.elapsed();
        let total_secs = total_elapsed.as_secs_f64();
        let avg_fps = frames.len() as f64 / total_secs.max(0.001);
        println!("   ✅ 帧提取完成!");
        println!("      • 成功提取: {} 帧", frames.len());
        println!("      • 总耗时: {:.2}秒 ({:.0}ms)", total_secs, total_elapsed.as_millis());
        println!("      • 平均速度: {:.2} 帧/秒", avg_fps);
        println!("      • 平均耗时: {:.2}ms/帧", total_elapsed.as_millis() as f64 / frames.len().max(1) as f64);
        
        Ok(frames)
    }

    /// 将 FFmpeg 帧转换为 DynamicImage
    fn frame_to_image(&self, frame: &ffmpeg::frame::Video) -> Result<DynamicImage> {
        let width = frame.width();
        let height = frame.height();
        let data = frame.data(0);
        
        // RGB24 格式：每个像素 3 字节
        let mut img_buf = image::RgbImage::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * frame.stride(0) as u32) + (x * 3)) as usize;
                if idx + 2 < data.len() {
                    let r = data[idx];
                    let g = data[idx + 1];
                    let b = data[idx + 2];
                    img_buf.put_pixel(x, y, image::Rgb([r, g, b]));
                }
            }
        }
        
        Ok(DynamicImage::ImageRgb8(img_buf))
    }
    
}