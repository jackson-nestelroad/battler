use std::{
    net::IpAddr,
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicUsize,
            Ordering,
        },
    },
};

use ahash::HashMap;
use thiserror::Error;

/// Error returned when a connection limit is reached.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ConnectionLimitError {
    #[error("global connection limit reached")]
    GlobalLimitReached,

    #[error("per-IP connection limit reached")]
    PerIpLimitReached,
}

#[derive(Debug)]
struct ConnectionTrackerInner {
    max_connections: Option<usize>,
    max_connections_per_ip: Option<usize>,
    exempt_loopback: bool,
    active_connections: AtomicUsize,
    connections_per_ip: Mutex<HashMap<IpAddr, usize>>,
}

impl ConnectionTrackerInner {
    fn lock_map(&self) -> std::sync::MutexGuard<'_, HashMap<IpAddr, usize>> {
        match self.connections_per_ip.lock() {
            Ok(m) => m,
            Err(p) => p.into_inner(),
        }
    }

    fn release(&self, ip: IpAddr, exempt: bool) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
        if !exempt && self.max_connections_per_ip.is_some() {
            let mut map = self.lock_map();
            if let Some(count) = map.get_mut(&ip) {
                if *count <= 1 {
                    map.remove(&ip);
                } else {
                    *count -= 1;
                }
            }
        }
    }
}

/// An RAII guard representing an active connection lease.
///
/// Decrements global and per-IP connection counts when dropped.
#[derive(Debug)]
pub(crate) struct ConnectionGuard {
    tracker: Arc<ConnectionTrackerInner>,
    ip: IpAddr,
    exempt: bool,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.tracker.release(self.ip, self.exempt);
    }
}

/// Thread-safe tracker for global and per-IP connection counts.
#[derive(Debug, Clone)]
pub(crate) struct ConnectionTracker {
    inner: Arc<ConnectionTrackerInner>,
}

impl ConnectionTracker {
    /// Creates a new connection tracker.
    pub(crate) fn new(
        max_connections: Option<usize>,
        max_connections_per_ip: Option<usize>,
        exempt_loopback: bool,
    ) -> Self {
        Self {
            inner: Arc::new(ConnectionTrackerInner {
                max_connections,
                max_connections_per_ip,
                exempt_loopback,
                active_connections: AtomicUsize::new(0),
                connections_per_ip: Mutex::new(HashMap::default()),
            }),
        }
    }

    /// Attempts to acquire a connection lease for the given IP address.
    pub(crate) fn try_acquire(&self, ip: IpAddr) -> Result<ConnectionGuard, ConnectionLimitError> {
        let is_exempt = self.inner.exempt_loopback && ip.is_loopback();

        // 1. Check and increment global limit
        let mut active = self.inner.active_connections.load(Ordering::Relaxed);
        loop {
            if let Some(max) = self.inner.max_connections {
                if active >= max {
                    return Err(ConnectionLimitError::GlobalLimitReached);
                }
            }
            match self.inner.active_connections.compare_exchange_weak(
                active,
                active + 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => active = actual,
            }
        }

        // 2. Check and increment per-IP limit (if not exempt)
        if !is_exempt {
            if let Some(max_per_ip) = self.inner.max_connections_per_ip {
                let mut map = self.inner.lock_map();
                let count = map.entry(ip).or_insert(0);
                if *count >= max_per_ip {
                    // Roll back global counter
                    self.inner.active_connections.fetch_sub(1, Ordering::SeqCst);
                    return Err(ConnectionLimitError::PerIpLimitReached);
                }
                *count += 1;
            }
        }

        Ok(ConnectionGuard {
            tracker: self.inner.clone(),
            ip,
            exempt: is_exempt,
        })
    }

    /// Returns the current number of active connections.
    #[cfg(test)]
    pub(crate) fn active_connections(&self) -> usize {
        self.inner.active_connections.load(Ordering::Relaxed)
    }

    /// Returns the number of active connections for a specific IP.
    #[cfg(test)]
    pub(crate) fn active_connections_for_ip(&self, ip: &IpAddr) -> usize {
        self.inner.lock_map().get(ip).copied().unwrap_or(0)
    }

    /// Returns the number of distinct tracked IPs in the tracking map.
    #[cfg(test)]
    pub(crate) fn tracked_ips_count(&self) -> usize {
        self.inner.lock_map().len()
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn global_limit_enforces_and_reclaims_slot() {
        let tracker = ConnectionTracker::new(Some(2), None, false);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let guard1 = tracker.try_acquire(ip).expect("first connection");
        let guard2 = tracker.try_acquire(ip).expect("second connection");
        assert_eq!(tracker.active_connections(), 2);

        // 3rd exceeds limit of 2
        let err = tracker.try_acquire(ip).unwrap_err();
        assert_eq!(err, ConnectionLimitError::GlobalLimitReached);

        // Drop guard1 and acquire again
        drop(guard1);
        assert_eq!(tracker.active_connections(), 1);

        let guard3 = tracker
            .try_acquire(ip)
            .expect("third connection after drop");
        assert_eq!(tracker.active_connections(), 2);

        drop(guard2);
        drop(guard3);
        assert_eq!(tracker.active_connections(), 0);
    }

    #[test]
    fn per_ip_limit_enforces_and_cleans_map() {
        let tracker = ConnectionTracker::new(None, Some(2), false);
        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        let g1 = tracker.try_acquire(ip1).expect("ip1 first");
        let g2 = tracker.try_acquire(ip1).expect("ip1 second");
        assert_eq!(tracker.active_connections_for_ip(&ip1), 2);

        // 3rd for ip1 fails
        assert_eq!(
            tracker.try_acquire(ip1).unwrap_err(),
            ConnectionLimitError::PerIpLimitReached
        );

        // ip2 can still connect
        let g3 = tracker.try_acquire(ip2).expect("ip2 first");
        assert_eq!(tracker.active_connections_for_ip(&ip2), 1);
        assert_eq!(tracker.tracked_ips_count(), 2);

        // Dropping ip1 guards removes ip1 from map
        drop(g1);
        assert_eq!(tracker.active_connections_for_ip(&ip1), 1);
        drop(g2);
        assert_eq!(tracker.active_connections_for_ip(&ip1), 0);
        assert_eq!(tracker.tracked_ips_count(), 1); // Only ip2 remains

        drop(g3);
        assert_eq!(tracker.tracked_ips_count(), 0); // Map completely clean
    }

    #[test]
    fn loopback_exemption_allows_unlimited_local_connections() {
        let tracker = ConnectionTracker::new(None, Some(1), true);
        let loopback = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let _g1 = tracker.try_acquire(loopback).expect("loopback 1");
        let _g2 = tracker.try_acquire(loopback).expect("loopback 2");
        let _g3 = tracker.try_acquire(loopback).expect("loopback 3");

        assert_eq!(tracker.active_connections(), 3);
        assert_eq!(tracker.tracked_ips_count(), 0); // Loopback not added to per-IP map

        // Non-loopback still constrained to 1
        let non_loopback = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50));
        let _g_ext = tracker.try_acquire(non_loopback).expect("external 1");
        assert!(tracker.try_acquire(non_loopback).is_err());
    }
}
