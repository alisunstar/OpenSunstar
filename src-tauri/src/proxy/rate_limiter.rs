//! 滑动窗口速率限制器
//!
//! 用于代理服务器的请求速率控制，防止本地进程滥用 API 配额。
//! 使用 Mutex 保护的滑动窗口计数器，每秒最多 `max_requests` 个请求。
//! 在 localhost 场景下 Mutex 争用极低，性能开销可忽略。

use std::sync::Mutex;
use std::time::Instant;

/// 滑动窗口速率限制器
pub struct RateLimiter {
    inner: Mutex<RateLimiterInner>,
    max_per_second: u32,
}

struct RateLimiterInner {
    window_start: Instant,
    count: u32,
}

impl RateLimiter {
    /// 创建速率限制器
    ///
    /// `max_per_second` — 每秒最大允许请求数
    pub fn new(max_per_second: u32) -> Self {
        Self {
            inner: Mutex::new(RateLimiterInner {
                window_start: Instant::now(),
                count: 0,
            }),
            max_per_second,
        }
    }

    /// 尝试获取一个请求许可
    ///
    /// 返回 `true` 表示允许，`false` 表示被限流
    pub fn try_acquire(&self) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return true, // Mutex poisoned — fail open to avoid blocking all traffic
        };

        let now = Instant::now();
        let elapsed = now.duration_since(inner.window_start);

        // 窗口过期 — 重置计数器
        if elapsed.as_secs() >= 1 {
            inner.window_start = now;
            inner.count = 1;
            return true;
        }

        // 窗口内 — 检查是否超限
        if inner.count < self.max_per_second {
            inner.count += 1;
            true
        } else {
            false
        }
    }

    /// 当前窗口内已用请求数（用于状态查询）
    pub fn current_count(&self) -> u32 {
        self.inner.lock().map(|g| g.count).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_allows_within_limit() {
        let limiter = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(limiter.try_acquire());
        }
    }

    #[test]
    fn test_blocks_over_limit() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire()); // 4th should be blocked
    }

    #[test]
    fn test_resets_after_window() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire()); // blocked

        thread::sleep(Duration::from_millis(1100)); // wait for window to expire

        assert!(limiter.try_acquire()); // should be allowed again
    }

    #[test]
    fn test_current_count() {
        let limiter = RateLimiter::new(10);
        assert_eq!(limiter.current_count(), 0);
        limiter.try_acquire();
        limiter.try_acquire();
        assert_eq!(limiter.current_count(), 2);
    }
}
