use std::io::Read;
use byteorder::{BigEndian, ReadBytesExt};

use {ErrorKind, Result};
use es::StreamType;
use ts::{Pid, VersionNumber};
use ts::psi::Psi;

/// Program Map Table.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pmt {
    pub program_num: u16,

    /// The packet identifier that contains the program clock reference (PCR).
    ///
    /// The PCR is used to improve the random access accuracy of the stream's timing
    /// that is derived from the program timestamp.
    pub pcr_pid: Option<Pid>,

    pub version_number: VersionNumber,
    pub table: Vec<EsInfo>,
}
impl Pmt {
    const TABLE_ID: u8 = 2;

    pub(super) fn read_from<R: Read>(reader: R) -> Result<Self> {
        let mut psi = track!(Psi::read_from(reader))?;
        track_assert_eq!(psi.tables.len(), 1, ErrorKind::InvalidInput);

        let table = psi.tables.pop().expect("Never fails");
        let header = table.header;
        track_assert_eq!(header.table_id, Self::TABLE_ID, ErrorKind::InvalidInput);
        track_assert!(!header.private_bit, ErrorKind::InvalidInput);

        let syntax = track_assert_some!(table.syntax.as_ref(), ErrorKind::InvalidInput);
        track_assert_eq!(syntax.section_number, 0, ErrorKind::InvalidInput);
        track_assert_eq!(syntax.last_section_number, 0, ErrorKind::InvalidInput);
        track_assert!(syntax.current_next_indicator, ErrorKind::InvalidInput);

        let mut reader = &syntax.table_data[..];

        let pcr_pid = track!(Pid::read_from(&mut reader))?;
        let pcr_pid = if pcr_pid.as_u16() == 0b0001_1111_1111_1111 {
            None
        } else {
            Some(pcr_pid)
        };

        let n = track_io!(reader.read_u16::<BigEndian>())?;
        track_assert_eq!(
            n & 0b1111_0000_0000_0000,
            0b1111_0000_0000_0000,
            ErrorKind::InvalidInput,
            "Unexpected reserved bits"
        );
        track_assert_eq!(
            n & 0b0000_1100_0000_0000,
            0,
            ErrorKind::InvalidInput,
            "Unexpected program info length unused bits"
        );
        let program_info_len = n & 0b0000_0011_1111_1111;
        track_assert_eq!(program_info_len, 0, ErrorKind::Unsupported);

        let mut table = Vec::new();
        while !reader.is_empty() {
            table.push(track!(EsInfo::read_from(&mut reader))?);
        }
        Ok(Pmt {
            program_num: syntax.table_id_extension,
            pcr_pid,
            version_number: syntax.version_number,
            table,
        })
    }
}

/// Elementary stream information.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EsInfo {
    pub stream_type: StreamType,

    /// The packet identifier that contains the stream type data.
    pub elementary_pid: Pid,

    pub descriptors: Vec<Descriptor>,
}
impl EsInfo {
    fn read_from<R: Read>(mut reader: R) -> Result<Self> {
        let stream_type = track_io!(reader.read_u8()).and_then(StreamType::from_u8)?;
        let elementary_pid = track!(Pid::read_from(&mut reader))?;

        let n = track_io!(reader.read_u16::<BigEndian>())?;
        track_assert_eq!(
            n & 0b1111_0000_0000_0000,
            0b1111_0000_0000_0000,
            ErrorKind::InvalidInput,
            "Unexpected reserved bits"
        );
        track_assert_eq!(
            n & 0b0000_1100_0000_0000,
            0,
            ErrorKind::InvalidInput,
            "Unexpected ES info length unused bits"
        );
        let es_info_len = n & 0b0000_0011_1111_1111;

        let mut reader = reader.take(u64::from(es_info_len));
        let mut descriptors = Vec::new();
        while reader.limit() > 0 {
            let d = track!(Descriptor::read_from(&mut reader))?;
            descriptors.push(d);
        }
        track_assert_eq!(reader.limit(), 0, ErrorKind::InvalidInput);

        Ok(EsInfo {
            stream_type,
            elementary_pid,
            descriptors,
        })
    }
}

/// Program or elementary stream descriptor.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Descriptor {
    pub tag: u8,
    pub data: Vec<u8>,
}
impl Descriptor {
    fn read_from<R: Read>(mut reader: R) -> Result<Self> {
        let tag = track_io!(reader.read_u8())?;
        let len = track_io!(reader.read_u8())?;
        let mut data = vec![0; len as usize];
        track_io!(reader.read_exact(&mut data))?;
        Ok(Descriptor { tag, data })
    }
}