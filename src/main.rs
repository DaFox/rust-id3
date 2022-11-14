use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;



/**
 *
 */
struct ID3 {
	header: ID3Header,
	frames: Vec<ID3Frame>
}

impl ID3 {
	pub fn find_frame_by_name(&self, name: &str) -> Option<&ID3Frame> {
		self.frames.iter().find(|frame| frame.id == name.to_string())
	}
}











fn read_bytes(file: &mut dyn Read, size: usize) -> std::io::Result<Vec<u8>> {
	let mut buffer = vec![0; size];
	file.read(&mut buffer)?;
	
	Ok(buffer)
}

fn read_u32_syncsafe(file: &mut dyn Read) -> std::io::Result<u32> {
	let mut buffer = [0u8; 4];
	file.read(&mut buffer)?;
	
	Ok(
		((buffer[0] as u32) << 21) + 
		((buffer[1] as u32) << 14) + 
		((buffer[2] as u32) << 7) + 
		(buffer[3] as u32)
	)
}

fn read_u32(file: &mut dyn Read) -> std::io::Result<u32> {
	let mut buffer = [0u8; 4];
	file.read(&mut buffer)?;
	Ok(u32::from_be_bytes(buffer[0..4].try_into().unwrap()))
}

fn read_u16(file: &mut dyn Read) -> std::io::Result<u16> {
	let mut buffer = [0u8; 2];
	file.read(&mut buffer)?;
	Ok(u16::from_be_bytes(buffer[0..2].try_into().unwrap()))
}

fn read_u8(file: &mut dyn Read) -> std::io::Result<u8> {
	Ok(u8::from_be_bytes(read_bytes(file, 1)?[0..1].try_into().unwrap()))
}

fn read_tag(mut file: &File) -> std::io::Result<String> {
	file.seek(SeekFrom::Start(0));
	
	let bytes = read_bytes(&mut file, 3)?;
	Ok(String::from_utf8_lossy(&bytes).to_string())
}


fn file_has_tag(file: &File) -> bool {
	read_tag(file).unwrap_or("".to_string()) == "ID3".to_string()
}

#[derive(Debug)]
struct ID3Header {
	version: (u8, u8),
	flags: u8,
	size: u32
}

impl ID3Header {
	fn read_from_file(mut file: &File) -> std::io::Result<ID3Header> {
		file.seek(SeekFrom::Start(0))?;
		
		// TODO implement a read check for this one
		// TODO implement proper error handling
		if read_tag(file)? != "ID3" {
			
		}
		
		let major = read_u8(&mut file)?;
		let minor = read_u8(&mut file)?;
		let version = (major, minor);
		
		let flags = read_u8(&mut file)?;
		let size = read_u32_syncsafe(&mut file)?;
		
		Ok(Self {
			version,
			flags,
			size
		})
	}
	
	fn has_id3_tag(file: &File) -> bool
	{
		read_tag(file).unwrap_or("".to_string()) == "ID3".to_string()
	}
}


struct ID3FrameHeader {
	id: String,
	size: u32,
	flags: u16	
}

#[derive(Debug)]
enum ID3FrameContent {
	Text {
		enc: u8,
		info: String
	},
	Unknown {
		content: Vec<u8>
	}
}

impl ID3FrameContent {
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
	fn decode_text(enc: u8, text: &[u8]) -> String {
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
	
	fn from_bytes(id: &str, content: Vec<u8>) -> ID3FrameContent {
		let enc = content[0];
		let info = &content[1..];
		
		match id {
			"TALB" | "TIT2" => ID3FrameContent::Text {
				enc,
				info: ID3FrameContent::decode_text(enc, info)
			},
			_ => ID3FrameContent::Unknown { content: vec![] }
		}
	}
	
	fn as_str(&self) -> String
	{
		match self {
			ID3FrameContent::Text { enc, info } => info.to_string(),
			_ => "".to_string()
		}
	}
}


#[derive(Debug)]
struct ID3Frame {
	id: String,
	size: u32,
	flags: u16,
	content: ID3FrameContent
}

impl ID3Frame {
	fn read_all_frames(mut file: &File, version: u8) -> std::io::Result<Vec<ID3Frame>> {
		let mut frames = vec![];
		
		// read only a single frame for now
		while let Some(f) = ID3Frame::read_frame(file, version)? {
			frames.push(f);
		}
		
		Ok(frames)
	}
	
	fn read_frame(mut file: &File, version: u8) -> std::io::Result<Option<ID3Frame>> {
		let id = read_bytes(&mut file, 4)?;
		
		if id[0] == 0 {
			file.seek(SeekFrom::Current(-4));
			return Ok(None);
		}
		
		let id = String::from_utf8_lossy(&id).to_string();
		
		let size = match version {
			4 => read_u32_syncsafe(&mut file)?,
			_ => read_u32(&mut file)?
		};
		
		let flags = read_u16(&mut file)?;
		let content = read_bytes(&mut file, size as usize)?;
		
		Ok(Some(ID3Frame {
			id: id.clone(),
			size,
			flags,
			content: ID3FrameContent::from_bytes(&id, content)
		}))
	}
	
	
}

fn main() -> std::io::Result<()> {
	let mut args = std::env::args();
	let this = args.next().unwrap();
	
	if args.len() < 1 {
		println!("Usage: {} <file>", &this);
		std::process::exit(1);
	}
	
	let path = args.next().unwrap();
	
	let mut file = File::open(path)?;
	let header = ID3Header::read_from_file(&file)?;
	let frames = ID3Frame::read_all_frames(&file, header.version.0)?;
	
	
	let id3 = ID3 {
		header: header,
		frames: frames
	};
	
	//let sum = frames.iter().fold(0, |sum, x| sum + x.size + 10);
	
	// Skip the padding. This is only necessary if a footer is present.
	//read_bytes(&mut file, (header.size - sum) as usize)?;

    println!("Title: {}", &id3.find_frame_by_name("TIT2").unwrap().content.as_str());
    println!("Album: {}", &id3.find_frame_by_name("TALB").unwrap().content.as_str());
    //println!("Hello, world! {:?}", frames.iter().map(|f| f.id.clone()));
    //println!("Hello, world! {:?}", padding);
	
	Ok(())
}
