"""人用输出：控制台使用 Unicode 文本流，重定向使用 UTF-8。"""

import errno
import os
import sys


def write(text, stream=None):
    stream = sys.stdout if stream is None else stream
    value = text + "\n"
    try:
        if stream.isatty() or not hasattr(stream, "buffer"):
            stream.write(value)
            stream.flush()
        else:
            stream.buffer.write(value.encode("utf-8"))
            stream.buffer.flush()
    except OSError as error:
        if error.errno in (errno.EPIPE, errno.EINVAL) or getattr(error, "winerror", None) in (109, 232):
            raise BrokenPipeError() from error
        raise


def silence_broken_pipe():
    # 避免解释器退出时再次刷新已断开的 stdout，导致 traceback 或退出码 120。
    try:
        with open(os.devnull, "wb") as sink:
            os.dup2(sink.fileno(), sys.stdout.fileno())
    except (AttributeError, OSError, ValueError):
        pass
