use std::sync::{Arc, Mutex};

use colored::Colorize;

pub type SharedLogger = Arc<Mutex<Logger>>;

struct LoggingTask {
    name: String,
}

pub struct Logger {
    task_stack: Vec<LoggingTask>,
    current_system: Option<String>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            task_stack: Vec::new(),
            current_system: None,
        }
    }

    fn format_task_stack(&self) -> String {
        self.task_stack.iter().fold(String::new(), |acc, task| {
            if acc.is_empty() {
                task.name.clone()
            } else {
                format!("{} / {}", acc, task.name)
            }
        })
    }

    pub fn info(&self, message: &str) {
        println!("ARC | {}{} : {}", "INFO".blue(), "".clear(), message);
    }

    pub fn warn(&self, message: &str) {
        println!("ARC | {}{} : {}", "WARN".yellow(), "".clear(), message);
    }

    pub fn current_system(&mut self, system_name: &str) {
        self.current_system = Some(system_name.to_string());

        println!("\nSYSTEM: {}\n", system_name);
    }

    pub fn enter_task(&mut self, task_name: &str) {
        let current_system = self.current_system.as_ref().expect("current system");

        if self.task_stack.is_empty() {
            println!("TASK : {} | {}", task_name, current_system);
        } else {
            println!(
                "TASK : {} > {} | {}",
                self.format_task_stack(),
                task_name,
                current_system
            );
        };

        self.task_stack.push(LoggingTask {
            name: task_name.to_string(),
        });
    }

    pub fn pop_task(&mut self) {
        let popped_task = self.task_stack.pop().expect("remove task from stack");
        let current_system = self.current_system.as_ref().expect("current system");

        if self.task_stack.is_empty() {
            println!("TASK : < {} | {}", popped_task.name, current_system);
        } else {
            println!(
                "TASK : {} < {} | {}",
                self.format_task_stack(),
                popped_task.name,
                current_system
            );
        };
    }

    pub fn reset_system(&mut self) {
        let current_system = self.current_system.clone().expect("current system");

        println!("\nSYSTEM : {} | ok\n", current_system);

        self.current_system = None;
    }
}
