# DEBUG 模式使用指南

## 概述

DEBUG 模式是一个测试功能，当启用时，函数会跳过实际的视频处理，直接返回成功响应。这用于快速验证：

1. ✅ 事件触发逻辑是否正确
2. ✅ 函数是否正确部署到函数计算
3. ✅ OSS 事件是否能正确传递到函数
4. ✅ HTTP 请求处理是否正常

## 启用方式

### 方法 1：在 s.yaml 中配置（推荐）

```yaml
resources:
  hello_world:
    component: fc3 
    props:
      environmentVariables:
        DEBUG: "true"  # 启用 DEBUG 模式
        DESTINATION_BUCKET: "your-destination-bucket"
        DESTINATION_REGION: "cn-hangzhou"
```

### 方法 2：在函数计算控制台配置

1. 登录阿里云函数计算控制台
2. 找到你的函数
3. 进入"环境变量"配置
4. 添加环境变量：
   - 键：`DEBUG`
   - 值：`true`

### 方法 3：临时测试（通过 HTTP 请求头）

如果需要在单次请求中测试，可以在代码中添加请求头支持（当前未实现）。

## 行为说明

### DEBUG 模式开启时

- ✅ 接收并解析 OSS 事件
- ✅ 记录事件信息到日志
- ✅ **跳过**视频下载
- ✅ **跳过**视频处理
- ✅ **跳过**结果上传
- ✅ 直接返回成功响应

### DEBUG 模式关闭时（默认）

- ✅ 正常执行完整的处理流程
- ✅ 下载视频
- ✅ 处理视频
- ✅ 上传结果

## 响应示例

### DEBUG 模式响应

```json
{
  "success": true,
  "message": "DEBUG 模式：事件接收成功，事件信息: bucket=source-bucket, key=videos/test.mp4, region=cn-hangzhou, eventName=ObjectCreated:Put",
  "result": null
}
```

### 正常模式响应

```json
{
  "success": true,
  "message": "成功处理视频，检测到 15 个场景，已上传到目标 bucket",
  "result": {
    "video_file": "/tmp/video-parse/xxx/video.mp4",
    "output_dir": "/tmp/video-parse/xxx/output",
    "scene_count": 15,
    "keyframes": ["keyframe_0000.jpg", ...],
    "audio_file": "audio.aac",
    "metadata_file": "metadata.json"
  }
}
```

## 日志输出

### DEBUG 模式日志

```
INFO: 收到 OSS Event: ...
INFO: DEBUG 模式已启用，跳过实际处理，直接返回成功
INFO: DEBUG 模式 - 事件信息: bucket=source-bucket, key=videos/test.mp4, region=cn-hangzhou, eventName=ObjectCreated:Put
```

### 正常模式日志

```
INFO: 收到 OSS Event: ...
INFO: 处理视频文件: bucket=source-bucket, key=videos/test.mp4, region=cn-hangzhou
INFO: 正在从 OSS 下载文件: ...
INFO: 文件已下载到: ...
INFO: 开始上传处理结果到目标 bucket: ...
...
```

## 使用场景

### 1. 首次部署测试

部署函数后，启用 DEBUG 模式测试：

```yaml
environmentVariables:
  DEBUG: "true"
```

上传一个测试文件到 OSS，检查：
- ✅ 函数是否被触发
- ✅ 事件是否正确传递
- ✅ 响应是否正常

### 2. 事件触发逻辑验证

验证 OSS 触发器配置是否正确：

1. 启用 DEBUG 模式
2. 在源 bucket 上传文件
3. 查看函数日志确认事件信息
4. 检查响应内容

### 3. 部署验证

快速验证函数是否正确部署：

1. 启用 DEBUG 模式
2. 发送测试请求
3. 确认返回成功响应
4. 关闭 DEBUG 模式，进行实际处理

## 测试步骤

### 步骤 1：启用 DEBUG 模式

在 `s.yaml` 中设置：

```yaml
environmentVariables:
  DEBUG: "true"
```

### 步骤 2：部署函数

```bash
s deploy
```

### 步骤 3：触发测试

在源 OSS bucket 上传一个测试文件（可以是任何文件，因为不会实际处理）。

### 步骤 4：查看结果

1. **查看函数日志**：
   ```bash
   s logs --tail
   ```

2. **查看响应**：
   - 函数应该返回成功响应
   - 响应中包含事件信息

3. **验证**：
   - ✅ 函数被触发
   - ✅ 事件信息正确
   - ✅ 响应格式正确

### 步骤 5：关闭 DEBUG 模式

测试通过后，关闭 DEBUG 模式：

```yaml
environmentVariables:
  # DEBUG: "true"  # 注释掉或删除
  DEBUG: "false"   # 或设置为 false
```

重新部署：

```bash
s deploy
```

## 注意事项

### ⚠️ 重要提醒

1. **生产环境不要启用 DEBUG 模式**
   - DEBUG 模式会跳过所有实际处理
   - 不会下载、处理或上传任何文件

2. **仅用于测试**
   - DEBUG 模式仅用于验证部署和事件触发
   - 不能用于实际业务处理

3. **环境变量是字符串**
   - 必须设置为 `"true"`（字符串）
   - 大小写不敏感（`"true"`, `"True"`, `"TRUE"` 都可以）

4. **需要重新部署**
   - 修改环境变量后需要重新部署函数
   - 使用 `s deploy` 命令

## 故障排查

### 问题：DEBUG 模式未生效

**检查项：**
1. 环境变量是否正确设置（`DEBUG: "true"`）
2. 是否重新部署了函数
3. 环境变量值是否为字符串 `"true"`

**验证方法：**
```bash
# 查看函数配置
s info

# 查看环境变量
s env list
```

### 问题：仍然执行了实际处理

**可能原因：**
1. 环境变量未正确设置
2. 环境变量值不是 `"true"`
3. 函数未重新部署

**解决方法：**
1. 确认环境变量配置
2. 重新部署函数
3. 查看日志确认 DEBUG 模式是否启用

## 完整配置示例

```yaml
edition: 3.0.0
name: video-parse-app
access: "{{ access }}"

vars:
  region: "cn-hangzhou"

resources:
  hello_world:
    component: fc3 
    actions:    
      pre-${regex('deploy|local')}:
        - run: make video-parse
    props:
      region: ${vars.region} 
      functionName: "velocn-video-parse-function"
      runtime: "custom"
      description: 'video-parse task'
      timeout: 600
      memorySize: 2048
      cpu: 2
      diskSize: 10240
      code: ./dist
      environmentVariables:
        # DEBUG 模式：设置为 true 启用测试模式
        DEBUG: "false"  # 生产环境设置为 false
        # DEBUG: "true"  # 测试时设置为 true
        
        # 目标 bucket 配置
        DESTINATION_BUCKET: "your-destination-bucket"
        DESTINATION_REGION: "cn-hangzhou"
        DESTINATION_PREFIX: "processed"
      customRuntimeConfig:
        command:
          - '/code/main'
          - 'serve'
        port: 9000
```

## 总结

DEBUG 模式是一个非常有用的测试工具，可以快速验证：

- ✅ 函数部署是否正确
- ✅ 事件触发是否正常
- ✅ OSS 事件传递是否正确
- ✅ HTTP 请求处理是否正常

**记住**：测试完成后，记得关闭 DEBUG 模式！
