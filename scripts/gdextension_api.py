from collections import defaultdict
from dataclasses import dataclass
from typing import List, Optional, Set, Dict, Any


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
    brief_description: str
    description: str


@dataclass
class ExtensionApi:
    """Raw gdextension_api.json data format"""

    classes: List[GodotClass]

    def classes_descended_from(self, root_class_name: str) -> Set[str]:
        inheritance_map = defaultdict(list)
        parent_map: Dict[str, Any] = {}

        for class_info in self.classes:
            if class_info.inherits is not None:
                name = class_info.name
                parent = class_info.inherits
                inheritance_map[parent].append(name)
                parent_map[name] = parent

        # Collect all Node-derived types
        classes: Set[str] = set()

        def collect_descendants(class_name: str):
            classes.add(class_name)
            for child in inheritance_map.get(class_name, []):
                collect_descendants(child)

        collect_descendants(root_class_name)
        return classes
