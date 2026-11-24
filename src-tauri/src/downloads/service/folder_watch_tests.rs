use super::changes_folder;
use notify::event::{AccessKind, AccessMode, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};
use notify::EventKind;

#[test]
fn reading_the_folder_is_not_a_change() {
    // sync_folder reads every file; these used to restart it every 500 ms.
    for kind in [
        EventKind::Access(AccessKind::Open(AccessMode::Any)),
        EventKind::Access(AccessKind::Read),
        EventKind::Access(AccessKind::Close(AccessMode::Read)),
        EventKind::Access(AccessKind::Any),
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)),
        EventKind::Any,
    ] {
        assert!(!changes_folder(&kind), "{kind:?}");
    }
}

#[test]
fn added_removed_renamed_and_written_files_are_changes() {
    for kind in [
        EventKind::Create(CreateKind::File),
        EventKind::Remove(RemoveKind::File),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
        EventKind::Modify(ModifyKind::Data(DataChange::Any)),
        EventKind::Access(AccessKind::Close(AccessMode::Write)),
    ] {
        assert!(changes_folder(&kind), "{kind:?}");
    }
}
