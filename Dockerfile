# 多阶段构建 - 构建阶段
FROM rust:1.75 as builder

# 设置工作目录
WORKDIR /app

# 复制 Cargo 文件
COPY Cargo.toml Cargo.lock ./

# 创建一个虚拟的 src/main.rs 来缓存依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs

# 构建依赖（这一层会被缓存）
RUN cargo build --release
RUN rm src/main.rs

# 复制源代码
COPY src ./src
COPY config.json ./
COPY *.json ./

# 构建应用
RUN cargo build --release

# 运行阶段 - 使用更小的基础镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建非root用户
RUN useradd -r -s /bin/false appuser

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/my_crate_demo /app/
COPY --from=builder /app/config.json /app/
COPY --from=builder /app/*.json /app/

# 更改文件所有者
RUN chown -R appuser:appuser /app

# 切换到非root用户
USER appuser

# 暴露端口
EXPOSE 8080

# 设置环境变量
ENV RUST_LOG=info
ENV ACTIX_WEB_BIND=0.0.0.0:8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动应用
CMD ["./my_crate_demo"]