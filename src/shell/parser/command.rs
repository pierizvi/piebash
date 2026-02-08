#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub args: Vec<String>,
    pub redirect_stdout: Option<Redirect>,
    pub redirect_stderr: Option<Redirect>,
    pub pipe_to: Option<Box<Command>>,
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub target: String,
    pub append: bool,
}

impl Command {
    pub fn new(name: String, args: Vec<String>) -> Self {
        Self {
            name,
            args,
            redirect_stdout: None,
            redirect_stderr: None,
            pipe_to: None,
        }
    }

    pub fn with_stdout_redirect(mut self, target: String, append: bool) -> Self {
        self.redirect_stdout = Some(Redirect { target, append });
        self
    }

    pub fn with_pipe(mut self, next: Command) -> Self {
        self.pipe_to = Some(Box::new(next));
        self
    }
}