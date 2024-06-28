#[derive(Debug, Clone)]
pub struct Timer {
    interval: u32,
    start_time: std::time::Instant,
    active: bool,
}

impl Timer {
    pub fn new(interval: u32) -> Timer {
        Timer {
            interval,
            start_time: std::time::Instant::now(),
            active: false,
        }
    }

    pub fn start(&mut self) {
        self.start_time = std::time::Instant::now();
        self.active = true;
    }

    pub fn start_imm(&mut self) {
        self.start_time =
            std::time::Instant::now() - std::time::Duration::from_secs(self.interval as u64);
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_up(&self) -> bool {
        self.active
    }

    pub fn is_expired(&self) -> bool {
        assert!(self.active, "Timer is not active");
        let elapsed = self.start_time.elapsed().as_secs();
        elapsed >= self.interval as u64
    }

    pub fn elapsed(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
