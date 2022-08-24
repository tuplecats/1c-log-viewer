pub fn sub_strings(string: &str, sub_len: usize) -> Vec<&str> {
    let mut subs = Vec::with_capacity(string.len() * 2 / sub_len);
    let mut iter = string.chars();
    let mut pos = 0;

    while pos < string.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(sub_len) {
            len += ch.len_utf8();
            if ch == '\n' {
                break;
            }
        }
        subs.push(&string[pos..pos + len]);
        pos += len;
    }
    subs
}