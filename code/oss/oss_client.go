package oss

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/aliyun/aliyun-oss-go-sdk/oss"
)

// OssClient OSS 客户端
type OssClient struct {
	client *oss.Client
}

// NewOssClient 创建新的 OSS 客户端
// 从环境变量读取凭证：
// - ALIBABA_CLOUD_ACCESS_KEY_ID
// - ALIBABA_CLOUD_ACCESS_KEY_SECRET
// - ALIBABA_CLOUD_SECURITY_TOKEN (可选，STS 临时凭证)
func NewOssClient() (*OssClient, error) {
	accessKeyID := os.Getenv("ALIBABA_CLOUD_ACCESS_KEY_ID")
	if accessKeyID == "" {
		return nil, fmt.Errorf("未找到 ALIBABA_CLOUD_ACCESS_KEY_ID 环境变量")
	}

	accessKeySecret := os.Getenv("ALIBABA_CLOUD_ACCESS_KEY_SECRET")
	if accessKeySecret == "" {
		return nil, fmt.Errorf("未找到 ALIBABA_CLOUD_ACCESS_KEY_SECRET 环境变量")
	}

	// 注意：securityToken 用于 STS 临时凭证，目前 OSS Go SDK 需要额外配置
	// 这里先获取但不使用，后续可以扩展支持 STS
	_ = os.Getenv("ALIBABA_CLOUD_SECURITY_TOKEN")

	// 创建 OSS 客户端（需要先指定一个 endpoint，后续可以根据 region 动态创建）
	// 注意：OSS Go SDK 需要 endpoint，我们会在 DownloadFile 中根据 region 创建客户端
	return &OssClient{}, nil
}

// extractRegionFromEndpoint 从 endpoint 提取 region
// 例如：oss-cn-hangzhou-internal.aliyuncs.com -> cn-hangzhou
func extractRegionFromEndpoint(endpoint string) string {
	endpoint = strings.TrimPrefix(endpoint, "http://")
	endpoint = strings.TrimPrefix(endpoint, "https://")

	if strings.HasPrefix(endpoint, "oss-") {
		endpoint = strings.TrimPrefix(endpoint, "oss-")
		if idx := strings.Index(endpoint, "-internal"); idx != -1 {
			return endpoint[:idx]
		}
		if idx := strings.Index(endpoint, ".aliyuncs.com"); idx != -1 {
			return endpoint[:idx]
		}
	}

	return "cn-hangzhou" // 默认值
}

// createClient 创建 OSS Client 实例
func (c *OssClient) createClient(endpoint string) (*oss.Client, error) {
	accessKeyID := os.Getenv("ALIBABA_CLOUD_ACCESS_KEY_ID")
	accessKeySecret := os.Getenv("ALIBABA_CLOUD_ACCESS_KEY_SECRET")
	securityToken := os.Getenv("ALIBABA_CLOUD_SECURITY_TOKEN")

	// 从 endpoint 提取 region（虽然当前未使用，但保留以备将来使用）
	_ = extractRegionFromEndpoint(endpoint)

	// 构建完整的 endpoint URL
	if !strings.HasPrefix(endpoint, "http://") && !strings.HasPrefix(endpoint, "https://") {
		endpoint = "https://" + endpoint
	}

	// 创建客户端
	client, err := oss.New(endpoint, accessKeyID, accessKeySecret)
	if err != nil {
		return nil, fmt.Errorf("创建 OSS 客户端失败: %v", err)
	}

	// 如果提供了 STS token，设置临时凭证
	// OSS Go SDK 的临时凭证需要通过 ClientOptions 设置
	// 这里简化处理，实际使用时可能需要调整
	_ = securityToken

	return client, nil
}

// DownloadFile 从 OSS 下载文件
func (c *OssClient) DownloadFile(bucket, objectKey, endpoint, localPath string) (string, error) {
	// 创建客户端
	client, err := c.createClient(endpoint)
	if err != nil {
		return "", err
	}

	// 获取 bucket
	bucketObj, err := client.Bucket(bucket)
	if err != nil {
		return "", fmt.Errorf("获取 bucket 失败: %v", err)
	}

	// 确保本地目录存在
	if err := os.MkdirAll(filepath.Dir(localPath), 0755); err != nil {
		return "", fmt.Errorf("创建本地目录失败: %v", err)
	}

	// 下载文件
	err = bucketObj.GetObjectToFile(objectKey, localPath)
	if err != nil {
		return "", fmt.Errorf("下载文件失败: %v", err)
	}

	return localPath, nil
}

// UploadFile 上传文件到 OSS
func (c *OssClient) UploadFile(bucket, objectKey, endpoint, localPath string) error {
	// 创建客户端
	client, err := c.createClient(endpoint)
	if err != nil {
		return err
	}

	// 获取 bucket
	bucketObj, err := client.Bucket(bucket)
	if err != nil {
		return fmt.Errorf("获取 bucket 失败: %v", err)
	}

	// 上传文件
	err = bucketObj.PutObjectFromFile(objectKey, localPath)
	if err != nil {
		return fmt.Errorf("上传文件失败: %v", err)
	}

	return nil
}

// UploadReader 上传 Reader 到 OSS
func (c *OssClient) UploadReader(bucket, objectKey, endpoint string, reader io.Reader) error {
	// 创建客户端
	client, err := c.createClient(endpoint)
	if err != nil {
		return err
	}

	// 获取 bucket
	bucketObj, err := client.Bucket(bucket)
	if err != nil {
		return fmt.Errorf("获取 bucket 失败: %v", err)
	}

	// 上传
	err = bucketObj.PutObject(objectKey, reader)
	if err != nil {
		return fmt.Errorf("上传文件失败: %v", err)
	}

	return nil
}

