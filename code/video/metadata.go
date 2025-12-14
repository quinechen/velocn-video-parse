package video

// SceneMetadata 单个场景的元数据
type SceneMetadata struct {
	SceneID      int     `json:"scene_id"`
	KeyframeFile string  `json:"keyframe_file"`
	StartTime    float64 `json:"start_time"`
	EndTime      float64 `json:"end_time"`
	Duration     float64 `json:"duration"`
}

// VideoMetadata 整个视频的元数据
type VideoMetadata struct {
	InputVideo    string          `json:"input_video"`
	TotalDuration float64         `json:"total_duration"`
	FPS           float64         `json:"fps"`
	Resolution    string          `json:"resolution"`
	SceneCount    int             `json:"scene_count"`
	AudioFile     string          `json:"audio_file"`
	Scenes        []SceneMetadata `json:"scenes"`
}

