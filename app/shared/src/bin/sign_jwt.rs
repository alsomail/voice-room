//! sign-jwt CLI（T-0000G）
//!
//! 仅供 E2E Seed 脚本使用，**禁止**进入生产路径。
//!
//! 用法：
//!   sign-jwt --sub <uuid> --role <user|admin|op|cs|fin> --ttl <seconds>
//!     → 从 env JWT_SECRET 读密钥，签发 JWT，stdout 输出单行 token。
//!     - role=user        → AppClaims   iss="voiceroom"
//!     - role=admin/op/cs/fin → AdminClaims iss="voiceroom-admin"
//!       admin→super_admin, op→operator, cs→cs, fin→finance
//!
//!   sign-jwt --uuid5 <name>
//!     → 在 E2E 命名空间下计算 UUIDv5，stdout 输出单行 uuid。
//!       (E2E_NS = 9b3e0c6a-1ec1-4d3f-93d4-e2e000000000)
//!
//! 退出码：0 成功；2 入参错误；3 缺少 JWT_SECRET；4 签发失败。
//! 安全：永不 echo JWT_SECRET 本身；错误信息只走 stderr。

use std::env;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;
use voice_room_shared::jwt::token::{encode_token, AdminClaims, AppClaims};

const E2E_NS: Uuid = Uuid::from_bytes([
    0x9b, 0x3e, 0x0c, 0x6a, 0x1e, 0xc1, 0x4d, 0x3f, 0x93, 0xd4, 0xe2, 0xe0, 0x00, 0x00, 0x00, 0x00,
]);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

fn usage() -> &'static str {
    "usage:\n  sign-jwt --sub <uuid> --role <user|admin|op|cs|fin> --ttl <seconds>\n  sign-jwt --uuid5 <name>"
}

fn parse_kv_args(args: &[String]) -> Result<std::collections::HashMap<String, String>, String> {
    let mut map = std::collections::HashMap::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(stripped) = a.strip_prefix("--") {
            let key = stripped.to_string();
            i += 1;
            if i >= args.len() {
                return Err(format!("missing value for --{}", key));
            }
            map.insert(key, args[i].clone());
            i += 1;
        } else {
            return Err(format!("unexpected arg: {}", a));
        }
    }
    Ok(map)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.is_empty() {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    }
    let kv = match parse_kv_args(&argv) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("sign-jwt: {}\n{}", e, usage());
            return ExitCode::from(2);
        }
    };

    // --uuid5 模式
    if let Some(name) = kv.get("uuid5") {
        let id = Uuid::new_v5(&E2E_NS, name.as_bytes());
        println!("{}", id);
        return ExitCode::SUCCESS;
    }

    // --sub/--role/--ttl 签发模式
    let sub = match kv.get("sub") {
        Some(v) => v.clone(),
        None => {
            eprintln!("sign-jwt: --sub is required\n{}", usage());
            return ExitCode::from(2);
        }
    };
    let role = match kv.get("role") {
        Some(v) => v.clone(),
        None => {
            eprintln!("sign-jwt: --role is required\n{}", usage());
            return ExitCode::from(2);
        }
    };
    let ttl: u64 = match kv.get("ttl").map(|s| s.parse::<u64>()) {
        Some(Ok(v)) => v,
        Some(Err(_)) => {
            eprintln!("sign-jwt: --ttl must be a non-negative integer");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("sign-jwt: --ttl is required\n{}", usage());
            return ExitCode::from(2);
        }
    };

    let secret = match env::var("JWT_SECRET") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("sign-jwt: JWT_SECRET env is required (do not echo it)");
            return ExitCode::from(3);
        }
    };

    let iat = now_secs();
    let exp = iat.saturating_add(ttl);

    let token = match role.as_str() {
        "user" => {
            let claims = AppClaims {
                sub,
                iss: "voiceroom".into(),
                exp,
                iat,
            };
            encode_token(&claims, secret.as_bytes())
        }
        "admin" | "op" | "cs" | "fin" => {
            let mapped = match role.as_str() {
                "admin" => "super_admin",
                "op" => "operator",
                "cs" => "cs",
                "fin" => "finance",
                _ => unreachable!(),
            };
            let claims = AdminClaims {
                sub,
                role: mapped.into(),
                iss: "voiceroom-admin".into(),
                exp,
                iat,
            };
            encode_token(&claims, secret.as_bytes())
        }
        other => {
            eprintln!("sign-jwt: unknown role '{}'\n{}", other, usage());
            return ExitCode::from(2);
        }
    };

    match token {
        Ok(t) => {
            println!("{}", t);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sign-jwt: encode failed: {}", e);
            ExitCode::from(4)
        }
    }
}
