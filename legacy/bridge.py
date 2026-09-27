"""兼容原有客户端配置的 Bridge 入口。"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "src"))
from bridge_mcp.cli import main

if __name__ == "__main__":
    main()
