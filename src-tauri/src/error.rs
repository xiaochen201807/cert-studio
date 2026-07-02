use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("Rcgen 证书生成错误: {0}")]
    Rcgen(#[from] rcgen::Error),

    #[error("Keyring 密钥环错误: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("JSON 序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("X509 解析错误: {0}")]
    X509(String),

    #[error("业务逻辑错误: {0}")]
    Custom(String),
}

// 必须为 AppError 实现 serde::Serialize，才能把错误回传给 Tauri 前端
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
