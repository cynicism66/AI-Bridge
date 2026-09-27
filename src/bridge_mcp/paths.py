"""项目路径和文件路径规范化。"""

import os
import posixpath


def norm_project(project):
    if not project or not str(project).strip():
        raise ValueError("缺少 project 参数：请填当前项目根目录的绝对路径")
    if not os.path.isabs(str(project).strip()):
        raise ValueError("project 必须是项目根目录的绝对路径")
    p = os.path.normcase(os.path.abspath(str(project).strip()))
    return p.replace("\\", "/").rstrip("/")


def norm_file(project, path):
    """统一成相对项目根目录、正斜杠、小写（Windows 不区分大小写）的写法。"""
    p = str(path).strip().replace("\\", "/")
    if os.path.isabs(p):
        full = os.path.normcase(os.path.abspath(p)).replace("\\", "/")
        if full.startswith(project + "/"):
            p = full[len(project) + 1:]
        else:
            p = full
    p = os.path.normcase(p).replace("\\", "/")
    return posixpath.normpath(p)
