# API接口拆分使用指南

## 概述

原有的单一API接口已拆分为两个独立的接口，提供更灵活的使用方式：

1. **位置美食API** (`/api/location-food`) - 返回位置信息和附近美食数据
2. **AI推荐API** (`/api/ai-recommendation`) - 返回AI智能推荐内容

## 接口详情

### 1. 位置美食API

**接口地址：** `POST /api/location-food`

**功能：** 根据提供的位置信息，返回地点坐标和附近美食数据

**请求格式：**
```json
{
    "location": "黑石礁地铁站"
}
```

**响应格式：**
```json
{
    "success": true,
    "message": "Location and food data retrieved successfully",
    "data": {
        "location_info": {
            "name": "黑石礁地铁站",
            "coordinates": {
                "longitude": 121.557527,
                "latitude": 38.874543
            }
        },
        "food_data": [
            {
                "name": "必胜客(辰熙店)",
                "address": "中山路688号辰熙广场购物中心1层",
                "type": "餐饮服务;快餐厅;必胜客",
                "distance": "42米"
            },
            {
                "name": "东四胡同四宝牛杂面(黑石礁店)",
                "address": "中山路700-SD007号",
                "type": "餐饮服务;餐饮相关场所;餐饮相关",
                "distance": "43米"
            }
        ],
        "search_config": {
            "radius": 1500,
            "max_results": 8,
            "food_types": "050000",
            "sort_by": "distance"
        }
    },
    "error": null
}
```

### 2. AI推荐API

**接口地址：** `POST /api/ai-recommendation`

**功能：** 基于位置和美食数据，返回AI智能推荐内容

**请求格式：**
```json
{
    "location": "黑石礁地铁站"
}
```

**响应格式：**
```json
{
    "success": true,
    "message": "AI recommendation generated successfully",
    "data": {
        "recommendation": "基于您的位置黑石礁地铁站，我为您推荐以下美食选择：\n\n🍕 **必胜客(辰熙店)** - 距离最近仅42米\n位于辰熙广场购物中心1层，是知名的国际连锁披萨品牌，适合家庭聚餐或朋友聚会。\n\n🍜 **东四胡同四宝牛杂面(黑石礁店)** - 43米\n正宗的牛杂面，汤浓味美，是当地特色小吃的不错选择。\n\n🦆 **开鑫鸭先生(辰熙天街店)** - 45米\n特色烤鸭店，就在地铁站C1口旁边，交通便利。\n\n建议您根据个人喜好和用餐需求选择，所有推荐餐厅距离都很近，步行即可到达。"
    },
    "error": null
}
```

## 使用场景

### 场景1：只需要位置和美食数据

如果您只需要获取位置信息和附近美食列表，而不需要AI推荐，可以只调用位置美食API：

```bash
curl -X POST http://127.0.0.1:8080/api/location-food \
  -H "Content-Type: application/json" \
  -d '{"location": "黑石礁地铁站"}'
```

### 场景2：只需要AI推荐内容

如果您只需要AI的智能推荐内容，可以只调用AI推荐API：

```bash
curl -X POST http://127.0.0.1:8080/api/ai-recommendation \
  -H "Content-Type: application/json" \
  -d '{"location": "黑石礁地铁站"}'
```

### 场景3：需要完整的数据和推荐

如果您需要完整的位置、美食数据和AI推荐，可以分别调用两个接口：

```javascript
// 获取位置和美食数据
const locationFoodResponse = await fetch('http://127.0.0.1:8080/api/location-food', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json'
    },
    body: JSON.stringify({location: '黑石礁地铁站'})
});
const locationFoodData = await locationFoodResponse.json();

// 获取AI推荐
const aiRecommendationResponse = await fetch('http://127.0.0.1:8080/api/ai-recommendation', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json'
    },
    body: JSON.stringify({location: '黑石礁地铁站'})
});
const aiRecommendationData = await aiRecommendationResponse.json();
```

## 优势

1. **灵活性提升**：可以根据需求选择性调用接口
2. **性能优化**：不需要AI推荐时可以避免AI模型调用，提高响应速度
3. **成本控制**：减少不必要的AI API调用，降低使用成本
4. **独立性**：两个功能模块相互独立，便于维护和扩展

## 兼容性说明

- 原有的 `/api/ai/content` 接口仍然保留，返回纯文本格式的AI推荐内容
- 地图API `/api/map` 和健康检查 `/health` 接口保持不变

## 错误处理

两个接口都遵循统一的错误响应格式：

```json
{
    "success": false,
    "message": "错误描述",
    "data": null,
    "error": "详细错误信息"
}
```

常见错误：
- 位置信息无效或未找到
- 网络连接问题
- API密钥配置错误
- AI服务暂时不可用