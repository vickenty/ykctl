use anyhow::Result;

const CMD_SELECT: u8 = 0xa4;
const CMD_READ_CONFIG: u8 = 0x1d;
const CMD_WRITE_CONFIG: u8 = 0x1c;
const STATUS_OK: u16 = 0x90;

pub fn find_device(supported: &[&[u8]]) -> Result<(String, pcsc::Card)> {
    let cx = pcsc::Context::establish(pcsc::Scope::System)?;
    let mut buf = vec![0; cx.list_readers_len()?];
    for name in cx.list_readers(&mut buf)? {
        for sname in supported {
            if starts_with_nocase(name.to_bytes(), sname) {
                let card = cx.connect(name, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)?;
                return Ok((name.to_string_lossy().into_owned(), card));
            }
        }
    }

    anyhow::bail!("no supported device found");
}

fn starts_with_nocase(s: &[u8], p: &[u8]) -> bool {
    s.len() >= p.len() && s[..p.len()].eq_ignore_ascii_case(p)
}

pub fn send(card: &pcsc::Card, header: &[u8], payload: &[u8], expect_sw: u16) -> Result<(u16, Vec<u8>)> {
    assert!(payload.len() < 256);
    let mut req = Vec::with_capacity(header.len() + 1 + payload.len());
    req.extend_from_slice(header);
    req.push(payload.len() as u8);
    req.extend_from_slice(payload);

    log::debug!("TX: {:x?}", &req);

    let mut buf = vec![0; 256];
    let len = {
        let res = card.transmit(&req, &mut buf)?;
        
        log::debug!("RX: {:02x?}", &res);

        res.len()
    };

    if len < 2 {
        anyhow::bail!("response is too short");
    }
    buf.truncate(len);

    let sw1 = buf.pop().unwrap() as u16;
    let sw2 = buf.pop().unwrap() as u16;
    let sw = sw1 << 8 | sw2;

    if sw != expect_sw {
        anyhow::bail!("unexpected response: {:x}", sw);
    }

    Ok((sw1 << 8 | sw2, buf))
}

pub fn select(card: &pcsc::Card, aid: &[u8]) -> Result<()> {
    let (_, res) = send(card, &[0, CMD_SELECT, 0x4, 0], aid, STATUS_OK)?;
    log::debug!("    {:?}", String::from_utf8_lossy(&res));
    
    Ok(())
}

pub fn read_config(card: &pcsc::Card) -> Result<Vec<u8>> {
    let (_, data) = send(&card, &[0, CMD_READ_CONFIG, 0, 0], &[], STATUS_OK)?;
    Ok(data)
}

pub fn write_config(card: &pcsc::Card, data: &[u8]) -> Result<()> {
    send(&card, &[0, CMD_WRITE_CONFIG, 0, 0], &data, STATUS_OK)?;
    Ok(())
}
