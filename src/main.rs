use std::{env, fs::File, io::{self, Read, Seek, SeekFrom, Write}, mem, slice};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct CompressedSection {
    pub size: i32,
    pub offset: i32
}

fn unpack(input_file: &mut File, output_file: &mut File) -> io::Result<()> {
    let mut data = vec![0u8; 0x1000];

    input_file.seek(SeekFrom::Start(0))?;
    input_file.read_exact(&mut data[..])?;

    let mut sections = vec![];

    for _ in 0..1024 {
        let mut section = CompressedSection { size: 0, offset: 0 };
        
        input_file.read_exact(unsafe {
            slice::from_raw_parts_mut(&mut section as *mut _ as *mut u8, mem::size_of_val(&section))
        })?;

        if section.size == 0 || section.offset < 0x3000 {
            break;
        }

        sections.push(section);
    }

    for section in sections {
        if section.offset == 0 && section.size == 0 {
            break;
        }
        
        let size = section.size.abs() as usize;
		let mut section_data = vec![0u8; size];
		
        input_file.seek(SeekFrom::Start(section.offset as u64))?;
        input_file.read_exact(&mut section_data[..])?;
        
        if section.size < 0 {
            data.extend_from_slice(&section_data[..])
        } else {
            match inflate::inflate_bytes_zlib(&section_data[..]) {
                Ok(inflated_data) => data.extend_from_slice(&inflated_data[..]),
                Err(error_message) => return Err(io::Error::new(io::ErrorKind::InvalidData, error_message))
            }
        }
    }

    output_file.write_all(&data[..])
}

fn pack(input_file: &mut File, output_file: &mut File, should_compress: bool) -> io::Result<()> {
    let mut data = vec![0u8; 0x1000];

    input_file.seek(SeekFrom::Start(0))?;
    input_file.read_exact(&mut data[..])?;
    
    data.resize(0x3000, 0);

    let mut sections = [CompressedSection { size: 0, offset: 0 }; 1024];
    
    for section in sections.iter_mut() {
        let mut section_data = vec![0u8; 0x40000];
        let len = input_file.read(&mut section_data[..])?;
        
        if len == 0 {
            break;
        }
        
        section.offset = data.len() as i32;
        
        if should_compress {
            let compressed_data = deflate::deflate_bytes_zlib_conf(&section_data[..len], deflate::CompressionOptions::default().with_window_bits(10));
            data.extend_from_slice(&compressed_data[..]);
            section.size = compressed_data.len() as i32;
            let padding = ((section.offset + section.size + 0x79) & (!0x79)) - (section.offset + section.size);
            data.extend(vec![0u8; padding as usize]);
        } else {
            data.extend_from_slice(&section_data[..len]);
            section.size = -(len as i32);
        }
    }
    
    &mut data[0x1000..0x3000].copy_from_slice(unsafe {
        slice::from_raw_parts(sections.as_ptr() as *const u8, 0x2000)
    });

    output_file.write_all(&data[..])?;
    
    Ok(())
}

fn print_usage() {
    println!("Usage: <decompress|compress|pack> <input> <output>");
    std::process::exit(-1);
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        print_usage();
    }

    let mut input_file = File::open(args[2].clone())?;
    let mut output_file = File::create(args[3].clone())?;
    
    if args[1].starts_with('d') {
        unpack(&mut input_file, &mut output_file)
    } else if args[1].starts_with('c') {
        pack(&mut input_file, &mut output_file, true)
    } else if args[1].starts_with('p') {
        pack(&mut input_file, &mut output_file, false)
    } else {
        Ok(print_usage())
    }
}
