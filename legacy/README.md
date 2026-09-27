# Python 历史版本（已冻结）

T05 已将 Claude 和 Codex 正式切换到 Rust。此目录只作为历史参考保留，不再增加功能，也不再作为一致性测试的目标。

`bridge.py`、`src/bridge_mcp/` 和 `tests/` 保存切换时的实现；代码只使用 Python 标准库，最低版本 3.10。`pyproject.toml` 保留旧项目元数据，不作为独立分发包发布。迁移 SQL 共用仓库根目录的 `migrations/`，运行参考代码需要完整仓库。

历史单元测试继续在 Windows、Ubuntu 和 Python 3.10、3.14 的 CI 中运行：

```powershell
cd D:\Bridge\legacy
python -X dev -W error -m unittest discover -s tests -v
```

测试创建临时数据库，不读写用户数据库。手动运行参考入口时，也应先把 `BRIDGE_DB` 设置为临时文件路径。

T05 的配置回滚步骤见[根 README](../README.md#t05-回滚)。恢复配置前必须恢复原入口及源码路径；仅还原配置备份会引用已归档的文件。后续 Rust 数据库升级不承诺旧代码继续兼容，回滚前应核对数据库版本。
