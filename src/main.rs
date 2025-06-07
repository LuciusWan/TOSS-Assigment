use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::{error::Error, fs, time::Duration, process};
use chrono::Local;
use serde_json::{Value, json};
use colored::*;
use actix_web::{web, App, HttpResponse, HttpServer, middleware, get, post};
use actix_cors::Cors;
use std::sync::Arc;


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

    // 添加角色设定和任务描述
    prompt.push_str("你是一位专业的美食推荐顾问，擅长根据地理位置和餐厅信息为用户提供个性化的餐饮建议。\n\n");
    
    prompt.push_str(&format!(
        "📍 用户位置：{}\n",
        location
    ));
    
    // 从food_data中提取搜索半径
    let radius = if let Some(radius) = food_data["radius"].as_str() {
        radius
    } else {
        "1000" // 默认值
    };
    
    prompt.push_str(&format!(
        "🔍 搜索范围：半径{}米\n\n",
        radius
    ));

    // 添加餐厅类型代码说明
    prompt.push_str("📋 餐厅类型说明：\n");
    prompt.push_str("• 050100: 中餐厅/综合餐厅\n");
    prompt.push_str("• 050200: 外国餐厅\n");
    prompt.push_str("• 050300: 快餐厅\n");
    prompt.push_str("• 050400: 休闲餐饮场所\n");
    prompt.push_str("• 050500: 咖啡厅\n\n");

    if let Some(pois) = food_data["pois"].as_array() {
        prompt.push_str("🍽️ 附近美食场所详情：\n");

        for (i, poi) in pois.iter().enumerate().take(8) { // 增加到8个餐厅
            let name = poi["name"].as_str().unwrap_or("未知餐厅");
            let address = poi["address"].as_str().unwrap_or("未知地址");
            let distance = poi["distance"].as_str().unwrap_or("未知距离");
            let typecode = poi["typecode"].as_str().unwrap_or("未知类型");
            
            // 根据类型代码添加餐厅类型描述
            let type_desc = match typecode {
                "050100" => "中餐厅/综合餐厅",
                "050200" => "外国餐厅",
                "050300" => "快餐厅",
                "050400" => "休闲餐饮",
                "050500" => "咖啡厅",
                _ => "其他餐饮"
            };

            prompt.push_str(&format!(
                "{}. 【{}】{}\n   📍 地址：{}\n   🚶 距离：{}米\n   🏷️ 类型：{} ({})\n\n",
                i + 1, type_desc, name, address, distance, typecode, type_desc
            ));
        }
    }

    prompt.push_str("🎯 请基于以上信息提供专业分析和推荐：\n\n");
    prompt.push_str("**1. 商务聚餐推荐** (1-3家)\n");
    prompt.push_str("   - 选择标准：环境优雅、服务专业、适合商务交流\n");
    prompt.push_str("   - 请说明推荐理由和特色\n\n");
    
    prompt.push_str("**2. 学生经济餐厅推荐** (1-2家)\n");
    prompt.push_str("   - 选择标准：价格实惠、分量足够、营养均衡\n");
    prompt.push_str("   - 请说明性价比优势\n\n");
    
    prompt.push_str("**3. 地理位置分析**\n");
    prompt.push_str("   - 分析各餐厅的交通便利性\n");
    prompt.push_str("   - 评估距离用户位置的合理性\n");
    prompt.push_str("   - 考虑周边环境和配套设施\n\n");
    
    prompt.push_str("**4. 综合评价与建议** (100字以内)\n");
    prompt.push_str("   - 总结该区域餐饮特色\n");
    prompt.push_str("   - 给出最佳用餐时段建议\n\n");
    
    prompt.push_str("📝 **输出要求：**\n");
    prompt.push_str("- 使用清晰的结构化格式\n");
    prompt.push_str("- 语言专业但易懂，避免使用emoji表情\n");
    prompt.push_str("- 每个推荐都要有具体理由\n");
    prompt.push_str("- 考虑不同用户群体的需求差异\n");
    prompt.push_str("- 如果信息不足，请诚实说明并给出替代建议");

    prompt
}

// API请求结构体定义
#[derive(Deserialize)]
struct LocationRequest {
    location: String,
    city: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiRequest {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MapRequest {
    location: String,
    zoom: Option<u8>,
    size: Option<String>,
    markers: Option<Vec<String>>,
}

// API响应结构体定义
#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
    data: Option<Value>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MapResponse {
    status: String,
    map_url: Option<String>,
    location: Option<String>,
    coordinates: Option<(f64, f64)>,
    message: String,
    timestamp: String,
}

// 处理API请求的函数
#[post("/api/ai")]
async fn food_recommendation_api(
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
            return Ok(HttpResponse::BadRequest().json(ApiResponse {
                success: false,
                message: "Failed to get location coordinates".to_string(),
                data: None,
                error: Some(e.to_string()),
            }));
        }
    };
    
    // 搜索附近美食
    let food_data = match search_food(&client, &config_clone, location).await {
        Ok(data) => data,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: "Failed to search for food".to_string(),
                data: None,
                error: Some(e.to_string()),
            }));
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
    Ok(HttpResponse::Ok().json(ApiResponse {
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
    }))
}

// 只返回AI生成的内容API
#[post("/api/ai/content")]
async fn ai_content_only(
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
            return Ok(HttpResponse::BadRequest().body(format!("获取位置坐标失败: {}", e)));
        }
    };
    
    // 搜索附近美食
    let food_data = match search_food(&client, &config_clone, location).await {
        Ok(data) => data,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().body(format!("搜索美食失败: {}", e)));
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
        Some(content) => Ok(HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(content)),
        None => Ok(HttpResponse::InternalServerError().body("无法获取AI推荐内容"))
    }
}

// 地图API - 获取指定地点的静态地图
#[post("/api/map")]
async fn get_map_api(req: web::Json<MapRequest>, data: web::Data<AppState>) -> Result<HttpResponse, actix_web::Error> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    // 获取地点坐标
    let coordinates = match get_location_for_map(&data.client, &data.config, &req.location).await {
        Ok(coords) => coords,
        Err(e) => {
            let error_response = MapResponse {
                status: "error".to_string(),
                map_url: None,
                location: Some(req.location.clone()),
                coordinates: None,
                message: format!("获取地点坐标失败: {}", e),
                timestamp,
            };
            return Ok(HttpResponse::BadRequest().json(error_response));
        }
    };
    
    // 生成静态地图URL
    let map_url = generate_static_map_url(&data.config, coordinates, &req);
    
    let response = MapResponse {
        status: "success".to_string(),
        map_url: Some(map_url),
        location: Some(req.location.clone()),
        coordinates: Some(coordinates),
        message: "地图生成成功".to_string(),
        timestamp,
    };
    
    Ok(HttpResponse::Ok().json(response))
}

// 获取地点坐标的辅助函数
async fn get_location_for_map(client: &Client, config: &Config, location: &str) -> Result<(f64, f64), Box<dyn Error>> {
    let mut url = reqwest::Url::parse("https://restapi.amap.com/v3/assistant/inputtips")?;
    url.query_pairs_mut()
        .append_pair("key", &config.api_key)
        .append_pair("keywords", location)
        .append_pair("city", &config.city);

    let response = client.get(url.clone())
        .header("User-Agent", &format!("{}-map-service", config.username))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("地图定位API失败: {}", response.status()).into());
    }

    let body = response.text().await?;
    let data: Value = serde_json::from_str(&body)?;

    if let Some(tips) = data["tips"].as_array() {
        if tips.is_empty() {
            return Err("未找到相关地点".into());
        }

        // 获取第一个有效位置
        for tip in tips {
            if let Some(location) = tip["location"].as_str() {
                let coords: Vec<&str> = location.split(',').collect();
                if coords.len() == 2 {
                    let longitude = coords[0].parse::<f64>()?;
                    let latitude = coords[1].parse::<f64>()?;
                    return Ok((longitude, latitude));
                }
            }
        }
    }
    
    Err("无法解析地点坐标".into())
}

// 生成高德静态地图URL
fn generate_static_map_url(config: &Config, coordinates: (f64, f64), req: &MapRequest) -> String {
    let (longitude, latitude) = coordinates;
    
    // 默认参数
    let zoom = req.zoom.unwrap_or(15); // 默认缩放级别
    let size = req.size.as_deref().unwrap_or("400*300"); // 默认尺寸
    
    // 构建基础URL
    let mut url = format!(
        "https://restapi.amap.com/v3/staticmap?location={},{}&zoom={}&size={}&markers=mid,,A:{},{}&key={}",
        longitude, latitude, zoom, size, longitude, latitude, config.api_key
    );
    
    // 添加额外的标记点（如果有）
    if let Some(markers) = &req.markers {
        for marker in markers {
            url.push_str(&format!("&markers={}", marker));
        }
    }
    
    url
}

// 健康检查API
#[get("/health")]
async fn health_check() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "ok",
        "service": "food-recommendation-api",
        "version": "1.0.0",
        "timestamp": Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    })))
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
    println!("🗺️ 地图API: http://127.0.0.1:8080/api/map");
    println!("🩺 健康检查: http://127.0.0.1:8080/health");
    
    HttpServer::new(move || {
        // 配置 CORS
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")  // 允许前端域名
            .allowed_origin("http://127.0.0.1:5173") // 也允许 127.0.0.1
            .allowed_origin("http://121.40.25.117") // 允许前端域名
            .allowed_origin("http://localhost:3000")  // 常见的React开发端口
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)  // 应用 CORS 中间件
            .wrap(middleware::Logger::default())
            .service(food_recommendation_api)
            .service(ai_content_only)
            .service(get_map_api)
            .service(health_check)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}