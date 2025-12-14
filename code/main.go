package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strconv"

	"github.com/gin-gonic/gin"
	"velocn-video-parse/handler"
	"velocn-video-parse/oss"
	"velocn-video-parse/types"
)

func main() {
	// 检测运行环境
	port := 9000
	if portStr := os.Getenv("FC_SERVER_PORT"); portStr != "" {
		if p, err := strconv.Atoi(portStr); err == nil {
			port = p
		}
	}

	// 设置 Gin 模式
	if os.Getenv("DEBUG") == "true" {
		gin.SetMode(gin.DebugMode)
	} else {
		gin.SetMode(gin.ReleaseMode)
	}

	// 创建 Gin 路由器
	router := gin.Default()

	// 添加 CORS 中间件
	router.Use(corsMiddleware())

	// Custom Runtime 协议端点
	router.POST("/initialize", handleInitialize)
	router.Any("/invoke", handleInvoke) // 支持所有 HTTP 方法（包括 POST）

	// 业务端点
	router.GET("/", handleRoot)
	router.GET("/health", handleHealth)
	router.Any("/process", handleProcess) // 支持所有 HTTP 方法（包括 POST）
	router.POST("/process/direct", handleDirectProcess)
	router.GET("/process/query", handleProcessQuery)

	// 启动服务器
	log.Printf("启动 HTTP 服务器，监听端口 %d", port)
	log.Printf("可用端点:")
	log.Printf("  - POST /initialize - Custom Runtime 初始化端点")
	log.Printf("  - POST /invoke - Custom Runtime 调用端点")
	log.Printf("  - GET  /health - 健康检查")
	log.Printf("  - POST /process - OSS 事件处理")
	log.Printf("  - POST /process/direct - 直接处理视频")
	log.Printf("  - GET  /process/query - 查询参数处理")

	if err := router.Run(fmt.Sprintf("0.0.0.0:%d", port)); err != nil {
		log.Fatalf("HTTP 服务器启动失败: %v", err)
	}
}

// corsMiddleware CORS 中间件
func corsMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization, x-fc-request-id")

		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(http.StatusOK)
			return
		}

		c.Next()
	}
}

// handleInitialize 处理 Custom Runtime /initialize 端点
func handleInitialize(c *gin.Context) {
	requestID := c.GetHeader("x-fc-request-id")
	log.Printf("FC Initialize Start RequestId: %s", requestID)

	c.String(http.StatusOK, "OK")

	log.Printf("FC Initialize End RequestId: %s", requestID)
}

// handleInvoke 处理 Custom Runtime /invoke 端点
func handleInvoke(c *gin.Context) {
	requestID := c.GetHeader("x-fc-request-id")
	log.Printf("FC Invoke Start RequestId: %s", requestID)

	// 读取请求体
	body, err := c.GetRawData()
	if err != nil {
		log.Printf("读取请求体失败: %v", err)
		c.JSON(http.StatusBadRequest, gin.H{
			"success": false,
			"message": fmt.Sprintf("读取请求体失败: %v", err),
		})
		return
	}

	log.Printf("请求体大小: %d 字节", len(body))
	if len(body) > 0 && len(body) < 2000 {
		log.Printf("请求体内容: %s", string(body))
	}

	// 创建 Handler
	h := handler.NewHandler()

	// 优先检查是否是 OSS 事件（直接解析为 OssEvent）
	log.Printf("开始检测事件类型...")
	var ossEvent oss.OssEvent
	ossCheckErr := json.Unmarshal(body, &ossEvent)
	log.Printf("OSS 事件检测: err=%v, events数量=%d", ossCheckErr, len(ossEvent.Events))

	if ossCheckErr == nil && len(ossEvent.Events) > 0 {
		// 成功解析为 OSS 事件
		log.Printf("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		log.Printf("检测到 OSS 事件，准备异步处理...")

		log.Printf("OSS 事件解析成功，事件数量: %d", len(ossEvent.Events))
		if len(ossEvent.Events) > 0 {
			eventItem := ossEvent.Events[0]
			log.Printf("事件类型: %s", eventItem.EventName)
			log.Printf("Bucket: %s", eventItem.Oss.Bucket.Name)
			log.Printf("Object Key: %s", eventItem.Oss.Object.Key)
			log.Printf("Region: %s", eventItem.Region)
		}

		// 先立即返回成功响应，避免函数超时
		response := types.ProcessResponse{
			Success: true,
			Message: "OSS 事件已接收，正在后台异步处理",
		}
		c.JSON(http.StatusOK, response)

		// 在后台异步处理 OSS 事件
		go func() {
			log.Printf("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
			log.Printf("开始异步处理 OSS 事件...")

			// 创建新的 Handler 实例用于异步处理
			asyncHandler := handler.NewHandler()

			// 处理 OSS 事件
			asyncResponse, err := asyncHandler.HandleOSSEvent(&ossEvent)
			if err != nil {
				log.Printf("异步处理 OSS 事件失败: %v", err)
				log.Printf("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
				return
			}

			log.Printf("异步处理 OSS 事件完成")
			if asyncResponse.Result != nil {
				log.Printf("处理结果: 场景数=%d, 关键帧数=%d",
					asyncResponse.Result.SceneCount,
					len(asyncResponse.Result.Keyframes))
			}
			log.Printf("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		}()

		log.Printf("已启动异步处理任务，立即返回响应")
		return
	}

	// 尝试解析为 HTTPTriggerEvent（JSON 格式）
	log.Printf("尝试解析为 HTTPTriggerEvent...")
	var httpEvent types.HTTPTriggerEvent
	httpCheckErr := json.Unmarshal(body, &httpEvent)
	log.Printf("HTTPTriggerEvent 检测: err=%v, RequestContext=%v", httpCheckErr, httpEvent.RequestContext != nil)

	if httpCheckErr == nil && httpEvent.RequestContext != nil {
		// 成功解析为 HTTPTriggerEvent
		log.Printf("解析为 HTTPTriggerEvent")

		// 获取路径和方法
		path := ""
		if httpEvent.RequestContext.Http.Path != "" {
			path = httpEvent.RequestContext.Http.Path
		} else if httpEvent.RawPath != nil {
			path = *httpEvent.RawPath
		}

		method := httpEvent.RequestContext.Http.Method
		if method == "" {
			method = "GET"
		}

		// 获取请求体
		var bodyBytes []byte
		if httpEvent.Body != nil {
			bodyBytes = []byte(*httpEvent.Body)
		}

		// 处理请求
		response, err := h.HandleRequest(method, path, bodyBytes, httpEvent.Headers, httpEvent.QueryParameters)
		if err != nil {
			log.Printf("处理请求失败: %v", err)
			c.JSON(http.StatusInternalServerError, gin.H{
				"success": false,
				"message": err.Error(),
			})
			return
		}

		// 设置响应头
		for k, v := range response.Headers {
			c.Header(k, v)
		}
		c.Data(response.StatusCode, "application/json", []byte(response.Body))
		return
	}

	// 其他原始事件
	log.Printf("无法识别的事件类型（既不是 OSS 事件也不是 HTTPTriggerEvent），返回原始内容")
	log.Printf("OSS 检测错误: %v", ossCheckErr)
	log.Printf("HTTPTriggerEvent 检测错误: %v", httpCheckErr)
	c.String(http.StatusOK, fmt.Sprintf("收到事件: %s", string(body)))

	log.Printf("FC Invoke End RequestId: %s", requestID)
}

// handleRoot 处理根端点
func handleRoot(c *gin.Context) {
	if c.Request.Method == "GET" {
		handleHealth(c)
		return
	}
	// 其他方法转发到 invoke
	handleInvoke(c)
}

// handleHealth 处理健康检查
func handleHealth(c *gin.Context) {
	h := handler.NewHandler()
	response := h.HandleHealthCheck(map[string]string{})
	c.Data(response.StatusCode, "application/json", []byte(response.Body))
}

// handleProcess 处理 OSS 事件（通过 /process 端点）
func handleProcess(c *gin.Context) {
	h := handler.NewHandler()

	// 读取请求体
	body, err := c.GetRawData()
	if err != nil {
		c.JSON(http.StatusBadRequest, types.ProcessResponse{
			Success: false,
			Message: fmt.Sprintf("读取请求体失败: %v", err),
		})
		return
	}

	if len(body) == 0 {
		c.JSON(http.StatusBadRequest, types.ProcessResponse{
			Success: false,
			Message: "请求体为空",
		})
		return
	}

	// 解析 OSS 事件
	var ossEvent oss.OssEvent
	if err := json.Unmarshal(body, &ossEvent); err != nil {
		c.JSON(http.StatusBadRequest, types.ProcessResponse{
			Success: false,
			Message: fmt.Sprintf("解析 JSON 失败: %v", err),
		})
		return
	}

	// 处理 OSS 事件
	response, err := h.HandleOSSEvent(&ossEvent)
	if err != nil {
		c.JSON(http.StatusInternalServerError, types.ProcessResponse{
			Success: false,
			Message: err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, response)
}

// handleDirectProcess 处理直接处理请求
func handleDirectProcess(c *gin.Context) {
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

	if err := c.ShouldBindJSON(&request); err != nil {
		c.JSON(http.StatusBadRequest, types.ProcessResponse{
			Success: false,
			Message: fmt.Sprintf("解析 JSON 失败: %v", err),
		})
		return
	}

	// 这里简化处理，实际应该调用完整的处理逻辑
	c.JSON(http.StatusOK, types.ProcessResponse{
		Success: true,
		Message: "处理请求已接收",
	})
}

// handleProcessQuery 通过查询参数处理视频
func handleProcessQuery(c *gin.Context) {
	input := c.Query("input")
	if input == "" {
		c.JSON(http.StatusBadRequest, types.ProcessResponse{
			Success: false,
			Message: "缺少 input 参数",
		})
		return
	}

	// 这里简化处理，实际应该调用完整的处理逻辑
	c.JSON(http.StatusOK, types.ProcessResponse{
		Success: true,
		Message: "处理请求已接收",
	})
}
