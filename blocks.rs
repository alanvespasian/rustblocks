pub struct Block {
    pub icon:     &'static str,
    pub command:  &'static str,
    pub interval: u32,
    pub signal:   u32,
}

pub static BLOCKS: &[Block] = &[
    Block { icon: "",    command: "TZ=America/New_York date '+%b %d %a, %Y'",                       interval: 30, signal: 0 },
    Block { icon: "",    command: "TZ=America/New_York date '+%R'",                                 interval: 5,  signal: 0 },
];

// Sets delimiter between status blocks. Empty string means no delimiter.
pub static DELIM: &str = " | ";
pub static DELIM_LEN: usize = 5;
