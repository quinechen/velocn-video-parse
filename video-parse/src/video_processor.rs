use ffmpeg_next as ffmpeg;
use image::DynamicImage;
use anyhow::{Context, Result};
use std::path::Path;

/// 视频处理器，负责解码视频并提取帧
pub struct VideoProcessor {
    input_path: String,
}

impl VideoProcessor {
    pub fn new(input_path: impl AsRef<Path>) -> Result<Self> {
        ffmpeg::init().context("初始化 FFmpeg 失败")?;
        
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

    /// 提取视频帧
    /// 返回 (时间戳(秒), 图像) 的向量
    pub fn extract_frames(&self, sample_rate: Option<f64>) -> Result<Vec<(f64, DynamicImage)>> {
        let mut ictx = ffmpeg::format::input(&self.input_path)
            .context("无法打开视频文件")?;
        
        let video_stream_index = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .context("未找到视频流")?
            .index();
        
        let mut decoder_context = ffmpeg::codec::context::Context::from_parameters(
            ictx.stream(video_stream_index).unwrap().parameters()
        ).context("无法创建解码器上下文")?;
        
        let mut decoder = decoder_context.decoder()
            .video()
            .context("无法创建视频解码器")?;
        
        let mut scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        ).context("无法创建缩放器")?;
        
        let time_base = ictx.stream(video_stream_index).unwrap().time_base();
        let fps = ictx.stream(video_stream_index).unwrap().avg_frame_rate();
        let fps_value = if fps.denominator() > 0 {
            fps.numerator() as f64 / fps.denominator() as f64
        } else {
            30.0
        };
        
        // 采样率：如果指定了，使用指定的；否则使用 fps
        let sample_rate = sample_rate.unwrap_or(fps_value);
        let frame_interval = (fps_value / sample_rate) as usize;
        
        let mut frames = Vec::new();
        let mut frame_count = 0;
        
        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet).context("发送数据包失败")?;
                
                let mut decoded = ffmpeg::frame::Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    // 按采样率提取帧
                    if frame_count % frame_interval == 0 {
                        let time_seconds = decoded.timestamp()
                            .map(|ts| {
                                let tb_num = time_base.numerator() as f64;
                                let tb_den = time_base.denominator() as f64;
                                ts as f64 * tb_num / tb_den
                            })
                            .unwrap_or_else(|| frame_count as f64 / fps_value);
                        
                        // 转换为 RGB24
                        let mut rgb_frame = ffmpeg::frame::Video::empty();
                        scaler.run(&decoded, &mut rgb_frame)
                            .context("缩放帧失败")?;
                        
                        // 转换为 DynamicImage
                        let img = self.frame_to_image(&rgb_frame)?;
                        frames.push((time_seconds, img));
                    }
                    frame_count += 1;
                }
            }
        }
        
        // 处理剩余的帧
        decoder.send_eof().context("发送 EOF 失败")?;
        let mut decoded = ffmpeg::frame::Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
            if frame_count % frame_interval == 0 {
                let time_seconds = decoded.timestamp()
                    .map(|ts| {
                        let tb_num = time_base.numerator() as f64;
                        let tb_den = time_base.denominator() as f64;
                        ts as f64 * tb_num / tb_den
                    })
                    .unwrap_or_else(|| frame_count as f64 / fps_value);
                
                let mut rgb_frame = ffmpeg::frame::Video::empty();
                scaler.run(&decoded, &mut rgb_frame)
                    .context("缩放帧失败")?;
                
                let img = self.frame_to_image(&rgb_frame)?;
                frames.push((time_seconds, img));
            }
            frame_count += 1;
        }
        
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