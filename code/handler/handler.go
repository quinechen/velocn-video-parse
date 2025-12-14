package handler

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/google/uuid"
	"velocn-video-parse/config"
	"velocn-video-parse/oss"
	"velocn-video-parse/types"
	"velocn-video-parse/video"
)

// Handler HTTP 请求处理器
type Handler struct {
	configLoader *config.ConfigLoader
}

// NewHandler 创建新的处理器
func NewHandler() *Handler {
	return &Handler{
		configLoader: &config.ConfigLoader{},
	}
}

// isVideoFile 检查文件是否是视频文件（通过扩展名）
func isVideoFile(filename string) bool {
	ext := strings.ToLower(filepath.Ext(filename))
	videoExtensions := []string{
		".mp4", ".avi", ".mov", ".mkv", ".flv", ".wmv", ".webm",
		".m4v", ".3gp", ".ogv", ".ts", ".mts", ".m2ts",
		".vob", ".asf", ".rm", ".rmvb", ".divx", ".xvid",
	}
	for _, videoExt := range videoExtensions {
		if ext == videoExt {
			return true
		}
	}
	return false
}

// HandleRequest 处理 HTTP 请求
func (h *Handler) HandleRequest(method, path string, body []byte, headers map[string]string, queryParams map[string]string) (*types.HTTPTriggerResponse, error) {
	// 添加 CORS 头
	corsHeaders := map[string]string{
		"Access-Control-Allow-Origin":  "*",
		"Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS",
		"Access-Control-Allow-Headers": "Content-Type, Authorization, x-fc-request-id",
	}

	// 处理 CORS 预检请求
	if method == "OPTIONS" {
		return types.NewHTTPTriggerResponse(200).WithHeaders(corsHeaders), nil
	}

	// 路由处理
	switch {
	case path == "/" || path == "/health":
		return h.HandleHealthCheck(corsHeaders), nil
	case path == "/initialize":
		return h.handleInitialize(corsHeaders), nil
	case strings.HasPrefix(path, "/invoke"):
		return h.handleInvoke(method, body, corsHeaders), nil
	case strings.HasPrefix(path, "/process"):
		if path == "/process/direct" {
			return h.handleDirectProcess(body, corsHeaders), nil
		} else if path == "/process/query" {
			return h.handleProcessQuery(queryParams, corsHeaders), nil
		} else {
			return h.handleOSSEventRequest(body, corsHeaders), nil
		}
	default:
		return types.NewHTTPTriggerResponse(404).
			WithHeaders(corsHeaders).
			WithBody(fmt.Sprintf(`{"success":false,"message":"未找到路径: %s"}`, path)), nil
	}
}

// HandleHealthCheck 健康检查（公开方法）
func (h *Handler) HandleHealthCheck(corsHeaders map[string]string) *types.HTTPTriggerResponse {
	response := map[string]interface{}{
		"success": true,
		"message": "server is running",
		"data": map[string]interface{}{
			"status":    "ok",
			"timestamp": time.Now().Format(time.RFC3339),
		},
	}
	body, _ := json.Marshal(response)
	return types.NewHTTPTriggerResponse(200).
		WithHeaders(corsHeaders).
		WithBody(string(body))
}

// handleInitialize 函数计算初始化端点
func (h *Handler) handleInitialize(corsHeaders map[string]string) *types.HTTPTriggerResponse {
	response := map[string]interface{}{
		"success": true,
		"message": "FunctionCompute 初始化完成",
		"data": map[string]interface{}{
			"timestamp": time.Now().Format(time.RFC3339),
		},
	}
	body, _ := json.Marshal(response)
	return types.NewHTTPTriggerResponse(200).
		WithHeaders(corsHeaders).
		WithBody(string(body))
}

// handleInvoke 函数计算调用端点
func (h *Handler) handleInvoke(method string, body []byte, corsHeaders map[string]string) *types.HTTPTriggerResponse {
	// 尝试解析为 OSS 事件
	if len(body) > 0 {
		var ossEvent oss.OssEvent
		if err := json.Unmarshal(body, &ossEvent); err == nil && len(ossEvent.Events) > 0 {
			log.Printf("检测到 OSS 事件，开始处理...")
			// 异步处理
			go func() {
				_, _ = h.HandleOSSEvent(&ossEvent)
			}()
		}
	}

	response := map[string]interface{}{
		"success": true,
		"message": "请求已接收",
		"data": map[string]interface{}{
			"timestamp": time.Now().Format(time.RFC3339),
		},
	}
	responseBody, _ := json.Marshal(response)
	return types.NewHTTPTriggerResponse(200).
		WithHeaders(corsHeaders).
		WithBody(string(responseBody))
}

// handleOSSEventRequest 处理 OSS 事件请求
func (h *Handler) handleOSSEventRequest(body []byte, corsHeaders map[string]string) *types.HTTPTriggerResponse {
	if len(body) == 0 {
		response := types.ProcessResponse{
			Success: false,
			Message: "请求体为空",
		}
		responseBody, _ := json.Marshal(response)
		return types.NewHTTPTriggerResponse(400).
			WithHeaders(corsHeaders).
			WithBody(string(responseBody))
	}

	var ossEvent oss.OssEvent
	if err := json.Unmarshal(body, &ossEvent); err != nil {
		response := types.ProcessResponse{
			Success: false,
			Message: fmt.Sprintf("解析 JSON 失败: %v", err),
		}
		responseBody, _ := json.Marshal(response)
		return types.NewHTTPTriggerResponse(400).
			WithHeaders(corsHeaders).
			WithBody(string(responseBody))
	}

	// 处理 OSS 事件
	response, err := h.HandleOSSEvent(&ossEvent)
	if err != nil {
		errorResponse := types.ProcessResponse{
			Success: false,
			Message: err.Error(),
		}
		responseBody, _ := json.Marshal(errorResponse)
		return types.NewHTTPTriggerResponse(500).
			WithHeaders(corsHeaders).
			WithBody(string(responseBody))
	}

	responseBody, _ := json.Marshal(response)
	return types.NewHTTPTriggerResponse(200).
		WithHeaders(corsHeaders).
		WithBody(string(responseBody))
}

// HandleOSSEvent 处理 OSS 事件
func (h *Handler) HandleOSSEvent(event *oss.OssEvent) (*types.ProcessResponse, error) {
	log.Println("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log.Printf("[OSS Event] Received OSS event trigger request")
	log.Printf("Event count: %d", len(event.Events))

	if len(event.Events) == 0 {
		return &types.ProcessResponse{
			Success: false,
			Message: "事件列表为空",
		}, nil
	}

	eventItem := event.Events[0]

	if !strings.HasPrefix(eventItem.EventName, "ObjectCreated") {
		log.Printf("[OSS Event] Unsupported event type: %s, skipping processing", eventItem.EventName)
		return &types.ProcessResponse{
			Success: false,
			Message: fmt.Sprintf("不支持的事件类型: %s", eventItem.EventName),
		}, nil
	}

	// 加载扩展配置
	extendedConfig := h.configLoader.LoadExtendedConfig("")

	if extendedConfig.DebugMode {
		log.Printf("DEBUG mode enabled, skipping actual processing")
		return &types.ProcessResponse{
			Success: true,
			Message: "DEBUG 模式：事件接收成功",
		}, nil
	}

	bucket := eventItem.Oss.Bucket.Name
	objectKey := eventItem.Oss.Object.Key
	region := eventItem.Region

	// 检查文件类型，只处理视频文件
	if !isVideoFile(objectKey) {
		log.Printf("[OSS Event] 跳过非视频文件: %s", objectKey)
		return &types.ProcessResponse{
			Success: true,
			Message: fmt.Sprintf("跳过非视频文件: %s", objectKey),
		}, nil
	}

	log.Printf("[OSS Event] 检测到视频文件，开始处理: %s", objectKey)

	// 创建临时目录
	requestID := os.Getenv("FC_REQUEST_ID")
	if requestID == "" {
		requestID = fmt.Sprintf("%d_%s", time.Now().Unix(), uuid.New().String())
	}

	tempDir := ""
	if extendedConfig.OutputPath != "" {
		tempDir = filepath.Join(extendedConfig.OutputPath, requestID)
	} else {
		tempDir = filepath.Join(os.TempDir(), "video-parse", requestID)
	}

	if err := os.MkdirAll(tempDir, 0755); err != nil {
		return nil, fmt.Errorf("创建临时目录失败: %v", err)
	}

	// 步骤 1: 下载视频文件
	log.Printf("📥 [OSS Event] 步骤 1/3: 开始下载视频文件...")
	log.Printf("  • Bucket: %s", bucket)
	log.Printf("  • Object Key: %s", objectKey)
	log.Printf("  • Region: %s", region)
	log.Printf("  • 文件大小: %d 字节", eventItem.Oss.Object.Size)

	ossClient, err := oss.NewOssClient()
	if err != nil {
		return nil, fmt.Errorf("创建 OSS 客户端失败: %v", err)
	}

	videoPathBuf := filepath.Base(objectKey)
	videoFilename := videoPathBuf
	if videoFilename == "" || videoFilename == "." {
		videoFilename = "video.mp4"
	}

	videoPath := filepath.Join(tempDir, videoFilename)
	endpoint := fmt.Sprintf("oss-%s-internal.aliyuncs.com", region)

	log.Printf("  • 下载到本地路径: %s", videoPath)
	downloadedPath, err := ossClient.DownloadFile(bucket, objectKey, endpoint, videoPath)
	if err != nil {
		return nil, fmt.Errorf("下载文件失败: %v", err)
	}
	log.Printf("✅ [OSS Event] 步骤 1/3: 视频文件下载完成")
	log.Printf("  • 本地文件路径: %s", downloadedPath)

	// 步骤 2: 创建输出目录
	log.Printf("📁 [OSS Event] 步骤 2/3: 准备输出目录...")
	outputDir := filepath.Join(tempDir, "output")
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return nil, fmt.Errorf("创建输出目录失败: %v", err)
	}
	log.Printf("✅ [OSS Event] 步骤 2/3: 输出目录准备完成")
	log.Printf("  • 输出目录: %s", outputDir)

	// 步骤 3: 处理视频（拉片）
	log.Printf("🎬 [OSS Event] 步骤 3/3: 开始处理视频（拉片）...")
	log.Printf("  • 处理配置:")
	log.Printf("    - 阈值: %.2f", extendedConfig.Process.Threshold)
	log.Printf("    - 最小场景时长: %.2f 秒", extendedConfig.Process.MinSceneDuration)
	log.Printf("    - 采样率: %.2f fps", extendedConfig.Process.SampleRate)

	cfg := extendedConfig.Process
	processResult, err := video.ProcessVideo(downloadedPath, outputDir, cfg)
	if err != nil {
		return nil, fmt.Errorf("处理视频失败: %v", err)
	}

	log.Printf("✅ [OSS Event] 步骤 3/3: 视频处理完成")
	log.Printf("  • 检测到场景数: %d 个", processResult.Metadata.SceneCount)
	log.Printf("  • 提取关键帧数: %d 个", len(processResult.KeyframeFiles))
	if processResult.AudioFile != "" {
		log.Printf("  • 音频文件: %s", processResult.AudioFile)
	}

	// 构建响应
	log.Printf("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log.Printf("✅ [OSS Event] 所有处理步骤完成")
	return &types.ProcessResponse{
		Success: true,
		Message: fmt.Sprintf("成功处理视频，检测到 %d 个场景，提取 %d 个关键帧",
			processResult.Metadata.SceneCount,
			len(processResult.KeyframeFiles)),
		Result: &types.ProcessResult{
			VideoFile:    downloadedPath,
			OutputDir:    outputDir,
			SceneCount:   processResult.Metadata.SceneCount,
			Keyframes:    processResult.KeyframeFiles,
			AudioFile:    processResult.AudioFile,
			MetadataFile: "metadata.json",
		},
	}, nil
}

// handleDirectProcess 直接处理请求
func (h *Handler) handleDirectProcess(body []byte, corsHeaders map[string]string) *types.HTTPTriggerResponse {
	var request struct {
		Input            string   `json:"input"`
		Output           string   `json:"output"`
		Threshold        *float64 `json:"threshold"`
		MinSceneDuration *float64 `json:"min_scene_duration"`
		SampleRate       *float64 `json:"sample_rate"`
		IsOSSPath        *bool    `json:"is_oss_path"`
		OSSBucket        string   `json:"oss_bucket"`
		OSSRegion        string   `json:"oss_region"`
	}

	if err := json.Unmarshal(body, &request); err != nil {
		response := types.ProcessResponse{
			Success: false,
			Message: fmt.Sprintf("解析 JSON 失败: %v", err),
		}
		responseBody, _ := json.Marshal(response)
		return types.NewHTTPTriggerResponse(400).
			WithHeaders(corsHeaders).
			WithBody(string(responseBody))
	}

	// 这里简化处理，实际应该调用完整的处理逻辑
	response := types.ProcessResponse{
		Success: true,
		Message: "处理请求已接收",
	}
	responseBody, _ := json.Marshal(response)
	return types.NewHTTPTriggerResponse(200).
		WithHeaders(corsHeaders).
		WithBody(string(responseBody))
}

// handleProcessQuery 通过查询参数处理视频
func (h *Handler) handleProcessQuery(queryParams map[string]string, corsHeaders map[string]string) *types.HTTPTriggerResponse {
	input := queryParams["input"]
	if input == "" {
		response := types.ProcessResponse{
			Success: false,
			Message: "缺少 input 参数",
		}
		responseBody, _ := json.Marshal(response)
		return types.NewHTTPTriggerResponse(400).
			WithHeaders(corsHeaders).
			WithBody(string(responseBody))
	}

	// 这里简化处理，实际应该调用完整的处理逻辑
	response := types.ProcessResponse{
		Success: true,
		Message: "处理请求已接收",
	}
	responseBody, _ := json.Marshal(response)
	return types.NewHTTPTriggerResponse(200).
		WithHeaders(corsHeaders).
		WithBody(string(responseBody))
}
