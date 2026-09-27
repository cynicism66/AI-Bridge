"""人用命令行及兼容 MCP 启动入口。"""

import argparse
import math
import os
import time

from . import output, store
from .paths import explicit_project, norm_project
from .history import history_text
from .server import serve
from .tools import fmt_msg, show_text


class Parser(argparse.ArgumentParser):
    def _print_message(self, message, file=None):
        if message:
            output.write(message.rstrip("\n"), stream=file)


def interval(value):
    try:
        seconds = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("间隔必须是大于 0 的有限秒数") from error
    if not math.isfinite(seconds) or seconds <= 0:
        raise argparse.ArgumentTypeError("间隔必须是大于 0 的有限秒数")
    return seconds


def parser():
    root = Parser(prog="bridge", description="Bridge：由用户控制的协作公告板")
    commands = root.add_subparsers(dest="command", title="命令")
    for name, help_text in [("on", "开启项目或全局开关"), ("off", "关闭项目或全局开关")]:
        command = commands.add_parser(name, help=help_text)
        command.add_argument("project", nargs="?", help="项目目录，默认当前目录")
        command.add_argument("--global", dest="global_switch", action="store_true", help="设置全局总开关")
    commands.add_parser("status", help="显示全局开关及所有项目状态")
    for name, help_text in [("show", "查看公告板"), ("read", "读取给 human 的未读消息"),
                            ("watch", "持续显示公告板变化"), ("post", "以 human 身份发消息"),
                            ("history", "查看项目交互历史")]:
        command = commands.add_parser(name, help=help_text)
        command.add_argument("project", nargs="?", help="项目目录，默认当前目录")
        if name == "post":
            command.add_argument("content", help="消息内容，有空格时请用引号包围")
            command.add_argument("--to", choices=("all", "claude", "codex"), default="all", help="收件人，默认 all")
        elif name == "watch":
            command.add_argument("--interval", type=interval, default=2.0, help="检查间隔秒数，默认 2")
        elif name == "history":
            command.add_argument("--limit", type=int, default=50, help="最近的条数，默认 50")
            command.add_argument("--agent", help="只显示指定身份")
            command.add_argument("--kind", choices=("status", "message", "claim", "release", "expire", "switch"))
    return root


def state_label(enabled):
    return "已开启" if enabled else "未开启"


def notice(state, project=True):
    if not state["global_enabled"]:
        return "提示：Bridge 全局已关闭；人用命令仍可使用。\n"
    if project and not state["project_enabled"]:
        return "提示：Bridge 未在此项目开启；人用命令仍可使用。\n"
    return ""


def status_text():
    state = store.switch_state()
    lines = [notice(state, project=False) + f"全局开关：{'已开启' if state['global_enabled'] else '已关闭'}"]
    for row in store.list_projects():
        effective = row["enabled"] and state["global_enabled"]
        lines.append(f"  {row['project']}：项目开关 {state_label(row['enabled'])}，"
                     f"有效状态 {state_label(effective)}，最近活动 {row['last'] or '-'}")
    if len(lines) == 1:
        lines.append("  （还没有项目）")
    return "\n".join(lines)


def project_text(project):
    state = store.switch_state(project)
    return notice(state) + f"启用状态：{state_label(state['enabled'])}\n" + show_text(project)


def watch(project, seconds):
    previous = None
    while True:
        current = project_text(project)
        if current != previous:
            output.write(current)
            previous = current
        time.sleep(seconds)


def run(args, root):
    if args.command == "status":
        output.write(status_text())
        return
    if args.command in ("on", "off") and args.global_switch:
        if args.project is not None:
            root.error("--global 不能同时指定项目")
        store.set_enabled(args.command == "on")
        state = store.switch_state()
        output.write(notice(state, project=False) + f"全局开关：{'已开启' if state['global_enabled'] else '已关闭'}")
        return
    directory = os.path.abspath(explicit_project(args.project if args.project is not None else os.getcwd()))
    project = norm_project(directory)
    if args.command in ("on", "off"):
        store.set_enabled(args.command == "on", project)
        state = store.switch_state(project)
        warning = "" if os.path.isdir(directory) else "提示：项目目录不存在，已按指定路径保存开关。\n"
        output.write(notice(state) + warning + f"项目开关：{state_label(state['project_enabled'])}（{project}）")
    elif args.command == "show":
        output.write(project_text(project))
    elif args.command == "post":
        content = args.content.strip()
        if not content:
            root.error("消息内容不能为空")
        prefix = notice(store.switch_state(project))
        result = store.send_message(project, "human", args.to, content)
        output.write(prefix + f"消息 #{result['id']} 已发送给 {'所有人' if args.to == 'all' else args.to}。")
    elif args.command == "read":
        prefix = notice(store.switch_state(project))
        rows = store.read_messages(project, "human", limit=None)
        text = "\n".join([f"未读消息（{len(rows)} 条，已标为已读）："] + [fmt_msg(row) for row in rows])
        output.write(prefix + (text if rows else "没有未读消息。"))
    elif args.command == "watch":
        watch(project, args.interval)
    elif args.command == "history":
        if args.limit < 0:
            root.error("limit 不能小于 0")
        output.write(history_text(project, args.limit, args.agent, args.kind))


def main(argv=None):
    try:
        root = parser()
        args = root.parse_args(argv)
        if args.command is None:
            serve()
        else:
            run(args, root)
    except BrokenPipeError:
        output.silence_broken_pipe()
    except KeyboardInterrupt:
        pass
    except ValueError as error:
        root.error(str(error))
