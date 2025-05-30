use serde::{Deserialize, Serialize};
use reqwest::blocking::Client;
use std::{error::Error, fs, time::Duration, process};
use chrono::Local;
use serde_json::{Value, json};
use colored::*;

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
            api_key: "".to_string(),
            output_file: "response_log.json".to_string(),
            food_radius: 1000,
            food_types: "050000".to_string(),
            max_food_results: 5,
            qwen_api_key: "".to_string(),
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

fn get_location(client: &Client, config: &Config) -> Result<(f64, f64), Box<dyn Error>> {
    let mut url = reqwest::Url::parse("https://restapi.amap.com/v3/assistant/inputtips")?;
    url.query_pairs_mut()
        .append_pair("key", &config.api_key)
        .append_pair("keywords", &config.keywords)
        .append_pair("city", &config.city);

    println!("🔍 查询地点坐标: {}", config.keywords.green());

    let response = client.get(url.clone())
        .header("User-Agent", &format!("{}-geo-service", config.username))
        .send()?;

    if !response.status().is_success() {
        return Err(format!("定位API失败: {}", response.status()).into());
    }

    let body = response.text()?;
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

fn search_food(client: &Client, config: &Config, location: (f64, f64)) -> Result<Value, Box<dyn Error>> {
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
        .send()?;

    if !response.status().is_success() {
        return Err(format!("美食搜索API失败: {}", response.status()).into());
    }

    let body = response.text()?;
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

fn ask_qwen(prompt: &str, config: &Config) -> Result<String, Box<dyn Error>> {
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
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text()?;
        return Err(format!("AI调用失败 ({}): {}", status, body).into());
    }

    let response_body = response.text()?;
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

fn generate_ai_prompt(food_data: &Value, config: &Config) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "用户位置：{}（{}）\n",
        config.keywords, config.city
    ));
    prompt.push_str(&format!(
        "搜索范围：半径{}米\n\n",
        config.food_radius
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

fn main() {
    println!("\n{}{}", "🗺️ 智能地理分析系统 ".bold().blue(), "v2.0".yellow());
    println!("{}", "=".repeat(40).dimmed());
    println!("{}", "集成高德地图API + 通义千问AI".bold());

    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("❌ 配置加载失败: {}", e);
            process::exit(1);
        }
    };

    println!("\n🔧 配置加载成功");
    println!("👤 用户: {}", config.username.green());
    println!("📍 目标地点: {}", config.keywords.green());
    println!("🏙️ 城市: {}", config.city.green());
    println!("🤖 AI模型: {}", config.qwen_model.green());

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

    // 第一步：获取地点坐标
    let location = match get_location(&client, &config) {
        Ok(loc) => loc,
        Err(e) => {
            println!("❌ 地点查询失败: {}", e);
            println!("💡 建议: 检查地点名称是否正确或尝试更换关键词");
            process::exit(1);
        }
    };

    // 第二步：搜索附近美食
    let food_data = match search_food(&client, &config, location) {
        Ok(data) => data,
        Err(e) => {
            println!("❌ 美食搜索失败: {}", e);
            process::exit(1);
        }
    };

    println!("{}", format_food_results(&food_data));

    // 第三步：调用AI进行分析
    let ai_prompt = generate_ai_prompt(&food_data, &config);

    // 尝试使用兼容模式
    println!("\n🚀 尝试兼容模式调用...");
    match ask_qwen(&ai_prompt, &config) {
        Ok(ai_response) => {
            println!("\n{}", "🌟 AI美食推荐分析:".bold().purple());
            println!("{}", ai_response);
        }
        Err(e) => {
            println!("❌ AI分析失败: {}", e);
            println!("💡 尝试备用方案: 使用qwen-turbo模型");

            // 尝试备用模型
            let mut backup_config = config.clone();
            backup_config.qwen_model = "qwen-turbo".to_string();

            match ask_qwen(&ai_prompt, &backup_config) {
                Ok(ai_response) => {
                    println!("\n{}", "🌟 AI美食推荐分析(备用模型):".bold().purple());
                    println!("{}", ai_response);
                }
                Err(e) => {
                    println!("❌ 备用模型也失败: {}", e);
                }
            }
        }
    }

    // 保存结果到文件
    let output = json!({
        "search_time": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "config": config,
        "food_results": food_data,
        "ai_prompt": ai_prompt
    });

    if let Err(e) = fs::write(&config.output_file, serde_json::to_string_pretty(&output).unwrap()) {
        println!("⚠️ 结果保存失败: {}", e);
    } else {
        println!("\n💾 完整结果已保存到: {}", config.output_file.green());
    }

    println!("\n⏰ 执行完成时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S").to_string().cyan());
}