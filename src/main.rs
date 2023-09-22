use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// XML to Bracket Notation parser
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Filepath to XML file
    #[arg(short = 'F', long, value_name = "FILE", value_parser = path_exists)]
    filepath: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert XML to Bracket Notation
    Convert,

    /// Compares output with additional file
    Compare {
        /// Filepath to Bracket Notation file
        #[arg(short = 'D', long, value_name = "DIFF_FILE", value_parser = path_exists)]
        diffpath: PathBuf,
    },
}

#[inline(always)]
fn safe_trans(s: String) -> String {
    let s = s.replace('{', r"\{");
    let s = s.replace('}', r"\}");

    s.replace('\\', r"\\}")
}

fn convert_file(filepath: &PathBuf, filename: &OsStr) -> std::io::Result<()> {
    let file = File::open(filepath)?;
    let file = BufReader::new(file);
    let mut xml_reader = Reader::from_reader(file);
    xml_reader.trim_text(true);
    let mut buf = Vec::new();

    let mut outpath = PathBuf::from(filename);
    outpath.set_extension("bracket");

    let outfile = File::create(outpath)?;
    let mut writer = BufWriter::new(outfile);

    // let mut parser = EventReader::new(file);
    // let (Ok(root)) = parser.next() else {
    //     panic!("No root element found!");
    // };
    let mut depth = 0;
    let mut trees = 0;
    loop {
        match xml_reader.read_event_into(&mut buf) {
            Err(e) => panic!(
                "Error at position {}: {:?}",
                xml_reader.buffer_position(),
                e
            ),
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                depth += 1;
                if depth > 1 {
                    let mut attrs = e.attributes().map(|a| a.unwrap()).collect::<Vec<_>>();
                    attrs.sort_by(|a, b| a.key.cmp(&b.key));
                    let name = e.name();
                    let s = String::from_utf8(name.into_inner().to_vec()).unwrap();
                    let s = safe_trans(s);
                    write!(writer, "{{{s}")?;
                    for attr in attrs.iter() {
                        let key = String::from_utf8(attr.key.0.to_vec()).unwrap();
                        let key = safe_trans(key);
                        let value = String::from_utf8(attr.clone().value.into_owned()).unwrap();
                        let value = safe_trans(value);
                        write!(writer, "{{{}{{{}}}}}", key, value)?;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if depth > 1 {
                    if let Ok(unescaped) = e.unescape() {
                        let s = unescaped.into_owned();
                        let s = safe_trans(s);
                        write!(writer, "{{{s}}}")?;
                    } else {
                        dbg!(e);
                        break;
                    }
                }
            }
            Ok(Event::End(_)) => {
                if depth > 1 {
                    write!(writer, "}}")?;
                }
                depth -= 1;
                if depth == 1 {
                    writeln!(writer)?;
                    trees += 1;

                    if trees % 100_000 == 0 {
                        println!("{} trees parsed", trees);
                    }
                }
            }
            _ => {}
        }
    }
    buf.clear();
    Ok(())
}

fn compare_files(filepath: &PathBuf, diffpath: &PathBuf) -> std::io::Result<()> {
    let f1 = File::open(filepath)?;
    let f2 = File::open(diffpath)?;

    let f1 = BufReader::new(f1);
    let f2 = BufReader::new(f2);

    let mut line_counter = 0;

    for (l1, l2) in f1.lines().zip(f2.lines()) {
        line_counter += 1;
        let l1 = l1.unwrap();
        let l2 = l2.unwrap();
        if l1 != l2 {
            println!("{} != {} on line {}", l1, l2, line_counter);
            break;
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let filename = args.filepath.file_name().unwrap();

    match &args.command {
        Some(Commands::Compare { diffpath }) => compare_files(&args.filepath, diffpath),
        _ => convert_file(&args.filepath, filename),
    }
}

fn path_exists(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.exists() {
        Ok(path)
    } else {
        Err("File not found!".to_string())
    }
}
