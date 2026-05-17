use anyhow::{anyhow, Result};
use tracing::warn;
use crate::joywatcher_ffi::REQUIRED_FFI_SYMBOLS;

pub trait JoyWatcherApi {
    fn connect_net(&mut self) -> Result<()>;
    fn disconnect_net(&mut self) -> Result<()>;
    fn disconnect_net_force(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct JoyWatcherConnectionPlan {
    required_symbols: &'static [&'static str],
}

impl Default for JoyWatcherConnectionPlan {
    fn default() -> Self {
        Self {
            required_symbols: REQUIRED_FFI_SYMBOLS,
        }
    }
}

impl JoyWatcherConnectionPlan {
    pub fn required_symbols(&self) -> &'static [&'static str] {
        self.required_symbols
    }

    pub fn summary(&self) -> &'static str {
        "ConnectNet を呼んだ回数ぶん DisconnectNet を返し、異常時は DisconnectNetForce を退避導線として使う"
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct JoyWatcherConnectionManager<A: JoyWatcherApi> {
    api: A,
    active_connections: usize,
}

#[cfg_attr(not(test), allow(dead_code))]
impl<A: JoyWatcherApi> JoyWatcherConnectionManager<A> {
    pub fn new(api: A) -> Self {
        Self {
            api,
            active_connections: 0,
        }
    }

    pub fn active_connection_count(&self) -> usize {
        self.active_connections
    }

    pub fn acquire(&mut self) -> Result<JoyWatcherConnectionLease<'_, A>> {
        self.api.connect_net()?;
        self.active_connections += 1;
        Ok(JoyWatcherConnectionLease {
            manager: self,
            released: false,
        })
    }

    pub fn force_disconnect_all(&mut self) -> Result<()> {
        if self.active_connections == 0 {
            return Ok(());
        }

        self.api.disconnect_net_force()?;
        self.active_connections = 0;
        Ok(())
    }

    fn release_one(&mut self) -> Result<()> {
        if self.active_connections == 0 {
            return Err(anyhow!(
                "DisconnectNet called more times than ConnectNet"
            ));
        }

        self.api.disconnect_net()?;
        self.active_connections -= 1;
        Ok(())
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct JoyWatcherConnectionLease<'a, A: JoyWatcherApi> {
    manager: &'a mut JoyWatcherConnectionManager<A>,
    released: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
impl<A: JoyWatcherApi> JoyWatcherConnectionLease<'_, A> {
    pub fn disconnect(mut self) -> Result<()> {
        self.released = true;
        self.manager.release_one()
    }
}

impl<A: JoyWatcherApi> Drop for JoyWatcherConnectionLease<'_, A> {
    fn drop(&mut self) {
        if self.released {
            return;
        }

        if let Err(error) = self.manager.release_one() {
            warn!("JoyWatcher connection lease drop failed: {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockJoyWatcherApi {
        connect_calls: usize,
        disconnect_calls: usize,
        force_disconnect_calls: usize,
    }

    impl JoyWatcherApi for MockJoyWatcherApi {
        fn connect_net(&mut self) -> Result<()> {
            self.connect_calls += 1;
            Ok(())
        }

        fn disconnect_net(&mut self) -> Result<()> {
            self.disconnect_calls += 1;
            Ok(())
        }

        fn disconnect_net_force(&mut self) -> Result<()> {
            self.force_disconnect_calls += 1;
            Ok(())
        }
    }

    #[test]
    fn acquire_and_disconnect_balance_connect_count() {
        let api = MockJoyWatcherApi::default();
        let mut manager = JoyWatcherConnectionManager::new(api);

        let lease = manager.acquire().expect("connect should succeed");
        lease.disconnect().expect("disconnect should succeed");

        assert_eq!(manager.active_connection_count(), 0);
        assert_eq!(manager.api.connect_calls, 1);
        assert_eq!(manager.api.disconnect_calls, 1);
    }

    #[test]
    fn force_disconnect_resets_active_connections() {
        let api = MockJoyWatcherApi::default();
        let mut manager = JoyWatcherConnectionManager::new(api);

        {
            let _lease = manager.acquire().expect("connect should succeed");
        }

        assert_eq!(manager.active_connection_count(), 0);

        manager.active_connections = 2;

        manager
            .force_disconnect_all()
            .expect("force disconnect should succeed");

        assert_eq!(manager.active_connection_count(), 0);
        assert_eq!(manager.api.force_disconnect_calls, 1);
    }

    #[test]
    fn release_without_connect_is_error() {
        let api = MockJoyWatcherApi::default();
        let mut manager = JoyWatcherConnectionManager::new(api);

        let error = manager.release_one().expect_err("should detect imbalance");
        assert!(error.to_string().contains("DisconnectNet called more times"));
    }
}