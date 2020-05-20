extern crate octane;
use octane::time::Time;

#[test]
fn start_date() {
    // the date starts on Thursday, 1 January 1970 00:00:00 (GMT) time stamp is the seconds passed
    let start_of_time = Time::now().unwrap().with_stamp(0).unwrap().format();
    assert_eq!(start_of_time, "Thu, 01 Jan 1970 00:00:00 GMT");
}

#[test]
fn some_time_stamp() {
    // the method should work for some arbitary time stamps after 1970 Jan 1
    let some_time = Time::now()
        .unwrap()
        .with_stamp(6043440870)
        .unwrap()
        .format();
    assert_eq!(some_time, "Sun, 05 Jul 2161 05:34:30 GMT");
}

#[test]
fn some_more_time_stamps() {
    let some_time = Time::now().unwrap().with_stamp(333452334).unwrap().format();
    assert_eq!(some_time, "Sat, 26 Jul 1980 09:38:54 GMT");
}
