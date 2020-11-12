pub struct State {
    pub value: i32,
}

impl State {
    pub fn test_increment(&mut self) {
        self.value += 1;
    }
    pub fn test_print(&self) {
        println!("testing printing: value is {}", self.value);
    }
}
