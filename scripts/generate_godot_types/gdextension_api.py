from collections import defaultdict
from dataclasses import dataclass
from typing import List, Optional, Set, Dict


@dataclass(frozen=True)
class VersionHeader:
    """Raw gdextension_api.json version header format"""

    version_major: int
    version_minor: int
    version_patch: int
    version_status: str
    version_build: str
    version_full_name: str
    precision: Optional[str]


@dataclass(frozen=True)
class GodotSignal:
    """Raw gdextension_api.json signal format"""

    name: str
    description: str


@dataclass(frozen=True)
class GodotClass:
    """Raw gdextension_api.json class format"""

    name: str
    api_type: str
    name: str
    is_refcounted: bool
    is_instantiable: bool
    inherits: Optional[str]
    api_type: str
    enums: Optional[List]
    methods: Optional[List]
    signals: Optional[List[GodotSignal]]
    brief_description: str
    description: str


@dataclass
class ExtensionApi:
    """Class representation of the gdextension_api.json data format"""

    header: VersionHeader
    classes: List[GodotClass]

    def classes_descended_from(self, root_class_name: str) -> List[str]:
        """Return an alphabetically sorted list of all classes descended from the given root class name"""
        inheritance_map = defaultdict(list)

        for class_info in self.classes:
            if class_info.inherits is not None:
                name = class_info.name
                parent = class_info.inherits
                inheritance_map[parent].append(name)

        # Collect all Node-derived types
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
