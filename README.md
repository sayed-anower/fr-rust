# ⚡ FR-RUST

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**FR-RUST** is an ultra-fast, developer-friendly web backend framework for Rust. Built on top of the robust ecosystem of Actix (one of the fastest web servers available), FR-RUST strips away the steep learning curve and verbose boilerplate. It provides an elegant, expressive, and simplified syntax without introducing any performance overhead.

---

## 🚀 Features

FR-RUST comes pre-packaged with out-of-the-box batteries to accelerate your backend development:

- **Minimalist Syntax:** Drastically reduced boilerplate with zero performance compromises.
- **Built-in Email Service:** Seamlessly connect and dispatch transactional emails.
- **OTP Service:** Native generation and lifecycle management for One-Time Passwords.
- **Advanced DB Management:** Simplified PostgreSQL/Database connection pooling and queries.
- **Crypto & Security Suite:** Effortless AES text encryption/decryption and hashing operations.
- **Expressive Responses:** Clean helpers for JSON, Strings, and HTTP status handling.
- **Redis Integration:** Fully integrated asynchronous caching and operations manager.
- **WebSockets:** Blazing-fast real-time bidirectional communication.

---

## ⚠️ Architectural Considerations

- **WebSocket Clustering:** Currently, WebSockets operate in-memory on a single node and do not natively sync across multi-server setups out-of-the-box. However, a single node is highly optimized and capable of handling millions of concurrent actions/users per second.
- **Opinionated Control:** To offer maximum developer velocity and an effortless learning curve, some low-level configurations are abstracted. Advanced users might occasionally feel a slight loss of granular underlying control compared to raw Actix.