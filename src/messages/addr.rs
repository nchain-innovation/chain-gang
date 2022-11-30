use crate::messages::message::Payload;
use crate::messages::node_addr_ex::NodeAddrEx;
use crate::util::{var_int, Error, Result, Serializable};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr};

/// Maximum number of addresses allowed in an Addr message
const MAX_ADDR_COUNT: u64 = 1000;

/// Known node addresses
#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct Addr {
    /// List of addresses of known nodes
    pub addrs: Vec<NodeAddrEx>,
}

impl Serializable<Addr> for Addr {
    fn read(reader: &mut dyn Read) -> Result<Addr> {
        let mut ret = Addr { addrs: Vec::new() };
        let count = var_int::read(reader)?;
        if count > MAX_ADDR_COUNT {
            let msg = format!("Too many addrs: {}", count);
            return Err(Error::BadData(msg));
        }
        for _i in 0..count {
            ret.addrs.push(NodeAddrEx::read(reader)?);
        }
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        var_int::write(self.addrs.len() as u64, writer)?;
        for item in self.addrs.iter() {
            item.write(writer)?;
        }
        Ok(())
    }
}

impl Payload<Addr> for Addr {
    fn size(&self) -> usize {
        var_int::size(self.addrs.len() as u64) + self.addrs.len() * NodeAddrEx::SIZE
    }
}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.addrs.len() <= 3 {
            f.debug_struct("Addr").field("addrs", &self.addrs).finish()
        } else {
            let s = format!("[<{} addrs>]", self.addrs.len());
            f.debug_struct("Addr").field("addrs", &s).finish()
        }
    }
}

/* Addrv2
    Addrv2 - https://developer.bitcoin.org/reference/p2p_networking.html
    BIP-155 address of peer https://github.com/bitcoin/bips/blob/master/bip-0155.mediawiki
*/
pub const IPV4_ADDRESS_LENGTH_BYTES: usize = 4;
pub const IPV6_ADDRESS_LENGTH_BYTES: usize = 16;
pub const TORV2_ADDRESS_LENGTH_BYTES: usize = 10;
pub const TORV3_ADDRESS_LENGTH_BYTES: usize = 32;
pub const I2P_ADDRESS_LENGTH_BYTES: usize = 32;
pub const CJDNS_ADDRESS_LENGTH_BYTES: usize = 16;

#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Torv2Addr([u8; TORV2_ADDRESS_LENGTH_BYTES]);

#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Torv3Addr([u8; TORV3_ADDRESS_LENGTH_BYTES]);

#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct I2PAddr([u8; I2P_ADDRESS_LENGTH_BYTES]);

#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct CJDNSAddr([u8; CJDNS_ADDRESS_LENGTH_BYTES]);

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Bip155 {
    IPV4(Ipv4Addr),
    IPV6(Ipv6Addr),
    TORV2(Torv2Addr),
    TORV3(Torv3Addr),
    I2P(I2PAddr),
    CJDNS(CJDNSAddr),
}

pub const IPV4_NETWORK_ID: u8 = 0x01;
pub const IPV6_NETWORK_ID: u8 = 0x02;
pub const TORV2_NETWORK_ID: u8 = 0x03;
pub const TORV3_NETWORK_ID: u8 = 0x04;
pub const I2P_NETWORK_ID: u8 = 0x05;
pub const CJDNS_NETWORK_ID: u8 = 0x06;

impl Bip155 {
    // Return the address length in bytes
    fn network_payload_size(&self) -> usize {
        match &self {
            Bip155::IPV4(_) => IPV4_ADDRESS_LENGTH_BYTES,
            Bip155::IPV6(_) => IPV6_ADDRESS_LENGTH_BYTES,
            Bip155::TORV2(_) => TORV2_ADDRESS_LENGTH_BYTES,
            Bip155::TORV3(_) => TORV3_ADDRESS_LENGTH_BYTES,
            Bip155::I2P(_) => I2P_ADDRESS_LENGTH_BYTES,
            Bip155::CJDNS(_) => CJDNS_ADDRESS_LENGTH_BYTES,
        }
    }
    // Return the network_id
    fn network_id(&self) -> u8 {
        match &self {
            Bip155::IPV4(_) => IPV4_NETWORK_ID,
            Bip155::IPV6(_) => IPV6_NETWORK_ID,
            Bip155::TORV2(_) => TORV2_NETWORK_ID,
            Bip155::TORV3(_) => TORV3_NETWORK_ID,
            Bip155::I2P(_) => I2P_NETWORK_ID,
            Bip155::CJDNS(_) => CJDNS_NETWORK_ID,
        }
    }
}

fn check_network_and_length(bip155_network_id: u8, length: u64) -> Result<()> {
    if bip155_network_id > CJDNS_NETWORK_ID {
        return Err(Error::BadData("Unknown network id".to_string()));
    }
    match bip155_network_id {
        IPV4_NETWORK_ID if length != IPV4_ADDRESS_LENGTH_BYTES as u64 => Err(Error::BadData(
            "Length incorrect for IPv4 address".to_string(),
        )),
        IPV6_NETWORK_ID if length != IPV6_ADDRESS_LENGTH_BYTES as u64 => Err(Error::BadData(
            "Length incorrect for IPv6 address".to_string(),
        )),
        TORV2_NETWORK_ID if length != TORV2_ADDRESS_LENGTH_BYTES as u64 => Err(Error::BadData(
            "Length incorrect for TorV2 address".to_string(),
        )),
        TORV3_NETWORK_ID if length != TORV3_ADDRESS_LENGTH_BYTES as u64 => Err(Error::BadData(
            "Length incorrect for TorV3 address".to_string(),
        )),
        I2P_NETWORK_ID if length != I2P_ADDRESS_LENGTH_BYTES as u64 => Err(Error::BadData(
            "Length incorrect for I2P address".to_string(),
        )),
        CJDNS_NETWORK_ID if length != CJDNS_ADDRESS_LENGTH_BYTES as u64 => Err(Error::BadData(
            "Length incorrect for CJDNS address".to_string(),
        )),
        _ => Ok(()),
    }
}

impl Serializable<Bip155> for Bip155 {
    fn read(reader: &mut dyn Read) -> Result<Bip155> {
        let bip155_network_id = reader.read_u8()?;
        let length = var_int::read(reader)?;
        check_network_and_length(bip155_network_id, length)?;

        match bip155_network_id {
            IPV4_NETWORK_ID => {
                let mut ip = [0; IPV4_ADDRESS_LENGTH_BYTES];
                reader.read_exact(&mut ip)?;
                let ip = Ipv4Addr::from(ip);
                Ok(Bip155::IPV4(ip))
            }
            IPV6_NETWORK_ID => {
                let mut ip = [0; IPV6_ADDRESS_LENGTH_BYTES];
                reader.read_exact(&mut ip)?;
                let ip = Ipv6Addr::from(ip);
                Ok(Bip155::IPV6(ip))
            }
            TORV2_NETWORK_ID => {
                let mut ip = [0; TORV2_ADDRESS_LENGTH_BYTES];
                reader.read_exact(&mut ip)?;
                let ip = Torv2Addr(ip);
                Ok(Bip155::TORV2(ip))
            }
            TORV3_NETWORK_ID => {
                let mut ip = [0; TORV3_ADDRESS_LENGTH_BYTES];
                reader.read_exact(&mut ip)?;
                let ip = Torv3Addr(ip);
                Ok(Bip155::TORV3(ip))
            }
            I2P_NETWORK_ID => {
                let mut ip = [0; I2P_ADDRESS_LENGTH_BYTES];
                reader.read_exact(&mut ip)?;
                let ip = I2PAddr(ip);
                Ok(Bip155::I2P(ip))
            }
            CJDNS_NETWORK_ID => {
                let mut ip = [0; CJDNS_ADDRESS_LENGTH_BYTES];
                reader.read_exact(&mut ip)?;
                let ip = CJDNSAddr(ip);
                Ok(Bip155::CJDNS(ip))
            }
            _ => Err(Error::BadData("Uknown network id".to_string())),
        }
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_u8(self.network_id())?;
        var_int::write(self.network_payload_size() as u64, writer)?;

        match self {
            Bip155::IPV4(addr) => writer.write_all(&addr.octets())?,
            Bip155::IPV6(addr) => writer.write_all(&addr.octets())?,
            Bip155::TORV2(addr) => writer.write_all(addr.0.as_slice())?,
            Bip155::TORV3(addr) => writer.write_all(addr.0.as_slice())?,
            Bip155::I2P(addr) => writer.write_all(addr.0.as_slice())?,
            Bip155::CJDNS(addr) => writer.write_all(addr.0.as_slice())?,
        };
        Ok(())
    }
}

impl Payload<Bip155> for Bip155 {
    fn size(&self) -> usize {
        1 + // as u8
        var_int::size(self.network_payload_size().try_into().unwrap()) +
        self.network_payload_size()
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct NodeAddrExV2 {
    /// Last connected time in seconds since the unix epoch
    pub last_connected_time: u32,
    /// Services flags for the node
    pub services: u64,
    /// IPV6 address for the node. IPV4 addresses may be used as IPV4-mapped IPV6 addresses.
    pub bip_address: Bip155,
    /// Port for Bitcoin P2P communication
    pub port: u16,
}

impl Serializable<NodeAddrExV2> for NodeAddrExV2 {
    fn read(reader: &mut dyn Read) -> Result<NodeAddrExV2> {
        Ok(NodeAddrExV2 {
            last_connected_time: reader.read_u32::<LittleEndian>()?,
            services: var_int::read(reader)?,
            bip_address: Bip155::read(reader)?,
            port: reader.read_u16::<BigEndian>()?,
        })
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_u32::<LittleEndian>(self.last_connected_time)?;
        var_int::write(self.services, writer)?;
        self.bip_address.write(writer)?;
        writer.write_u16::<BigEndian>(self.port)?;
        Ok(())
    }
}

impl Payload<NodeAddrExV2> for NodeAddrExV2 {
    fn size(&self) -> usize {
        4 + // last_connected_time u32
        var_int::size(self.services) +
        self.bip_address.size() +
        2 // port u16
    }
}

impl fmt::Debug for NodeAddrExV2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NodeAddrExV2")
            .field("last_connected_time", &self.last_connected_time)
            .field("services", &self.services)
            .field("bip_address", &self.bip_address)
            .field("port", &self.port)
            .finish()
    }
}

#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct AddrV2 {
    /// List of addresses of known nodes
    pub addrs: Vec<NodeAddrExV2>,
}

impl Serializable<AddrV2> for AddrV2 {
    fn read(reader: &mut dyn Read) -> Result<AddrV2> {
        let mut ret = AddrV2 { addrs: Vec::new() };
        let count = var_int::read(reader)?;
        if count > MAX_ADDR_COUNT {
            let msg = format!("Too many addrs: {}", count);
            return Err(Error::BadData(msg));
        }
        for _i in 0..count {
            ret.addrs.push(NodeAddrExV2::read(reader)?);
        }
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        var_int::write(self.addrs.len() as u64, writer)?;
        for item in self.addrs.iter() {
            item.write(writer)?;
        }
        Ok(())
    }
}

impl fmt::Debug for AddrV2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.addrs.len() <= 3 {
            f.debug_struct("AddrV2")
                .field("addrs", &self.addrs)
                .finish()
        } else {
            let s = format!("[<{} addrs>]", self.addrs.len());
            f.debug_struct("AddrV2").field("addrs", &s).finish()
        }
    }
}

impl Payload<AddrV2> for AddrV2 {
    fn size(&self) -> usize {
        var_int::size(self.addrs.len() as u64) + self.addrs.iter().map(|x| x.size()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::NodeAddr;
    use hex;
    use std::io::Cursor;
    use std::net::Ipv6Addr;

    #[test]
    fn read_bytes() {
        let b = hex::decode(
            "013c93dd5a250000000000000000000000000000000000ffff43cdb3a1479d".as_bytes(),
        )
        .unwrap();
        let a = Addr::read(&mut Cursor::new(&b)).unwrap();
        assert!(a.addrs.len() == 1);
        assert!(a.addrs[0].last_connected_time == 1524470588);
        assert!(a.addrs[0].addr.services == 37);
        let ip = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 67, 205, 179, 161];
        assert!(a.addrs[0].addr.ip.octets() == ip);
        assert!(a.addrs[0].addr.port == 18333);
    }

    #[test]
    fn write_read_addr() {
        let mut v = Vec::new();
        let addr1 = NodeAddrEx {
            last_connected_time: 100,
            addr: NodeAddr {
                services: 900,
                ip: Ipv6Addr::from([1; 16]),
                port: 2000,
            },
        };
        let addr2 = NodeAddrEx {
            last_connected_time: 200,
            addr: NodeAddr {
                services: 800,
                ip: Ipv6Addr::from([2; 16]),
                port: 3000,
            },
        };
        let addr3 = NodeAddrEx {
            last_connected_time: 700,
            addr: NodeAddr {
                services: 900,
                ip: Ipv6Addr::from([3; 16]),
                port: 4000,
            },
        };
        let f = Addr {
            addrs: vec![addr1, addr2, addr3],
        };
        f.write(&mut v).unwrap();
        assert!(v.len() == f.size());
        assert!(Addr::read(&mut Cursor::new(&v)).unwrap() == f);
    }

    #[test]
    fn write_read_addrv2() {
        let mut v = Vec::new();
        let addr1 = NodeAddrExV2 {
            last_connected_time: 100,
            services: 900,
            bip_address: Bip155::IPV6(Ipv6Addr::from([1; 16])),
            port: 2000,
        };
        let addr2 = NodeAddrExV2 {
            last_connected_time: 200,
            services: 800,
            bip_address: Bip155::IPV6(Ipv6Addr::from([2; 16])),
            port: 3000,
        };
        let addr3 = NodeAddrExV2 {
            last_connected_time: 700,
            services: 900,
            bip_address: Bip155::IPV6(Ipv6Addr::from([3; 16])),
            port: 4000,
        };
        let f = AddrV2 {
            addrs: vec![addr1, addr2, addr3],
        };
        f.write(&mut v).unwrap();
        dbg!(v.len());
        dbg!(f.size());
        assert!(v.len() == f.size());
        assert!(AddrV2::read(&mut Cursor::new(&v)).unwrap() == f);
    }
}
