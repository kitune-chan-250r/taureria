use crate::{AppState, ZipArchive};
use s_zip::StreamingZipReader;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use serde::Serialize;

#[derive(Serialize)]
pub struct ImageData {
    data: String, // Base64-encoded image bytes
    mime: String, // MIME type (e.g. "image/png")
}

// =========ビューワー関連のコマンドで利用するUtility関数群========
// 将来的に別ファイルに切り出す可能性あり

/**
 * ビューアウィンドウを作成する
 */
async fn create_viewer_window(
    label: String,
    page_count: usize,
    app: tauri::AppHandle,
) -> Result<tauri::WebviewWindow, String> {
    // ユニークなウィンドウラベルを生成
    let url = format!("index.html/#/viewer?p={}", page_count);
    // 新しいウィンドウを作成（viewer.html はサブウィンドウのHTML）
    let window = WebviewWindowBuilder::new(&app, label, WebviewUrl::App(url.into()))
        .title("Image Viewer")
        .visible(true)
        .inner_size(600.0, 900.0)
        .build()
        .map_err(|e| e.to_string())?;

    println!("Opened viewer window with label: {}", window.label());

    Ok(window)
}

/**
 * ZIPファイルを開いて画像リストを作成し、バックエンドの共有メモリに保存する
 * return: ページ数
 */
async fn unzip_and_get_images(
    path: String,
    window_label: String,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    // メモリ上でZIPファイルを開く
    let reader =
        StreamingZipReader::open(&path).map_err(|e| format!("Failed to open ZIP: {}", e))?;

    // readerから画像のみ抽出してファイル名順にソートしてリスト化
    let mut images: Vec<String> = reader
        .entries()
        .iter()
        .filter_map(|entry| {
            let name = &entry.name;
            // filter out non-image files by extension
            if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();
    images.sort(); // sort by filename (lexical order)

    let image_count = images.len();
    println!("Loaded ZIP with {} images", image_count);

    let mut data = state.0.lock().await;
    data.insert(
        window_label,
        ZipArchive {
            reader,
            entries: images, // 所有権は渡してしまう,何か問題があればcloneするが、サイズが大きくなる可能性のあるデータであることに留意
        },
    );
    Ok(image_count)
}

// ================================

/**
 * 指定されたパスを受け渡しつつ、新しいビューアウィンドウを開く
 * zipの初期化処理はウィンドウが開かれたタイミングで送信されるコマンドで実行
 * -> 変更計画
 * - OpenViewerWindowはウィンドウ初期化からZIP読み込み、画像リストの送信までの流れを担当する
 */
#[tauri::command]
pub async fn open_viewer_window(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // ビューワーウィンドウを開く際の処理の流れ
    // 1. ラベルを作成
    // 2. open_zipコマンドを呼び出してZIPを読み込み
    // 3.準備が出来たらビューワー用ウィンドウを作成し、表示

    // ユニークなウィンドウラベルを生成
    let label = format!("viewer-{}", Uuid::new_v4());
    // 対象ファイルのunzipと画像データ群をAppStateに保存
    let page_count = unzip_and_get_images(path.clone(), label.clone(), state).await?;
    // 新規ウィンドウ作成
    let window = create_viewer_window(label, page_count, app).await?;
    // フロントに対してページ数を通知

    Ok(())
}

/**
 * windowラベルに対応するZIPアーカイブから、指定されたインデックスの画像を取得して返す
 */
#[tauri::command]
pub async fn get_image(
    index: usize,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<ImageData, String> {
    let label = window.label();
    println!("start get_image with index: {} in window {}", index, label);
    let mut guard = state.0.lock().await;
    let archive = guard.get_mut(label).ok_or("No ZIP loaded")?;

    // Get the filename and MIME type
    let name = &archive.entries.get(index).ok_or("Index out of range")?;
    let mime = if name.ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };

    // Read and decode the image entry
    // StreamingZipReader needs &mut for reading entries
    let bytes = archive
        .reader
        .read_entry_by_name(name)
        .map_err(|e| format!("Failed to read {}: {}", name, e))?;

    // Base64-encode the image data
    let encoded = base64::encode(&bytes);

    Ok(ImageData {
        data: encoded,
        mime: mime.to_string(),
    })
}
