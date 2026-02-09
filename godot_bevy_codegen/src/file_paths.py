from pathlib import Path
from typing import List


class FilePaths:
    """Keeps track of all file paths used in the generation pipeline"""

    # Remember to update all_generated_files() if adding/removing files
    project_root = Path(__file__).parent.parent.parent
    interop_path = project_root / "godot-bevy" / "src" / "interop"
    scene_tree_plugin_path = (
        project_root / "godot-bevy" / "src" / "plugins" / "scene_tree"
    )
    extension_api_path = project_root / "godot_extension_api"
    node_markers_path = interop_path / "node_markers"
    node_markers_dispatcher_file = interop_path / "node_markers.rs"
    type_checking_path = scene_tree_plugin_path / "node_type_checking"
    type_checking_dispatcher_file = scene_tree_plugin_path / "node_type_checking.rs"
    gdscript_plugin_path = project_root / "addons" / "godot-bevy"
    gdscript_watcher_path = (
        gdscript_plugin_path / "optimized_scene_tree_watcher_versions"
    )
    gdscript_watcher_current_file = (
        gdscript_plugin_path / "optimized_scene_tree_watcher.gd"
    )
    signal_names_path = interop_path / "signal_names"
    signal_names_dispatcher_file = interop_path / "signal_names.rs"

    @staticmethod
    def extension_api_file(version: str) -> Path:
        return FilePaths.extension_api_path / f"extension_api{version}.json"

    @staticmethod
    def node_markers_file(version: str) -> Path:
        return (
            FilePaths.node_markers_path / f"node_markers{version.replace('.', '_')}.rs"
        )

    @staticmethod
    def type_checking_file(version: str) -> Path:
        return (
            FilePaths.type_checking_path
            / f"type_checking{version.replace('.', '_')}.rs"
        )

    @staticmethod
    def gdscript_watcher_file(version: str) -> Path:
        return (
            FilePaths.gdscript_watcher_path
            / f"optimized_scene_tree_watcher{version.replace('.', '_')}.gd_ignore"
        )

    @staticmethod
    def signal_names_file(version: str) -> Path:
        return (
            FilePaths.signal_names_path / f"signal_names{version.replace('.', '_')}.rs"
        )

    @staticmethod
    def all_generated_files(api_versions: List[str]) -> List[Path]:
        paths = []
        for version in api_versions:
            paths.append(FilePaths.extension_api_file(version))
            paths.append(FilePaths.node_markers_file(version))
            paths.append(FilePaths.type_checking_file(version))
            paths.append(FilePaths.gdscript_watcher_file(version))
            paths.append(FilePaths.signal_names_file(version))
        paths.append(FilePaths.node_markers_dispatcher_file)
        paths.append(FilePaths.type_checking_dispatcher_file)
        paths.append(FilePaths.signal_names_dispatcher_file)
        return sorted(paths)
