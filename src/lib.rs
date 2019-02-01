// TODO integer overflow check
use enumx::*;
use enumx_derive::EnumX;

mod buffer;

pub use buffer::Buffer;

#[derive(Debug)]
pub struct RespParser(Parser);

pub type Result<T> = std::result::Result<T, Error>;

impl Default for RespParser {
    fn default() -> Self {
        RespParser(Parser::Init(InitParser))
    }
}

impl RespParser {
    pub fn parse(&mut self, buffer: &mut Buffer) -> Result<Option<Message>> {
        let mut parser = Parser::Init(InitParser);
        ::std::mem::swap(&mut parser, &mut self.0);
        match parser.parse(buffer).downcast() {
            ParseOutputType::Err(e) => Err(e),
            ParseOutputType::Message(m) => Ok(Some(m)),
            ParseOutputType::Parser(p) => {
                self.0 = p;
                if buffer.iter().size_hint().0 == 0 {
                    Ok(None)
                } else {
                    self.parse(buffer)
                }
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    ContentWrong,
}

#[derive(EnumX, Eq, PartialEq, Debug)]
pub enum Message {
    SimpleString(Vec<u8>),
    Error(Vec<u8>),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
    Array(Vec<Message>),
    Inline(Vec<u8>),
}

#[derive(EnumX, Debug)]
enum Parser {
    Init(InitParser),
    SimpleString(SimpleStringParser),
    Error(ErrorParser),
    Integer(IntegerParser),
    BulkSize(BulkSizeParser),
    BulkBody(BulkBodyParser),
    ArraySize(ArraySizeParser),
    ArrayBody(ArrayBodyParser),
    Inline(InlineParser),
}

enum ParseOutputType {
    Err(Error),
    Message(Message),
    Parser(Parser),
}

impl Parser {
    fn parse(self, buffer: &mut Buffer) -> ParseOutput {
        match self {
            Parser::Init(x) => x.parse(buffer),
            Parser::SimpleString(x) => x.parse(buffer),
            Parser::Error(x) => x.parse(buffer),
            Parser::Integer(x) => x.parse(buffer),
            Parser::BulkSize(x) => x.parse(buffer),
            Parser::BulkBody(x) => x.parse(buffer),
            Parser::ArraySize(x) => x.parse(buffer),
            Parser::ArrayBody(x) => x.parse(buffer),
            Parser::Inline(x) => x.parse(buffer),
        }
    }
}

#[derive(EnumX)]
enum ParseOutput {
    Err(Error),

    // Message
    Message(Message),
    // Parser
    InitParser(InitParser),
    SimpleStringParser(SimpleStringParser),
    ErrorParser(ErrorParser),
    IntegerParser(IntegerParser),
    BulkSizeParser(BulkSizeParser),
    BulkBodyParser(BulkBodyParser),
    ArraySizeParser(ArraySizeParser),
    ArrayBodyParser(ArrayBodyParser),
    InlineParser(InlineParser),
}

impl ParseOutput {
    fn downcast(self) -> ParseOutputType {
        match self {
            ParseOutput::Err(x) => ParseOutputType::Err(x),

            ParseOutput::Message(x) => ParseOutputType::Message(x),

            ParseOutput::InitParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::SimpleStringParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::ErrorParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::IntegerParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::BulkSizeParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::BulkBodyParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::ArraySizeParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::ArrayBodyParser(x) => ParseOutputType::Parser(x.into_enum()),
            ParseOutput::InlineParser(x) => ParseOutputType::Parser(x.into_enum()),
        }
    }
}

#[derive(Debug)]
struct InitParser;

impl InitParser {
    fn parse(self, buffer: &mut Buffer) -> ParseOutput {
        let result = match buffer.iter().next() {
            None => self.into_enum(),
            Some(input) => match input {
                b'+' => SimpleStringParser::default().into_enum(),
                b'-' => ErrorParser::default().into_enum(),
                b':' => IntegerParser::default().into_enum(),
                b'$' => BulkSizeParser::default().into_enum(),
                b'*' => ArraySizeParser::default().into_enum(),
                _ => InlineParser::default().into_enum(),
            },
        };
        let _ = buffer.pop_scanned_buffer();
        result
    }
}

#[derive(Debug, Default)]
struct SimpleStringParser;

impl SimpleStringParser {
    fn parse(self, buffer: &mut Buffer) -> ParseOutput {
        if buffer.iter().position(|x| x == b'\n') == None {
            return self.into_enum();
        }

        let mut data = buffer.pop_scanned_buffer();
        if data.pop() != Some(b'\n') {
            return Error::ContentWrong.into_enum();
        }
        if data.pop() != Some(b'\r') {
            return Error::ContentWrong.into_enum();
        }
        Message::SimpleString(data).into_enum()
    }
}

#[derive(Debug, Default)]
struct ErrorParser;

impl ErrorParser {
    fn parse(self, buffer: &mut Buffer) -> ParseOutput {
        if buffer.iter().position(|x| x == b'\n') == None {
            return self.into_enum();
        }

        let mut data = buffer.pop_scanned_buffer();
        if data.pop() != Some(b'\n') {
            return Error::ContentWrong.into_enum();
        }
        if data.pop() != Some(b'\r') {
            return Error::ContentWrong.into_enum();
        }
        Message::Error(data).into_enum()
    }
}

#[derive(Debug, Default)]
struct IntegerParser(i64);

impl IntegerParser {
    fn parse(mut self, buffer: &mut Buffer) -> ParseOutput {
        for i in buffer.iter() {
            match i {
                x @ b'0'..=b'9' => self.0 = self.0 * 10 + i64::from(x - b'0'),
                b'-' => self.0 = -self.0,
                b'\r' => {}
                b'\n' => {
                    buffer.pop_scanned_buffer();
                    return Message::Integer(self.0).into_enum();
                }
                _ => return Error::ContentWrong.into_enum(),
            }
        }
        self.into_enum()
    }
}

#[derive(Debug, Default)]
struct BulkSizeParser(isize);

impl BulkSizeParser {
    fn parse(mut self, buffer: &mut Buffer) -> ParseOutput {
        for i in buffer.iter() {
            match i {
                x @ b'0'..=b'9' => self.0 = self.0 * 10 + isize::from(x - b'0'),
                b'-' => self.0 = -self.0,
                b'\r' => {}
                b'\n' => {
                    buffer.pop_scanned_buffer();
                    return match self.0 {
                        x if x < 0 => Message::Bulk(None).into_enum(),
                        x if x > 0 => BulkBodyParser {
                            expect: (x + 2) as usize,
                        }
                            .into_enum(),
                        _ => Message::Bulk(Some(vec![])).into_enum(),
                    };
                }
                _ => return Error::ContentWrong.into_enum(),
            }
        }
        self.into_enum()
    }
}

#[derive(Debug)]
struct BulkBodyParser {
    expect: usize,
}

impl BulkBodyParser {
    fn parse(mut self, buffer: &mut Buffer) -> ParseOutput {
        let scanned = buffer.consume(self.expect);
        self.expect -= scanned;
        if self.expect > 0 {
            return self.into_enum();
        }

        let mut data = buffer.pop_scanned_buffer();
        if data.ends_with(b"\r\n") {
            data.pop();
            data.pop();
            Message::Bulk(Some(data)).into_enum()
        } else {
            Error::ContentWrong.into_enum()
        }
    }
}

#[derive(Debug, Default)]
struct ArraySizeParser(usize);

impl ArraySizeParser {
    fn parse(mut self, buffer: &mut Buffer) -> ParseOutput {
        for i in buffer.iter() {
            match i {
                x @ b'0'..=b'9' => self.0 = self.0 * 10 + usize::from(x - b'0'),
                b'\r' => {}
                b'\n' => {
                    buffer.pop_scanned_buffer();
                    return match self.0 {
                        0 => Message::Array(vec![]).into_enum(),
                        x => ArrayBodyParser {
                            message: Vec::with_capacity(x),
                            expect: x,
                            current: Box::new(InitParser.into_enum()),
                        }
                            .into_enum(),
                    };
                }
                _ => return Error::ContentWrong.into_enum(),
            }
        }
        self.into_enum()
    }
}

#[derive(Debug)]
struct ArrayBodyParser {
    expect: usize,
    message: Vec<Message>,
    current: Box<Parser>,
}

impl ArrayBodyParser {
    fn parse(mut self, buffer: &mut Buffer) -> ParseOutput {
        if self.expect == self.message.len() {
            return Message::Array(self.message).into_enum();
        }

        match self.current.parse(buffer).downcast() {
            ParseOutputType::Err(e) => e.into_enum(),
            ParseOutputType::Message(m) => {
                self.message.push(m);

                self.current = Box::new(InitParser.into_enum());

                self.parse(buffer)
            }
            ParseOutputType::Parser(p) => {
                self.current = Box::new(p);
                self.into_enum()
            }
        }
    }
}

#[derive(Debug, Default)]
struct InlineParser;

impl InlineParser {
    fn parse(self, buffer: &mut Buffer) -> ParseOutput {
        if buffer.iter().position(|x| x == b'\n') == None {
            return self.into_enum();
        }

        let mut data = buffer.pop_scanned_buffer();
        if data.pop() != Some(b'\n') {
            return Error::ContentWrong.into_enum();
        }
        if data.pop() != Some(b'\r') {
            return Error::ContentWrong.into_enum();
        }
        Message::Inline(data).into_enum()
    }
}
