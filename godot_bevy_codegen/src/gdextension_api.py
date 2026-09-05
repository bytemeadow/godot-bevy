from collections import defaultdict
from dataclasses import dataclass
from typing import List, Optional, Set, Dict


@dataclass(frozen=True)
class VersionHeader:
    version_major: int
    version_minor: int
    version_patch: int
    version_status: str
    version_build: str
    version_full_name: str
    precision: Optional[str]


@dataclass(frozen=True)
class GodotSignal:
    name: str
    description: str


@dataclass(frozen=True)
class GodotClass:
    name: str
    api_type: str
    is_refcounted: bool
    is_instantiable: bool
    inherits: Optional[str]
    enums: Optional[List]
    methods: Optional[List]
    signals: Optional[List[GodotSignal]]
    brief_description: str
    description: str


@dataclass
class ExtensionApi:
    header: VersionHeader
    classes: List[GodotClass]

    def classes_descended_from(self, root_class_name: str) -> List[str]:
        """Results include the root and are alphabetically sorted."""
        inheritance_map = defaultdict(list)

        for class_info in self.classes:
            if class_info.inherits is not None:
                name = class_info.name
                parent = class_info.inherits
                inheritance_map[parent].append(name)

        classes: Set[str] = set()

        def collect_descendants(class_name: str):
            classes.add(class_name)
            for child in inheritance_map.get(class_name, []):
                collect_descendants(child)

        collect_descendants(root_class_name)
        return sorted(classes)

    def parent_map(self) -> Dict[str, str]:
        parent_map: Dict[str, str] = {}

        for class_info in self.classes:
            if class_info.inherits is not None:
                name = class_info.name
                parent = class_info.inherits
                parent_map[name] = parent

        return parent_map
