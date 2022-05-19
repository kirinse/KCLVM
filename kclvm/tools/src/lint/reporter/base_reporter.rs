use super::stdout_reporter::StdoutReporter;
use super::super::message::message::{Message, MSG};
#[derive(Debug)]
pub enum Reporter{
    Stdout,
}

pub struct BaseReporter{
    pub kind: Reporter,
    pub sub_reporter: Box<dyn DisplayMsg>,
}



struct ReporterFacotry{}
impl ReporterFacotry{
    pub fn new_reporter(reporter: &Reporter) -> Box<dyn DisplayMsg>{
        match reporter{
            Stdout => Box::new(StdoutReporter::new()),
            _ => Box::new(StdoutReporter::new()),
        }
    }
}

impl BaseReporter{
    pub fn new(kind: Reporter) -> Self{
        let sub_reporter = ReporterFacotry::new_reporter(&kind);
        Self { kind, sub_reporter }
    }
    pub fn print_msg(&self, msgs: &Vec<Message>) {
        let c = &self.sub_reporter;
        c.print_msg(msgs)
    }
}
pub trait DisplayMsg {
    fn print_msg(&self, msgs: &Vec<Message>);
}

