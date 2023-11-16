// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Some utilities for parsing the PDF files used in SA

use std::collections::HashMap;
use std::path::Path;
use pdf::content::{Op, TextDrawAdjusted};
use pdf::font::ToUnicodeMap;
use pdf::primitive::PdfString;

#[derive(Debug,Clone)]
/// Where text will be drawn
pub struct TextStatus {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub font_name : Option<String>,
}

/// A streaming parser for PDF files.
/// Implement one of these that does the appropriate thing when some text is seen.
pub trait PDFInterpreter {
    fn new_page(&mut self);
    fn text(&mut self,status:&TextStatus,text:Vec<String>);

    fn parse_pdf(&mut self,path:impl AsRef<Path>) -> anyhow::Result<()> {
        let pdf = pdf::file::FileOptions::cached().open(path)?;
        for page in pdf.pages() {
            self.new_page();
            let page = page?;
            //       let mut fonts = HashMap::new();
            let mut to_unicode_map : HashMap<String,ToUnicodeMap> = HashMap::new();
            if let Ok(resources) = page.resources() {
                for (i, font) in resources.fonts.values().enumerate() {
                    let name = match &font.name {
                        Some(name) => name.as_str().trim_start_matches("CIDFont+").into(),
                        None => i.to_string(),
                    };
//                println!("Font name {} encoding {:?} to_unicode {:?}",name,font.encoding(),font.to_unicode(&pdf));
                    if let Some(Ok(to_unicode)) = font.to_unicode(&pdf) {
                        to_unicode_map.insert(name.clone(),to_unicode);
                    }
//                fonts.insert(name, font.clone());
                }
            }
            let mut current_unicode_map : Option<&ToUnicodeMap> = None;
            let mut text_status = TextStatus {
                x: 0.0,
                y: 0.0,
                size: 0.0,
                font_name: None,
            };
            if let Some(content) = &page.contents {
                for op in content.operations(&pdf)? {
                    //println!("operator {:?} ",op);
                    match op {
                        Op::Save => {}
                        Op::Restore => {}
                        Op::Transform { .. } => {}
                        Op::GraphicsState { .. } => {}
                        Op::TextFont { name,size } => {
                            text_status.size=size;
                            let name = name.as_str().to_string();
                            if !text_status.font_name.iter().any(|n|n==&name) {
                                current_unicode_map = to_unicode_map.get(&name);
                                text_status.font_name=Some(name);
                            }
                        }
                        Op::MoveTextPosition { .. } => {}
                        Op::SetTextMatrix { matrix } => {
                            text_status.x = matrix.e;
                            text_status.y = matrix.f;
                        }
                        Op::TextNewline => {}
                        Op::TextDraw { text } => {
                            let text = pdf_string_to_string(text,current_unicode_map);
                            self.text(&text_status,vec![text]);
                        }
                        Op::TextDrawAdjusted { array } => {
                            let mut prior_text : Vec<String> = vec![];
                            let mut text:String = String::new();
                            for e in array {
                                match e {
                                    TextDrawAdjusted::Text(t) => {
                                        text.push_str(&pdf_string_to_string(t,current_unicode_map));
                                    }
                                    TextDrawAdjusted::Spacing(s) => {
                                        if s< -500.0 {
                                            prior_text.push(text);
                                            text=String::new();
                                        }
                                    }
                                }
                            }
                            prior_text.push(text);
                            self.text(&text_status,prior_text);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

}


/// A simple testing tool that just prints received calls out on the screen.
pub struct JustDisplayPDFInterpreter {}

impl PDFInterpreter for JustDisplayPDFInterpreter {
    fn new_page(&mut self) {
        println!("New Page");
    }

    fn text(&mut self, status: &TextStatus, text: Vec<String>) {
        println!("{:?} : {:?}",status,text);
    }
}


fn pdf_string_to_string(t:PdfString,current_unicode_map:Option<&ToUnicodeMap>) -> String {
    if let Some(to_unicode) = current_unicode_map {
        let mut text = String::new();
        let data : Vec<u8> = t.data.iter().cloned().collect();
        for c in data.chunks(2) {
            let c = ((c[0] as u16)<<8)+(c[1] as u16);
            if let Some(s) = to_unicode.get(c) {
                text.push_str(s);
            }
        }
        text
    } else {
        t.to_string_lossy()
    }
}