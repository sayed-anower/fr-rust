error[E0599]: the method `as_dyn_error` exists for reference `&argon2::password_hash::Error`, but its trait bounds were not satisfied
  --> src/crypto/crypto.rs:40:14
   |
40 |     Argon2(#[from] argon2::password_hash::Error),
   |              ^^^^ method cannot be called on `&argon2::password_hash::Error` due to unsatisfied trait bounds
   |
  ::: /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/password-hash-0.5.0/src/errors.rs:14:1
   |
14 | pub enum Error {
   | -------------- doesn't satisfy `_: AsDynError<'_>` or `argon2::password_hash::Error: StdError`
   |
   = note: the following trait bounds were not satisfied:
           `argon2::password_hash::Error: StdError`
           which is required by `argon2::password_hash::Error: thiserror::__private18::AsDynError<'_>`
           `&argon2::password_hash::Error: StdError`
           which is required by `&argon2::password_hash::Error: thiserror::__private18::AsDynError<'_>`

warning: unused variable: `content_disposition`
  --> src/res/res.rs:55:13
   |
55 |         let content_disposition = field.content_disposition();
   |             ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_content_disposition`
   |
   = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/linkv/linkv.rs:20:58
   |
20 |     pub async fn generate_token(&self, user_id: &str) -> anyhow::Result<String> {
   |                                                          ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/linkv/linkv.rs:34:69
   |
34 |     pub async fn verify_token(&self, user_id: &str, token: &str) -> anyhow::Result<bool> {
   |                                                                     ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/otp/otp.rs:23:69
   |
23 |     pub async fn generate_otp(&self, user_id: &str, digits: u32) -> anyhow::Result<String> {
   |                                                                     ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/otp/otp.rs:36:65
   |
36 |     pub async fn verify_otp(&self, user_id: &str, otp: &str) -> anyhow::Result<bool> {
   |                                                                 ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/redis/redis.rs:14:30
   |
14 |     pub fn new(url: &str) -> anyhow::Result<Self> {
   |                              ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/redis/redis.rs:24:43
   |
24 |     pub async fn get_connection(&self) -> anyhow::Result<Connection> {
   |                                           ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/redis/redis.rs:30:68
   |
30 |    pub async fn publish(&self, event_name: &str, content: &str) -> anyhow::Result<()> {
   |                                                                    ^^^^^^ use of unresolved module or unlinked crate `anyhow`
   |
   = help: if you wanted to use a crate named `anyhow`, use `cargo add anyhow` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src/redis/redis.rs:38:56
   |
38 |     pub async fn subscribe(&self, event_name: &str) -> anyhow::Result<redis::aio::PubSubStream> {
   |                                                        ^^^^^^ use of unresolved module or unlinked crate `anyhow`