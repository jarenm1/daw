//! Command dispatch system for the DAW
//!
//! This crate provides a command pattern that decouples UI actions from
//! data mutations. Commands can be dispatched from any frontend (GUI, CLI, etc.)
//! and are executed against the core project data.
//!
//! Inspired by the-editor's dispatch system.

use std::collections::HashMap;

pub mod commands;

use anyhow::Result;
use daw_core::Project;
use parking_lot::RwLock;
use std::sync::Arc;

/// A command that can be executed against a project
pub trait Command: Send + Sync {
    /// Execute the command, returning true if the project was modified
    fn execute(&self, project: &mut Project) -> Result<bool>;

    /// Command name for logging/debugging
    fn name(&self) -> &str;

    /// Whether this command can be undone
    fn is_undoable(&self) -> bool {
        true
    }
}

/// Type-erased command storage
pub type CommandBox = Box<dyn Command>;

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub modified: bool,
    pub undoable: bool,
}

/// The command dispatcher
pub struct Dispatcher {
    /// Command registry
    registry: HashMap<String, Box<dyn Fn(&str) -> Result<CommandBox>>>,
    /// Undo history
    undo_stack: Vec<CommandBox>,
    /// Redo history  
    redo_stack: Vec<CommandBox>,
    /// Max undo depth
    max_undo: usize,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 100,
        }
    }

    /// Register a command type
    pub fn register<F>(&mut self, name: impl Into<String>, factory: F)
    where
        F: Fn(&str) -> Result<CommandBox> + 'static,
    {
        self.registry.insert(name.into(), Box::new(factory));
    }

    /// Dispatch a command by name with JSON arguments
    pub fn dispatch(
        &mut self,
        name: &str,
        args: &str,
        project: &mut Project,
    ) -> Result<CommandResult> {
        let factory = self
            .registry
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", name))?;

        let command = factory(args)?;
        let undoable = command.is_undoable();
        let modified = command.execute(project)?;

        if modified && undoable {
            // Clear redo stack on new action
            self.redo_stack.clear();

            // Add to undo stack
            self.undo_stack.push(command);

            // Truncate if needed
            if self.undo_stack.len() > self.max_undo {
                self.undo_stack.remove(0);
            }
        }

        Ok(CommandResult { modified, undoable })
    }

    /// Execute a pre-built command directly
    pub fn execute(&mut self, command: CommandBox, project: &mut Project) -> Result<CommandResult> {
        let undoable = command.is_undoable();
        let modified = command.execute(project)?;

        if modified && undoable {
            self.redo_stack.clear();
            self.undo_stack.push(command);

            if self.undo_stack.len() > self.max_undo {
                self.undo_stack.remove(0);
            }
        }

        Ok(CommandResult { modified, undoable })
    }

    /// Undo the last command
    pub fn undo(&mut self, project: &mut Project) -> Result<bool> {
        if let Some(command) = self.undo_stack.pop() {
            // In a real implementation, commands would implement undo
            // For now, just push to redo stack
            self.redo_stack.push(command);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self, _project: &mut Project) -> Result<bool> {
        // In a real implementation, replay the command
        Ok(false)
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Set max undo depth
    pub fn set_max_undo(&mut self, max: usize) {
        self.max_undo = max;
        while self.undo_stack.len() > max {
            self.undo_stack.remove(0);
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple command executor that doesn't track history
pub struct SimpleExecutor {
    registry: HashMap<String, Box<dyn Fn(&str) -> Result<CommandBox>>>,
}

impl SimpleExecutor {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: impl Into<String>, factory: F)
    where
        F: Fn(&str) -> Result<CommandBox> + 'static,
    {
        self.registry.insert(name.into(), Box::new(factory));
    }

    pub fn execute(&self, name: &str, args: &str, project: &mut Project) -> Result<bool> {
        let factory = self
            .registry
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", name))?;

        let command = factory(args)?;
        command.execute(project)
    }
}

impl Default for SimpleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe command dispatcher
pub struct SharedDispatcher {
    inner: Arc<RwLock<Dispatcher>>,
}

impl SharedDispatcher {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Dispatcher::new())),
        }
    }

    pub fn dispatch(&self, name: &str, args: &str, project: &mut Project) -> Result<CommandResult> {
        let mut dispatcher = self.inner.write();
        dispatcher.dispatch(name, args, project)
    }

    pub fn execute(&self, command: CommandBox, project: &mut Project) -> Result<CommandResult> {
        let mut dispatcher = self.inner.write();
        dispatcher.execute(command, project)
    }

    pub fn can_undo(&self) -> bool {
        self.inner.read().can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.inner.read().can_redo()
    }
}

impl Default for SharedDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedDispatcher {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Macro for defining command structs
#[macro_export]
macro_rules! define_command {
    (
        $name:ident {
            $($field:ident: $type:ty),*
        }
        execute($project:ident) $body:block
    ) => {
        pub struct $name {
            $(pub $field: $type,)*
        }

        impl $crate::Command for $name {
            fn execute(&self, $project: &mut daw_core::Project) -> anyhow::Result<bool> {
                let _ = self;
                $body
            }

            fn name(&self) -> &str {
                stringify!($name)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCommand;

    impl Command for TestCommand {
        fn execute(&self, project: &mut Project) -> Result<bool> {
            project.mark_dirty();
            Ok(true)
        }

        fn name(&self) -> &str {
            "TestCommand"
        }
    }

    #[test]
    fn test_dispatcher() {
        let mut dispatcher = Dispatcher::new();
        let mut project = Project::default();

        let cmd = Box::new(TestCommand);
        let result = dispatcher.execute(cmd, &mut project).unwrap();

        assert!(result.modified);
        assert!(dispatcher.can_undo());
    }
}
