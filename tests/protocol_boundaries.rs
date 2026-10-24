// SPDX-License-Identifier: AGPL-3.0-only
use lasercat::grbl::{Decoder, Stream};
use std::collections::VecDeque;

#[test]
fn serial_messages_decode_identically_at_every_fragment_boundary() {
    let wire = b"Grbl 1.1h\r\nok\n<Run|WPos:12.3,4.5,0>\r\nerror:2\n[MSG:ready]\nok\n";
    let expected = Decoder::default().push(wire).unwrap();
    for split in 0..=wire.len() {
        let mut decoder = Decoder::default();
        let mut actual = decoder.push(&wire[..split]).unwrap();
        actual.extend(decoder.push(&wire[split..]).unwrap());
        assert_eq!(actual, expected, "split {split}");
    }
    for chunk_size in 1..=wire.len() {
        let mut decoder = Decoder::default();
        let actual: Vec<_> = wire
            .chunks(chunk_size)
            .flat_map(|chunk| decoder.push(chunk).unwrap())
            .collect();
        assert_eq!(actual, expected, "chunk size {chunk_size}");
    }
}

#[test]
fn oversized_response_can_be_followed_by_a_valid_response() {
    let mut decoder = Decoder::default();
    assert!(decoder.push(&[b'x'; 4096]).unwrap().is_empty());
    assert!(decoder.push(b"x").is_err());
    assert_eq!(decoder.push(b"\r\nok\n").unwrap(), vec!["ok"]);
}

#[test]
fn varied_line_lengths_respect_buffer_and_fifo_until_physical_completion() {
    let lines: Vec<_> = [1, 60, 126, 2, 64, 61, 125, 10, 80, 46]
        .into_iter()
        .cycle()
        .take(100)
        .map(|length| "X".repeat(length))
        .collect();
    let mut stream = Stream::default();
    stream.begin(lines.clone()).unwrap();
    let mut inflight = VecDeque::new();
    let mut sent = vec![];
    while stream.accepted < lines.len() {
        while let Some(line) = stream.next_line() {
            inflight.push_back(line.len());
            sent.push(line.trim_end().to_owned());
            assert!(inflight.iter().sum::<usize>() <= 127);
        }
        assert!(inflight.pop_front().is_some(), "sender made no progress");
        stream.acknowledge().unwrap();
        assert!(!stream.status(false));
    }
    assert_eq!(sent, lines);
    assert!(stream.active());
    assert!(!stream.status(true));
    stream.queried_after_ack = true;
    assert!(stream.status(true));
    assert!(!stream.active());
}

#[test]
fn cancellation_discards_old_queue_and_acknowledgements_before_a_new_job() {
    let mut stream = Stream::default();
    stream.begin(vec!["G1 X1".into(); 40]).unwrap();
    assert!(stream.next_line().is_some());
    assert!(stream.begin(vec!["M5".into()]).is_err());
    stream.clear();
    assert!(stream.next_line().is_none());
    assert!(stream.acknowledge().is_err());
    stream.begin(vec!["M5".into()]).unwrap();
    assert_eq!(stream.next_line().as_deref(), Some("M5\n"));
    stream.acknowledge().unwrap();
    assert_eq!(stream.accepted, 1);
    assert_eq!(stream.total, 1);
}
