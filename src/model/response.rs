use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BaseResponse {
    pub rc: i32,
    pub rd: String,
}

impl BaseResponse {
    pub fn new(rc: i32, rd: String) -> Self {
        BaseResponse { rc, rd }
    }

    pub fn fails(rd: String) -> Self {
        BaseResponse { rc: -1, rd }
    }

    pub fn success(rd: String) -> Self {
        BaseResponse { rc: 0, rd }
    }

    pub fn set_result(&mut self, rc: i32, rd: String) {
        self.rc = rc;
        self.rd = rd;
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BaseResponseWithRequestId {
    pub rc: i32,
    pub rd: String,
    pub req_id: Option<String>,
}

impl BaseResponseWithRequestId {
    pub fn new(resp: BaseResponse, req_id: Option<String>) -> Self {
        BaseResponseWithRequestId {
            rc: resp.rc,
            rd: resp.rd,
            req_id,
        }
    }
}

impl Display for BaseResponseWithRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = json!({
            "rc": self.rc,
            "rd": self.rd,
            "req_id": self.req_id
        });
        write!(f, "{}", r.to_string())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EntityBaseResponse<T> {
    pub rc: i32,
    pub rd: String,
    pub data: Option<T>,
}

impl<T> EntityBaseResponse<T> {
    pub fn new(rc: i32, rd: String) -> Self {
        EntityBaseResponse { rc, rd, data: None }
    }

    pub fn fails(rd: String) -> Self {
        EntityBaseResponse {
            rc: -1,
            rd,
            data: None,
        }
    }

    pub fn fails_with_response(response: BaseResponse) -> Self {
        EntityBaseResponse {
            rc: response.rc,
            rd: response.rd,
            data: None,
        }
    }

    pub fn success(rd: String, data: Option<T>) -> Self {
        EntityBaseResponse { rc: 0, rd, data }
    }
}

impl<T> IntoResponse for EntityBaseResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ListResponse<T> {
    pub rc: i32,
    pub rd: String,
    pub data: Option<Vec<T>>,
    pub total: Option<i64>,
}

impl<T> ListResponse<T> {
    pub fn new(rc: i32, rd: String, data: Option<Vec<T>>, total: Option<i64>) -> Self {
        ListResponse {
            rc,
            rd,
            data,
            total,
        }
    }

    pub fn fail(rd: String) -> Self {
        ListResponse {
            rc: -1,
            rd,
            data: Some(vec![]),
            total: Some(0),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BaseListResponseWithRequestId<T> {
    pub rc: i32,
    pub rd: String,
    pub data: Option<Vec<T>>,
    pub total: Option<i64>,
    pub req_id: Option<String>,
}

impl<T> BaseListResponseWithRequestId<T> {
    pub fn new(resp: ListResponse<T>, req_id: Option<String>) -> Self {
        BaseListResponseWithRequestId {
            rc: resp.rc,
            rd: resp.rd,
            data: resp.data,
            total: resp.total,
            req_id,
        }
    }
}
