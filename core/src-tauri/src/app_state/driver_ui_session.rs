#[derive(Clone, Debug)]
pub struct DriverUiSessionState {
    pub target_driver_id: Option<String>,
    #[allow(dead_code)]
    pub driver_type: String,
    pub process_active: bool,
}
