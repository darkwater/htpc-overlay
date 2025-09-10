use http::Uri;

pub const M_SEARCH: &[u8] = b"M-SEARCH * HTTP/1.1\r
Host: 239.255.255.250:1900\r
Man: \"ssdp:discover\"\r
Mx: 2\r
St: urn:schemas-upnp-org:device:MediaRenderer:1\r
\r
";

#[derive(Debug)]
pub struct Notify {
    pub location: Uri,
}

impl Notify {
    pub fn from_response(response: &[u8]) -> Option<Self> {
        let mut lines = response
            .split(|&b| b == b'\n')
            .map(|line| line.trim_ascii());

        let mut first_line = lines.next()?.split(|&b| b == b' ');
        let _version = first_line.next()?;
        let _status_code = first_line.next()?;

        let mut headers = lines.map_while(|line| line.split_once(|&b| b == b':'));
        for (name, value) in &mut headers {
            if name.eq_ignore_ascii_case(b"Location") {
                let location = std::str::from_utf8(value).ok()?.trim();
                let location = location.parse().ok()?;
                return Some(Notify { location });
            }
        }

        None
    }
}
