/// Returns (tag, header length, total length)
pub fn parse_tlv(data: &[u8]) -> Option<(u16, usize, usize)> {
    let (tag, l1) = parse_tag(data)?;
    let (len, l2) = parse_len(&data[l1..])?;
    Some((tag, l1 + l2, l1 + l2 + len))
}

fn parse_tag(data: &[u8]) -> Option<(u16, usize)> {
    let i = data.get(0).copied()? as u16;
    if i & 0x1f != 0x1f {
        return Some((i, 1));
    }

    let j = *data.get(1)? as u16;
    Some((i << 8 | j, 2))
}

fn parse_len(data: &[u8]) -> Option<(usize, usize)> {
    let i = data.get(0).copied()? as usize;
    if i > 0x80 {
        let j = i - 0x80 + 1;
        let mut k = 0usize;
        for i in 1..j {
            k = (k << 8) | *data.get(i)? as usize;
        }
        return Some((k, j+1));
    }

    Some((i, 1))
}

pub struct Iter<'a> {
    slice: &'a [u8],
    offset: usize,
}

impl Iter<'_> {
    pub fn new(slice: &[u8]) -> Iter {
        Iter {
            slice,
            offset: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (u16, &'a [u8]);
    
    fn next(&mut self) -> Option<(u16, &'a [u8])> {
        let (tag, beg, end) = parse_tlv(&self.slice[self.offset..])?;
        let beg = self.offset + beg;
        let end = self.offset + end;
        let data = &self.slice[beg..end];
        self.offset = end;
        Some((tag, data))
    }
}

pub fn write(v: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    let tag = tag.to_be_bytes();
    if tag[0] > 0 {
        assert!(tag[0] & 0x1f == 0x1f);
        v.push(tag[0]);
    }
    v.push(tag[1]);

    let len = payload.len();
    assert!(len < 0x80); // there is an encoding for larger values, but we don't need them
    v.push(len as u8);

    v.extend_from_slice(&payload);
}
