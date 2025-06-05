use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::{error::Error, fs, time::Duration, process};
use chrono::Local;
use serde_json::{Value, json};
use colored::*;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware, get, post};
use std::sync::Arc;
use actix_web_lab::sse::{self, Sse};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    username: String,
    attempts: u32,
    keywords: String,
    city: String,
    api_key: String,
    output_file: String,
    food_radius: u32,
    food_types: String,
    max_food_results: u32,
    qwen_api_key: String,
    qwen_model: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            username: "default_user".to_string(),
            attempts: 3,
            keywords: "大连理工大学开发区校区".to_string(),
            city: "大连".to_string(),
            api_key: "1a3d892d650f273ea24b3e2be9beea00".to_string(),
            output_file: "response_log.json".to_string(),
            food_radius: 1000,
            food_types: "050000".to_string(),
            max_food_results: 5,
            qwen_api_key: "sk-f05ef9cd88fd436ea4be2b2e3edae7f4".to_string(),
            qwen_model: "qwen3-235b-a22b".to_string(),
        }
    }
}

fn load_config() -> Result<Config, Box<dyn Error>> {
    let config_path = "config.json";
    match fs::read_to_string(config_path) {
        Ok(contents) => {
            let config: Config = serde_json::from_str(&contents)?;
            Ok(config)
        }
        Err(_) => {
            println!("⚠️  配置文件未找到，创建默认配置");
            let default_config = Config::default();
            fs::write(
                config_path,
                serde_json::to_string_pretty(&default_config)?
            )?;
            println!("✅  已创建默认配置文件: {}", config_path);
            Ok(default_config)
        }
    }
}

async fn get_location(client: &Client, config: &Config) -> Result<(f64, f64), Box<dyn Error>> {
    let mut url = reqwest::Url::parse("https://restapi.amap.com/v3/assistant/inputtips")?;
    url.query_pairs_mut()
        .append_pair("key", &config.api_key)
        .append_pair("keywords", &config.keywords)
        .append_pair("city", &config.city);

    println!("🔍 查询地点坐标: {}", config.keywords.green());

    let response = client.get(url.clone())
        .header("User-Agent", &format!("{}-geo-service", config.username))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("定位API失败: {}", response.status()).into());
    }

    let body = response.text().await?;
    let data: Value = serde_json::from_str(&body)?;

    if let Some(tips) = data["tips"].as_array() {
        if tips.is_empty() {
            return Err("未找到相关地点".into());
        }

        // 尝试获取第一个有效位置
        for tip in tips {
            if let Some(location) = tip["location"].as_str() {
                let coords: Vec<&str> = location.split(',').collect();
                if coords.len() == 2 {
                    let longitude = coords[0].parse::<f64>()?;
                    let latitude = coords[1].parse::<f64>()?;
                    println!("✅ 坐标解析成功: {:.6}, {:.6}", longitude, latitude);
                    return Ok((longitude, latitude));
                }
            }
        }
    }

    Err("无法解析坐标，请检查API响应结构".into())
}

async fn search_food(client: &Client, config: &Config, location: (f64, f64)) -> Result<Value, Box<dyn Error>> {
    let (longitude, latitude) = location;
    let location_str = format!("{},{}", longitude, latitude);

    let mut url = reqwest::Url::parse("https://restapi.amap.com/v3/place/around")?;
    url.query_pairs_mut()
        .append_pair("key", &config.api_key)
        .append_pair("location", &location_str)
        .append_pair("types", &config.food_types)
        .append_pair("radius", &config.food_radius.to_string())
        .append_pair("offset", &config.max_food_results.to_string())
        .append_pair("extensions", "base");

    println!("\n🍽️  正在搜索附近美食...");
    println!("📍 中心位置: {}", config.keywords.green());
    println!("🗺️ 坐标: {:.6}, {:.6}", longitude, latitude);
    println!("🔍 参数: 半径{}米 | 类型: {} | 最大结果: {}",
             config.food_radius, config.food_types, config.max_food_results);

    let response = client.get(url)
        .header("User-Agent", &format!("{}-food-service", config.username))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("美食搜索API失败: {}", response.status()).into());
    }

    let body = response.text().await?;
    let data: Value = serde_json::from_str(&body)?;
    Ok(data)
}

fn format_food_results(data: &Value) -> String {
    let mut result = String::new();

    if let Some(pois) = data["pois"].as_array() {
        if pois.is_empty() {
            return "🔍 附近未找到美食场所".to_string();
        }

        result.push_str(&format!("\n🍴 找到 {} 家美食场所:\n", pois.len().to_string().green()));

        for (i, poi) in pois.iter().enumerate() {
            let name = poi["name"].as_str().unwrap_or("未知名称");
            let address = poi["address"].as_str().unwrap_or("未知地址");
            let distance = poi["distance"].as_str().unwrap_or("未知距离");
            let typecode = poi["typecode"].as_str().unwrap_or("未知类型");

            result.push_str(&format!("\n{}. {}", (i + 1).to_string().cyan().bold(), name.bold()));
            result.push_str(&format!("\n   📍 地址: {}", address));
            result.push_str(&format!("\n   📏 距离: {}米", distance));
            result.push_str(&format!("\n   🏷️ 类型: {}", typecode));

            if let Some(tel) = poi["tel"].as_str() {
                if !tel.is_empty() {
                    result.push_str(&format!("\n   📞 电话: {}", tel.blue()));
                }
            }
        }
    } else {
        result.push_str("⚠️  未找到有效美食数据");
    }

    result
}

#[derive(Serialize, Debug)]
struct QwenRequest {
    model: String,
    messages: Vec<QwenMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    enable_thinking: bool,  // 添加通义千问要求的特殊参数
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct QwenMessage {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct QwenResponse {
    choices: Vec<QwenChoice>,
}

#[derive(Deserialize, Debug)]
struct QwenChoice {
    message: QwenMessage,
}

async fn ask_qwen(prompt: &str, config: &Config) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let url = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";

    // 构建符合通义千问API要求的请求
    let request = QwenRequest {
        model: config.qwen_model.clone(),
        messages: vec![
            QwenMessage {
                role: "system".to_string(),
                content: "你是一个专业的美食评论家，擅长根据用户提供的地点信息给出专业、简洁的美食推荐。".to_string(),
            },
            QwenMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }
        ],
        temperature: Some(0.7),
        enable_thinking: false,  // 非流式调用必须设置为false
    };

    println!("\n🧠 正在调用通义千问AI分析...");
    println!("🤖 模型: {}", config.qwen_model.green());

    let response = client.post(url)
        .header("Authorization", format!("Bearer {}", config.qwen_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;
        return Err(format!("AI调用失败 ({}): {}", status, body).into());
    }

    let response_body = response.text().await?;
    println!("🔍 AI原始响应: {}", response_body);  // 调试输出

    // 尝试解析响应
    match serde_json::from_str::<QwenResponse>(&response_body) {
        Ok(qwen_response) => {
            if let Some(first_choice) = qwen_response.choices.first() {
                Ok(first_choice.message.content.clone())
            } else {
                Err("AI返回了空回复".into())
            }
        }
        Err(e) => {
            // 尝试解析错误消息
            if let Ok(error_value) = serde_json::from_str::<Value>(&response_body) {
                if let Some(error_msg) = error_value["error"]["message"].as_str() {
                    return Err(format!("AI解析失败: {}", error_msg).into());
                }
            }
            Err(format!("JSON解析失败: {} | 原始响应: {}", e, response_body).into())
        }
    }
}

fn generate_ai_prompt(food_data: &Value, location: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "用户位置：{}\n",
        location
    ));
    
    // 从food_data中提取搜索半径
    let radius = if let Some(radius) = food_data["radius"].as_str() {
        radius
    } else {
        "1000" // 默认值
    };
    
    prompt.push_str(&format!(
        "搜索范围：半径{}米\n\n",
        radius
    ));

    if let Some(pois) = food_data["pois"].as_array() {
        prompt.push_str("找到以下美食场所：\n");

        for (i, poi) in pois.iter().enumerate().take(5) {
            let name = poi["name"].as_str().unwrap_or("未知餐厅");
            let address = poi["address"].as_str().unwrap_or("未知地址");
            let distance = poi["distance"].as_str().unwrap_or("未知距离");
            let typecode = poi["typecode"].as_str().unwrap_or("未知类型");

            prompt.push_str(&format!(
                "{}. {}（{}米）\n  地址：{}\n  类型：{}\n",
                i + 1, name, distance, address, typecode
            ));
        }
    }

    prompt.push_str("\n请根据以上信息：\n");
    prompt.push_str("1. 推荐1-3个最适合商务聚餐的餐厅\n");
    prompt.push_str("2. 推荐1-2个性价比最高的学生餐厅\n");
    prompt.push_str("3. 分析这些餐厅的地理位置优势\n");
    prompt.push_str("4. 给出整体评价（不超过100字）\n");
    prompt.push_str("请用专业但简洁的语言回答，不要使用表情符号。");

    prompt
}

// API请求结构体定义
#[derive(Deserialize)]
struct LocationRequest {
    location: String,
    city: Option<String>,
}

// API响应结构体定义
#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
    data: Option<Value>,
    error: Option<String>,
}

// 处理API请求的函数
#[post("/api/ai")]
async fn food_recommendation_api(
    app_data: web::Data<AppState>,
    req: web::Json<LocationRequest>,
) -> impl Responder {
    let config = app_data.config.clone();
    let client = app_data.client.clone();
    
    // 创建一个可修改的配置副本
    let mut config_clone = (*config).clone();
    
    // 使用请求中的位置信息
    if !req.location.is_empty() {
        config_clone.keywords = req.location.clone();
        println!("📍 使用请求位置: {}", config_clone.keywords.green());
    }
    
    // 获取地点坐标
    let location = match get_location(&client, &config_clone).await {
        Ok(loc) => loc,
        Err(e) => {
            return web::Json(ApiResponse {
                success: false,
                message: "Failed to get location coordinates".to_string(),
                data: None,
                error: Some(e.to_string()),
            });
        }
    };
    
    // 搜索附近美食
    let food_data = match search_food(&client, &config_clone, location).await {
        Ok(data) => data,
        Err(e) => {
            return web::Json(ApiResponse {
                success: false,
                message: "Failed to search for food".to_string(),
                data: None,
                error: Some(e.to_string()),
            });
        }
    };
    
    // 生成AI提示并调用AI进行分析
    let ai_prompt = generate_ai_prompt(&food_data, &config_clone.keywords);
    let ai_response = match ask_qwen(&ai_prompt, &config_clone).await {
        Ok(response) => Some(response),
        Err(_) => {
            // 尝试备用模型
            let mut backup_config = config_clone.clone();
            backup_config.qwen_model = "qwen-turbo".to_string();
            
            match ask_qwen(&ai_prompt, &backup_config).await {
                Ok(response) => Some(response),
                Err(_) => None,
            }
        }
    };
    
    // 构建响应
    web::Json(ApiResponse {
        success: ai_response.is_some(),
        message: if ai_response.is_some() {
            "Food recommendations generated successfully".to_string()
        } else {
            "Failed to generate AI recommendations".to_string()
        },
        data: if let Some(ref recommendation) = ai_response {
            Some(json!({
                "location": config_clone.keywords,
                "coordinates": location,
                "food_data": food_data,
                "recommendation": recommendation,
                "config": config_clone,
            }))
        } else {
            None
        },
        error: if ai_response.is_none() {
            Some("Failed to get AI response from both primary and backup models".to_string())
        } else {
            None
        },
    })
}

// 只返回AI生成的内容API
#[post("/api/ai/content")]
async fn ai_content_only(
    app_data: web::Data<AppState>,
    req: web::Json<LocationRequest>,
) -> impl Responder {
    let config = app_data.config.clone();
    let client = app_data.client.clone();
    
    // 创建一个可修改的配置副本
    let mut config_clone = (*config).clone();
    
    // 使用请求中的位置信息
    if !req.location.is_empty() {
        config_clone.keywords = req.location.clone();
        println!("📍 使用请求位置: {}", config_clone.keywords.green());
    }
    
    // 获取地点坐标
    let location = match get_location(&client, &config_clone).await {
        Ok(loc) => loc,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("获取位置坐标失败: {}", e));
        }
    };
    
    // 搜索附近美食
    let food_data = match search_food(&client, &config_clone, location).await {
        Ok(data) => data,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("搜索美食失败: {}", e));
        }
    };
    
    // 生成AI提示并调用AI进行分析
    let ai_prompt = generate_ai_prompt(&food_data, &config_clone.keywords);
    let ai_response = match ask_qwen(&ai_prompt, &config_clone).await {
        Ok(response) => Some(response),
        Err(_) => {
            // 尝试备用模型
            let mut backup_config = config_clone.clone();
            backup_config.qwen_model = "qwen-turbo".to_string();
            
            match ask_qwen(&ai_prompt, &backup_config).await {
                Ok(response) => Some(response),
                Err(_) => None,
            }
        }
    };
    
    // 只返回AI生成的内容
    match ai_response {
        Some(content) => HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(content),
        None => HttpResponse::InternalServerError().body("无法获取AI推荐内容")
    }
}

// 健康检查API
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "service": "food-recommendation-api",
        "version": "1.0.0",
        "timestamp": Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }))
}

// 应用状态结构体
struct AppState {
    config: Arc<Config>,
    client: Client,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    println!("\n{}{}", "🗺️ 智能地理分析系统 ".bold().blue(), "v3.0".yellow());
    println!("{}", "=".repeat(40).dimmed());
    println!("{}", "集成高德地图API + 通义千问AI + Web API".bold());
    
    // 加载配置
    let config = match load_config() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            println!("❌ 配置加载失败: {}", e);
            process::exit(1);
        }
    };
    
    println!("\n🔧 配置加载成功");
    println!("👤 用户: {}", config.username.green());
    println!("🏙️ 默认城市: {}", config.city.green());
    println!("🤖 AI模型: {}", config.qwen_model.green());
    
    // 创建HTTP客户端
    let client = match Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 客户端创建失败: {}", e);
            process::exit(1);
        }
    };
    
    // 创建应用状态
    let app_state = web::Data::new(AppState {
        config: config.clone(),
        client,
    });
    
    // 启动Web服务器
    println!("\n🚀 启动Web API服务...");
    println!("📡 监听地址: http://127.0.0.1:8080");
    println!("🔌 完整数据API: http://127.0.0.1:8080/api/ai");
    println!("📝 纯文本API: http://127.0.0.1:8080/api/ai/content");
    println!("📊 流式API: http://127.0.0.1:8080/api/ai/stream");
    println!("🩺 健康检查: http://127.0.0.1:8080/health");
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
                    .add(("Access-Control-Allow-Headers", "Content-Type, Authorization"))
            )
            .service(food_recommendation_api)
            .service(ai_content_only)
            /*.service(ai_stream)*/
            .service(health_check)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
/*use futures::stream::{self, StreamExt}; // 添加这个导入
use actix_web::web::Bytes;  // 添加这个导入
// 自定义SSE流结构体
struct SseMessageStream {
    chunks: Vec<String>,
    current: usize,
    end_sent: bool, // 标记是否已发送结束事件
}

impl SseMessageStream {
    fn new(content: String) -> Self {
        // 将内容按行分割成多个块，过滤空行
        let chunks = content
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        SseMessageStream {
            chunks,
            current: 0,
            end_sent: false,
        }
    }
}

impl Stream for SseMessageStream {
    type Item = Result<sse::Event, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        if this.current < this.chunks.len() {
            // 获取当前块并构造JSON响应
            let chunk = &this.chunks[this.current];
            let json_data = serde_json::json!({
                "type": "chunk",
                "content": chunk
            }).to_string();

            // 移动到下一块
            this.current += 1;

            // 返回当前块作为SSE事件
            Poll::Ready(Some(Ok(sse::Event::Data(sse::Data::new(json_data)))))
        } else if !this.end_sent {
            // 发送结束标记
            this.end_sent = true;
            let end_event = sse::Event::Data(sse::Data::new(
                serde_json::json!({
                    "type": "end",
                    "content": "stream_completed"
                }).to_string()
            ));
            Poll::Ready(Some(Ok(end_event)))
        } else {
            // 所有块都已发送，流结束
            Poll::Ready(None)
        }
    }
}

// SSE流式API接口
#[post("/api/ai/stream")]
async fn ai_stream(
    app_data: web::Data<AppState>,
    req: web::Json<LocationRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let config = app_data.config.clone();
    let client = app_data.client.clone();

    // 创建一个可修改的配置副本
    let mut config_clone = (*config).clone();

    // 使用请求中的位置信息
    if !req.location.is_empty() {
        config_clone.keywords = req.location.clone();
        println!("📍 使用请求位置: {}", config_clone.keywords.green());
    }

    // 获取地点坐标
    let location = match get_location(&client, &config_clone).await {
        Ok(loc) => loc,
        Err(e) => {
            // 创建错误消息的 SSE 格式字符串
            let error_message = format!(
                "data: {}\n\n",
                serde_json::json!({
                "type": "error",
                "content": format!("获取位置坐标失败: {}", e)
            })
            );

            // 转换为 Bytes
            let bytes = Bytes::from(error_message);

            // 创建单个事件的流
            return Ok(HttpResponse::BadRequest()
                .content_type("text/event-stream")
                .streaming(stream::once(async { Ok(bytes) })));
        }
    };

    // 搜索附近美食
    let food_data = match search_food(&client, &config_clone, location).await {
        Ok(data) => data,
        Err(e) => {
            let error_message = format!(
                "data: {}\n\n",
                serde_json::json!({
                "type": "error",
                "content": format!("搜索美食失败: {}", e)
            })
            );
            let bytes = Bytes::from(error_message);

            return Ok(HttpResponse::InternalServerError()
                .content_type("text/event-stream")
                .streaming(stream::once(async { Ok(bytes) })));
        }
    };

    // 生成AI提示并调用AI进行分析
    let ai_prompt = generate_ai_prompt(&food_data, &config_clone.keywords);

    // 尝试主模型
    let mut ai_response = ask_qwen(&ai_prompt, &config_clone).await;

    // 如果主模型失败，尝试备用模型
    if ai_response.is_err() {
        let mut backup_config = config_clone.clone();
        backup_config.qwen_model = "qwen-turbo".to_string();
        ai_response = ask_qwen(&ai_prompt, &backup_config).await;
    }

    // 返回AI生成的内容作为SSE流
    match ai_response {
        Ok(content) => {
            // 将内容分割成行
            let lines: Vec<String> = content.split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            // 创建事件流
            let stream = futures::stream::iter(lines.into_iter().map(|line| {
                let json_data = serde_json::json!({
                "type": "chunk",
                "content": line
            }).to_string();
                Ok(Bytes::from(format!("data: {}\n\n", json_data)))
            }))
                .chain(futures::stream::once(async {
                    let end_event = serde_json::json!({
                "type": "end",
                "content": "stream_completed"
            }).to_string();
                    Ok(Bytes::from(format!("data: {}\n\n", end_event)))
                }));

            Ok(HttpResponse::Ok()
                .content_type("text/event-stream")
                .streaming(stream))
        },
        // AI调用失败处理
        Err(e) => {
            let error_message = format!(
                "data: {}\n\n",
                serde_json::json!({
                "type": "error",
                "content": format!("无法获取AI推荐内容: {}", e)
            })
            );
            let bytes = Bytes::from(error_message);

            Ok(HttpResponse::InternalServerError()
                .content_type("text/event-stream")
                .streaming(stream::once(async { Ok(bytes) })))
        }
    }
}*/