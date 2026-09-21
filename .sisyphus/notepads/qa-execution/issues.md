# QA Issues Found

## Issue 1: Missing autobo_nfes table in database
- **Severity**: MEDIUM
- **Scenario**: 3 - Database migrations
- **Details**: Migration file `0201__autobo_nfes.sql` exists on disk but was **never applied** to the database. The migration tracking table `autobo_migrations` shows only versions 100-105 (old numbering 0100-0105), none of which correspond to the nfe migration. The `autobo_nfes` table is absent from the database.
- **Impact**: Any NF-e related commands that query `autobo_nfes` will fail at runtime with relation-not-found errors.
- **Root cause**: Migration files were renumbered from 010x to 020x, but the 0201 nfe migration has no counterpart in the old numbering. The app likely runs migrations based on filename prefix, and the DB expects the old numbering scheme.

## Issue 2: Migration numbering mismatch
- **Severity**: LOW
- **Details**: Migration files on disk use `0200`-`0205` prefix, but database `autobo_migrations` table tracks them as `0100`-`0105`. The old `0100_autobo_faturamento_fields` has no `0200` counterpart on disk.
- **Impact**: Could cause confusion or failure when new migrations need to be applied.
