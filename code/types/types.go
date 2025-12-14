package types

// HTTPTriggerEvent HTTP Trigger Request Event
type HTTPTriggerEvent struct {
	Version         *string           `json:"version"`
	RawPath         *string           `json:"rawPath"`
	Headers         map[string]string `json:"headers"`
	QueryParameters map[string]string `json:"queryParameters"`
	Body            *string           `json:"body"`
	IsBase64Encoded *bool             `json:"isBase64Encoded"`
	RequestContext  *struct {
		AccountId    string `json:"accountId"`
		DomainName   string `json:"domainName"`
		DomainPrefix string `json:"domainPrefix"`
		RequestId    string `json:"requestId"`
		Time         string `json:"time"`
		TimeEpoch    string `json:"timeEpoch"`
		Http         struct {
			Method    string `json:"method"`
			Path      string `json:"path"`
			Protocol  string `json:"protocol"`
			SourceIp  string `json:"sourceIp"`
			UserAgent string `json:"userAgent"`
		} `json:"http"`
	} `json:"requestContext"`
}

// HTTPTriggerResponse HTTP Trigger Response struct
type HTTPTriggerResponse struct {
	StatusCode      int               `json:"statusCode"`
	Headers         map[string]string `json:"headers,omitempty"`
	IsBase64Encoded bool              `json:"isBase64Encoded,omitempty"`
	Body            string            `json:"body"`
}

// NewHTTPTriggerResponse 创建新的 HTTP Trigger 响应
func NewHTTPTriggerResponse(statusCode int) *HTTPTriggerResponse {
	return &HTTPTriggerResponse{StatusCode: statusCode}
}

// WithStatusCode 设置状态码
func (h *HTTPTriggerResponse) WithStatusCode(statusCode int) *HTTPTriggerResponse {
	h.StatusCode = statusCode
	return h
}

// WithHeaders 设置响应头
func (h *HTTPTriggerResponse) WithHeaders(headers map[string]string) *HTTPTriggerResponse {
	h.Headers = headers
	return h
}

// WithIsBase64Encoded 设置是否 Base64 编码
func (h *HTTPTriggerResponse) WithIsBase64Encoded(isBase64Encoded bool) *HTTPTriggerResponse {
	h.IsBase64Encoded = isBase64Encoded
	return h
}

// WithBody 设置响应体
func (h *HTTPTriggerResponse) WithBody(body string) *HTTPTriggerResponse {
	h.Body = body
	return h
}

// ProcessResponse 处理请求的响应
type ProcessResponse struct {
	Success bool         `json:"success"`
	Message string       `json:"message"`
	Result  *ProcessResult `json:"result,omitempty"`
}

// ProcessResult 处理结果
type ProcessResult struct {
	VideoFile    string   `json:"video_file"`
	OutputDir    string   `json:"output_dir"`
	SceneCount   int      `json:"scene_count"`
	Keyframes    []string `json:"keyframes"`
	AudioFile    string   `json:"audio_file"`
	MetadataFile string   `json:"metadata_file"`
}


