# 地图API使用指南

## 概述

本项目新增了地图API功能，基于高德地图静态地图服务，可以为指定地点生成地图图片URL。该API与现有的美食推荐功能完美结合，为用户提供可视化的地理位置信息。

## API端点

**POST** `/api/map`

## 请求格式

### 请求头
```
Content-Type: application/json
```

### 请求体参数

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|------|--------|---------|
| location | string | ✅ | - | 要查询的地点名称或地址 |
| zoom | integer | ❌ | 15 | 地图缩放级别 (3-18) |
| size | string | ❌ | "400*300" | 地图尺寸 (宽*高) |
| markers | array | ❌ | - | 额外标记点数组 |

### 缩放级别说明
- **3-5**: 国家/省份级别
- **6-10**: 城市级别
- **11-14**: 区县级别
- **15-16**: 街道级别
- **17-18**: 建筑物级别

## 请求示例

### 基础请求
```json
{
  "location": "大连理工大学开发区校区"
}
```

### 高级请求
```json
{
  "location": "大连理工大学开发区校区",
  "zoom": 16,
  "size": "600*400",
  "markers": [
    "large,0xFF0000,A:121.766007,39.052395",
    "mid,0x00FF00,B:121.767007,39.053395"
  ]
}
```

### 餐厅地图请求
```json
{
  "location": "海底捞火锅(大连开发区万达店)",
  "zoom": 18,
  "size": "800*600"
}
```

## 响应格式

### 成功响应
```json
{
  "status": "success",
  "map_url": "https://restapi.amap.com/v3/staticmap?location=121.766007,39.052395&zoom=15&size=400*300&markers=mid,,A:121.766007,39.052395&key=YOUR_API_KEY",
  "location": "大连理工大学开发区校区",
  "coordinates": [121.766007, 39.052395],
  "message": "地图生成成功",
  "timestamp": "2024-01-15 14:30:25"
}
```

### 错误响应
```json
{
  "status": "error",
  "map_url": null,
  "location": "不存在的地点",
  "coordinates": null,
  "message": "获取地点坐标失败: 未找到相关地点",
  "timestamp": "2024-01-15 14:30:25"
}
```

## 标记点格式

标记点使用以下格式：`size,color,label:longitude,latitude`

### 尺寸选项
- `large`: 大标记
- `mid`: 中等标记
- `small`: 小标记

### 颜色格式
- 十六进制颜色：`0xFF0000` (红色)
- 预定义颜色：`red`, `blue`, `green`, `yellow`, `purple`, `orange`

### 标签
- 单个字符：`A`, `B`, `C`, `1`, `2`, `3`

### 示例
```
large,0xFF0000,A:121.766007,39.052395  // 大红色标记A
mid,blue,B:121.767007,39.053395       // 中等蓝色标记B
small,0x00FF00,1:121.768007,39.054395 // 小绿色标记1
```

## cURL 测试命令

### 基础测试
```bash
curl -X POST http://127.0.0.1:8080/api/map \
  -H "Content-Type: application/json" \
  -d '{"location": "大连理工大学开发区校区"}'
```

### 高级测试
```bash
curl -X POST http://127.0.0.1:8080/api/map \
  -H "Content-Type: application/json" \
  -d '{
    "location": "大连理工大学开发区校区",
    "zoom": 16,
    "size": "600*400",
    "markers": ["large,0xFF0000,A:121.766007,39.052395"]
  }'
```

## 与美食推荐API的结合使用

### 工作流程
1. 调用 `/api/ai` 获取美食推荐
2. 从响应中提取餐厅位置信息
3. 调用 `/api/map` 生成包含餐厅位置的地图
4. 在前端同时显示推荐内容和地图

### 示例代码 (JavaScript)
```javascript
// 1. 获取美食推荐
const foodResponse = await fetch('/api/ai', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ message: '推荐附近的餐厅' })
});

const foodData = await foodResponse.json();

// 2. 生成地图
const mapResponse = await fetch('/api/map', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    location: '大连理工大学开发区校区',
    zoom: 15,
    size: '600*400'
  })
});

const mapData = await mapResponse.json();

// 3. 显示结果
if (mapData.status === 'success') {
  document.getElementById('map').src = mapData.map_url;
}
```

## 错误处理

### 常见错误

| 错误信息 | 原因 | 解决方案 |
|----------|------|----------|
| 未找到相关地点 | 地点名称不准确 | 使用更具体的地址或地标名称 |
| 地图定位API失败 | 网络问题或API限制 | 检查网络连接，确认API配额 |
| 无法解析地点坐标 | 返回数据格式异常 | 联系技术支持 |

### 最佳实践

1. **地点名称**: 使用具体的地址或知名地标
2. **缓存策略**: 对相同地点的地图URL进行缓存
3. **错误重试**: 实现自动重试机制
4. **用户体验**: 提供加载状态和错误提示

## 技术实现

### 核心功能
- 地点坐标查询（高德地图输入提示API）
- 静态地图生成（高德地图静态地图API）
- 自定义标记点支持
- 错误处理和日志记录

### 性能优化
- 异步处理
- 请求超时控制
- 响应缓存机制
- 资源清理

## 配置说明

地图API使用与美食推荐相同的高德地图API密钥，配置在 `config.json` 中：

```json
{
  "api_key": "your_amap_api_key",
  "city": "大连"
}
```

## 限制说明

- 地图尺寸最大：1024*1024
- 缩放级别范围：3-18
- 标记点数量：建议不超过10个
- API调用频率：遵循高德地图API限制

## 更新日志

### v1.0.0 (2024-01-15)
- ✅ 基础地图API实现
- ✅ 地点坐标查询
- ✅ 静态地图生成
- ✅ 自定义标记点支持
- ✅ 错误处理机制
- ✅ API文档和示例

## 支持与反馈

如有问题或建议，请通过以下方式联系：
- 技术支持：查看服务日志
- 功能建议：提交功能请求
- Bug报告：提供详细的错误信息和重现步骤