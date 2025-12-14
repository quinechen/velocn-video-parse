package handler

import (
	"encoding/json"
	"testing"

	"velocn-video-parse/oss"
)

// TestRealOSSEvent 测试真实的 OSS 事件
func TestRealOSSEvent(t *testing.T) {
	// 真实的 OSS 事件 JSON
	realEventJSON := `{
		"events": [{
			"eventName": "ObjectCreated:PostObject",
			"eventSource": "acs:oss",
			"eventTime": "2025-12-13T17:17:27.000Z",
			"eventVersion": "1.0",
			"oss": {
				"bucket": {
					"arn": "acs:oss:cn-hangzhou:1015382456283553:velocn-video-parse-bucket",
					"name": "velocn-video-parse-bucket",
					"ownerIdentity": "1015382456283553",
					"virtualBucket": ""
				},
				"object": {
					"deltaSize": 6226470,
					"eTag": "4EE26171F77B43046F41951766916291",
					"key": "v0d00fg10000d4o3ctvog65qqlt2lm00.MP4",
					"objectMeta": {
						"mimeType": "video/mp4"
					},
					"size": 6226470
				},
				"ossSchemaVersion": "1.0",
				"ruleId": "c2894da93273f4497dc3def50a4896bed7b55d38"
			},
			"region": "cn-hangzhou",
			"requestParameters": {
				"sourceIPAddress": "124.90.188.215"
			},
			"responseElements": {
				"requestId": "693D9FA7667085383239654E"
			},
			"userIdentity": {
				"principalId": "1015382456283553"
			}
		}]
	}`

	// 解析 OSS 事件
	var ossEvent oss.OssEvent
	if err := json.Unmarshal([]byte(realEventJSON), &ossEvent); err != nil {
		t.Fatalf("解析 OSS 事件失败: %v", err)
	}

	// 验证事件解析
	if len(ossEvent.Events) != 1 {
		t.Fatalf("期望 1 个事件，实际得到 %d 个", len(ossEvent.Events))
	}

	eventItem := ossEvent.Events[0]

	// 验证事件字段
	if eventItem.EventName != "ObjectCreated:PostObject" {
		t.Errorf("期望事件类型 ObjectCreated:PostObject，实际得到 %s", eventItem.EventName)
	}

	if eventItem.Oss.Bucket.Name != "velocn-video-parse-bucket" {
		t.Errorf("期望 bucket 名称 velocn-video-parse-bucket，实际得到 %s", eventItem.Oss.Bucket.Name)
	}

	if eventItem.Oss.Object.Key != "v0d00fg10000d4o3ctvog65qqlt2lm00.MP4" {
		t.Errorf("期望 object key v0d00fg10000d4o3ctvog65qqlt2lm00.MP4，实际得到 %s", eventItem.Oss.Object.Key)
	}

	if eventItem.Region != "cn-hangzhou" {
		t.Errorf("期望 region cn-hangzhou，实际得到 %s", eventItem.Region)
	}

	// 验证文件大小
	if eventItem.Oss.Object.Size != 6226470 {
		t.Errorf("期望文件大小 6226470，实际得到 %d", eventItem.Oss.Object.Size)
	}

	t.Logf("✅ OSS 事件解析成功")
	t.Logf("  • 事件类型: %s", eventItem.EventName)
	t.Logf("  • Bucket: %s", eventItem.Oss.Bucket.Name)
	t.Logf("  • Object Key: %s", eventItem.Oss.Object.Key)
	t.Logf("  • Region: %s", eventItem.Region)
	t.Logf("  • 文件大小: %d 字节", eventItem.Oss.Object.Size)
	t.Logf("  • MIME 类型: %s", eventItem.Oss.Object.ObjectMeta.MimeType)

	// 测试视频文件检测
	if !isVideoFile(eventItem.Oss.Object.Key) {
		t.Errorf("文件 %s 应该被识别为视频文件", eventItem.Oss.Object.Key)
	}

	t.Logf("✅ 视频文件检测通过")
}

// TestOSSEventWithStringOwnerIdentity 测试 ownerIdentity 为字符串的情况
func TestOSSEventWithStringOwnerIdentity(t *testing.T) {
	// ownerIdentity 为字符串格式的事件
	eventJSON := `{
		"events": [{
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
					"key": "image/a.jpg",
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
		}]
	}`

	var ossEvent oss.OssEvent
	if err := json.Unmarshal([]byte(eventJSON), &ossEvent); err != nil {
		t.Fatalf("解析 OSS 事件失败: %v", err)
	}

	if len(ossEvent.Events) != 1 {
		t.Fatalf("期望 1 个事件，实际得到 %d 个", len(ossEvent.Events))
	}

	eventItem := ossEvent.Events[0]

	// 验证 ownerIdentity 被正确解析（应该是字符串）
	// 注意：根据我们的 UnmarshalJSON 实现，ownerIdentity 应该被解析为字符串
	if eventItem.Oss.Bucket.OwnerIdentity.PrincipalID != "1237050315505689" {
		t.Errorf("期望 ownerIdentity 为 1237050315505689，实际得到 %s", eventItem.Oss.Bucket.OwnerIdentity.PrincipalID)
	}

	t.Logf("✅ 字符串格式的 ownerIdentity 解析成功")
}

