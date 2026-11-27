//! The whole clipboard, not only its text: every item with every type
//! (images, files, rich text). Saved before Hutarka borrows the clipboard
//! for ⌘C or ⌘V and put back afterwards.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};

/// Items of a pasteboard: for each item, its (type, bytes) pairs. Plain data,
/// so it can move to the thread that restores it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saved(Vec<Vec<(String, Vec<u8>)>>);

/// Everything on the general pasteboard.
pub fn save() -> Saved {
    save_from(&NSPasteboard::generalPasteboard())
}

/// Puts `saved` back on the general pasteboard (empty: clears it).
pub fn restore(saved: &Saved) {
    restore_to(&NSPasteboard::generalPasteboard(), saved);
}

fn save_from(board: &NSPasteboard) -> Saved {
    let Some(items) = board.pasteboardItems() else {
        return Saved::default();
    };
    let saved = items
        .iter()
        .map(|item| {
            item.types()
                .iter()
                .filter_map(|ty| {
                    item.dataForType(&ty)
                        .map(|data| (ty.to_string(), data.to_vec()))
                })
                .collect()
        })
        .collect();
    Saved(saved)
}

fn restore_to(board: &NSPasteboard, saved: &Saved) {
    board.clearContents();
    if saved.0.is_empty() {
        return;
    }
    let items: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = saved
        .0
        .iter()
        .map(|types| {
            let item = NSPasteboardItem::new();
            for (ty, bytes) in types {
                item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(ty));
            }
            ProtocolObject::from_retained(item)
        })
        .collect();
    if !board.writeObjects(&NSArray::from_retained_slice(&items)) {
        log::error!("cannot restore the clipboard");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_app_kit::NSPasteboard;

    #[test]
    fn save_and_restore_keep_every_type_of_every_item() {
        let board = NSPasteboard::pasteboardWithUniqueName();
        let items = vec![vec![
            ("public.utf8-plain-text".to_string(), b"hello".to_vec()),
            ("public.png".to_string(), vec![0x89, b'P', b'N', b'G']),
        ]];
        restore_to(&board, &Saved(items.clone()));
        let saved = save_from(&board);
        board.clearContents();
        restore_to(&board, &saved);
        let mut again = save_from(&board).0;
        for item in &mut again {
            item.sort();
        }
        let mut expected = items;
        expected[0].sort();
        assert_eq!(again, expected);
    }

    #[test]
    fn restoring_nothing_empties_the_board() {
        let board = NSPasteboard::pasteboardWithUniqueName();
        restore_to(
            &board,
            &Saved(vec![vec![("public.utf8-plain-text".into(), b"x".to_vec())]]),
        );
        restore_to(&board, &Saved(Vec::new()));
        assert!(save_from(&board).0.is_empty());
    }
}
