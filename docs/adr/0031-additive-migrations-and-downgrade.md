# Additive migrations, newer-profile tolerance and pre-migration backups

sqlx refuses to start when a database has applied migrations that the binary doesn't know. The four migrations of 2026-11-12 (`20261112000000…030000`) were added without a compatibility decision. Once a profile has been opened by a build that contains them, the installed Kaze 2.0.5 fails at startup:

```
migration 20261112000000 was previously applied but is missing in the resolved migrations
```

Decision:

| Area | Rule |
| --- | --- |
| Shape | Migrations are **additive only**: `CREATE TABLE/INDEX … IF NOT EXISTS` and `ALTER TABLE … ADD COLUMN` (nullable or with a default). Never drop, rename or change the meaning of a column. Older code must keep working on a newer schema. |
| Tolerance | `database::migration_guard::migrator()` sets `set_ignore_missing(true)`. A build opens a profile that a newer build migrated, and ignores the migrations it doesn't know. This is safe only because migrations are additive. |
| Backup | Before running migrations on an existing profile with pending versions, `backup_before_migrate` writes `<db>.bak-before-<version>` with `VACUUM INTO`, which gives a consistent copy under WAL. A new database or one with nothing pending is not copied, and an existing backup with that name is never overwritten. |
| Process | Each new migration still needs a line in this ADR's log and an update of the count in `no_new_migrations_without_a_compatibility_decision`. `npm run audit:conventions` keeps flagging new migrations so that the check is deliberate. |

Builds that already shipped (2.0.5 and older) can't be changed. They don't ignore unknown migrations. To go back to one of them after this build has migrated the profile, restore the backup: quit Kaze, then copy `music_player.db.bak-before-<version>` over `music_player.db` and delete the `-wal`/`-shm` files. Changes made after the upgrade are lost. Otherwise, stay on the newer version. Profiles migrated before this ADR have no automatic backup.

Alternatives:
- Declaring "no downgrades" with no code change. This was rejected: every later release would strand the previous one again.
- Down migrations. This was rejected: sqlx would need reversible pairs for every change, and an old binary can't run down migrations it doesn't know.
- Ignoring missing migrations without a backup. This was rejected: it helps future builds but gives no way back to builds that already shipped.

Consequences:
- From this build on, moving to an older build is safe as long as migrations stay additive.
- One extra copy of the database per version that migrates the profile, about the size of the profile. Old copies are not pruned automatically.

Migration log: `20261112000000` artist similarity, `20261112010000` entity feedback, `20261112020000` audio features, `20261112030000` exploration slots (`ADD COLUMN rec_slot`). All are additive.

Tests: `src-tauri/tests/migration_guard_tests.rs`
- A profile with an unknown applied migration still opens. Without the fix, this fails with the 2.0.5 error.
- A pending migration produces a backup that holds the pre-migration state.
- New and current profiles are not copied.

Also checked by hand on a copy of the real profile (1,050 tracks): it opens, and a simulated pending migration produced a backup that passes `PRAGMA integrity_check`.
