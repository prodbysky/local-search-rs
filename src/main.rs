use raylib::prelude::RaylibDraw;
use raylib::text::RaylibFont;
use std::{
    collections::HashMap, io::{BufReader, Read}, str::FromStr
};
use serde::{Serialize, Deserialize};

use wincode::{SchemaRead, SchemaWrite};

// TODO: Reindex on directory update
// TODO: Flesh out the settings menu
// TODO: Animations?
// TODO: Optimization (when it becomes an issue)
// TODO: Setup links to applications for different file types to open on click
// TODO: More keybinds
// TODO: On indexing/refreshing the model do some sort of multithreading so the UI does not hang and also we can report indexing progress

#[derive(Serialize, Deserialize, Default, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

// NOTE: Here we use serde (toml) since its a config file come on guys
#[derive(Serialize, Deserialize, Default, Debug)]
struct Config {
    document_directories: Vec<String>,
    font_name: Option<String>,
    background_color: Option<Color>,
    foreground_color: Option<Color>,

    idle_color: Option<Color>,
    hovered_color: Option<Color>,
    clicked_color: Option<Color>,
}

const FONT: &[u8] = include_bytes!("../assets/GeistMonoNerdFontMono-Regular.otf");

struct App {
    raylib_thread: raylib::prelude::RaylibThread,
    raylib_handle: raylib::prelude::RaylibHandle,
    font: raylib::text::Font,
    bg_color: raylib::color::Color,
    fg_color: raylib::color::Color,
    idle_color: raylib::color::Color,
    hover_color: raylib::color::Color,
    click_color: raylib::color::Color,

    model: HashMap<String, Document>,
    docs: Vec<String>,
    query: String,

    query_box_selected: bool,

    scroll_velocity: raylib::math::Vector2,
    doc_offset: f32,

    conf: Config,

    display_profile_data: bool,

    index_file: std::path::PathBuf,
    boot_time: std::time::Duration,
    boot_index_time: std::time::Duration,
    update_time: std::time::Duration,
    draw_time: std::time::Duration,
    last_query_time: std::time::Duration,
    reindex_time: std::time::Duration,
}

impl App {
    fn init_directories() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let app_dirs = platform_dirs::AppDirs::new(Some("local-search"), false).unwrap();
        let mut document_base_dir = platform_dirs::UserDirs::new().unwrap().document_dir;
        document_base_dir.push("local-search");

        let config_file = app_dirs.config_dir.join("config.toml");
        let index_file = app_dirs.state_dir.join("index.bin");
        std::fs::create_dir_all(&app_dirs.config_dir).unwrap();
        std::fs::create_dir_all(&app_dirs.state_dir).unwrap();
        std::fs::create_dir_all(&document_base_dir).unwrap();
        (document_base_dir, config_file, index_file)
    }

    fn init_config(document_base_dir: &std::path::Path, config_file: &std::path::Path) -> Config {
        let mut config = Config::default();
        config
            .document_directories
            .push(document_base_dir.to_string_lossy().to_string());
        if config_file.exists() {
            let conf_file_content = std::fs::read_to_string(config_file).unwrap();
            config = toml::de::from_str(&conf_file_content).unwrap();
            for p in &mut config.document_directories {
                let np = std::path::PathBuf::from_str(p).unwrap();
                let mut copy = document_base_dir.to_path_buf();
                copy.push(np);
                *p = copy.to_string_lossy().to_string();
            }
        } else {
            std::fs::write(config_file, toml::ser::to_string_pretty(&config).unwrap()).unwrap();
        }
        config
    }

    fn init_model(index_file: &std::path::Path, conf: &Config) -> HashMap<String, Document> {
        let mut model = HashMap::new();
        if index_file.exists() {
            let mut b_reader = std::io::BufReader::new(
                std::fs::File::open(index_file).unwrap(),
            );
            let mut bytes = vec![];
            b_reader.read_to_end(&mut bytes).unwrap();
            model = wincode::deserialize(&bytes).unwrap();
        } else {
            for p in &conf.document_directories {
                let m = analyze_dir(&std::path::PathBuf::from(p)).unwrap();
                m.into_iter().for_each(|(k, v)| {
                    model.insert(k, v);
                });
            }
            std::fs::write(
                index_file,
                wincode::serialize(&model).unwrap(),
            )
            .unwrap();
        }
        model
    }

    pub fn new() -> Self {
        let init = std::time::Instant::now();
        let (mut h, t) = raylib::init()
            .msaa_4x()
            .size(1280, 720)
            .resizable()
            .vsync()
            .log_level(raylib::ffi::TraceLogLevel::LOG_FATAL)
            .build();

        h.set_exit_key(None);

        let (document_base_dir, config_file, index_file) = Self::init_directories();

        let config = Self::init_config(&document_base_dir, &config_file);

        let model_begin = std::time::Instant::now();
        let model = Self::init_model(&index_file, &config);
        let model_end = model_begin.elapsed();

        let font = if let Some(name) = &config.font_name {
            let cache = rust_fontconfig::FcFontCache::build();
            let mut trace = Vec::new();
            let results = cache.query(
                &rust_fontconfig::FcPattern {
                    name: Some(name.clone()),
                    ..Default::default()
                },
                &mut trace,
            );
            match results {
                Some(r) => {
                    let bytes = cache.get_font_bytes(&r.id).unwrap();
                    h.load_font_from_memory(&t, ".ttf", &bytes, 64, None)
                        .unwrap()
                }
                None => {
                    eprintln!(
                        "[WARN]: Failed to find font {}, falling back to built in font",
                        &name
                    );
                    h.load_font_from_memory(&t, ".otf", FONT, 64, None).unwrap()
                }
            }
        } else {
            h.load_font_from_memory(&t, ".otf", FONT, 64, None).unwrap()
        };

        let bg_color = if let Some(c) = &config.background_color {
            raylib::color::Color::new(c.r, c.g, c.b, 255)
        } else {
            raylib::color::Color::new(0x18, 0x18, 0x18, 255)
        };

        let fg_color = if let Some(c) = &config.foreground_color {
            raylib::color::Color::new(c.r, c.g, c.b, 255)
        } else {
            raylib::color::Color::new(0xbb, 0xbb, 0xbb, 255)
        };

        let idle_color = if let Some(c) = &config.idle_color {
            raylib::color::Color::new(c.r, c.g, c.b, 255)
        } else {
            raylib::color::Color::new(20, 20, 20, 255)
        };

        let hover_color = if let Some(c) = &config.hovered_color {
            raylib::color::Color::new(c.r, c.g, c.b, 255)
        } else {
            raylib::color::Color::new(30, 30, 30, 255)
        };

        let click_color = if let Some(c) = &config.clicked_color {
            raylib::color::Color::new(c.r, c.g, c.b, 255)
        } else {
            raylib::color::Color::new(40, 40, 40, 255)
        };

        Self {
            raylib_thread: t,
            raylib_handle: h,
            font,
            bg_color,
            fg_color,
            idle_color,
            hover_color,
            click_color,
            doc_offset: 0.0,
            docs: vec![],
            model,
            query: String::new(),
            query_box_selected: false,
            scroll_velocity: raylib::math::Vector2::zero(),
            conf: config,
            index_file,
            boot_time: init.elapsed(),
            boot_index_time: model_end,
            update_time: std::time::Duration::from_secs(0),
            draw_time: std::time::Duration::from_secs(0),
            last_query_time: std::time::Duration::from_secs(0),
            reindex_time: std::time::Duration::from_secs(0),
            display_profile_data: false
        }
    }

    // only reindexes the files (does not refresh the config)
    fn refresh_model(&mut self) {
        self.model.clear();
        let reindex = std::time::Instant::now();
        for p in &self.conf.document_directories {
            let m = analyze_dir(&std::path::PathBuf::from(p)).unwrap();
            m.into_iter().for_each(|(k, v)| {
                self.model.insert(k, v);
            });
        }
        self.reindex_time = reindex.elapsed();
        std::fs::write(
            &self.index_file,
            wincode::serialize(&self.model).unwrap(),
        )
        .unwrap();
    }

    pub fn run(mut self) {
        let label_text = "local search";
        let label_size = self.font.measure_text(label_text, 64.0, 0.0);

        while !self.raylib_handle.window_should_close() {
            let w_w = self.raylib_handle.get_screen_width();
            let w_h = self.raylib_handle.get_screen_height();

            let update_time = std::time::Instant::now();

            let label_pos = raylib::math::Vector2::new(
                (w_w as f32 / 2.0) - label_size.x / 2.0,
                w_h as f32 / 32.0,
            );
            let search_rect = raylib::math::Rectangle::new(
                w_w as f32 / 64.0,
                label_pos.y + label_size.y * 1.5 + w_h as f32 / 64.0,
                w_w as f32 - (w_w as f32 / 32.0),
                label_size.y * 0.75,
            );
            let mut search_color = self.idle_color;
            if search_rect.check_collision_point_rec(self.raylib_handle.get_mouse_position()) {
                search_color = self.hover_color;

                if self
                    .raylib_handle
                    .is_mouse_button_down(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT)
                {
                    search_color = self.click_color;
                    self.query_box_selected = true;
                }
            }
            if self
                .raylib_handle
                .is_mouse_button_down(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT)
                && !search_rect.check_collision_point_rec(self.raylib_handle.get_mouse_position())
                || self
                    .raylib_handle
                    .is_key_down(raylib::consts::KeyboardKey::KEY_ESCAPE)
            {
                self.query_box_selected = false;
            }
            if self
                .raylib_handle
                .is_key_down(raylib::consts::KeyboardKey::KEY_SLASH)
            {
                self.query_box_selected = true;
            }
            if self.query_box_selected {
                search_color = self.hover_color;
            }

            self.scroll_velocity.y += self.raylib_handle.get_mouse_wheel_move_v().y * 10000.0;
            self.scroll_velocity.y /= 1.2;
            self.doc_offset += self.scroll_velocity.y * self.raylib_handle.get_frame_time();
            self.doc_offset = self.doc_offset.clamp(-f32::MAX, 0.0);

            if self.query_box_selected {
                if (self
                    .raylib_handle
                    .is_key_pressed(raylib::consts::KeyboardKey::KEY_BACKSPACE)
                    || self
                        .raylib_handle
                        .is_key_pressed_repeat(raylib::consts::KeyboardKey::KEY_BACKSPACE))
                    && !self.query.is_empty()
                {
                    self.query.pop();
                }

                while let Some(k) = self.raylib_handle.get_key_pressed() {
                    let k = char::from_u32(k as u32);
                    if let Some(k) = k {
                        if k.is_ascii_alphanumeric() || k == ' ' {
                            if !k.is_ascii_control() {
                                self.query.push(k.to_ascii_lowercase());
                            }
                        }
                    }
                }

                if self
                    .raylib_handle
                    .is_key_down(raylib::consts::KeyboardKey::KEY_ENTER)
                {
                    let terms: Vec<_> = self.query.split_whitespace().collect();
                    let t = std::time::Instant::now();
                    self.docs = do_query(&self.model, &terms);
                    self.last_query_time = t.elapsed();
                    self.doc_offset = 0.0;
                }
            }

            if self
                .raylib_handle
                .is_key_pressed(raylib::consts::KeyboardKey::KEY_R)
                && !self.query_box_selected
            {
                let t = std::time::Instant::now();
                self.refresh_model();
                self.reindex_time = t.elapsed();
                self.docs.clear();
                self.doc_offset = 0.0;
            }

            if self
                .raylib_handle
                .is_key_down(raylib::consts::KeyboardKey::KEY_LEFT_CONTROL) && self.raylib_handle.is_key_pressed(raylib::consts::KeyboardKey::KEY_D) {
                self.display_profile_data = !self.display_profile_data;
            }

            self.update_time = update_time.elapsed();

            let mut d = self.raylib_handle.begin_drawing(&self.raylib_thread);

            let draw_time = std::time::Instant::now();
            d.clear_background(self.bg_color);

            for (i, doc) in self.docs.iter().enumerate() {
                let mut rect = search_rect;
                rect.y += self.doc_offset;
                rect.y += (i + 1) as f32 * rect.height * 1.1;
                let mut result_color = self.idle_color;
                if rect.check_collision_point_rec(d.get_mouse_position()) {
                    result_color = self.hover_color;

                    if d.is_mouse_button_down(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT) {
                        result_color = self.click_color;
                    }
                }
                if rect.y < w_h as f32 && rect.y > 0.0 {
                    d.draw_rectangle_rounded(rect, 0.1, 10, result_color);
                    d.draw_text_ex(
                        &self.font,
                        doc,
                        raylib::math::Vector2::new(
                            rect.x + rect.width / 128.0,
                            rect.y + rect.height / 4.0,
                        ),
                        32.0,
                        0.0,
                        self.fg_color,
                    );
                }
            }

            // draws a mask for the search results so when the user scrolls down the search results
            // don't clutter up the query bar area
            d.draw_rectangle(
                0,
                0,
                w_w,
                (search_rect.y + search_rect.height) as i32,
                self.bg_color,
            );

            // ehhh i dont know how i feel about the label i dont want to be so pretentious
            d.draw_text_ex(&self.font, label_text, label_pos, 64.0, 0.0, self.fg_color);

            d.draw_rectangle_rounded(search_rect, 0.1, 10, search_color);
            d.draw_text_ex(
                &self.font,
                &self.query,
                raylib::math::Vector2::new(
                    search_rect.x + search_rect.x / 16.0 + (search_rect.x + search_rect.x / 128.0),
                    search_rect.y + search_rect.y / 16.0,
                ),
                32.0,
                0.0,
                self.fg_color,
            );
            d.draw_rectangle_rounded(
                raylib::math::Rectangle::new(
                    w_w as f32 / 128.0,
                    -96.0 + w_h as f32 - w_w as f32 / 128.0,
                    96.0,
                    96.0,
                ),
                0.1,
                10,
                self.idle_color,
            );
            self.draw_time = draw_time.elapsed();

            if self.display_profile_data {
                d.draw_rectangle(0, w_h - 300, 800, 300, raylib::color::Color::new(0, 0, 0, 127));
                d.draw_text_ex(
                    &self.font,
                    &format!("Update time: {} sec.\nDraw time  : {} sec.\nSearch time: {} sec.\nIndex time: {} sec.\nBoot time: {} sec.\nBoot index time: {} sec.", 
                        self.update_time.as_secs_f32(), 
                        self.draw_time.as_secs_f32(), 
                        self.last_query_time.as_secs_f32(), 
                        self.reindex_time.as_secs_f32(),
                        self.boot_time.as_secs_f32(),
                        self.boot_index_time.as_secs_f32()
                        ),
                    raylib::math::Vector2::new(0.0, (w_h - 300) as f32),
                    32.0,
                    0.0,
                    raylib::color::Color::WHITE,
                );
            }
        }
        // NOTE: Because the drop order causes the raylib handle to be closed before any assets get
        // unloaded we HAVE to drop them manually before EOL
        drop(self.font);
    }
}

fn main() {
    App::new().run();
}

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

fn analyze_dir(p: &std::path::Path) -> Result<HashMap<String, Document>, ()> {
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
struct Document {
    words: HashMap<String, usize>,
}

fn do_query(model: &HashMap<String, Document>, terms: &[&str]) -> Vec<String> {
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
