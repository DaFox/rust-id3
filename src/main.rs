use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;

type ByteReaderResult<T> = Result<T, Box<dyn std::error::Error>>;

trait ByteReader {
	fn read_bytes(&mut self, size: usize) -> ByteReaderResult<Vec<u8>>;
	fn seek(&mut self, mode: SeekFrom) -> ByteReaderResult<u64>;

	fn read_u8(&mut self) -> ByteReaderResult<u8> {
		Ok(u8::from_be_bytes(self.read_bytes(1)?.try_into().unwrap()))
	}
	
	fn read_u16(&mut self) -> ByteReaderResult<u16> {
		Ok(u16::from_be_bytes(self.read_bytes(2)?.try_into().unwrap()))
	}
	
	fn read_u32(&mut self) -> ByteReaderResult<u32> {
		Ok(u32::from_be_bytes(self.read_bytes(4)?.try_into().unwrap()))
	}
	
	fn read_u32_syncsafe(&mut self) -> ByteReaderResult<u32> {
		let mut buffer = self.read_bytes(4)?;
		
		Ok(
			((buffer[0] as u32) << 21) + 
			((buffer[1] as u32) << 14) + 
			((buffer[2] as u32) << 7) + 
			(buffer[3] as u32)
		)
	}
}

impl<T: Read + Seek> ByteReader for T {
	fn read_bytes(&mut self, size: usize) -> ByteReaderResult<Vec<u8>> {
		let mut buffer = vec![0; size];
		self.read(&mut buffer)?;
	
		Ok(buffer)
	}
	
	fn seek(&mut self, mode: SeekFrom) -> ByteReaderResult<u64> {
		Ok(Seek::seek(self, mode)?)
	}
}




/**
 *
 */
 #[derive(Debug)]
struct ID3 {
	header: ID3Header,
	body: ID3Body
}

impl ID3 {
	/**
	 * Create an empty ID3 tag instance
	 */
	pub fn new() -> Self {
		Self {
			header: ID3Header::new(),
			body: ID3Body::new()
		}
	}
	
	pub fn new_from_byte_reader(reader: &mut dyn ByteReader) -> Result<Self, Box<dyn std::error::Error>> {
		let header = ID3Header::new_from_byte_reader(reader)?;
		let mut size = header.size;
		
		let mut extended_header: Option<ID3ExtendedHeader> = None;
		
		if header.flags.contains(ID3HeaderFlags::EXTENDED_HEADER) {
			extended_header = Some(ID3ExtendedHeader::new_from_byte_reader(reader)?);
			size -= extended_header.unwrap().size;
		}
		
		let body = ID3Body::new_from_byte_reader(reader, header.version.0)?;
		
		Ok(Self {
			header,
			body
		})
	}
	
	pub fn find_frame_by_name(&self, name: &str) -> Option<&ID3Frame> {
		self.body.find_frame_by_name(name)
	}
	
	pub fn find_frames_by_name(&self, name: &str) -> Vec<&ID3Frame> {
		self.body.find_frames_by_name(name)
	}
}

use std::fmt;

impl fmt::Display for ID3 {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, {}", self.header, self.body)
    }
}

#[derive(Debug)]
struct ID3Body {
	frames: Vec<ID3Frame>
}

impl ID3Body {
	fn new() -> Self {
		Self {
			frames: vec![]
		}
	}
	
	fn new_from_byte_reader(reader: &mut dyn ByteReader, version: u8) -> Result<Self, Box<dyn std::error::Error>> {
		let mut frames = vec![];
		
		// read only a single frame for now
		while let Some(frame) = ID3Frame::new_from_byte_reader(reader, version)? {
			frames.push(frame);
		}
		
		Ok(Self {
			frames
		})
	}
	
	fn find_frame_by_name(&self, name: &str) -> Option<&ID3Frame> {
		self.frames.iter().find(|frame| frame.get_header().id == name.to_string())
	}
	
	fn find_frames_by_name(&self, name: &str) -> Vec<&ID3Frame> {
		self.frames.iter().filter(|frame| frame.get_header().id == name.to_string()).collect()
	}
}

impl fmt::Display for ID3Body {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Frames: {}", self.frames.len())
    }
}

use bitflags::bitflags;

bitflags!{
	struct ID3HeaderFlags: u8 {
		const UNSYNCHRONISATION = 0b10000000;
		const EXTENDED_HEADER   = 0b01000000;
		const EXPERIMENTAL      = 0b00100000;
	}
}

#[derive(Debug)]
struct ID3ExtendedHeader {
	size: u32
}

impl ID3ExtendedHeader {
	
	// TODO: parse extended header content header
	fn new_from_byte_reader(reader: &mut dyn ByteReader) -> Result<Self, Box<dyn std::error::Error>> {
		let size = reader.read_u32_syncsafe()?;
		
		Ok(Self { 
			size
		})
	}
}

#[derive(Debug)]
struct ID3Header {
	version: (u8, u8),
	flags: ID3HeaderFlags,
	size: u32
}

impl ID3Header {
	fn new() -> Self {
		Self {
			version: (3, 0),
			flags: ID3HeaderFlags::empty(),
			size: 0
		}
	}
	
	fn new_from_byte_reader(reader: &mut dyn ByteReader) -> Result<Self, Box<dyn std::error::Error>> {
		// TODO implement a read check for this one
		// TODO implement proper error handling
		if Self::read_tag(reader)? != "ID3" {
			
		}
		
		let major = reader.read_u8()?;
		let minor = reader.read_u8()?;
		let version = (major, minor);
		
		let flags = ID3HeaderFlags::from_bits_truncate(reader.read_u8()?);
		let size = reader.read_u32_syncsafe()?;
		
		Ok(Self {
			version,
			flags,
			size
		})
	}
	
	fn read_tag(reader: &mut dyn ByteReader) -> Result<String, Box<dyn std::error::Error>> {
		let bytes = reader.read_bytes(3)?;
		Ok(String::from_utf8_lossy(&bytes).to_string())
	}
}

impl fmt::Display for ID3Header {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ID3 v2.{}.{} - Size: {}, Flags: {:b}", self.version.0, self.version.1, self.size, self.flags.bits())
    }
}

// v2.3 flags
bitflags!{
	struct ID3FrameHeaderFlagsV3: u32 {
		const TAG_ALTER_PRESERVED   = 0b1000000000000000;
		const FILE_ALTER_PRESERVED  = 0b0100000000000000;
		const READ_ONLY             = 0b0010000000000000;
		const COMPRESSED            = 0b0000000010000000;
		const ENCRYPTED             = 0b0000000001000000;
		const GROUPING              = 0b0000000000100000;
	}
}

impl Into<ID3FrameHeaderFlags> for ID3FrameHeaderFlagsV3 {
	fn into(self) -> ID3FrameHeaderFlags {
		let mut flags = ID3FrameHeaderFlags::empty();
		
		flags.set(ID3FrameHeaderFlags::TAG_ALTER_PRESERVED, self.contains(Self::TAG_ALTER_PRESERVED));
		flags.set(ID3FrameHeaderFlags::FILE_ALTER_PRESERVED, self.contains(Self::FILE_ALTER_PRESERVED));
		flags.set(ID3FrameHeaderFlags::READ_ONLY, self.contains(Self::READ_ONLY));
		flags.set(ID3FrameHeaderFlags::COMPRESSED, self.contains(Self::COMPRESSED));
		flags.set(ID3FrameHeaderFlags::ENCRYPTED, self.contains(Self::ENCRYPTED));
		flags.set(ID3FrameHeaderFlags::GROUPING, self.contains(Self::GROUPING));
		
		flags
	}
}

bitflags!{
	struct ID3FrameHeaderFlags: u32 {
		// v2.4 flags (<< 16)
		const TAG_ALTER_PRESERVED   = 0b0100000000000000;
		const FILE_ALTER_PRESERVED  = 0b0010000000000000;
		const READ_ONLY             = 0b0001000000000000;
		const GROUPING              = 0b0000000001000000;
		const COMPRESSED            = 0b0000000000001000;
		const ENCRYPTED             = 0b0000000000000100;
		const UNSYNCHRONISATION     = 0b0000000000000010;
		const DATA_LENGTH_INDICATOR = 0b0000000000000001;
	}
}

#[derive(Debug)]
struct ID3FrameHeader {
	id: String,
	size: u32,
	header_size: u32,
	uncompressed_body_size: Option<u32>,
	grouping_id: Option<u8>,
	flags: ID3FrameHeaderFlags	
}

impl ID3FrameHeader {
	
	fn new_from_byte_reader(reader: &mut dyn ByteReader, version: u8) -> Result<Option<Self>, Box<dyn std::error::Error>> {
		let id = reader.read_bytes(4)?;
		
		if id[0] == 0 {
			reader.seek(SeekFrom::Current(-4))?;
			return Ok(None);
		}
		
		let id = String::from_utf8_lossy(&id).to_string();
		
		let size = match version {
			4 => reader.read_u32_syncsafe()?,
			_ => reader.read_u32()?
		};
		
		// NOTE: v2.4 has different flags?
		let flags: ID3FrameHeaderFlags = match version {
			4 => ID3FrameHeaderFlags::from_bits_truncate(reader.read_u16()?.into()),
			_ => ID3FrameHeaderFlagsV3::from_bits_truncate(reader.read_u16()?.into()).into()
		};
		
		let mut header_size = 10;
		let mut grouping_id: Option<u8> = None;
		
		if flags.contains(ID3FrameHeaderFlags::GROUPING) {
			grouping_id = Some(reader.read_u8()?);
			header_size += 1;
		}
		
		let mut uncompressed_body_size: Option<u32> = None;
		
		if flags.contains(ID3FrameHeaderFlags::DATA_LENGTH_INDICATOR) {
			uncompressed_body_size = Some(reader.read_u32_syncsafe()?);
			header_size += 4;
		}
		
		if flags.contains(ID3FrameHeaderFlags::COMPRESSED) {
			todo!("Compressed frames");
		}
		
		if flags.contains(ID3FrameHeaderFlags::ENCRYPTED) {
			todo!("Encrypted frames");
		}
		
		Ok(Some(Self {
			id: id.clone(),
			size,
			header_size,
			uncompressed_body_size,
			flags,
			grouping_id
		}))
	}
	
	fn is_text_frame(&self) -> bool {
		self.id.starts_with("T") && self.id != "TXXX"
	}
}


#[derive(Debug)]
enum ID3Frame {
	Text {
		header: ID3FrameHeader,
		text: String
	},
	Unknown {
		header: ID3FrameHeader,
		body: Vec<u8>
	}
}

impl ID3Frame {
	fn new_from_byte_reader(reader: &mut dyn ByteReader, version: u8) -> Result<Option<ID3Frame>, Box<dyn std::error::Error>> {
		let header = ID3FrameHeader::new_from_byte_reader(reader, version)?;
		
		match header {
			Some(header) => {
				let buffer = reader.read_bytes(header.size as usize)?;

				if header.is_text_frame() {
					Ok(Some(Self::Text {
						header: header,
						text: Self::text_frame_content(buffer)
					}))
				} else {
					Ok(Some(Self::Unknown {
						header: header,
						body: buffer
					}))
				}
			},
			None => {
				reader.seek(SeekFrom::Current(-4))?;
				Ok(None)
			}
		}
	}
	
	fn get_header(&self) -> &ID3FrameHeader {
		match self {
			Self::Text { header, text } => header,
			Self::Unknown { header, body } => header
		}
	}
	
	/*
	
	 Frames that allow different types of text encoding contains a text
     encoding description byte. Possible encodings:

     $00   ISO-8859-1 [ISO-8859-1]. Terminated with $00.
     $01   UTF-16 [UTF-16] encoded Unicode [UNICODE] with BOM. All
           strings in the same frame SHALL have the same byteorder.
           Terminated with $00 00.
     $02   UTF-16BE [UTF-16] encoded Unicode [UNICODE] without BOM.
           Terminated with $00 00.
     $03   UTF-8 [UTF-8] encoded Unicode [UNICODE]. Terminated with $00.
	 
	*/
	fn text_frame_content(content: Vec<u8>) -> String {
		let enc = content[0];
		let text = &content[1..];
		
		match enc {
			0x00 | 0x03 => {
				// Some tags seem to have the text not terminated with \0
				let mut end = text.len();
				
				for i in 0..text.len() {
					if text[i] == 0 {
						end = i;
						break;
					}
				}
				
				String::from_utf8_lossy(&text[0..end]).to_string()
			},
			_ => String::new()
		}
	}
}


impl fmt::Display for ID3Frame {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Text { header, text } => write!(f, "Text({})", text),
			Self::Unknown { header, body } => write!(f, "Unknown()")
		}
        
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut args = std::env::args();
	let this = args.next().unwrap();
	
	if args.len() < 1 {
		println!("Usage: {} <file>", &this);
		std::process::exit(1);
	}
	
	let path = args.next().unwrap();
	
	let mut reader = File::open(path)?;
	let id3 = ID3::new_from_byte_reader(&mut reader)?;
	
	println!("{}", id3);
	println!("{}", id3.find_frame_by_name("TALB").unwrap());
	println!("{}", id3.find_frame_by_name("TIT2").unwrap());
	
	//let frames = ID3Frame::read_all_frames(&reader, header.version.0)?;
	
	
	/*let id3 = ID3 {
		header: header,
		frames: vec![]
	};
	
	//let sum = frames.iter().fold(0, |sum, x| sum + x.size + 10);
	
	// Skip the padding. This is only necessary if a footer is present.
	//read_bytes(&mut file, (header.size - sum) as usize)?;

    //println!("Title: {}", &id3.find_frame_by_name("TIT2").unwrap().content.as_str());
    //println!("Album: {}", &id3.find_frame_by_name("TALB").unwrap().content.as_str());
    //println!("Hello, world! {:?}", frames.iter().map(|f| f.id.clone()));
    
	*/
	Ok(())
}
