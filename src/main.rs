use anyhow::{Context, Result};
use clap::Parser;
use log::{info, warn};
// 引入正确的库和类型
use tun_rs::{AsyncDevice, DeviceBuilder, Layer};

const DEFAULT_TAP_NAME: &str = "tap0";
const DEFAULT_MTU: i32 = 1500;

/// 生成一个随机的、本地管理的MAC地址。
fn generate_random_mac() -> [u8; 6] {
    let mut mac: [u8; 6] = rand::random();
    mac[0] |= 0x02; // 设置 "Locally Administered" 位
    mac[0] &= 0xfe; // 清除 "Multicast" 位
    mac
}

/// 将 "xx:xx:xx:xx:xx:xx" 格式的字符串解析为 [u8; 6]。
fn parse_mac_address(s: &str) -> Result<[u8; 6], String> {
    let parts: Vec<&str> = s.split(|c| c == ':' || c == '-').collect();
    if parts.len() != 6 {
        return Err(format!(
            "无效的MAC地址格式 '{}'。期望的格式是 xx:xx:xx:xx:xx:xx",
            s
        ));
    }
    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16)
            .map_err(|e| format!("无效的十六进制部分 '{}': {}", part, e))?;
    }
    Ok(mac)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// TAP 设备的名称
    #[arg(long, default_value = DEFAULT_TAP_NAME)]
    name: String,

    /// TAP 设备的 MTU (最大传输单元)
    #[arg(long, default_value_t = DEFAULT_MTU.try_into().unwrap())]
    mtu: u16,

    /// TAP 设备的 MAC 地址 (例如: 0a:0b:0c:0d:0e:0f)
    /// 如果未提供，将生成一个随机的本地管理地址。
    #[arg(long, value_parser = parse_mac_address)]
    mac: Option<[u8; 6]>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志记录器
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 解析命令行参数
    let cli = Cli::parse();

    // 确定要使用的MAC地址
    let node_mac = match cli.mac {
        Some(mac) => {
            let mac_str = mac
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":");
            info!("使用命令行提供的MAC地址: {}", mac_str);
            mac
        }
        None => {
            let mac = generate_random_mac();
            let mac_str = mac
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":");
            warn!("未提供MAC地址，已生成随机地址: {}", mac_str);
            mac
        }
    };

    info!("正在创建TAP设备...");
    info!("  名称: {}", cli.name);
    info!("  MTU: {}", cli.mtu);

    // 使用 `tun` 库的 Device::builder()
    let mut builder = DeviceBuilder::new()
        .name(cli.name)
        .mac_addr(node_mac)
        .layer(Layer::L2)
        .mtu(cli.mtu);

    let device = builder.build_async().context("创建TAP设备失败")?;
    info!("TAP设备 '{}' 创建成功!", device.name()?);
    info!("设备已启动，按 Ctrl+C 退出。");

    // 等待终止信号 (Ctrl+C)
    tokio::signal::ctrl_c().await?;

    info!("接收到终止信号，正在关闭程序...");

    // 当 `device` 变量离开作用域时，`tun` 库的 Drop 实现会自动清理和关闭设备。
    // 无需手动关闭。

    Ok(())
}
