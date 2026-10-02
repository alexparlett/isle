/// A damped spring stepped with a fixed sub-step.
pub struct Spring {
    pub value: f64,
    pub velocity: f64,
    pub target: f64,
    stiffness: f64,
    damping: f64,
}

impl Spring {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
            stiffness: 420.0,
            damping: 34.0,
        }
    }

    pub fn step(&mut self, dt: f64) {
        let steps = (dt / 0.002).ceil().max(1.0);
        let h = dt / steps;
        for _ in 0..steps as usize {
            let force = -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += force * h;
            self.value += self.velocity * h;
        }
        if self.settled() {
            self.value = self.target;
            self.velocity = 0.0;
        }
    }

    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() < 0.05 && self.velocity.abs() < 0.05
    }
}
