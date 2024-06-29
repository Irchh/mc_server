#[derive(Debug)]
#[repr(u8)]
pub enum CommandNodeType {
    Root = 0b00,
    Literal = 0b01,
    Argument = 0b10,
}

#[derive(Debug)]
pub enum CommandParsers {
    Bool,
    Float { min: Option<f32>, max: Option<f32> },
    Double { min: Option<f64>, max: Option<f64> },
    Integer { min: Option<i32>, max: Option<i32> },
}

impl CommandParsers {
    pub fn id(&self) -> i32 {
        match self {
            CommandParsers::Bool => 0,
            CommandParsers::Float { .. } => 1,
            CommandParsers::Double { .. } => 2,
            CommandParsers::Integer { .. } => 3,
        }
    }

    pub fn properties(&self) -> Vec<u8> {
        match self {
            CommandParsers::Bool => vec![],
            CommandParsers::Float { min, max } => {
                let flags = 0x1 * min.is_some() as u8 | 0x2 * max.is_some() as u8;
                let mut bytes = vec![flags];
                if let Some(min) = min {
                    bytes.append(&mut min.to_be_bytes().to_vec())
                }
                if let Some(max) = max {
                    bytes.append(&mut max.to_be_bytes().to_vec())
                }
                bytes
            }
            CommandParsers::Double { min, max } => {
                let flags = 0x1 * min.is_some() as u8 | 0x2 * max.is_some() as u8;
                let mut bytes = vec![flags];
                if let Some(min) = min {
                    bytes.append(&mut min.to_be_bytes().to_vec())
                }
                if let Some(max) = max {
                    bytes.append(&mut max.to_be_bytes().to_vec())
                }
                bytes
            }
            CommandParsers::Integer { min, max } => {
                let flags = 0x1 * min.is_some() as u8 | 0x2 * max.is_some() as u8;
                let mut bytes = vec![flags];
                if let Some(min) = min {
                    bytes.append(&mut min.to_be_bytes().to_vec())
                }
                if let Some(max) = max {
                    bytes.append(&mut max.to_be_bytes().to_vec())
                }
                bytes
            }
        }
    }
}

#[derive(Debug)]
pub struct CommandNode {
    pub node_type: CommandNodeType,
    pub is_executable: bool,
    pub children: Vec<i32>,
    pub redirect: Option<i32>,
    pub name: Option<String>,
    pub parser: Option<CommandParsers>,
    pub suggestions_type: Option<String>,
}

impl CommandNode {
    pub fn commands() -> Vec<Self> {
        let mut commands = vec![];
        commands.push(CommandNode {
            node_type: CommandNodeType::Root,
            is_executable: false,
            children: vec![],
            redirect: None,
            name: None,
            parser: None,
            suggestions_type: None,
        });

        commands.push(Self::literal("place", false, None, None));
        commands.push(Self::argument("id", true, CommandParsers::Integer { min: Some(0), max: None }, None, None));
        commands.get_mut(0).unwrap().children.push(1);
        commands.get_mut(1).unwrap().children.push(2);
        commands
    }

    pub fn literal<S: Into<String>>(name: S, is_executable: bool, redirect: Option<i32>, suggestions_type: Option<String>) -> Self {
        Self {
            node_type: CommandNodeType::Literal,
            is_executable,
            children: vec![],
            redirect,
            name: Some(name.into()),
            parser: None,
            suggestions_type,
        }
    }

    pub fn argument<S: Into<String>>(name: S, is_executable: bool, parser: CommandParsers, redirect: Option<i32>, suggestions_type: Option<String>) -> Self {
        Self {
            node_type: CommandNodeType::Argument,
            is_executable,
            children: vec![],
            redirect,
            name: Some(name.into()),
            parser: Some(parser),
            suggestions_type,
        }
    }
}