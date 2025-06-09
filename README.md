# 智能地理美食推荐系统

## 项目概述

本项目是一个基于 Rust 开发的智能地理美食推荐系统，集成了高德地图 API 和通义千问 AI 模型，提供位置查询、美食搜索、AI 智能推荐和地图可视化等功能。系统采用 RESTful API 架构，支持跨平台部署和 Docker 容器化。
[前端工程地址](https://github.com/LuciusWan/TOSS-FrontEnd)


## 技术栈

- **后端语言**: Rust (Edition 2021)
- **Web 框架**: Actix-Web 4.4
- **地图服务**: 高德地图 API
- **AI 模型**: 通义千问 (Qwen)
- **数据格式**: JSON
- **容器化**: Docker + Docker Compose
- **跨域支持**: CORS
- **日志系统**: env_logger

## 核心功能

### 1. 位置信息查询
- 基于高德地图 API 的地理位置解析
- 支持地址到坐标的转换
- 精确的经纬度定位

### 2. 美食数据搜索
- 指定半径内的美食店铺搜索
- 按距离排序的搜索结果
- 详细的店铺信息（名称、地址、类型、距离）

### 3. AI 智能推荐
- 集成通义千问 AI 模型
- 基于位置和美食数据的智能分析
- 个性化推荐内容生成

### 4. 地图可视化
- 静态地图生成
- 自定义标记点
- 可调节缩放级别和尺寸

### 5. 模块化 API 设计
- 位置美食数据接口
- AI 推荐内容接口
- 地图生成接口
- 健康检查接口

## API 接口文档

### 基础信息
- **服务地址**: `http://127.0.0.1:8080`
- **请求方式**: POST/GET
- **数据格式**: JSON

### 接口列表

#### 1. 位置美食数据接口
```
POST /api/location-food
```
**功能**: 获取指定位置的坐标信息和附近美食数据

**请求示例**:
```json
{
    "location": "大连理工大学开发区校区"
}
```

**响应示例**:
```json
{
    "success": true,
    "message": "Location and food data retrieved successfully",
    "data": {
        "location_info": {
            "name": "大连理工大学开发区校区",
            "coordinates": {
                "longitude": 121.557527,
                "latitude": 38.874543
            }
        },
        "food_data": [
            {
                "name": "餐厅名称",
                "address": "详细地址",
                "type": "餐厅类型",
                "distance": "距离"
            }
        ],
        "search_config": {
            "radius": 1500,
            "max_results": 8,
            "food_types": "050000",
            "sort_by": "distance"
        }
    }
}
```

#### 2. AI 推荐接口
```
POST /api/ai-recommendation
```
**功能**: 基于位置和美食数据生成 AI 智能推荐

**请求示例**:
```json
{
    "location": "大连理工大学开发区校区"
}
```

**响应示例**:
```json
{
    "success": true,
    "message": "AI recommendation generated successfully",
    "data": {
        "recommendation": "基于您的位置，我为您推荐以下美食选择..."
    }
}
```

#### 3. 地图生成接口
```
POST /api/map
```
**功能**: 生成指定位置的静态地图

**请求示例**:
```json
{
    "location": "大连理工大学开发区校区",
    "zoom": 15,
    "size": "400*300",
    "markers": ["121.557527,38.874543"]
}
```

#### 4. 纯文本推荐接口
```
POST /api/ai/content
```
**功能**: 返回纯文本格式的 AI 推荐内容

#### 5. 健康检查接口
```
GET /health
```
**功能**: 服务健康状态检查

## 项目结构

```
d:\untitled/
├── src/
│   └── main.rs              # 主程序文件
├── .cargo/
│   └── config.toml          # Cargo 配置
├── Cargo.toml               # 项目依赖配置
├── Cargo.lock               # 依赖锁定文件
├── config.json              # 应用配置文件
├── Dockerfile               # Docker 镜像构建文件
├── docker-compose.yml       # Docker Compose 配置
├── .dockerignore            # Docker 忽略文件
├── Cross.toml               # 交叉编译配置
├── README.md                # 项目说明文档
├── API_SPLIT_GUIDE.md       # API 拆分使用指南
├── DOCKER_DEPLOYMENT.md     # Docker 部署指南
├── MAP_API_GUIDE.md         # 地图 API 使用指南
└── map_api_example.json     # 地图 API 示例
```

## 配置说明

### config.json 配置文件
```json
{
  "username": "汇报演示",
  "attempts": 5,
  "keywords": "大连理工大学开发区校区",
  "city": "大连",
  "api_key": "高德地图API密钥",
  "output_file": "smart_food_analysis.json",
  "food_radius": 1500,
  "food_types": "050000",
  "max_food_results": 8,
  "qwen_api_key": "通义千问API密钥",
  "qwen_model": "qwen3-235b-a22b"
}
```

### 主要配置项说明
- `api_key`: 高德地图 API 密钥
- `qwen_api_key`: 通义千问 AI API 密钥
- `food_radius`: 美食搜索半径（米）
- `max_food_results`: 最大返回结果数
- `food_types`: 美食类型代码

## 安装与运行

### 环境要求
- Rust 1.75+
- 高德地图 API 账号
- 通义千问 API 账号

### 本地开发

1. **克隆项目**
```bash
git clone <repository-url>
cd untitled
```

2. **配置环境**
```bash
# 安装 Rust（如果未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 更新配置文件
cp config.json.example config.json
# 编辑 config.json，填入你的 API 密钥
```

3. **编译运行**
```bash
# 开发模式运行
cargo run

# 生产模式编译
cargo build --release
```

4. **访问服务**
```
服务地址: http://127.0.0.1:8080
健康检查: http://127.0.0.1:8080/health
```

### Docker 部署

#### 使用 Docker Compose（推荐）
```bash
# 构建并启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down
```

#### 直接使用 Docker
```bash
# 构建镜像
docker build -t food-recommendation-api .

# 运行容器
docker run -d \
  --name food-api \
  -p 8080:8080 \
  -e RUST_LOG=info \
  food-recommendation-api
```

### 跨平台编译

项目支持 Linux 平台交叉编译：

```bash
# 安装交叉编译工具
cargo install cross

# 编译 Linux 版本
cross build --target x86_64-unknown-linux-gnu --release
```

## 使用示例

### cURL 测试

```bash
# 测试位置美食接口
curl -X POST http://127.0.0.1:8080/api/location-food \
  -H "Content-Type: application/json" \
  -d '{"location": "大连理工大学开发区校区"}'

# 测试 AI 推荐接口
curl -X POST http://127.0.0.1:8080/api/ai-recommendation \
  -H "Content-Type: application/json" \
  -d '{"location": "大连理工大学开发区校区"}'

# 测试地图接口
curl -X POST http://127.0.0.1:8080/api/map \
  -H "Content-Type: application/json" \
  -d '{"location": "大连理工大学开发区校区", "zoom": 15}'
```

### JavaScript 集成

```javascript
// 获取位置和美食数据
async function getLocationFood(location) {
    const response = await fetch('http://127.0.0.1:8080/api/location-food', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({ location })
    });
    return await response.json();
}

// 获取 AI 推荐
async function getAIRecommendation(location) {
    const response = await fetch('http://127.0.0.1:8080/api/ai-recommendation', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({ location })
    });
    return await response.json();
}
```

## 项目特色

### 1. 模块化设计
- API 接口功能拆分，支持独立调用
- 位置查询与 AI 推荐解耦
- 便于功能扩展和维护

### 2. 高性能架构
- Rust 语言的内存安全和高性能
- Actix-Web 异步处理框架
- 高并发请求支持

### 3. 容器化部署
- 多阶段 Docker 构建
- 优化的镜像大小（~100MB）
- 生产环境就绪

### 4. 跨平台支持
- Windows/Linux/macOS 兼容
- 交叉编译支持
- 云原生部署

### 5. 完善的错误处理
- 统一的错误响应格式
- 备用模型支持
- 详细的日志记录

## 技术亮点

1. **异步编程**: 使用 Rust 的 async/await 模式，提供高并发处理能力
2. **内存安全**: Rust 的所有权系统确保内存安全，避免常见的内存错误
3. **类型安全**: 强类型系统和编译时检查，减少运行时错误
4. **API 设计**: RESTful 风格，统一的响应格式，易于集成
5. **配置管理**: 灵活的 JSON 配置文件，支持运行时配置
6. **日志系统**: 结构化日志输出，便于调试和监控

## 性能指标

- **响应时间**: 平均 < 500ms
- **并发支持**: 1000+ 并发请求
- **内存占用**: < 50MB
- **镜像大小**: ~100MB
- **启动时间**: < 3s

## 扩展功能

### 已实现
- [x] 位置查询和美食搜索
- [x] AI 智能推荐
- [x] 静态地图生成
- [x] Docker 容器化
- [x] 跨域支持
- [x] 健康检查
- [x] 模块化 API

### 计划中
- [ ] 用户认证系统
- [ ] 数据缓存机制
- [ ] 实时地图更新
- [ ] 多语言支持
- [ ] 移动端适配

## 故障排除

### 常见问题

1. **API 密钥错误**
   - 检查 config.json 中的 API 密钥配置
   - 确认高德地图和通义千问账号状态

2. **网络连接问题**
   - 检查网络连接
   - 确认防火墙设置

3. **端口占用**
   - 检查 8080 端口是否被占用
   - 使用 `netstat -an | grep 8080` 查看端口状态

4. **Docker 构建失败**
   - 检查 Docker 版本
   - 清理 Docker 缓存：`docker system prune`

### 日志查看

```bash
# 查看应用日志
RUST_LOG=debug cargo run

# Docker 容器日志
docker logs food-api

# Docker Compose 日志
docker-compose logs -f
```

## 贡献指南

1. Fork 项目
2. 创建功能分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

## 联系方式

- 项目维护者: LuciusWan
- 邮箱: 18099488938@163.com
- 项目地址: [[项目仓库地址]](https://github.com/LuciusWan/TOSS-Assigment)

## 致谢

- 高德地图 API 提供地理位置服务
- 通义千问 AI 提供智能推荐能力
- Rust 社区提供优秀的开发工具和库
- Actix-Web 框架提供高性能 Web 服务支持

---

**注**: 本项目仅用于学习和演示目的，请遵守相关 API 服务的使用条款。
