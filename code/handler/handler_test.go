package handler

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"velocn-video-parse/oss"
)

// TestHandleOSSEvent 测试 OSS 事件处理
func TestHandleOSSEvent(t *testing.T) {
	// 检查测试视频文件是否存在
	testVideoPath := filepath.Join("testdata", "test.mp4")
	if _, err := os.Stat(testVideoPath); os.IsNotExist(err) {
		t.Skipf("测试视频文件不存在: %s，跳过测试", testVideoPath)
		return
	}

	// 构造 OSS 事件
	ossEvent := &oss.OssEvent{
		Events: []oss.OssEventItem{
			{
				EventName:    "ObjectCreated:PutObject",
				EventSource:  "acs:oss",
				EventTime:    "2017-04-21T12:46:37.000Z",
				EventVersion: "1.0",
				Oss: oss.OssInfo{
					Bucket: oss.BucketInfo{
						ARN:  "acs:oss:cn-shanghai:1237050315505689:test-bucket",
						Name: "test-bucket",
						OwnerIdentity: oss.UserIdentity{
							PrincipalID: "1237050315505689",
						},
					},
					Object: oss.ObjectInfo{
						ETag: "test-etag",
						Key:  "test.mp4",
						Size: 1024000,
					},
					OssSchemaVersion: "1.0",
					RuleID:           "test-rule-id",
				},
				Region: "cn-shanghai",
				RequestParameters: oss.RequestParameters{
					SourceIPAddress: "127.0.0.1",
				},
				ResponseElements: oss.ResponseElements{
					RequestID: "test-request-id",
				},
				UserIdentity: oss.UserIdentity{
					PrincipalID: "test-user-id",
				},
			},
		},
	}

	// 注意：这个测试需要实际的 OSS 访问，所以可能会失败
	// 在实际测试中，可能需要 mock OSS 客户端
	t.Logf("测试 OSS 事件处理（需要 OSS 访问权限）")

	// 由于需要 OSS 访问，这里只测试事件解析
	eventJSON, err := json.Marshal(ossEvent)
	if err != nil {
		t.Fatalf("序列化 OSS 事件失败: %v", err)
	}

	var parsedEvent oss.OssEvent
	if err := json.Unmarshal(eventJSON, &parsedEvent); err != nil {
		t.Fatalf("反序列化 OSS 事件失败: %v", err)
	}

	if len(parsedEvent.Events) != 1 {
		t.Errorf("期望 1 个事件，实际: %d", len(parsedEvent.Events))
	}

	eventItem := parsedEvent.Events[0]
	if eventItem.EventName != "ObjectCreated:PutObject" {
		t.Errorf("期望事件类型 ObjectCreated:PutObject，实际: %s", eventItem.EventName)
	}

	if eventItem.Oss.Object.Key != "test.mp4" {
		t.Errorf("期望对象键 test.mp4，实际: %s", eventItem.Oss.Object.Key)
	}

	// 测试视频文件过滤
	if !isVideoFile(eventItem.Oss.Object.Key) {
		t.Errorf("期望 test.mp4 被识别为视频文件")
	}

	t.Logf("✅ OSS 事件解析测试通过")
}

// TestHandleOSSEventNonVideoFile 测试非视频文件的过滤
func TestHandleOSSEventNonVideoFile(t *testing.T) {
	// 构造非视频文件的 OSS 事件
	ossEvent := &oss.OssEvent{
		Events: []oss.OssEventItem{
			{
				EventName:    "ObjectCreated:PutObject",
				EventSource:  "acs:oss",
				EventTime:    "2017-04-21T12:46:37.000Z",
				EventVersion: "1.0",
				Oss: oss.OssInfo{
					Bucket: oss.BucketInfo{
						ARN:  "acs:oss:cn-shanghai:1237050315505689:test-bucket",
						Name: "test-bucket",
						OwnerIdentity: oss.UserIdentity{
							PrincipalID: "1237050315505689",
						},
					},
					Object: oss.ObjectInfo{
						ETag: "test-etag",
						Key:  "image/test.jpg", // 非视频文件
						Size: 102400,
					},
					OssSchemaVersion: "1.0",
					RuleID:           "test-rule-id",
				},
				Region: "cn-shanghai",
				RequestParameters: oss.RequestParameters{
					SourceIPAddress: "127.0.0.1",
				},
				ResponseElements: oss.ResponseElements{
					RequestID: "test-request-id",
				},
				UserIdentity: oss.UserIdentity{
					PrincipalID: "test-user-id",
				},
			},
		},
	}

	// 测试非视频文件应该被过滤
	if isVideoFile(ossEvent.Events[0].Oss.Object.Key) {
		t.Errorf("期望 test.jpg 不被识别为视频文件")
	}

	// 由于是非视频文件，HandleOSSEvent 应该返回跳过消息
	// 但需要设置 DEBUG 模式或 mock OSS 客户端
	// 这里只测试文件类型检测
	t.Logf("✅ 非视频文件过滤测试通过")
}

// TestOSSEventParsing 测试 OSS 事件 JSON 解析
func TestOSSEventParsing(t *testing.T) {
	// 模拟真实的 OSS 事件 JSON（ownerIdentity 为字符串）
	jsonStr := `{
		"events": [
			{
				"eventName": "ObjectCreated:PutObject",
				"eventSource": "acs:oss",
				"eventTime": "2017-04-21T12:46:37.000Z",
				"eventVersion": "1.0",
				"oss": {
					"bucket": {
						"arn": "acs:oss:cn-shanghai:1237050315505689:bucketname",
						"name": "bucketname",
						"ownerIdentity": "1237050315505689",
						"virtualBucket": ""
					},
					"object": {
						"deltaSize": 122539,
						"eTag": "688A7BF4F233DC9C88A80BF985AB7329",
						"key": "videos/test.mp4",
						"size": 122539
					},
					"ossSchemaVersion": "1.0",
					"ruleId": "9adac8e253828f4f7c0466d941fa3db81161e853"
				},
				"region": "cn-shanghai",
				"requestParameters": {
					"sourceIPAddress": "140.205.128.221"
				},
				"responseElements": {
					"requestId": "58F9FF2D3DF792092E12044C"
				},
				"userIdentity": {
					"principalId": "262561392693583141"
				}
			}
		]
	}`

	var ossEvent oss.OssEvent
	if err := json.Unmarshal([]byte(jsonStr), &ossEvent); err != nil {
		t.Fatalf("解析 OSS 事件 JSON 失败: %v", err)
	}

	if len(ossEvent.Events) != 1 {
		t.Errorf("期望 1 个事件，实际: %d", len(ossEvent.Events))
	}

	eventItem := ossEvent.Events[0]

	// 验证 ownerIdentity（字符串格式）
	if eventItem.Oss.Bucket.OwnerIdentity.PrincipalID != "1237050315505689" {
		t.Errorf("期望 ownerIdentity 为 1237050315505689，实际: %s",
			eventItem.Oss.Bucket.OwnerIdentity.PrincipalID)
	}

	// 验证对象键
	if eventItem.Oss.Object.Key != "videos/test.mp4" {
		t.Errorf("期望对象键为 videos/test.mp4，实际: %s", eventItem.Oss.Object.Key)
	}

	// 验证是视频文件
	if !isVideoFile(eventItem.Oss.Object.Key) {
		t.Errorf("期望 videos/test.mp4 被识别为视频文件")
	}

	t.Logf("✅ OSS 事件 JSON 解析测试通过")
}
