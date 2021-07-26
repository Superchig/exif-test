use std::error;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("You must input the name of the exif file.");

        return Ok(());
    }

    let bytes = std::fs::read(&args[1])?;

    let exif_header = b"Exif\x00\x00";
    let exif_header_index = find_bytes(&bytes, exif_header).expect("Unable to find Exif header!");

    let tiff_index = exif_header_index + exif_header.len();
    let tiff_bytes = &bytes[tiff_index..];

    let byte_order = match &tiff_bytes[0..=1] {
        b"II" => Endian::LittleEndian,
        b"MM" => Endian::BigEndian,
        _ => panic!("Unable to determine endianness of TIFF section!"),
    };

    println!("Byte order: {:?}", byte_order);

    if tiff_bytes[2] != 42 && tiff_bytes[3] != 42 {
        panic!("Could not confirm existence of TIFF section with 42!");
    }

    // From the beginning of the TIFF section
    let first_ifd_offset = usizeify(&tiff_bytes[4..=7], byte_order);

    let num_ifd_entries = usizeify(
        &tiff_bytes[first_ifd_offset..first_ifd_offset + 2],
        byte_order,
    );

    println!("Number of entries in first IFD: {}", num_ifd_entries);

    let first_ifd_entry_offset = first_ifd_offset + 2;

    let mut ifd_entries = vec![];
    for entry_index in 0..num_ifd_entries {
        let entry_bytes = &tiff_bytes[first_ifd_entry_offset + (12 * entry_index)..];
        let entry = IFDEntry::from_slice(entry_bytes, byte_order);
        ifd_entries.push(entry);

        println!(
            "IFD Entry {}: {:#?}",
            entry_index,
            ifd_entries.last().unwrap()
        );
    }

    // println!("test: {:x?}", usizeify(b"\x00\x60\x00\x00", Endian::BigEndian));
    println!("test: 0x{:08x?}", usizeify(b"\x10\x0a\x10\x00", Endian::BigEndian));

    Ok(())
}

#[derive(Debug)]
struct IFDEntry {
    tag: EntryTag,
    field_type: EntryType,
    count: u32,
    value_offset: u32,
}

impl IFDEntry {
    fn from_slice(ifd_bytes: &[u8], byte_order: Endian) -> IFDEntry {
        let mut ifd_advance = 0;

        // Bytes 0-1
        let entry_tag = usizeify(take_bytes(ifd_bytes, &mut ifd_advance, 2), byte_order);

        assert_eq!(ifd_advance, 2);

        let field_type_hex = take_bytes(ifd_bytes, &mut ifd_advance, 2);
        let field_type = usizeify(field_type_hex, byte_order);

        let count = usizeify(take_bytes(ifd_bytes, &mut ifd_advance, 4), byte_order);

        // FIXME(Chris): Correctly read in the byte order on big-endian TIFF data
        let value_offset = usizeify(
            // match byte_order {
            //     Endian::LittleEndian => {
            //         if count <= 4 {
            //             &take_bytes(ifd_bytes, &mut ifd_advance, 4)[..count]
            //         } else {
            //             &take_bytes(ifd_bytes, &mut ifd_advance, 4)
            //         }
            //     }
            //     Endian::BigEndian => {
            //         if count <= 4 {
            //             &take_bytes(ifd_bytes, &mut ifd_advance, 4)[count..]
            //         } else {
            //             &take_bytes(ifd_bytes, &mut ifd_advance, 4)
            //         }
            //     }
            // },
            &take_bytes(ifd_bytes, &mut ifd_advance, 4),
            byte_order,
            // if count <= 4 {
            //     count
            // } else {
            //     4
            // },
        );

        IFDEntry {
            tag: EntryTag::from_usize(entry_tag),
            field_type: EntryType::from_usize(field_type),
            count: count as u32,
            value_offset: value_offset as u32,
        }
    }
}

#[derive(Debug)]
enum EntryTag {
    Orientation = 274,
    Unimplemented,
}

impl EntryTag {
    fn from_usize(value: usize) -> EntryTag {
        match value {
            274 => EntryTag::Orientation,
            _ => EntryTag::Unimplemented,
        }
    }
}

#[derive(Debug)]
enum EntryType {
    Short = 3,
    Unimplemented,
}

impl EntryType {
    fn from_usize(value: usize) -> EntryType {
        match value {
            3 => EntryType::Short,
            _ => EntryType::Unimplemented,
        }
    }
}

fn take_bytes<'a>(bytes: &'a [u8], byte_advance: &mut usize, n: usize) -> &'a [u8] {
    let old_advance = *byte_advance;

    *byte_advance += n;

    &bytes[old_advance..old_advance + n]
}

#[derive(Debug, Copy, Clone)]
enum Endian {
    LittleEndian,
    BigEndian,
}

// Converts a slice of bytes into a usize, depending on the Endianness
// NOTE(Chris): It seems like we could probably do this faster by using an unsafe copy of memory
// from the slice into a usize value.
fn usizeify(bytes: &[u8], byte_order: Endian) -> usize {
    match byte_order {
        Endian::LittleEndian => bytes.iter().enumerate().fold(0usize, |sum, (index, byte)| {
            sum + ((*byte as usize) << (index * 8))
        }),
        Endian::BigEndian => bytes
            .iter()
            .rev()
            .enumerate()
            .fold(0usize, |sum, (index, byte)| {
                sum + ((*byte as usize) << (index * 8))
            }),
    }
}

fn usizeify_n(bytes: &[u8], byte_order: Endian, n: usize) -> usize {
    match byte_order {
        Endian::LittleEndian => bytes
            .iter()
            .take(n)
            .enumerate()
            .fold(0usize, |sum, (index, byte)| {
                sum + ((*byte as usize) << (index * 8))
            }),
        Endian::BigEndian => bytes
            .iter()
            .rev()
            .take(n)
            .enumerate()
            .fold(0usize, |sum, (index, byte)| {
                sum + ((*byte as usize) << (index * 8))
            }),
    }
}

// fn find_bytes_bool(haystack: &[u8], needle: &[u8]) -> bool {
//     match find_bytes(haystack, needle) {
//         Some(_) => true,
//         None => false,
//     }
// }

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let mut count = 0;
    for (index, byte) in haystack.iter().enumerate() {
        if *byte == needle[count] {
            count += 1;
        } else {
            count = 0;
        }

        if count == needle.len() {
            // Add 1 because index is 0-based but needle.len() is not
            return Some(index - needle.len() + 1);
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_bytes() {
        let haystack = b"blah blah blah \x01\x01\x24\x59\xab\xde\xad\xbe\xef wow this is something";
        let needle = b"Exif\x00\x00";

        assert_eq!(find_bytes(haystack, needle), None);

        let haystack2 = b"blah blah blah \x01\x01\x24\x59\xabExif\x00\x00\xde\xad\xbe\xef wow this
            is something";

        assert_eq!(find_bytes(haystack2, needle), Some(20));
        if let Some(index) = find_bytes(haystack2, needle) {
            assert_eq!(&haystack2[index..index + needle.len()], needle);
        }
    }

    #[test]
    fn test_usizeify() {
        assert_eq!(usizeify(b"\x12\x34\x56\x78", Endian::BigEndian), 305419896);
        assert_eq!(
            usizeify(b"\x78\x56\x34\x12", Endian::LittleEndian),
            305419896
        );
    }
}
