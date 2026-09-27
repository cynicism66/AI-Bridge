"""命令行入口：启动 MCP 服务或查看公告板。"""

import sys

from .server import serve
from .tools import show_text


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "show":
        text = show_text(sys.argv[2] if len(sys.argv) > 2 else None)
        sys.stdout.buffer.write((text + "\n").encode("utf-8"))
        sys.stdout.buffer.flush()
    else:
        serve()
