use actix_web::{web, FromRequest};
use argon2::{
    password_hash::{Error as PasswordHashError, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{de::Deserializer, Deserialize, Serialize};
use validator::Validate;

use crate::{api::error, modules::user::schema::UserRole};

lazy_static::lazy_static! {
  static ref ARGON2: Argon2<'static> = Argon2::default();
}

pub fn hash_password(password: &str) -> Result<String, error::SystemError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = ARGON2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> Result<bool, error::SystemError> {
    let parsed_hash = PasswordHash::new(hash)?;
    match ARGON2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(PasswordHashError::Password) => Ok(false),
        Err(e) => Err(error::SystemError::HashError(e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TypeClaims {
    RefreshToken,
    AccessToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: uuid::Uuid,
    pub iat: u64,
    pub exp: u64,
    pub jti: Option<uuid::Uuid>,
    pub role: UserRole,
    pub _type: Option<TypeClaims>,
}

impl Claims {
    pub fn new(sub: &uuid::Uuid, role: &UserRole, exp: u64) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Claims { sub: *sub, iat: now, exp: now + exp, role: role.clone(), jti: None, _type: None }
    }

    pub fn with_jti(mut self, jti: uuid::Uuid) -> Self {
        self.jti = Some(jti);
        self
    }

    pub fn with_type(mut self, _type: TypeClaims) -> Self {
        self._type = Some(_type);
        self
    }

    pub fn encode(&self, secret: &[u8]) -> Result<String, error::SystemError> {
        let header = Header::new(Algorithm::HS256);
        let token = encode(&header, self, &EncodingKey::from_secret(secret))?;
        Ok(token)
    }

    #[allow(unused)]
    pub fn decode(token: &str, secret: &[u8]) -> Result<Self, error::SystemError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = false;
        let token_data = decode::<Self>(token, &DecodingKey::from_secret(secret), &validation)?;
        Ok(token_data.claims)
    }
}

pub fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(de).map(Some)
}
pub struct ValidatedJson<T>(pub T);

impl<T> FromRequest for ValidatedJson<T>
where
    T: Validate + serde::de::DeserializeOwned + 'static,
{
    type Error = error::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let fut = web::Json::<T>::from_request(req, payload);

        Box::pin(async move {
            let json = fut.await.map_err(|e| error::Error::BadRequest(e.to_string().into()))?;
            let model = json.into_inner();
            model.validate().map_err(|e| error::Error::BadRequest(e.to_string().into()))?;
            Ok(ValidatedJson(model))
        })
    }
}

pub struct ValidatedQuery<T>(pub T);

impl<T> FromRequest for ValidatedQuery<T>
where
    T: Validate + serde::de::DeserializeOwned + 'static,
{
    type Error = error::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let fut = web::Query::<T>::from_request(req, payload);

        Box::pin(async move {
            let query = fut.await.map_err(|e| error::Error::BadRequest(e.to_string().into()))?;
            query.validate().map_err(|e| error::Error::BadRequest(e.to_string().into()))?;
            Ok(ValidatedQuery(query.into_inner()))
        })
    }
}
