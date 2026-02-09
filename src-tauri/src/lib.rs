#![feature(try_blocks)]

use crate::cli::Args;
use rusqlite::{Connection, Row};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use gtk::{EventBox, HeaderBar};
use gtk::prelude::{BinExt, Cast, GtkWindowExt};
use tauri::{command, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub struct States {
    db: Mutex<Option<Connection>>,
    db_path: PathBuf,
}

#[command]
fn open_and_list_dates(password: String, state: State<'_, States>) -> Result<Vec<i64>, String> {
    let res: Result<Vec<i64>, String> = try {
        let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;

        // 在新连接上执行 PRAGMA key
        let _: Option<String> = conn
            .query_row(&format!("PRAGMA key = '{}';", password), [], |_| Ok(None))
            .unwrap_or(None);

        // 验证解密是否成功
        let _: i32 = conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))
            .map_err(|_| "密码错误：无法解密数据库".to_string())?;

        // 将成功解密的连接保存到全局状态中
        let mut db_slot = state.db.lock().unwrap();
        *db_slot = Some(conn);

        // 获取初始日期列表
        fetch_dates_internal(db_slot.as_ref().unwrap(), "".to_string())?
    };

    res
}

#[command]
fn search_diary(state: State<'_, States>, query_str: String) -> Result<Vec<i64>, String> {
    let db_lock = state.db.lock().unwrap();
    let conn = db_lock.as_ref().ok_or("数据库未解锁")?;

    fetch_dates_internal(conn, query_str)
}

#[command]
fn get_diary_content(state: State<'_, States>, date: i64) -> Result<String, String> {
    let db_lock = state.db.lock().unwrap();
    let conn = db_lock.as_ref().ok_or("数据库未解锁")?;

    let content: String = conn
        .query_row(
            "SELECT content FROM diary WHERE \"date\" = ?",
            [date],
            |row: &Row| row.get(0),
        )
        .map_err(|e| format!("内容读取失败: {}", e))?;

    Ok(content)
}

// 内部函数：复用连接查询日期
fn fetch_dates_internal(conn: &Connection, query_str: String) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT \"date\" FROM diary
             WHERE instr(lower(\"date\"), lower(?1)) > 0
                OR instr(lower(content), lower(?1)) > 0
             ORDER BY \"date\" DESC",
        )
        .map_err(|e| e.to_string())?;

    let date_iter = stmt
        .query_map([query_str], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let mut dates = Vec::new();
    for date in date_iter {
        dates.push(date.map_err(|e| e.to_string())?);
    }
    Ok(dates)
}

// 保存日记内容的指令
#[command]
fn save_diary_content(state: State<'_, States>, date: i64, content: String) -> Result<(), String> {
    let db_lock = state.db.lock().unwrap();
    let conn = db_lock.as_ref().ok_or("数据库未解锁")?;

    conn.execute(
        "INSERT INTO diary (date, content)
         VALUES (?1, ?2)
         ON CONFLICT(date) DO UPDATE SET content = excluded.content",
        [date.to_string(), content],
    )
        .map_err(|e| format!("保存失败: {}", e))?;

    Ok(())
}

#[command]
fn delete_diary(date: i64, state: State<'_, States>) -> Result<(), String> {
    let db_lock = state.db.lock().unwrap();
    let conn = db_lock.as_ref().ok_or("数据库未解锁")?;

    conn.execute(
        "DELETE FROM diary WHERE date = ?",
        [date.to_string()],
    )
        .map_err(|e| format!("删除失败: {}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(args: Args) {
    let title = args
        .db_path
        .file_name()
        .map(|x| format!("Diary - {}", Path::new(x).display()))
        .unwrap_or("Diary".into());

    tauri::Builder::default()
        .setup(move |app| {
            let win = app.get_webview_window("main").unwrap();
            // https://github.com/tauri-apps/tauri/issues/13885
            win.eval("setTimeout('window.location.reload()', 100)")?;

            win.set_title(&title)?;
            match win.gtk_window().unwrap().titlebar() {
                Some(titlebar) => {
                    // Wayland
                    let event_box = titlebar.downcast::<EventBox>().unwrap();
                    let header_bar = event_box.child().unwrap().downcast::<HeaderBar>().unwrap();
                    use gtk::prelude::HeaderBarExt;
                    header_bar.set_title(Some(&title));
                }
                None => (),
            }
            Ok(())
        })
        .manage(States {
            db: Mutex::default(),
            db_path: args.db_path,
        })
        .invoke_handler(tauri::generate_handler![
            open_and_list_dates,
            search_diary,
            get_diary_content,
            save_diary_content,
            delete_diary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}

pub mod cli {
    use clap::Parser;
    use std::path::PathBuf;

    /// 个人日记程序的命令行配置
    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Args {
        /// 数据库文件路径 (默认为 ./diary.db)
        #[arg(default_value = "./diary.db")]
        pub db_path: PathBuf,
    }
}
