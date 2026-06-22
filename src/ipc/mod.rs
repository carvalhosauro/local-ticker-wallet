use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Action {
    AddTransaction,
    ListTransactions,
    DeleteTransaction,
    GetPositions,
    GetPositionDetail,
    RefreshNow,
    Search,
    GetQuote,
    Import,
    Export,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub action: Action,
    pub payload: serde_json::Value,
}

impl Request {
    pub fn new(action: Action, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "request".into(),
            action,
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    NotFound,
    ProviderDown,
    BadRequest,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Response {
    Ok { id: String, data: serde_json::Value },
    Error { id: String, error: ErrorBody },
}

impl Response {
    pub fn ok(id: String, data: serde_json::Value) -> Self {
        Response::Ok { id, data }
    }

    pub fn err(id: String, code: ErrorCode, message: String) -> Self {
        Response::Error {
            id,
            error: ErrorBody { code, message },
        }
    }
}

pub async fn write_msg<W: AsyncWrite + Unpin, T: Serialize>(
    w: &mut W,
    msg: &T,
) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    w.write_all(line.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

pub async fn read_line<R: AsyncBufRead + Unpin>(r: &mut R) -> anyhow::Result<Option<String>> {
    let mut buf = String::new();
    let n = r.read_line(&mut buf).await?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(buf.trim_end().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_envelope_shape() {
        let req = Request::new(Action::Ping, serde_json::json!({}));
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "request");
        assert_eq!(v["action"], "Ping");
        assert!(v["id"].as_str().unwrap().len() >= 8);
    }

    #[test]
    fn ok_response_shape() {
        let r = Response::ok("abc".into(), serde_json::json!({"pong": true}));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["id"], "abc");
        assert_eq!(v["status"], "ok");
        assert_eq!(v["data"]["pong"], true);
    }

    #[test]
    fn err_response_shape() {
        let r = Response::err("abc".into(), ErrorCode::NotFound, "missing".into());
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["status"], "error");
        assert_eq!(v["error"]["code"], "NOT_FOUND");
        assert_eq!(v["error"]["message"], "missing");
    }
}
