"""
检查器包初始化。
每个检查器模块必须导出 `run(config: dict) -> CheckResult` 函数。
"""

import importlib
import pkgutil
from pathlib import Path
from dataclasses import dataclass, field

__all__ = ["CheckResult", "get_all_checkers", "load_checker"]


@dataclass
class CheckResult:
    """单个检查器的检查结果。

    Attributes:
        passed:     是否全部通过（无 CONFIRMED 问题）
        confirmed:  确定存在的问题列表，每项为 "file:line -> 描述" 格式字符串
        review_needed: 需人工判断的问题列表，每项为 "file:line -> 描述" 格式字符串
        skipped:    如果不适用，说明跳过原因；否则为空字符串
    """
    passed: bool = True
    confirmed: list = field(default_factory=list)
    review_needed: list = field(default_factory=list)
    skipped: str = ""

    @property
    def has_findings(self) -> bool:
        return bool(self.confirmed) or bool(self.review_needed)


def get_all_checkers() -> dict[str, str]:
    """返回所有可用检查器的名称到模块路径的映射。

    Returns:
        { "route_check": "checkers.route_check", ... }
    """
    checkers_dir = Path(__file__).parent
    result = {}
    for finder, name, ispkg in pkgutil.iter_modules([str(checkers_dir)]):
        if not name.startswith("_"):
            result[name] = f"checkers.{name}"
    return result


def load_checker(module_path: str):
    """按模块路径加载检查器模块。"""
    return importlib.import_module(f".{module_path}", package=__package__)
