use std::f32::consts::PI;

use lopdf::{
    content::{Content, Operation},
    dictionary, Dictionary, Document, Object, ObjectId, Stream,
};
use serde::Serialize;
use tauri::{Emitter, Manager, Window};

#[derive(Clone, Serialize)]
struct ProgressPayload {
    current: usize,
    total: usize,
}

#[tauri::command]
async fn add_pdf_watermark(
    window: Window,
    input_path: String,
    output_path: String,
    watermark_text: String,
) -> Result<(), String> {
    let app_handle = window.app_handle().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let mut document = Document::load(&input_path).map_err(|err| err.to_string())?;
        let pages = document.get_pages();
        let total = pages.len();

        if total == 0 {
            return Err("PDF 没有可处理的页面。".to_string());
        }

        if total > 1000 {
            return Err("当前版本仅支持处理 1000 页以内的 PDF。".to_string());
        }

        ensure_catalog_version(&mut document);

        for (index, (_, page_id)) in pages.iter().enumerate() {
            apply_watermark_to_page(&mut document, *page_id, &watermark_text)
                .map_err(|err| err.to_string())?;

            app_handle
                .emit(
                    "watermark://progress",
                    ProgressPayload {
                        current: index + 1,
                        total,
                    },
                )
                .map_err(|err: tauri::Error| err.to_string())?;
        }

        document.prune_objects();
        document.compress();
        document
            .save(output_path)
            .map(|_| ())
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

fn ensure_catalog_version(document: &mut Document) {
    if document.version.is_empty() {
        document.version = "1.5".to_string();
    }
}

fn apply_watermark_to_page(
    document: &mut Document,
    page_id: ObjectId,
    watermark_text: &str,
) -> lopdf::Result<()> {
    let page_snapshot = document.get_object(page_id)?.as_dict()?.clone();
    let media_box = read_media_box(&page_snapshot);
    let (width, height) = (media_box.2 - media_box.0, media_box.3 - media_box.1);
    let current_contents = page_snapshot.get(b"Contents").ok().cloned();

    let mut resources = resolve_page_resources(document, &page_snapshot)?;
    let mut fonts = resolve_resource_subdict(document, &resources, b"Font")?;
    let mut ext_g_state = resolve_resource_subdict(document, &resources, b"ExtGState")?;

    let font_name = insert_font_resource(document, &mut fonts);
    let gs_name = insert_ext_gstate_resource(document, &mut ext_g_state);
    resources.set("Font", Object::Dictionary(fonts));
    resources.set("ExtGState", Object::Dictionary(ext_g_state));

    let overlay_stream = build_watermark_stream(width, height, watermark_text, &font_name, &gs_name);
    let overlay_id = document.add_object(overlay_stream);
    let contents = build_updated_contents(document, current_contents, overlay_id);

    let page_object = document.get_object_mut(page_id)?;
    let page_dict = page_object.as_dict_mut()?;
    page_dict.set("Resources", Object::Dictionary(resources));
    page_dict.set("Contents", contents);

    Ok(())
}

fn resolve_page_resources(document: &Document, page_dict: &Dictionary) -> lopdf::Result<Dictionary> {
    match page_dict.get(b"Resources") {
        Ok(Object::Dictionary(dict)) => Ok(dict.clone()),
        Ok(Object::Reference(id)) => Ok(document.get_object(*id)?.as_dict()?.clone()),
        _ => Ok(Dictionary::new()),
    }
}

fn resolve_resource_subdict(
    document: &Document,
    resources: &Dictionary,
    key: &[u8],
) -> lopdf::Result<Dictionary> {
    match resources.get(key) {
        Ok(Object::Dictionary(dict)) => Ok(dict.clone()),
        Ok(Object::Reference(id)) => Ok(document.get_object(*id)?.as_dict()?.clone()),
        _ => Ok(Dictionary::new()),
    }
}

fn insert_font_resource(document: &mut Document, fonts: &mut Dictionary) -> Vec<u8> {
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding"
    });

    let resource_name = next_resource_name(fonts, "FWM");
    fonts.set(resource_name.clone(), Object::Reference(font_id));
    resource_name.into_bytes()
}

fn insert_ext_gstate_resource(document: &mut Document, ext_g_state: &mut Dictionary) -> Vec<u8> {
    let gs_id = document.add_object(dictionary! {
        "Type" => "ExtGState",
        "ca" => Object::Real(0.18_f32),
        "CA" => Object::Real(0.18_f32)
    });

    let resource_name = next_resource_name(ext_g_state, "GSWM");
    ext_g_state.set(resource_name.clone(), Object::Reference(gs_id));
    resource_name.into_bytes()
}

fn read_media_box(page_dict: &Dictionary) -> (f32, f32, f32, f32) {
    let default_box = (0.0, 0.0, 595.0, 842.0);

    let media_box = match page_dict.get(b"MediaBox") {
        Ok(Object::Array(media_box)) => media_box,
        _ => return default_box,
    };

    if media_box.len() != 4 {
        return default_box;
    }

    let to_f32 = |object: &Object| -> Option<f32> {
        match object {
            Object::Integer(value) => Some(*value as f32),
            Object::Real(value) => Some(*value as f32),
            _ => None,
        }
    };

    match (
        to_f32(&media_box[0]),
        to_f32(&media_box[1]),
        to_f32(&media_box[2]),
        to_f32(&media_box[3]),
    ) {
        (Some(x1), Some(y1), Some(x2), Some(y2)) => (x1, y1, x2, y2),
        _ => default_box,
    }
}

fn next_resource_name(dict: &Dictionary, prefix: &str) -> String {
    let mut index = 1;
    loop {
        let name = format!("{prefix}{index}");
        if !dict.has(name.as_bytes()) {
            return name;
        }
        index += 1;
    }
}

fn build_updated_contents(
    document: &mut Document,
    current_contents: Option<Object>,
    overlay_id: ObjectId,
) -> Object {
    match current_contents {
        Some(Object::Reference(existing_id)) => Object::Array(vec![
            Object::Reference(existing_id),
            Object::Reference(overlay_id),
        ]),
        Some(Object::Array(existing)) => {
            let mut all_streams = existing.clone();
            all_streams.push(Object::Reference(overlay_id));
            Object::Array(all_streams)
        }
        Some(Object::Stream(stream)) => {
            let current_id = document.add_object(Object::Stream(stream));
            Object::Array(vec![Object::Reference(current_id), Object::Reference(overlay_id)])
        }
        Some(other) => {
            let current_id = document.add_object(other);
            Object::Array(vec![Object::Reference(current_id), Object::Reference(overlay_id)])
        }
        None => Object::Reference(overlay_id),
    }
}

fn build_watermark_stream(
    width: f32,
    height: f32,
    watermark_text: &str,
    font_name: &[u8],
    gs_name: &[u8],
) -> Stream {
    let spacing_x = 110.0;
    let spacing_y = 85.0;
    let start_x = -width * 0.35;
    let start_y = -height * 0.2;
    let end_x = width * 1.2;
    let end_y = height * 1.2;
    let angle = 32.0_f32 * PI / 180.0;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let encoded_text = Object::string_literal(watermark_text);

    let mut operations = vec![
        Operation::new("q", vec![]),
        Operation::new("gs", vec![Object::Name(gs_name.to_vec())]),
        Operation::new(
            "rg",
            vec![
                Object::Real(0.6_f32),
                Object::Real(0.64_f32),
                Object::Real(0.72_f32),
            ],
        ),
    ];

    let mut y = start_y;
    while y < end_y {
        let mut x = start_x;
        while x < end_x {
            operations.push(Operation::new("BT", vec![]));
            operations.push(Operation::new(
                "Tm",
                vec![
                    Object::Real(cos_a),
                    Object::Real(sin_a),
                    Object::Real(-sin_a),
                    Object::Real(cos_a),
                    Object::Real(x),
                    Object::Real(y),
                ],
            ));
            operations.push(Operation::new(
                "Tf",
                vec![Object::Name(font_name.to_vec()), Object::Integer(10)],
            ));
            operations.push(Operation::new("Tj", vec![encoded_text.clone()]));
            operations.push(Operation::new("ET", vec![]));
            x += spacing_x;
        }
        y += spacing_y;
    }

    operations.push(Operation::new("Q", vec![]));

    let content = Content { operations };

    Stream::new(dictionary! {}, content.encode().unwrap_or_default())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![add_pdf_watermark])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
