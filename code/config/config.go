package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"

	"github.com/go-ini/ini"
)

// ProcessConfig 视频处理配置
type ProcessConfig struct {
	Threshold         float64 // 场景变化检测阈值
	MinSceneDuration  float64 // 最小场景持续时间（秒）
	SampleRate        float64 // 帧采样率（每秒采样多少帧）
	WebhookURL        string  // Webhook URL（处理完成后回调）
}

// DefaultProcessConfig 返回默认配置
func DefaultProcessConfig() ProcessConfig {
	return ProcessConfig{
		Threshold:        0.35,
		MinSceneDuration: 0.8,
		SampleRate:       0.5,
		WebhookURL:       "",
	}
}

// ExtendedConfig 扩展配置（包含输出路径、OSS配置等）
type ExtendedConfig struct {
	Process           ProcessConfig
	DebugMode         bool
	OutputPath        string
	DestinationBucket string
	DestinationRegion string
	DestinationPrefix string
	LogLevel          string
}

// ConfigLoader 配置加载器
type ConfigLoader struct{}

// LoadConfig 从多个源加载配置，优先级：命令行参数 > 环境变量 > 配置文件 > 默认值
func (c *ConfigLoader) LoadConfig(
	configFile string,
	threshold *float64,
	minSceneDuration *float64,
	sampleRate *float64,
	webhookURL string,
) (ProcessConfig, error) {
	// 1. 先加载配置文件（如果存在）
	var fileConfig *ProcessConfig
	if configFile != "" {
		if cfg, err := c.loadFromFile(configFile); err == nil {
			fileConfig = &cfg
		}
	} else {
		if cfg, err := c.loadFromDefaultLocations(); err == nil {
			fileConfig = &cfg
		}
	}

	// 2. 加载环境变量
	envThreshold := c.getEnvFloat("VIDEO_PARSE_THRESHOLD")
	envMinSceneDuration := c.getEnvFloat("VIDEO_PARSE_MIN_SCENE_DURATION")
	envSampleRate := c.getEnvFloat("VIDEO_PARSE_SAMPLE_RATE")
	envWebhookURL := os.Getenv("VIDEO_PARSE_WEBHOOK_URL")

	// 3. 合并配置（优先级：命令行 > 环境变量 > 配置文件 > 默认值）
	config := DefaultProcessConfig()

	if threshold != nil {
		config.Threshold = *threshold
	} else if envThreshold != nil {
		config.Threshold = *envThreshold
	} else if fileConfig != nil {
		config.Threshold = fileConfig.Threshold
	}

	if minSceneDuration != nil {
		config.MinSceneDuration = *minSceneDuration
	} else if envMinSceneDuration != nil {
		config.MinSceneDuration = *envMinSceneDuration
	} else if fileConfig != nil {
		config.MinSceneDuration = fileConfig.MinSceneDuration
	}

	if sampleRate != nil {
		config.SampleRate = *sampleRate
	} else if envSampleRate != nil {
		config.SampleRate = *envSampleRate
	} else if fileConfig != nil {
		config.SampleRate = fileConfig.SampleRate
	}

	if webhookURL != "" {
		config.WebhookURL = webhookURL
	} else if envWebhookURL != "" {
		config.WebhookURL = envWebhookURL
	} else if fileConfig != nil {
		config.WebhookURL = fileConfig.WebhookURL
	}

	return config, nil
}

// LoadExtendedConfig 加载扩展配置
func (c *ConfigLoader) LoadExtendedConfig(configFile string) ExtendedConfig {
	// 1. 加载视频处理配置
	processConfig, _ := c.LoadConfig(configFile, nil, nil, nil, "")

	// 2. 加载配置文件（如果存在）
	var fileConfig *ExtendedConfig
	if configFile != "" {
		if cfg, err := c.loadExtendedFromFile(configFile); err == nil {
			fileConfig = &cfg
		}
	} else {
		if cfg, err := c.loadExtendedFromDefaultLocations(); err == nil {
			fileConfig = &cfg
		}
	}

	// 3. 加载环境变量
	debugMode := c.getEnvBool("DEBUG")
	if fileConfig != nil {
		debugMode = fileConfig.DebugMode
	}

	outputPath := os.Getenv("OUTPUT_PATH")
	if outputPath == "" && fileConfig != nil {
		outputPath = fileConfig.OutputPath
	}

	destinationBucket := os.Getenv("DESTINATION_BUCKET")
	if destinationBucket == "" && fileConfig != nil {
		destinationBucket = fileConfig.DestinationBucket
	}

	destinationRegion := os.Getenv("DESTINATION_REGION")
	if destinationRegion == "" && fileConfig != nil {
		destinationRegion = fileConfig.DestinationRegion
	}

	destinationPrefix := os.Getenv("DESTINATION_PREFIX")
	if destinationPrefix == "" && fileConfig != nil {
		destinationPrefix = fileConfig.DestinationPrefix
	} else if destinationPrefix == "" {
		destinationPrefix = "processed"
	}

	logLevel := os.Getenv("LOG_LEVEL")
	if logLevel == "" && fileConfig != nil {
		logLevel = fileConfig.LogLevel
	} else if logLevel == "" {
		logLevel = "info"
	}

	return ExtendedConfig{
		Process:           processConfig,
		DebugMode:         debugMode,
		OutputPath:        outputPath,
		DestinationBucket: destinationBucket,
		DestinationRegion: destinationRegion,
		DestinationPrefix: destinationPrefix,
		LogLevel:          logLevel,
	}
}

// loadFromFile 从INI配置文件加载配置
func (c *ConfigLoader) loadFromFile(configPath string) (ProcessConfig, error) {
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return ProcessConfig{}, fmt.Errorf("配置文件不存在: %s", configPath)
	}

	cfg, err := ini.Load(configPath)
	if err != nil {
		return ProcessConfig{}, fmt.Errorf("读取配置文件失败: %v", err)
	}

	// 尝试从 [video_parse] 节读取，如果没有则使用 [DEFAULT] 节
	section := cfg.Section("video_parse")
	if section == nil {
		section = cfg.Section("DEFAULT")
	}

	config := DefaultProcessConfig()

	if threshold, err := section.Key("threshold").Float64(); err == nil {
		config.Threshold = threshold
	}

	if minSceneDuration, err := section.Key("min_scene_duration").Float64(); err == nil {
		config.MinSceneDuration = minSceneDuration
	}

	if sampleRate, err := section.Key("sample_rate").Float64(); err == nil {
		config.SampleRate = sampleRate
	}

	if webhookURL := section.Key("webhook_url").String(); webhookURL != "" {
		config.WebhookURL = webhookURL
	}

	return config, nil
}

// loadFromDefaultLocations 从默认位置加载配置文件
func (c *ConfigLoader) loadFromDefaultLocations() (ProcessConfig, error) {
	locations := []string{
		"video-parse.ini",
		".video-parse.ini",
	}

	if home, err := os.UserHomeDir(); err == nil {
		locations = append(locations, filepath.Join(home, ".video-parse.ini"))
	}

	locations = append(locations, "/etc/video-parse.ini")

	for _, loc := range locations {
		if _, err := os.Stat(loc); err == nil {
			return c.loadFromFile(loc)
		}
	}

	return ProcessConfig{}, fmt.Errorf("未找到配置文件")
}

// loadExtendedFromFile 从INI配置文件加载扩展配置
func (c *ConfigLoader) loadExtendedFromFile(configPath string) (ExtendedConfig, error) {
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return ExtendedConfig{}, fmt.Errorf("配置文件不存在: %s", configPath)
	}

	cfg, err := ini.Load(configPath)
	if err != nil {
		return ExtendedConfig{}, fmt.Errorf("读取配置文件失败: %v", err)
	}

	// 加载视频处理配置
	processConfig, _ := c.loadFromFile(configPath)

	videoParseSection := cfg.Section("video_parse")
	if videoParseSection == nil {
		videoParseSection = cfg.Section("DEFAULT")
	}

	ossSection := cfg.Section("oss")
	if ossSection == nil {
		ossSection = cfg.Section("DEFAULT")
	}

	loggingSection := cfg.Section("logging")
	if loggingSection == nil {
		loggingSection = cfg.Section("DEFAULT")
	}

	debugMode := false
	if debugModeStr := videoParseSection.Key("debug_mode").String(); debugModeStr != "" {
		debugMode = debugModeStr == "true"
	}

	outputPath := videoParseSection.Key("output_path").String()

	destinationBucket := ossSection.Key("destination_bucket").String()
	destinationRegion := ossSection.Key("destination_region").String()
	destinationPrefix := ossSection.Key("destination_prefix").String()
	if destinationPrefix == "" {
		destinationPrefix = "processed"
	}

	logLevel := loggingSection.Key("level").String()
	if logLevel == "" {
		logLevel = "info"
	}

	return ExtendedConfig{
		Process:           processConfig,
		DebugMode:         debugMode,
		OutputPath:        outputPath,
		DestinationBucket: destinationBucket,
		DestinationRegion: destinationRegion,
		DestinationPrefix: destinationPrefix,
		LogLevel:          logLevel,
	}, nil
}

// loadExtendedFromDefaultLocations 从默认位置加载扩展配置文件
func (c *ConfigLoader) loadExtendedFromDefaultLocations() (ExtendedConfig, error) {
	locations := []string{
		"video-parse.ini",
		".video-parse.ini",
	}

	if home, err := os.UserHomeDir(); err == nil {
		locations = append(locations, filepath.Join(home, ".video-parse.ini"))
	}

	locations = append(locations, "/etc/video-parse.ini")

	for _, loc := range locations {
		if _, err := os.Stat(loc); err == nil {
			return c.loadExtendedFromFile(loc)
		}
	}

	return ExtendedConfig{}, fmt.Errorf("未找到配置文件")
}

// getEnvFloat 从环境变量获取浮点数
func (c *ConfigLoader) getEnvFloat(key string) *float64 {
	if val := os.Getenv(key); val != "" {
		if f, err := strconv.ParseFloat(val, 64); err == nil {
			return &f
		}
	}
	return nil
}

// getEnvBool 从环境变量获取布尔值
func (c *ConfigLoader) getEnvBool(key string) bool {
	val := os.Getenv(key)
	return val == "true" || val == "1" || val == "TRUE"
}

