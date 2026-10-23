use crate::constants::{
    DAYS_IN_MONTH, DAYS_PER_100Y, DAYS_PER_400Y, DAYS_PER_4Y, LEAPOCH, MONTHS, SP, WEEKS,
};
use std::cell::RefCell;
use std::convert::TryInto;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    // HTTP dates have one-second precision. Keeping the formatted value per
    // worker thread avoids recalculating the calendar for every response and
    // needs neither a lock nor another dependency.
    static HTTP_DATE_CACHE: RefCell<Option<(u64, String)>> = RefCell::new(None);
}

pub(crate) struct Time {
    min: i64,
    sec: i64,
    hour: i64,
    week: i64,
    year: i64,
    month: i64,
    day: i64,
}

impl Time {
    pub(crate) fn now_http_date() -> Option<String> {
        let stamp = Self::unix_timestamp()?;
        HTTP_DATE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some((cached_stamp, cached_date)) = cache.as_ref() {
                if *cached_stamp == stamp {
                    return Some(cached_date.clone());
                }
            }
            let stamp_i64 = stamp.try_into().ok()?;
            let date = Self::time(stamp_i64)?.format();
            *cache = Some((stamp, date.clone()));
            Some(date)
        })
    }
    fn unix_timestamp() -> Option<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs())
    }
    pub(crate) fn format(&self) -> String {
        let (month_day, week_day) = match (self.month_day(), self.week_day()) {
            (Some(month_day), Some(week_day)) => (month_day, week_day),
            _ => return String::new(),
        };
        // An IMF-fixdate is always 29 ASCII bytes. Reserving it once avoids the
        // temporary strings previously created for every individual field.
        let mut output = String::with_capacity(29);
        write!(
            &mut output,
            "{},{}{:02}{}{}{}{}{}{:02}:{:02}:{:02}{}GMT",
            week_day,
            SP,
            self.day,
            SP,
            month_day,
            SP,
            self.year,
            SP,
            self.hour,
            self.min,
            self.sec,
            SP
        )
        .expect("writing a date to a String cannot fail");
        output
    }
    /*
    pub fn with_stamp(self, stamp: i64) -> Option<Self> {
        Self::time(stamp)
    }
    */
    fn week_day(&self) -> Option<&'static str> {
        let week: usize = self.week.try_into().ok()?;
        WEEKS.get(week).copied()
    }
    fn month_day(&self) -> Option<&'static str> {
        let month: usize = self.month.try_into().ok()?;
        MONTHS.get(month).copied()
    }

    pub(crate) fn time(stamp: i64) -> Option<Self> {
        let secs = (stamp) - LEAPOCH;
        let mut days = secs / 86400;
        let mut remsecs = secs % 86400;
        if remsecs < 0 {
            remsecs += 86400;
            days -= 1;
        }
        let mut wday = (days + 3) % 7;
        if wday < 0 {
            wday += 7;
        }
        let mut qc_cycles = days / (DAYS_PER_400Y);
        let mut remdays = days % (DAYS_PER_400Y);
        if remdays < 0 {
            remdays += DAYS_PER_400Y;
            qc_cycles -= 1;
        }
        let mut c_cycles = remdays / DAYS_PER_100Y;
        if c_cycles == 4 {
            c_cycles -= 1;
        }
        remdays -= c_cycles * DAYS_PER_100Y;
        let mut q_cycles = remdays / DAYS_PER_4Y;
        if q_cycles == 25 {
            q_cycles -= 1;
        }
        remdays -= q_cycles * DAYS_PER_4Y;
        let mut remyears = remdays / 365;
        if remyears == 4 {
            remyears -= 1;
        }
        remdays -= remyears * 365;
        let years = remyears + 4 * q_cycles + 100 * c_cycles + 400 * qc_cycles;
        let mut month: usize = 0;
        while DAYS_IN_MONTH[month] <= remdays {
            remdays -= DAYS_IN_MONTH[month];
            month += 1;
        }
        let mut final_year = years + 100;
        let mut final_mon = (month as i64) + 2;
        if final_mon >= 12 {
            final_mon -= 12;
            final_year += 1;
        }
        Some(Time {
            min: (remsecs / 60) % 60,
            sec: remsecs % 60,
            hour: remsecs / 3600,
            week: wday,
            day: remdays + 1,
            year: 1900 + final_year,
            month: final_mon,
        })
    }
}

mod tests {
    #![cfg(test)]
    use crate::time::Time;

    #[test]
    fn start_date() {
        // the date starts on Thursday, 1 January 1970 00:00:00 (GMT) time stamp is the seconds passed
        let start_of_time = Time::time(0).unwrap().format();
        assert_eq!(start_of_time, "Thu, 01 Jan 1970 00:00:00 GMT");
    }

    #[test]
    fn some_time_stamp() {
        // the method should work for some arbitary time stamps after 1970 Jan 1
        let some_time = Time::time(6043440870).unwrap().format();
        assert_eq!(some_time, "Sun, 05 Jul 2161 05:34:30 GMT");
    }

    #[test]
    fn some_more_time_stamps() {
        let some_time = Time::time(333452334).unwrap().format();
        assert_eq!(some_time, "Sat, 26 Jul 1980 09:38:54 GMT");
    }

    #[test]
    fn september_is_three_letters() {
        let some_time = Time::time(1789461005).unwrap().format();
        assert_eq!(some_time, "Tue, 15 Sep 2026 08:30:05 GMT");
    }
}
