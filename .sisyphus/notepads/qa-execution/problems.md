# Unresolved Problems

## 1. autobo_nfes table migration not applied
- Migration exists on disk (0201__autobo_nfes.sql) but was never run
- DB does not have autobo_nfes table
- NFe import/listing commands will fail at runtime
- **Action needed**: Run the nfe migration or fix migration numbering

## 2. Migration numbering inconsistency
- Disk: 0200-0205 | DB tracking: 0100-0105
- Old 0100 (faturamento_fields) has no 020x disk counterpart
- New 0201 (nfes) has no 010x DB counterpart
- **Action needed**: Align migration versions between disk and DB
