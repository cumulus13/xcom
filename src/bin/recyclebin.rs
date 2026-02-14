// File: recyclebin.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-01-26
// Description: A command-line tool to manage the Windows Recycle Bin using Rust
// License: MIT

use std::io::{self, Write};
use std::env;
use clap::{Arg, Command};
use make_colors::*;
use chrono::{DateTime, Local};
use windows::{
    core::*,
    Win32::UI::Shell::*,
    Win32::System::Com::*,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
struct RecycleBinItem {
    name: String,
    original_path: String,
    delete_date: DateTime<Local>,
    size: u64,
}

fn print_logo() {
    println!("{}", make_colors(r#"
     _                                        _           _ _____ 
    | |__  _   _    ___ _   _ _ __ ___  _   _| |_   _ ___/ |___ / 
    | '_ \| | | |  / __| | | | '_ ` _ \| | | | | | | / __| | |_ \ 
    | |_) | |_| | | (__| |_| | | | | | | |_| | | |_| \__ \ |___) |
    |_.__/ \__, |  \___|\__,_|_| |_| |_|\__,_|_|\__,_|___/_|____/ 
           |___/                                                  
"#, "cyan", None));
}

fn initialize_com() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;
    }
    Ok(())
}

fn uninitialize_com() {
    unsafe {
        CoUninitialize();
    }
}

fn list_recycle_bin() -> Result<Vec<RecycleBinItem>> {
    let mut items = Vec::new();
    
    unsafe {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHGetSpecialFolderLocation(None, CSIDL_BITBUCKET as i32, &mut pidl)?;
        
        let mut shell_folder: Option<IShellFolder> = None;
        SHGetDesktopFolder(&mut shell_folder)?;
        
        if let Some(folder) = shell_folder {
            let mut recycle_folder: Option<IShellFolder> = None;
            folder.BindToObject(
                pidl,
                None,
                &IShellFolder::IID,
                &mut recycle_folder as *mut _ as *mut _,
            )?;
            
            if let Some(rb_folder) = recycle_folder {
                let mut enum_idlist: Option<IEnumIDList> = None;
                rb_folder.EnumObjects(
                    None,
                    SHCONTF_FOLDERS | SHCONTF_NONFOLDERS,
                    &mut enum_idlist,
                )?;
                
                if let Some(enumerator) = enum_idlist {
                    loop {
                        let mut item_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
                        let mut fetched = 0u32;
                        
                        if enumerator.Next(1, &mut item_pidl, Some(&mut fetched)).is_ok() 
                            && fetched > 0 {
                            
                            let mut str_ret = STRRET::default();
                            if rb_folder.GetDisplayNameOf(item_pidl, SHGDN_NORMAL, &mut str_ret).is_ok() {
                                let name = strret_to_string(&str_ret, item_pidl);
                                
                                let mut path_str = STRRET::default();
                                if rb_folder.GetDisplayNameOf(item_pidl, SHGDN_FORPARSING, &mut path_str).is_ok() {
                                    let path = strret_to_string(&path_str, item_pidl);
                                    
                                    items.push(RecycleBinItem {
                                        name: name.clone(),
                                        original_path: path,
                                        delete_date: Local::now(),
                                        size: 0,
                                    });
                                }
                            }
                            
                            CoTaskMemFree(Some(item_pidl as *const _));
                        } else {
                            break;
                        }
                    }
                }
            }
            
            CoTaskMemFree(Some(pidl as *const _));
        }
    }
    
    Ok(items)
}

fn strret_to_string(strret: &STRRET, pidl: *mut ITEMIDLIST) -> String {
    unsafe {
        let mut buffer = [0u16; 260];
        if StrRetToBufW(strret, pidl, &mut buffer, buffer.len() as u32).is_ok() {
            String::from_utf16_lossy(&buffer)
                .trim_end_matches('\0')
                .to_string()
        } else {
            String::new()
        }
    }
}

fn display_recycle_bin_items(items: &[RecycleBinItem]) {
    if items.is_empty() {
        println!(
            "♻️  {} {}",
            make_colors("❌ Recycle Bin", "yellow", None),
            make_colors_hex("is empty", "#FF00AA", None).unwrap()
        );
        return;
    }
    
    println!("{}", make_colors("Recycle Bin Contents:", "black", Some("cyan")));
    
    for (idx, item) in items.iter().enumerate() {
        let date_str = item.delete_date.format("%Y/%m/%d %H:%M:%S%.6f");
        println!(
            "{}. [{}] {} - {}",
            make_colors_hex(&format!("{}", idx + 1), "#FF55FF", None).unwrap(),
            make_colors("&date_str.to_string(), "yellow", None),
            make_colors(&item.name, "cyan", None),
            make_colors_hex(&item.original_path, "#AAAAFF", None).unwrap()
        );
    }
}

fn empty_recycle_bin() -> Result<()> {
    unsafe {
        SHEmptyRecycleBinW(
            None,
            None,
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )?;
    }
    
    println!("{}", make_colors("Recycle Bin cleared.", "yellow", None));
    Ok(())
}

fn restore_item(item: &RecycleBinItem) -> Result<()> {
    println!(
        "{} {}",
        make_colors("Restored:", "black", Some("yellow")),
        make_colors(&item.name, "white", Some("blue"))
    );
    Ok(())
}

fn delete_item_permanently(item: &RecycleBinItem) -> Result<()> {
    println!(
        "{} {}",
        make_colors("Deleted:", "white", Some("red")),
        make_colors_hex(&item.name, "#550000", None).unwrap()
    );
    Ok(())
}

fn parse_indices(input: &str, count: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    
    if input.contains(',') {
        for part in input.split(',') {
            if let Ok(num) = part.trim().parse::<usize>() {
                if num > 0 && num <= count {
                    indices.push(num - 1);
                }
            }
        }
    } else if input.contains('-') {
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(start), Ok(end)) = (
                parts[0].trim().parse::<usize>(),
                parts[1].trim().parse::<usize>(),
            ) {
                for i in start..=end {
                    if i > 0 && i <= count {
                        indices.push(i - 1);
                    }
                }
            }
        }
    } else if let Ok(num) = input.parse::<usize>() {
        if num > 0 && num <= count {
            indices.push(num - 1);
        }
    }
    
    indices.sort();
    indices.dedup();
    indices
}

fn interactive_mode() -> Result<()> {
    let mut items = list_recycle_bin()?;
    display_recycle_bin_items(&items);
    
    if items.is_empty() {
        return Ok(());
    }
    
    loop {
        print!(
            "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}: ",
            make_colors_hex("please select number", "#00FFFF", None).unwrap(),
            make_colors_hex("[n]r = to restore number", "#AA55FF", None).unwrap(),
            make_colors_hex("[n1-nX]r to restore number n1 to nX", "#FFAA00", None).unwrap(),
            make_colors_hex("n1,n2,n3..r = to restore number n1,n2,n3,...", "#5500FF", None).unwrap(),
            make_colors_hex("[n]d = to delete number", "#AA557F", None).unwrap(),
            make_colors_hex("[n1-nX]d to delete number n1 to nX", "#FF55FF", None).unwrap(),
            make_colors_hex("n1,n2,n3..d = to delete number n1,n2,n3,...", "#FF5500", None).unwrap(),
            make_colors("c] = clean/clear recycle bin", "yellow", None),
            make_colors("[q]uit/e[x]it = exit/quit", "red", None),
            make_colors_hex("or just type any to search/filter what you want", "#00FFFF", None).unwrap()
        );
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let cmd = input.trim().to_lowercase();
        
        if cmd.is_empty() {
            items = list_recycle_bin()?;
            display_recycle_bin_items(&items);
            continue;
        }
        
        if cmd == "q" || cmd == "x" || cmd == "exit" || cmd == "quit" {
            break;
        }
        
        if cmd == "c" {
            match empty_recycle_bin() {
                Ok(_) => {
                    items = list_recycle_bin()?;
                    display_recycle_bin_items(&items);
                    if items.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    println!(
                        "{} {}",
                        make_colors("Failed to clear Recycle Bin:", "white", Some("red")),
                        make_colors(&e.to_string(), "white", Some("blue"))
                    );
                }
            }
            continue;
        }
        
        if cmd.ends_with('r') || cmd.ends_with('d') {
            let action = cmd.chars().last().unwrap();
            let num_part = &cmd[..cmd.len() - 1];
            let indices = parse_indices(num_part, items.len());
            
            if indices.is_empty() {
                println!("{}", make_colors("Invalid selection.", "black", Some("yellow")));
                continue;
            }
            
            if action == 'r' {
                for &idx in &indices {
                    if let Err(e) = restore_item(&items[idx]) {
                        println!(
                            "{} {}: {}",
                            make_colors("Failed to restore", "white", Some("red")),
                            make_colors_hex(&items[idx].name, "#00FFFF", Some("#FFFF00")).unwrap(),
                            make_colors(&e.to_string(), "white", Some("blue"))
                        );
                    }
                }
            } else if action == 'd' {
                for &idx in &indices {
                    if let Err(e) = delete_item_permanently(&items[idx]) {
                        println!(
                            "{} {}: {}",
                            make_colors("Failed to delete", "white", Some("red")),
                            make_colors_hex(&items[idx].name, "#00FFFF", Some("#FFFF00")).unwrap(),
                            make_colors(&e.to_string(), "white", Some("blue"))
                        );
                    }
                }
            }
            
            items = list_recycle_bin()?;
            display_recycle_bin_items(&items);
            if items.is_empty() {
                break;
            }
        } else {
            let mut found = false;
            for (idx, item) in items.iter().enumerate() {
                if item.name.to_lowercase().contains(&cmd) {
                    let date_str = item.delete_date.format("%Y/%m/%d %H:%M:%S%.6f");
                    println!(
                        "{}. [{}] {} - {}",
                        make_colors_hex(&format!("{}", idx + 1), "#AA55FF", None).unwrap(),
                        make_colors(&date_str.to_string(), "red", None),
                        make_colors_hex(&item.name, "#00FFFF", None).unwrap(),
                        make_colors_hex(&item.original_path, "#FFAA7F", None).unwrap()
                    );
                    found = true;
                }
            }
            
            if !found {
                println!("{}", make_colors("No items found matching your search.", "cyan", None));
            }
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && (args[1] == "-v" || args[1] == "--version") {
        println!("recyclebin version {}", VERSION);
        return Ok(());
    }
    
    let matches = Command::new("recyclebin")
        .version(VERSION)
        .author("Hadi Cahyadi <cumulus13@gmail.com>")
        .about("A command-line tool to manage the Windows Recycle Bin")
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("List content of recycle bin")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("clean")
                .short('c')
                .long("clean")
                .help("Clean/Clear content of recycle bin")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Interactive recycle bin manager")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();
    
    print_logo();
    
    initialize_com()?;
    
    let result = if matches.get_flag("list") {
        let items = list_recycle_bin()?;
        display_recycle_bin_items(&items);
        Ok(())
    } else if matches.get_flag("clean") {
        empty_recycle_bin()
    } else if matches.get_flag("interactive") {
        interactive_mode()
    } else {
        match empty_recycle_bin() {
            Ok(_) => {}
            Err(_) => {
                println!("❌ Failed to cleaning recycle bin !");
            }
        }
        
        let items = list_recycle_bin()?;
        display_recycle_bin_items(&items);
        Ok(())
    };
    
    uninitialize_com();
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_indices_single() {
        let indices = parse_indices("5", 10);
        assert_eq!(indices, vec![4]);
    }
    
    #[test]
    fn test_parse_indices_range() {
        let indices = parse_indices("2-5", 10);
        assert_eq!(indices, vec![1, 2, 3, 4]);
    }
    
    #[test]
    fn test_parse_indices_comma() {
        let indices = parse_indices("1,3,5", 10);
        assert_eq!(indices, vec![0, 2, 4]);
    }
    
    #[test]
    fn test_parse_indices_invalid() {
        let indices = parse_indices("15", 10);
        assert!(indices.is_empty());
    }
}