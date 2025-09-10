use core::net::{Ipv4Addr, SocketAddrV4};
use std::{io::ErrorKind, net::UdpSocket};

use ehttp::Request;
use http::Uri;

use crate::{command::Event, ui::toast::Toast};

mod description;
mod search;

pub struct Dlna {
    socket: UdpSocket,
    devices: Vec<DlnaDevice>,
}

const SSDP_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;

impl Dlna {
    pub fn new() -> Self {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .expect("Failed to bind UDP socket");

        socket
            .set_nonblocking(true)
            .expect("Failed to set non-blocking");

        socket.set_broadcast(true).expect("Failed to set broadcast");

        socket.set_multicast_ttl_v4(2).expect("Failed to set TTL");

        socket
            .join_multicast_v4(&SSDP_ADDR, &Ipv4Addr::UNSPECIFIED)
            .expect("Failed to join multicast group");

        socket
            .send_to(search::M_SEARCH, (SSDP_ADDR, SSDP_PORT))
            .expect("Failed to send M-SEARCH message");

        Dlna { socket, devices: Vec::new() }
    }

    pub fn update(&mut self) -> Event {
        let mut buf = [0; 2048];

        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, address)) => {
                    eprintln!("[DLNA] Received {} bytes from {}", size, address);
                    let msg = &buf[..size];
                    let Some(notify) = search::Notify::from_response(msg) else {
                        eprintln!("[DLNA] Failed to parse NOTIFY message");
                        continue;
                    };

                    let res = ehttp::fetch_blocking(&Request::get(&notify.location))
                        .expect("Failed to fetch device description");

                    assert_eq!(res.status, 200, "Failed to fetch device description");

                    let root =
                        quick_xml::de::from_reader::<_, description::Root>(res.bytes.as_slice())
                            .expect("Failed to parse device description");

                    let name = root.device.friendly_name.clone();

                    let mut device = DlnaDevice {
                        description: root,
                        location: notify.location,
                        volume: 0,
                    };

                    device.get_volume();

                    self.devices.push(device);

                    return Event::Toast(Toast::DlnaDeviceDiscovered { name });
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    break Event::None;
                }
                Err(e) => {
                    eprintln!("Error receiving from socket: {}", e);
                    break Event::None;
                }
            }
        }
    }

    pub fn devices(&mut self) -> &mut [DlnaDevice] {
        &mut self.devices
    }
}

impl Default for Dlna {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DlnaDevice {
    description: description::Root,
    location: Uri,
    volume: u8,
}

impl DlnaDevice {
    pub fn friendly_name(&self) -> &str {
        &self.description.device.friendly_name
    }

    #[expect(dead_code)]
    pub fn icons(&self) -> &[description::Icon] {
        &self.description.device.icon_list
    }

    #[expect(dead_code)]
    pub fn services(&self) -> &[description::Service] {
        &self.description.device.service_list
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: u8) {
        let volume = volume.clamp(0, 100);

        let req = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:SetVolume xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1">
      <InstanceID>0</InstanceID>
      <Channel>Master</Channel>
      <DesiredVolume>%%VOL%%</DesiredVolume>
    </u:SetVolume>
  </s:Body>
</s:Envelope>"#;

        let (before, after) = req.split_once("%%VOL%%").unwrap();
        let body = format!("{before}{volume}{after}");

        let url = Uri::builder()
            .scheme(self.location.scheme().unwrap().clone())
            .authority(self.location.authority().unwrap().as_str())
            .path_and_query("/upnp/control/RenderingControl1")
            .build()
            .unwrap();

        let mut req = Request::post(url, body.into());
        req.headers
            .insert("Content-Type", "text/xml; charset=\"utf-8\"");
        req.headers
            .insert("SOAPACTION", "\"urn:schemas-upnp-org:service:RenderingControl:1#SetVolume\"");

        ehttp::fetch(req, |res| eprintln!("SetVolume result: {:?}", res));

        self.volume = volume;
    }

    pub fn get_volume(&mut self) {
        let req = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:GetVolume xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1">
      <InstanceID>0</InstanceID>
      <Channel>Master</Channel>
    </u:GetVolume>
  </s:Body>
</s:Envelope>"#;

        let url = Uri::builder()
            .scheme(self.location.scheme().unwrap().clone())
            .authority(self.location.authority().unwrap().as_str())
            .path_and_query("/upnp/control/RenderingControl1")
            .build()
            .unwrap();

        let mut req = Request::post(url, req.into());
        req.headers
            .insert("Content-Type", "text/xml; charset=\"utf-8\"");
        req.headers
            .insert("SOAPACTION", "\"urn:schemas-upnp-org:service:RenderingControl:1#GetVolume\"");

        let res = ehttp::fetch_blocking(&req).expect("Failed to fetch GetVolume");

        assert_eq!(res.status, 200, "Failed to fetch GetVolume");

        self.volume = std::str::from_utf8(&res.bytes)
            .expect("GetVolume response not valid UTF-8")
            .split_once("<CurrentVolume>")
            .expect("Failed to find <CurrentVolume> in GetVolume response")
            .1
            .split_once("</CurrentVolume>")
            .expect("Failed to find </CurrentVolume> in GetVolume response")
            .0
            .parse()
            .expect("Failed to parse CurrentVolume from GetVolume response")
    }
}
