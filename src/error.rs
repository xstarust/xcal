use std::fmt;

/// 天文历法错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalxError {
    /// 解析错误
    Parse { kind: &'static str, input: String },
    /// 值超出范围
    OutOfRange {
        param: &'static str,
        value: String,
        min: Option<String>,
        max: Option<String>,
    },
    /// 不支持的操作
    NotSupported(&'static str),
    /// 其他错误
    Other(String),
}

impl std::error::Error for CalxError {}

impl fmt::Display for CalxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalxError::Parse { kind, input } => {
                write!(f, "无法解析 {}: {}", kind, input)
            }
            CalxError::OutOfRange {
                param,
                value,
                min,
                max,
            } => {
                write!(f, "参数 {} 的值 {} 超出范围", param, value)?;
                if let Some(min) = min {
                    write!(f, " (最小值: {})", min)?;
                }
                if let Some(max) = max {
                    write!(f, " (最大值: {})", max)?;
                }
                Ok(())
            }
            CalxError::NotSupported(msg) => write!(f, "不支持的操作: {}", msg),
            CalxError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<std::string::FromUtf8Error> for CalxError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        CalxError::Other("UTF-8 解码错误".into())
    }
}

impl From<std::num::ParseIntError> for CalxError {
    fn from(err: std::num::ParseIntError) -> Self {
        CalxError::Other(format!("数字解析错误: {}", err))
    }
}

impl From<std::num::ParseFloatError> for CalxError {
    fn from(err: std::num::ParseFloatError) -> Self {
        CalxError::Other(format!("浮点数解析错误: {}", err))
    }
}
