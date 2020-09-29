use anyhow::Result;

mod device;
mod tlv;

const SUPPORTED: &[&[u8]] = &[
    b"yubico yubikey",
];

const AID_MGR: &[u8] = &[0xa0, 0, 0, 0x5, 0x27, 0x47, 0x11, 0x17];

const TAG_VERSION: u16 = 0x05;
const TAG_USB_ENABLED: u16 = 0x03; 
const TAG_REBOOT: u16 = 0x0c;
const APPLICATION_OTP: u16 = 0x01;
const TRANSPORT_CCID: u16 = 0x04;

#[derive(Debug)]
struct Conf {
    usb_enabled: Option<u16>,
    can_write: bool,
}

impl Conf {
    fn from_device(conf: &[u8]) -> Conf {
        let (clen, conf) = conf.split_at(1);
        assert!(clen[0] as usize == conf.len());

        let mut usb_enabled = None;
        let mut can_write = false;

        for (tag, val) in tlv::Iter::new(conf) {
            if tag == TAG_USB_ENABLED {
                assert!(val.len() == 2);
                usb_enabled = Some((val[0] as u16) << 8 | val[1] as u16);
            }
            if tag == TAG_VERSION {
                assert!(val.len() == 3);
                if &*val >= &[5, 0, 0] {
                    can_write = true;
                }
            }
        }

        Conf {
            usb_enabled,
            can_write,
        }
    }

    fn is_usb_enabled(&self, func: u16) -> bool {
        self.usb_enabled.unwrap_or(0) & func == func
    }

    fn set_usb_enabled(&mut self, func: u16, enabled: bool) {
        let mut val = self.usb_enabled.unwrap_or(0);

        if enabled {
            val |= func;
        } else {
            val &= !func;
        }
        self.usb_enabled = Some(val);
    }

    fn to_device(&self, reset: bool, out: &mut Vec<u8>) {
        out.push(0); // placeholder for len

        if let Some(val) = self.usb_enabled {
            tlv::write(out, TAG_USB_ENABLED, &val.to_be_bytes());
        }
        if reset {
            tlv::write(out, TAG_REBOOT, &[]);
        }

        assert!(out.len() < 256);
        out[0] = (out.len() - 1) as u8;
    }
}

enum OptMode {
    Enable,
    Disable,
    Toggle,
    Show,
}

struct Opts {
    mode: Option<OptMode>,
    help: bool,
}

impl Opts {
    fn from_args() -> Result<Opts> {
        let mut mode = None;
        let mut help = false;

        for opt in std::env::args().skip(1) {
            match &opt[..] {
                "-e" => mode = Some(OptMode::Enable),
                "-d" => mode = Some(OptMode::Disable),
                "-t" => mode = Some(OptMode::Toggle),
                "-s" => mode = Some(OptMode::Show),
                "-h" => help = true,
                opt => anyhow::bail!("invalid option {}, try -h", opt),
            }
        }
        
        Ok(Opts {
            mode,
            help,
        })
    }
}

fn main() -> Result<()> {
    std_logger::init();

    let opts = Opts::from_args()?;

    if opts.help {
        println!("{} [-edts]", std::env::args().nth(0).as_deref().unwrap_or("ykctl"));
        println!("  -e   enable OTP");
        println!("  -d   disable OTP");
        println!("  -t   toggle OTP");
        println!("  -s   show current status");
        println!("  -h   show this help");
        return Ok(());
    }

    let (name, dev) = device::find_device(SUPPORTED)?;
    log::info!("Connected to: {:?}", name);

    device::select(&dev, AID_MGR)?;
    let conf = device::read_config(&dev)?;

    let mut conf = Conf::from_device(&conf);
    log::debug!("{:x?}", conf);

    let old_enable = conf.is_usb_enabled(APPLICATION_OTP);
    let mut new_enable = old_enable;
    
    match opts.mode.unwrap_or(OptMode::Show) {
        OptMode::Enable => new_enable = true,
        OptMode::Disable => new_enable = false,
        OptMode::Toggle => new_enable = !old_enable,
        OptMode::Show => println!("OTP is {}", if new_enable { "on" } else { "off" }),
    }

    if new_enable == old_enable {
        return Ok(());
    }

    if !conf.can_write {
        anyhow::bail!("writing configuration is not supported for this device");
    }

    // Sanity check that CCID (USB cardreader interface) is still enabled in the config we are
    // about to write.
    if !conf.is_usb_enabled(TRANSPORT_CCID) {
        anyhow::bail!("CCID transport must be enabled on the device");
    }

    log::info!("OTP new state: {}", new_enable);

    conf.set_usb_enabled(APPLICATION_OTP, new_enable);

    let mut out = Vec::new();
    conf.to_device(true, &mut out);

    device::write_config(&dev, &out)?;

    Ok(())
}
