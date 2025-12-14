package oss

import "encoding/json"

// OssEvent 阿里云 OSS Event 数据结构
type OssEvent struct {
	Events []OssEventItem `json:"events"`
}

// OssEventItem OSS Event 项
type OssEventItem struct {
	EventName         string            `json:"eventName"`
	EventSource       string            `json:"eventSource"`
	EventTime         string            `json:"eventTime"`
	EventVersion      string            `json:"eventVersion"`
	Oss               OssInfo           `json:"oss"`
	Region            string            `json:"region"`
	RequestParameters RequestParameters `json:"requestParameters"`
	ResponseElements  ResponseElements  `json:"responseElements"`
	UserIdentity      UserIdentity      `json:"userIdentity"`
}

// OssInfo OSS 信息
type OssInfo struct {
	Bucket           BucketInfo `json:"bucket"`
	Object           ObjectInfo `json:"object"`
	OssSchemaVersion string     `json:"ossSchemaVersion"`
	RuleID           string     `json:"ruleId"`
}

// BucketInfo Bucket 信息
type BucketInfo struct {
	ARN                     string       `json:"arn"`
	Name                    string       `json:"name"`
	OwnerIdentity           UserIdentity `json:"ownerIdentity"`
	VirtualHostedBucketName string       `json:"virtualBucket,omitempty"`
}

// ObjectInfo 对象信息
type ObjectInfo struct {
	DeltaSize  *int64      `json:"deltaSize,omitempty"`
	ETag       string      `json:"eTag"`
	Key        string      `json:"key"`
	ObjectMeta *ObjectMeta `json:"objectMeta,omitempty"`
	Size       int64       `json:"size"`
}

// ObjectMeta 对象元数据
type ObjectMeta struct {
	MimeType string `json:"mimeType,omitempty"`
}

// RequestParameters 请求参数
type RequestParameters struct {
	SourceIPAddress string `json:"sourceIPAddress"`
}

// ResponseElements 响应元素
type ResponseElements struct {
	RequestID string `json:"requestId"`
}

// UserIdentity 用户身份
// 注意：ownerIdentity 字段在 OSS 事件中可能是字符串或对象
type UserIdentity struct {
	PrincipalID string `json:"principalId"`
}

// UnmarshalJSON 自定义反序列化，支持字符串或对象两种格式
func (u *UserIdentity) UnmarshalJSON(data []byte) error {
	// 先尝试作为字符串解析
	var str string
	if err := json.Unmarshal(data, &str); err == nil {
		u.PrincipalID = str
		return nil
	}

	// 如果不是字符串，尝试作为对象解析
	var obj struct {
		PrincipalID string `json:"principalId"`
	}
	if err := json.Unmarshal(data, &obj); err != nil {
		return err
	}
	u.PrincipalID = obj.PrincipalID
	return nil
}

// ProcessResponse 处理请求的响应
type ProcessResponse struct {
	Success bool           `json:"success"`
	Message string         `json:"message"`
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
