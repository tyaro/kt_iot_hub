use anyhow::{anyhow, Result};

pub trait JoyWatcherBridgeApi {
    fn connect_net(&mut self) -> Result<()>;
    fn disconnect_net(&mut self) -> Result<()>;
    fn disconnect_net_force(&mut self) -> Result<()>;
}

#[derive(Debug)]
pub struct ConnectionManager<A: JoyWatcherBridgeApi> {
    api: A,
    active_connections: usize,
}

impl<A: JoyWatcherBridgeApi> ConnectionManager<A> {
    pub fn new(api: A) -> Self {
        Self {
            api,
            active_connections: 0,
        }
    }

    pub fn connect(&mut self) -> Result<usize> {
        self.api.connect_net()?;
        self.active_connections += 1;
        Ok(self.active_connections)
    }

    pub fn disconnect(&mut self) -> Result<usize> {
        if self.active_connections == 0 {
            return Err(anyhow!("DisconnectNet called more times than ConnectNet"));
        }

        self.api.disconnect_net()?;
        self.active_connections -= 1;
        Ok(self.active_connections)
    }

    pub fn force_disconnect(&mut self) -> Result<usize> {
        if self.active_connections > 0 {
            self.api.disconnect_net_force()?;
        }
        self.active_connections = 0;
        Ok(self.active_connections)
    }

    pub fn ensure_connected(&self) -> Result<()> {
        if self.active_connections == 0 {
            return Err(anyhow!("JoyWatcher bridge is not connected"));
        }
        Ok(())
    }

    pub fn api_mut(&mut self) -> &mut A {
        &mut self.api
    }

    pub fn api_ref(&self) -> &A {
        &self.api
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct MockApi {
        connect_calls: usize,
        disconnect_calls: usize,
        force_calls: usize,
    }

    impl JoyWatcherBridgeApi for MockApi {
        fn connect_net(&mut self) -> Result<()> {
            self.connect_calls += 1;
            Ok(())
        }

        fn disconnect_net(&mut self) -> Result<()> {
            self.disconnect_calls += 1;
            Ok(())
        }

        fn disconnect_net_force(&mut self) -> Result<()> {
            self.force_calls += 1;
            Ok(())
        }
    }

    #[test]
    fn connect_and_disconnect_balance_counts() {
        let mut manager = ConnectionManager::new(MockApi::default());

        assert_eq!(manager.connect().unwrap(), 1);
        assert_eq!(manager.disconnect().unwrap(), 0);
    }

    #[test]
    fn disconnect_without_connect_fails() {
        let mut manager = ConnectionManager::new(MockApi::default());

        let error = manager.disconnect().expect_err("disconnect should fail");
        assert!(error.to_string().contains("more times"));
    }

    #[test]
    fn force_disconnect_resets_count() {
        let mut manager = ConnectionManager::new(MockApi::default());
        manager.connect().unwrap();
        manager.connect().unwrap();

        assert_eq!(manager.force_disconnect().unwrap(), 0);
        assert!(manager.ensure_connected().is_err());
    }
}
