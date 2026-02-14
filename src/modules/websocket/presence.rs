/// Presence Service
///
/// Module quản lý trạng thái online/offline của users sử dụng Redis.
/// Lấy cảm hứng từ kiến trúc của Messenger/Instagram:
///
/// - Sử dụng Redis SET với TTL cho trạng thái online (ephemeral, không ghi DB)
/// - Heartbeat refresh TTL mỗi 15s, TTL = 60s → tự động offline nếu mất kết nối
/// - Lưu `last_seen` timestamp khi user offline (persistent trong Redis, không có TTL)
/// - Pipeline batch queries cho hiệu năng khi query nhiều users
///
/// Redis key schema:
/// - `presence:{user_id}` → "1" (TTL 60s) - user đang online
/// - `last_seen:{user_id}` → ISO 8601 timestamp - thời điểm offline cuối cùng
use deadpool_redis::redis::{self, AsyncCommands};
use uuid::Uuid;

use crate::api::error;

/// TTL cho presence key (giây). Được refresh mỗi HEARTBEAT_INTERVAL (15s).
/// Nếu client mất kết nối mà server không nhận được disconnect,
/// key sẽ tự expire sau 60s.
const PRESENCE_TTL: u64 = 60;

const PRESENCE_PREFIX: &str = "presence:";
const LAST_SEEN_PREFIX: &str = "last_seen:";

/// Service quản lý presence state trong Redis
#[derive(Clone)]
pub struct PresenceService {
    pool: deadpool_redis::Pool,
}

impl PresenceService {
    /// Tạo PresenceService mới với Redis pool
    pub fn new(pool: deadpool_redis::Pool) -> Self {
        Self { pool }
    }

    /// Đánh dấu user online: SET presence:{user_id} = "1" với TTL
    pub async fn set_online(&self, user_id: Uuid) -> Result<(), error::SystemError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{PRESENCE_PREFIX}{user_id}");
        conn.set_ex::<_, _, ()>(&key, "1", PRESENCE_TTL).await?;
        Ok(())
    }

    /// Đánh dấu user offline: xóa presence key, lưu last_seen timestamp
    pub async fn set_offline(&self, user_id: Uuid) -> Result<(), error::SystemError> {
        let mut conn = self.pool.get().await?;
        let presence_key = format!("{PRESENCE_PREFIX}{user_id}");
        let last_seen_key = format!("{LAST_SEEN_PREFIX}{user_id}");
        let now = chrono::Utc::now().to_rfc3339();

        // Pipeline: xóa presence + set last_seen trong 1 round-trip
        redis::pipe()
            .del(&presence_key)
            .set(&last_seen_key, &now)
            .query_async::<()>(&mut *conn)
            .await?;

        Ok(())
    }

    /// Refresh TTL cho presence key (gọi mỗi heartbeat interval)
    pub async fn refresh_presence(&self, user_id: Uuid) -> Result<(), error::SystemError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{PRESENCE_PREFIX}{user_id}");
        conn.expire::<_, bool>(&key, PRESENCE_TTL as i64).await?;
        Ok(())
    }

    /// Kiểm tra 1 user có online không
    pub async fn is_online(&self, user_id: Uuid) -> Result<bool, error::SystemError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{PRESENCE_PREFIX}{user_id}");
        let exists: bool = conn.exists(&key).await?;
        Ok(exists)
    }

    /// Batch query trạng thái online/offline + last_seen cho nhiều users.
    /// Sử dụng Redis pipeline để giảm round-trips.
    ///
    /// Returns: Vec<(user_id, is_online, last_seen)>
    pub async fn get_online_status_batch(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<PresenceInfo>, error::SystemError> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.pool.get().await?;

        // Step 1: Pipeline EXISTS cho tất cả users
        let mut pipe = redis::pipe();
        for user_id in user_ids {
            pipe.exists(format!("{PRESENCE_PREFIX}{user_id}"));
        }
        let online_flags: Vec<bool> = pipe.query_async(&mut *conn).await?;

        // Step 2: Pipeline GET last_seen cho offline users
        let offline_indices: Vec<usize> = online_flags
            .iter()
            .enumerate()
            .filter(|(_, &is_online)| !is_online)
            .map(|(i, _)| i)
            .collect();

        let last_seens: Vec<Option<String>> = if !offline_indices.is_empty() {
            let mut ls_pipe = redis::pipe();
            for &idx in &offline_indices {
                ls_pipe.get(format!("{LAST_SEEN_PREFIX}{}", user_ids[idx]));
            }
            ls_pipe.query_async(&mut *conn).await?
        } else {
            vec![]
        };

        // Step 3: Combine results
        let mut results = Vec::with_capacity(user_ids.len());
        let mut ls_idx = 0;

        for (i, user_id) in user_ids.iter().enumerate() {
            let is_online = online_flags[i];
            let last_seen = if !is_online && ls_idx < last_seens.len() {
                let ls = last_seens[ls_idx].clone();
                ls_idx += 1;
                ls
            } else {
                None
            };

            results.push(PresenceInfo {
                user_id: *user_id,
                is_online,
                last_seen,
            });
        }

        Ok(results)
    }

    /// Lấy last_seen của 1 user
    pub async fn get_last_seen(
        &self,
        user_id: Uuid,
    ) -> Result<Option<String>, error::SystemError> {
        let mut conn = self.pool.get().await?;
        let key = format!("{LAST_SEEN_PREFIX}{user_id}");
        let last_seen: Option<String> = conn.get(&key).await?;
        Ok(last_seen)
    }
}

/// Thông tin presence của 1 user
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PresenceInfo {
    pub user_id: Uuid,
    pub is_online: bool,
    pub last_seen: Option<String>,
}
