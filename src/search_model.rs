use std::{collections::HashMap, str::FromStr, io::BufReader};
use wincode::{SchemaRead, SchemaWrite};

fn create_document_from_text(text: &str) -> Document {
    let stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
    let mut words_map = HashMap::new();
    let mut current_word = String::new();

    let add_to_map = |word: &str, map: &mut HashMap<String, usize>| {
        if !word.is_empty() {
            let word = stemmer.stem(&word.to_lowercase()).to_string();
            *map.entry(word).or_insert(0) += 1;
        }
    };

    for c in text.chars() {
        if c.is_alphanumeric() || c == '\'' || c == '-' {
            current_word.push(c);
        } else {
            add_to_map(&current_word, &mut words_map);
            current_word.clear();
            if !c.is_whitespace() {
                add_to_map(&c.to_string(), &mut words_map);
            }
        }
    }

    add_to_map(&current_word, &mut words_map);

    Document { words: words_map }
}

#[derive(Debug)]
enum FileType {
    Xml,
    Pdf,
}

impl FromStr for FileType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "xml" | "xhtml" => Ok(Self::Xml),
            "pdf" => Ok(Self::Pdf),
            x => {
                eprintln!("[ERR]: File is of unindexable type {x}");
                Err(())
            }
        }
    }
}


fn analyze_file(p: &std::path::Path) -> Result<(String, Document), ()> {
    match p.extension() {
        None => {
            eprintln!("[ERR]: File is binary or other type of non-indexable file");
            return Err(());
        }
        Some(s) => match s.to_str().unwrap().parse() {
            Ok(FileType::Xml) => {
                let file = BufReader::new(std::fs::File::open(p).unwrap());
                let parser = xml::EventReader::new(file);
                let mut text = String::with_capacity(1024 * 1024);
                for e in parser {
                    match e {
                        Ok(xml::reader::XmlEvent::Characters(c)) => {
                            text.push_str(&c);
                            text.push(' ');
                        }
                        Err(e) => {
                            eprintln!("{}", e);
                        }
                        _ => {}
                    }
                }
                Ok((
                    p.to_string_lossy().to_string(),
                    create_document_from_text(&text),
                ))
            }
            Ok(FileType::Pdf) => {
                let doc = lopdf::Document::load(&p).unwrap();
                if doc.is_encrypted() {
                    eprintln!("[WARN]: Skipping encrypted .pdf file {}", p.display());
                    return Err(());
                }
                let page_nums: Vec<u32> = doc.get_pages().into_keys().collect();
                let text = doc.extract_text(&page_nums).unwrap();
                Ok((
                    p.to_string_lossy().to_string(),
                    create_document_from_text(&text),
                ))
            }
            Err(()) => {
                eprintln!("Ignoring binary file");
                Err(())
            }
        },
    }
}

pub fn analyze_dir(p: &std::path::Path) -> Result<HashMap<String, Document>, ()> {
    let mut map = HashMap::new();
    let mut on_going = vec![];
    for d in p.read_dir().unwrap() {
        let d = d.unwrap();
        if d.metadata().unwrap().is_file() {
            let Ok((p, f)) = analyze_file(&d.path()) else {
                continue;
            };
            map.insert(p, f);
        } else {
            let process = std::thread::spawn(move || analyze_dir(&d.path()));
            on_going.push(process);
        }
    }
    for p in on_going {
        let x = p.join().unwrap().unwrap();
        x.into_iter().for_each(|(k, v)| {
            map.insert(k, v);
        });
    }
    Ok(map)
}

#[derive(Debug, SchemaRead, SchemaWrite)]
pub struct Document {
    words: HashMap<String, usize>,
}

pub fn do_query(model: &HashMap<String, Document>, terms: &[&str]) -> Vec<String> {
    let en_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
    let mut docs = vec![];
    for (path, data) in model {
        let mut point = 0.0;
        for t in terms {
            let t = en_stemmer.stem(&t.to_lowercase()).to_string();
            let count = match data.words.get(&t) {
                None => {
                    continue;
                }
                Some(t) => *t,
            };
            let tf = count as f64 / data.words.values().copied().sum::<usize>() as f64;
            let idf = (model.iter().count() as f64
                / model
                    .iter()
                    .filter(|(_, d)| d.words.contains_key(&t))
                    .count() as f64)
                .log2();
            point += tf * idf;
        }
        docs.push((path, point));
    }
    docs.sort_by(|(_, b1), (_, a1)| a1.total_cmp(b1));
    docs.iter()
        .map(|(p, d)| (p, d))
        .filter(|(_p, d)| **d != 0.0)
        .map(|(p, _)| p.to_owned().clone())
        .collect()
}
