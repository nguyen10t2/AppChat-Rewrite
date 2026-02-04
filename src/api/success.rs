#![allow(unused)]
use actix_web::HttpResponse;
use std::borrow::Cow;

#[derive(serde::Serialize)]
pub struct SuccessData<T: serde::Serialize> {
    pub data: Option<T>,
    pub message: Option<Cow<'static, str>>,
}

pub struct Success<T: serde::Serialize> {
    pub status: actix_web::http::StatusCode,
    pub body: Option<SuccessData<T>>,
    pub cookies: Vec<actix_web::cookie::Cookie<'static>>,
}

impl<T: serde::Serialize> Success<T> {
    pub fn ok(data: Option<T>) -> Self {
        Self {
            status: actix_web::http::StatusCode::OK,
            body: Some(SuccessData { data, message: None }),
            cookies: Vec::new(),
        }
    }

    pub fn message<M>(mut self, msg: M) -> Self
    where
        M: Into<Cow<'static, str>>,
    {
        if let Some(body) = &mut self.body {
            body.message = Some(msg.into());
        }
        self
    }

    pub fn cookies(mut self, cookies: Vec<actix_web::cookie::Cookie<'static>>) -> Self {
        self.cookies = cookies;
        self
    }

    pub fn created(data: Option<T>) -> Self {
        Self {
            status: actix_web::http::StatusCode::CREATED,
            body: Some(SuccessData { data, message: None }),
            cookies: Vec::new(),
        }
    }

    pub fn no_content() -> Self {
        Self { status: actix_web::http::StatusCode::NO_CONTENT, body: None, cookies: Vec::new() }
    }
}

impl<T: serde::Serialize> actix_web::Responder for Success<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let mut response = HttpResponse::build(self.status);

        for cookie in self.cookies {
            response.cookie(cookie);
        }

        match self.body {
            Some(body) => response.json(body),
            None => response.finish(),
        }
    }
}
