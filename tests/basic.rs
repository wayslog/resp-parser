use resp_parser::{Buffer, Message, RespParser, Result};
use std::io::Write;

fn correctness_test(data: &[u8], expect_message: Message) {
    test_help(data, Ok(Some(expect_message)))
}

fn test_help(data: &[u8], expect: Result<Option<Message>>) {
    let mut buffer = Buffer::default();
    buffer.write(data).unwrap();

    let mut parser = RespParser::default();
    assert_eq!(expect, parser.parse(&mut buffer));
}

#[test]
fn test_resp_parse_plain() {
    correctness_test(
        b"+baka for you\r\n",
        Message::SimpleString(b"baka for you".to_vec()),
    );

    correctness_test(
        b"-boy next door\r\n",
        Message::Error(b"boy next door".to_vec()),
    );

    correctness_test(b":1024\r\n", Message::Integer(1024));
}

#[test]
fn test_resp_parse_bulk_ok() {
    correctness_test(b"$5\r\nojbk\n\r\n", Message::Bulk(Some(b"ojbk\n".to_vec())));
}

#[test]
fn test_resp_parse_array() {
    correctness_test(
        b"*2\r\n$1\r\na\r\n$5\r\nojbk\n\r\n",
        Message::Array(vec![
            Message::Bulk(Some(b"a".to_vec())),
            Message::Bulk(Some(b"ojbk\n".to_vec())),
        ]),
    )
}
