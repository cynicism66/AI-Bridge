-- 历史已读记录保留 NULL，不推测此前的读取时间。
ALTER TABLE reads ADD COLUMN read_at TEXT;
