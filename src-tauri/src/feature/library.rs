use anyhow::Context;
use chrono::{Local, NaiveDate};
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use tauri::Manager;
use zip::ZipArchive;

/**
 * ライブラリ管理
 * libinfo.json でライブラリ一覧を管理
 */
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Library {
    pub id: String, // UUID, pathがかぶることはないからいらないかも
    pub name: String,
    pub path: String,
}

impl Library {
    pub fn new(id: String, name: String, path: String) -> Self {
        Library { id, name, path }
    }
}

/**
 * ライブラリ一覧情報
 */
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LibraryInfo {
    pub library_lists: Vec<Library>,
}

impl LibraryInfo {
    pub fn new() -> Self {
        LibraryInfo {
            library_lists: Vec::new(),
        }
    }
}

/**
 * ライブラリ内のコンテンツ情報
 * contentinfo.json でコンテンツ一覧を管理
 */
#[derive(Serialize, Deserialize, Clone)]
pub struct Content {
    pub id: String, // UUID, カバー画像は末尾に拡張子をつける
    pub file_name: String,
    pub added: NaiveDate,
}

/// ライブラリメタデータ
/// libmetadata.jsonで管理する
#[derive(Serialize, Deserialize, Clone)]
struct LibraryMetadata {
    pub contents: HashMap<String, Content>, // key: file_name
}

impl LibraryMetadata {
    pub fn new() -> Self {
        LibraryMetadata {
            contents: HashMap::new(),
        }
    }
}

/**
 * パスが存在するか確認する
 */
fn is_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

/**
 * ライブラリメタデータを読み込む
 */
fn load_metadata(path: &str) -> anyhow::Result<LibraryMetadata> {
    let path = Path::new(path);
    let file = fs::File::open(path).with_context(|| format!("Failed to open {:?}", path))?;

    let reader = io::BufReader::new(file);
    let metadata: LibraryMetadata = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to parse JSON from {:?}", path))?;

    Ok(metadata)
}

/**
 * libraryInfoのパスを取得する
 */
fn get_library_info_path(app: tauri::AppHandle) -> anyhow::Result<String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .with_context(|| format!("Failed to get app data dir."))?;
    let library_info_path = format!(
        "{}/libraryinfo.json",
        data_dir.clone().to_string_lossy().into_owned()
    );

    Ok(library_info_path)
}

/**
 * ライブラリの一覧情報をファイルから取得する
 */
fn load_library_info(app: tauri::AppHandle) -> anyhow::Result<LibraryInfo> {
    let library_info_path_string = get_library_info_path(app)?;
    let path = Path::new(&library_info_path_string);
    let file = fs::File::open(path).with_context(|| format!("Failed to open {:?}", path))?;

    let reader = io::BufReader::new(file);
    let library_info: LibraryInfo = serde_json::from_reader(reader)
        .with_context(|| format!("Failed to parse JSON from {:?}", path))?;

    Ok(library_info)
}

/**
 * 作成したライブラリをライブラリリストに追加して保存
 * TODO: 引数にLibraryInfoとLibraryMetadataに付与した共通のトレイトを指定することで
 * save_metadata_atomicと共通化する
 */
fn save_library_info_atomic(path: &str, library_ifno: &LibraryInfo) -> anyhow::Result<()> {
    let dir = PathBuf::from(path);

    fs::create_dir_all(path).with_context(|| format!("Failed to create directory: {:?}", dir))?;
    let target_path = dir.join("libraryinfo.json");
    let temp_path = dir.join("libraryinfo.json.tmp");

    // tmpファイルに書き込む
    let file = fs::File::create(&temp_path)
        .with_context(|| format!("Failed to create temporary file: {:?}", temp_path))?;
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, library_ifno)
        .with_context(|| format!("Failed to write JSON to temporary file: {:?}", temp_path))?;

    writer
        .flush()
        .with_context(|| format!("Failed to flush temporary file: {:?}", temp_path))?;

    // 3. OS に fsync を要求（超重要）
    writer.get_ref().sync_all().context("Failed to sync file")?;

    // 4. Windows 対策：既存ファイルを削除
    if target_path.exists() {
        fs::remove_file(&target_path)
            .with_context(|| format!("Failed to remove old file: {:?}", target_path))?;
    }

    // 5. atomic rename
    fs::rename(&temp_path, &target_path).with_context(|| "Failed to rename temp file to target")?;
    println!("ライブラリリストのセーブ完了");

    Ok(())
}

/**
 * 作成したライブラリのメタデータを保存する
 */
fn save_metadata_atomic(path: &str, metadata: &LibraryMetadata) -> anyhow::Result<()> {
    let dir = PathBuf::from(path);

    fs::create_dir_all(path).with_context(|| format!("Failed to create directory: {:?}", dir))?;
    let target_path = dir.join("libmetadata.json");
    let temp_path = dir.join("libmetadata.json.tmp");

    // tmpファイルに書き込む
    let file = fs::File::create(&temp_path)
        .with_context(|| format!("Failed to create temporary file: {:?}", temp_path))?;
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, metadata)
        .with_context(|| format!("Failed to write JSON to temporary file: {:?}", temp_path))?;

    writer
        .flush()
        .with_context(|| format!("Failed to flush temporary file: {:?}", temp_path))?;

    // 3. OS に fsync を要求（超重要）
    writer.get_ref().sync_all().context("Failed to sync file")?;

    // 4. Windows 対策：既存ファイルを削除
    if target_path.exists() {
        fs::remove_file(&target_path)
            .with_context(|| format!("Failed to remove old file: {:?}", target_path))?;
    }

    // 5. atomic rename
    fs::rename(&temp_path, &target_path).with_context(|| "Failed to rename temp file to target")?;
    println!("メタデータのセーブ完了");

    Ok(())
}

/**
 * 新しいライブラリを作成する
 */
#[tauri::command]
pub async fn create(path: String, name: String, app: tauri::AppHandle) -> Result<(), String> {
    // ライブラリ一覧を管理するファイルが存在していない場合は作成する
    let library_info_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir. {}", e))?;
    let path_string = library_info_path.to_string_lossy().into_owned();
    if !is_exists(&path_string) {
        match fs::create_dir_all(&library_info_path) {
            Ok(()) => {
                // 書き込み
                let library_info = LibraryInfo::new();
                let library_info_json = serde_json::to_string_pretty(&library_info)
                    .map_err(|e| format!("Failed to serialize library_info, {}", e))?;
                fs::write(
                    format!("{}/libraryinfo.json", &path_string),
                    library_info_json,
                )
                .map_err(|e| format!("Failed to write libraryinfo.json, {}", e))?;
            }
            Err(e) => {
                // 作成に失敗した場合は以降の処理に進めない
                // フロントにエラーを伝搬する
                eprint!("Failed to create library info. {}", e);
            }
        }
    }

    // ライブラリメタデータの各パスが存在するかを確認し、存在しない場合は作成する
    let taureria_path = format!("{}/.taureria", path);
    let thumbnails_path = format!("{}/thumbnails", taureria_path);
    let libmetadata_path = format!("{}/libmetadata.json", taureria_path);

    if !is_exists(&taureria_path) {
        // .taureria を作成
        fs::create_dir_all(&taureria_path)
            .map_err(|e| format!("failed to create ./taureria directory, {}", e))?;
    }

    if !is_exists(&thumbnails_path) {
        // ./taureria/thumbnails を作成
        fs::create_dir_all(&thumbnails_path)
            .map_err(|e| format!("Failed to create thumbnails directory, {}", e))?;
    }

    if !is_exists(&libmetadata_path) {
        // 設定ファイルを作成
        let metadata = LibraryMetadata::new();
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("Failed to serialize metadata, {}", e))?;
        fs::write(libmetadata_path, metadata_json)
            .map_err(|e| format!("Failed to write libmetadata.json, {}", e))?;
    }

    // ライブラリ情報を一覧に保存
    update(path.clone()).await?;

    // 最後にライブラリリストに保存する
    let mut librarys_info = load_library_info(app).map_err(|e| e.to_string())?;
    let library_id = uuid::Uuid::new_v4().to_string();
    let new_library = Library::new(library_id, name, path);
    librarys_info.library_lists.push(new_library);

    save_library_info_atomic(&path_string, &librarys_info)
        .map_err(|e| format!("Failed to save library Info. {}", e))?;
    println!("ライブラリリストの保存完了");

    Ok(())
}

/**
 * ライブラリの一覧を取得する
 */
#[tauri::command]
pub async fn list(app: tauri::AppHandle) -> Result<Vec<Library>, String> {
    let librarys_info = load_library_info(app).map_err(|e| e.to_string())?;
    Ok(librarys_info.library_lists)
}

/// ZIP内の1枚目の画像をサムネイルとして保存
pub fn create_thumbnail_from_zip(
    zip_path: &Path,
    thumbnail_dir: &Path,
    content_id: &str,
) -> anyhow::Result<()> {
    let file =
        fs::File::open(zip_path).with_context(|| format!("Failed to open zip: {}", content_id))?;

    let mut archive =
        ZipArchive::new(file).with_context(|| format!("Invalid zip archive: {}", content_id))?;

    // ZIP内ファイル名を取得・ソート
    let mut image_indices: Vec<usize> = (0..archive.len())
        .filter(|&i| {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_lowercase();
                name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png")
            } else {
                false
            }
        })
        .collect();

    image_indices.sort_by_key(|&i| {
        archive
            .by_index(i)
            .ok()
            .map(|f| f.name().to_string())
            .unwrap_or_default()
    });

    let first_image_index = image_indices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No image files found in zip: {}", content_id))?;

    let mut image_file = archive
        .by_index(*first_image_index)
        .with_context(|| format!("Failed to read image from zip: {}", content_id))?;

    let mut buffer = Vec::with_capacity(image_file.size() as usize);
    image_file
        .read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read image bytes: {}", content_id))?;

    // 画像デコード
    let image = image::load_from_memory(&buffer)
        .with_context(|| format!("Failed to decode image: {}", content_id))?;

    // サムネイル化（最大辺512px、アスペクト比維持）
    let thumbnail = resize_thumbnail(image, 512);

    fs::create_dir_all(thumbnail_dir)
        .with_context(|| format!("Failed to create thumbnail dir: {}", content_id))?;

    let output_path = thumbnail_dir.join(format!("{}.jpg", content_id));

    // JPEG保存（品質80）
    let mut out = fs::File::create(&output_path)
        .with_context(|| format!("Failed to create thumbnail file: {}", content_id))?;

    thumbnail
        .write_to(&mut out, ImageFormat::Jpeg)
        .with_context(|| format!("Failed to save thumbnail: {}", content_id))?;

    Ok(())
}

/// アスペクト比を維持したリサイズ
fn resize_thumbnail(image: DynamicImage, max_size: u32) -> DynamicImage {
    let (w, h) = image.dimensions();

    if w <= max_size && h <= max_size {
        return image;
    }

    let scale = max_size as f32 / w.max(h) as f32;
    let new_w = (w as f32 * scale) as u32;
    let new_h = (h as f32 * scale) as u32;

    image.thumbnail(new_w, new_h)
}

/**
 * ライブラリ内のコンテンツ情報を最新化し、contentinfo.jsonを更新する
 */
#[tauri::command]
pub async fn update(library_path: String) -> Result<(), String> {
    let start = Instant::now();

    // 対象のパスにあるZIPファイルのファイル名一覧を取得
    let zip_files: Vec<String> = fs::read_dir(&library_path)
        .map_err(|e| format!("Failed to read directory, {}", e))?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "zip") {
                    path.file_name()
                        .and_then(|name| name.to_str().map(String::from))
                } else {
                    None
                }
            })
        })
        .collect();

    let mut existing_metadata =
        load_metadata(&format!("{}/.taureria/libmetadata.json", library_path))
            .map_err(|e| format!("Failed to load existing metadata, {}", e))?;

    // 差分を求めて、差分のみ処理する
    let temp_existing: HashSet<&String> = existing_metadata.contents.keys().collect();
    let targets = zip_files
        .iter()
        .filter(|file_name| !temp_existing.contains(file_name))
        .cloned()
        .collect::<Vec<String>>();

    println!("[update] 処理対象のZIPファイル数: {}", targets.len());

    targets
        .iter()
        .try_for_each(|file_name| -> anyhow::Result<()> {
            let content_id = uuid::Uuid::new_v4().to_string();
            // サムネイルを作成
            let zip_path = Path::new(&library_path).join(file_name);
            let thumbnail_dir = Path::new(&library_path)
                .join(".taureria")
                .join("thumbnails");

            // サムネイルを作成する
            match create_thumbnail_from_zip(&zip_path, &thumbnail_dir, &content_id) {
                Ok(()) => {
                    // 作成に成功した場合のみメタデータに書き込む
                    // コンテンツ情報を作成
                    let content = Content {
                        id: content_id,
                        file_name: file_name.clone(),
                        added: Local::now().naive_local().date(),
                    };

                    existing_metadata
                        .contents
                        .insert(file_name.clone(), content);

                    println!(
                        "[update] サムネイル作成済みコンテンツ数: {}",
                        existing_metadata.clone().contents.len()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[update] create thumbnail error. filename: {}, {}",
                        file_name, e
                    );
                }
            };

            Ok(())
        })
        .map_err(|e| format!("Failed to process zip files, {}", e))?;

    save_metadata_atomic(&format!("{}/.taureria", library_path), &existing_metadata)
        .map_err(|e| format!("Failed to update existing metadata file, {}", e))?;

    let elapsed = start.elapsed();
    println!(
        "[update] 実行時間: {:.3}ms ({}μs)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_micros()
    );

    Ok(())
}

/**
 * 指定されたライブラリのコンテンツ一覧を返す
 */
#[tauri::command]
pub async fn contents_list(library_path: String) -> Result<Vec<Content>, String> {
    let existing_metadata = load_metadata(&format!("{}/.taureria/libmetadata.json", library_path))
        .map_err(|e| format!("Failed to load existing metadata, {}", e))?;

    Ok(existing_metadata.contents.into_values().collect())
}

/**
 * 指定されたライブラリを削除する
 */
#[tauri::command]
pub async fn delete(library_path: String, app: tauri::AppHandle) -> Result<(), String> {
    // .taureria配下を再帰的に削除する
    let taureria_path = format!("{}/.taureria", &library_path);

    match fs::remove_dir_all(&taureria_path) {
        Ok(()) => {
            println!(".taureria を削除しました: {}", taureria_path);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // すでに存在しない場合は警告に留めて続行
            eprintln!(".taureria が見つかりません（スキップ）: {}", taureria_path);
        }
        Err(e) => {
            // それ以外のエラー（権限不足など）は呼び出し元に伝搬
            return Err(format!("Failed to remove .taureria: {}", e));
        }
    }

    //
    let mut librarys_info = load_library_info(app.clone()).map_err(|e| e.to_string())?;
    println!("before: {:?}", librarys_info.clone());
    librarys_info
        .library_lists
        .retain(|lib| library_path.ne(&lib.path));

    println!("after: {:?}", librarys_info.clone());

    let library_info_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir. {}", e))?;
    let path_string = library_info_path.to_string_lossy().into_owned();

    save_library_info_atomic(&path_string, &librarys_info)
        .map_err(|e| format!("Failed to save library Info. {}", e))?;

    Ok(())
}
