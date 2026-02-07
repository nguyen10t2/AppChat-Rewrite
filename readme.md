# App Chat Rust

Dự án này là một ứng dụng chat backend được xây dựng bằng Rust, sử dụng framework Actix-web.

## Giới thiệu

Đây là một API backend cho một ứng dụng chat thời gian thực, cung cấp các chức năng cơ bản như đăng ký, đăng nhập, quản lý bạn bè, tạo cuộc trò chuyện và gửi tin nhắn.

## Tính năng

-   **Xác thực người dùng**: Đăng ký và đăng nhập bằng email và mật khẩu.
-   **Quản lý bạn bè**: Gửi, chấp nhận, từ chối lời mời kết bạn.
-   **Trò chuyện**: Tạo cuộc trò chuyện trực tiếp (1-1) và nhóm.
-   **Nhắn tin**: Gửi và nhận tin nhắn trong các cuộc trò chuyện.
-   **Bảo mật**: Sử dụng JSON Web Tokens (JWT) để xác thực và phân quyền.

## Công nghệ sử dụng

-   **Ngôn ngữ**: [Rust](https://www.rust-lang.org/)
-   **Framework**: [Actix-web](https://actix.rs/)
-   **Cơ sở dữ liệu**: [PostgreSQL](https://www.postgresql.org/)
-   **ORM/Query Builder**: [SQLx](https://github.com/launchbadge/sqlx)
-   **Caching**: [Redis](https://redis.io/)
-   **Xác thực**: [JSON Web Tokens (JWT)](https://jwt.io/)
-   **Mã hóa mật khẩu**: [Argon2](https://en.wikipedia.org/wiki/Argon2)

## Bắt đầu

### Yêu cầu

-   Rust (phiên bản 1.56 trở lên)
-   PostgreSQL
-   Redis
-   `sqlx-cli` để chạy migrations

### Cài đặt

1.  Clone a repository:

    ```sh
    git clone <repository-url>
    cd AppChatRust/backend
    ```

2.  Sao chép file `.env.example` thành `.env` và cấu hình các biến môi trường:

    ```sh
    cp .env.example .env
    ```

    Cập nhật các thông tin sau trong file `.env`:

    ```env
    DATABASE_URL=postgres://user:password@localhost/database_name
    REDIS_URL=redis://127.0.0.1/
    JWT_SECRET=your_jwt_secret
    ```

3.  Chạy database migrations:

    ```sh
    sqlx migrate run
    ```

### Chạy ứng dụng

```sh
cargo run
```

Ứng dụng sẽ chạy tại `http://localhost:8080`.

## API Endpoints

Dưới đây là một số endpoints chính:

### Public Endpoints (Không yêu cầu xác thực)

-   `GET /chao`: Endpoint chào mừng, trả về thông tin chào mừng bằng tiếng Việt và tiếng Anh.
-   `POST /api/auth/register`: Đăng ký người dùng mới.
-   `POST /api/auth/login`: Đăng nhập.

### Private Endpoints (Yêu cầu xác thực)

-   `GET /api/users`: Lấy danh sách người dùng.
-   `POST /api/friends/request`: Gửi lời mời kết bạn.
-   `POST /api/conversations/`: Tạo cuộc trò chuyện mới.
-   `GET /api/conversations/{id}/messages`: Lấy tin nhắn trong cuộc trò chuyện.
-   `POST /api/messages/direct`: Gửi tin nhắn trực tiếp.
-   `POST /api/messages/group`: Gửi tin nhắn nhóm.

## Đóng góp

Mọi đóng góp đều được chào đón. Vui lòng tạo một Pull Request để đóng góp.

## Giấy phép

Dự án này được cấp phép theo giấy phép MIT.
