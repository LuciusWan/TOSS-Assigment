# Docker 部署指南

本项目提供了完整的 Docker 部署方案，支持单容器部署和 Docker Compose 编排。

## 快速开始

### 方式一：使用 Docker Compose（推荐）

```bash
# 构建并启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down
```

### 方式二：直接使用 Docker

```bash
# 构建镜像
docker build -t food-recommendation-api .

# 运行容器
docker run -d \
  --name food-api \
  -p 8080:8080 \
  -e RUST_LOG=info \
  food-recommendation-api

# 查看日志
docker logs -f food-api

# 停止容器
docker stop food-api
docker rm food-api
```

## 配置说明

### 环境变量

- `RUST_LOG`: 日志级别（debug, info, warn, error）
- `ACTIX_WEB_BIND`: 服务绑定地址，默认 `0.0.0.0:8080`

### 端口映射

- 容器内端口：8080
- 主机端口：8080（可修改）

### 健康检查

容器会自动进行健康检查，访问 `/health` 端点验证服务状态。

## 生产环境部署

### 1. 构建优化镜像

```bash
# 使用多阶段构建，减小镜像体积
docker build -t food-recommendation-api:latest .
```

### 2. 安全配置

- 容器以非 root 用户运行
- 只暴露必要的端口
- 使用只读挂载配置文件

### 3. 资源限制

在 `docker-compose.yml` 中添加资源限制：

```yaml
services:
  food-recommendation-api:
    # ... 其他配置
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

### 4. 日志管理

```yaml
services:
  food-recommendation-api:
    # ... 其他配置
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

## 故障排除

### 查看容器状态

```bash
# 查看运行中的容器
docker ps

# 查看所有容器（包括停止的）
docker ps -a

# 查看容器详细信息
docker inspect food-api
```

### 查看日志

```bash
# 查看实时日志
docker logs -f food-api

# 查看最近的日志
docker logs --tail 100 food-api
```

### 进入容器调试

```bash
# 进入运行中的容器
docker exec -it food-api /bin/bash
```

### 常见问题

1. **端口冲突**：修改 `docker-compose.yml` 中的端口映射
2. **内存不足**：增加 Docker 的内存限制
3. **网络问题**：检查防火墙和网络配置

## API 测试

服务启动后，可以通过以下方式测试：

```bash
# 健康检查
curl http://localhost:8080/health

# 测试 API 端点
curl -X POST http://localhost:8080/api/ai \
  -H "Content-Type: application/json" \
  -d '{"message": "推荐一些餐厅"}'
```

## 镜像信息

- 基础镜像：`debian:bookworm-slim`
- 构建工具：`rust:1.75`
- 最终镜像大小：约 100MB
- 安全特性：非 root 用户运行

## 更新部署

```bash
# 重新构建并部署
docker-compose down
docker-compose build --no-cache
docker-compose up -d

# 或者使用 Docker
docker stop food-api
docker rm food-api
docker build -t food-recommendation-api .
docker run -d --name food-api -p 8080:8080 food-recommendation-api
```