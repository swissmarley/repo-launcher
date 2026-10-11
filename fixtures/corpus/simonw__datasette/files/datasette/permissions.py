import contextvars
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, NamedTuple
            f"{self.__class__.__name__}(parent={self.parent!r}, child={self.child!r})"
                f"Resource {cls.__name__} creates a 3-level hierarchy: "
                f"{cls.parent_class.parent_class.__name__} -> {cls.parent_class.__name__} -> {cls.__name__}. "