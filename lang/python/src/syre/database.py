from typing import Union, Any
import io
import subprocess
import importlib.resources as pkg_resources
from enum import StrEnum
import inspect
import socket
import os
import json
from datetime import datetime
import platform
from uuid import uuid4 as uuid

import zmq

from syre import _LEGACY_
from .types import OptStr, Tags, Metadata
from .common import CONTAINER_ID_KEY, PROJECT_ID_KEY, ANALYSIS_ID_KEY, assets_file_of, flags_file_of
from .resources import Container, Asset, dict_to_container, dict_to_asset

OptTags = Union[Tags, None]
OptMetadata = Union[Metadata, None]
if _LEGACY_:
    from typing import List

    Containers = List[Container]
    Assets = List[Asset]
else:
    Containers = list[Container]
    Assets = list[Asset]

LOCALHOST = "127.0.0.1"
SYRE_PORT = 7047
SOCKET_TIMEOUT = 10_000

if platform.system() == "Windows":
    ROOT_DIR = "\\"
else:
    ROOT_DIR = "/"


class FlagSeverity(StrEnum):
    Info = "Info"
    Warning = "Warning"
    Error = "Error"


class Database:
    """
    A Syre Database.
    """

    def __init__(self, dev_root: OptStr = None, chdir: bool = True):
        """
        Create a new Syre Database.

        Args:
            dev_root (OptStr, optional): Interactive graph root.
                Used to set the development root when using a script interactively.
                The script will set its graph root to the `Container` at the given path.
                This is ignored if the script is being run by a runner.

            chdir (bool, optional): Change the working directory to the analysis root. Defaults to `True`.
        """
        self._ctx: zmq.Context = zmq.Context()
        self._socket: zmq.Socket = self._ctx.socket(zmq.REQ)
        self._socket.setsockopt(zmq.SNDTIMEO, SOCKET_TIMEOUT)
        self._socket.setsockopt(zmq.RCVTIMEO, SOCKET_TIMEOUT)
        self._socket.connect(f"tcp://{LOCALHOST}:{SYRE_PORT}")
        if not self._is_database_available():
            exe_base_name = "syre-local-database"
            os_name = platform.system()
            if os_name == "Linux":
                exe_name = f"{exe_base_name}-x86_64-unknown-linux-gnu"
            elif os_name == "Darwin":
                mac_system = platform.processor()
                if mac_system == "arm":
                    exe_name = f"{exe_base_name}-aarch64-apple-darwin"
                else:
                    exe_name = f"{exe_base_name}-x86_64-apple-darwin"
            elif os_name == "Windows":
                exe_name = f"{exe_base_name}-x86_64-pc-windows-msvc.exe"
            else:
                raise OSError()

            if _LEGACY_:
                with pkg_resources.path("syre", "bin") as path:
                    exe_path = str(path.joinpath(exe_name))
            else:
                exe_path = (
                    pkg_resources.files("syre").joinpath("bin").joinpath(exe_name)
                )

            subprocess.Popen(exe_path, start_new_session=True)

        project_id: OptStr = os.getenv(PROJECT_ID_KEY)
        root_path: OptStr = os.getenv(CONTAINER_ID_KEY)
        if project_id is None and root_path is None:
            if dev_root is None:
                raise ValueError("`dev_root` must be set")

            self._init_dev(dev_root, chdir)
        elif project_id is not None and root_path is not None:
            self._init_prod(project_id, root_path, chdir)
        else:
            raise RuntimeError(
                f"`{PROJECT_ID_KEY}` and `{CONTAINER_ID_KEY}` must both be either set or not set"
            )

    def _init_dev(self, dev_root: str, chdir: bool):
        """Initialize the database in a dev environment.
        """
        # TODO: Allow relative paths
        # See `inspect.stack`
        if not os.path.isabs(dev_root):
            raise ValueError("`dev_root` must be an absolute path")

        os_name = platform.system()
        if os_name == "Windows":
            dev_root = windows_ensure_unc_path(dev_root)

        self._root_path: str = os.path.normpath(dev_root)
        if not os.path.exists(self._root_path):
            raise RuntimeError("Root path does not exist")

        self._socket.send_json({"State": "ProjectManifest"})
        project_manifest = self._socket.recv_json()
        if "Ok" not in project_manifest:
            raise RuntimeError("Could not get projects")

        project_manifest = project_manifest["Ok"]
        project_path = None
        for path in project_manifest:
            try:
                common = os.path.commonpath([path, self._root_path])
            except ValueError:
                continue

            if common == path:
                project_path = path
                break

        if project_path is None:
            raise RuntimeError("Path is not in a project")

        self._socket.send_json({"Project": {"Get": project_path}})
        project = self._socket.recv_json()
        if project is None:
            raise RuntimeError("Could not get project")

        assert project["path"] == project_path
        project = project["fs_resource"]
        if "Present" not in project:
            raise RuntimeError("Project folder is missing")

        project_properties = project["Present"]["properties"]
        if "Ok" not in project_properties:
            raise RuntimeError("Project properties are not valid")
        project_properties = project_properties["Ok"]
        self._project = project_properties["rid"]

        self._data_root = os.path.join(project_path, project_properties["data_root"])
        self._root = ensure_root_path(os.path.relpath(self._root_path, self._data_root))
        self._socket.send_json(
            {"Container": {"Get": {"project": self._project, "container": self._root}}}
        )
        root = self._socket.recv_json()
        root = root["Ok"]
        if root is None:
            raise RuntimeError("Could not get root container")

        root_properties = root["properties"]
        if "Err" in root_properties:
            raise RuntimeError(
                f"Root container properties file is corrupt: {root_properties['Err']}"
            )
        root_properties = root_properties["Ok"]
        self._root_id: str = root_properties["rid"]
        
        user = self._active_user()
        self._creator = {"User": {"Id": user}}

        if chdir:
            analysis_root = project_properties["analysis_root"]
            if analysis_root is None:
                raise RuntimeError("Analysis root is not set, can not change directory")

            analysis_path = os.path.join(project_path, analysis_root)
            os.chdir(analysis_path)

    def _init_prod(self, project: str, root: str, chdir: bool):
        """Initialize the database in a production environment.
        i.e. When being run by a runner.

        Args:
            project (str): Project id.
            root (str): Root container path.

        Raises:
            RuntimeError: If the root container can not be found.
        """
        self._project = project
        self._root = ensure_root_path(root)
        self._socket.send_json({"Project": {"GetById": self._project}})
        project = self._socket.recv_json()
        if project is None:
            raise RuntimeError("Could not get project")

        [project_path, project] = project
        project_properties = project["properties"]
        if "Ok" not in project_properties:
            raise RuntimeError("Project properties are not valid")
        project_properties = project_properties["Ok"]

        if platform.system() == "Windows":
            if self._root.startswith(ROOT_DIR):
                container_graph_path = self._root[len(ROOT_DIR):]
            else:
                raise RuntimeError(f"Invalid path for {CONTAINER_ID_KEY}")
        else:
            container_graph_path = os.path.relpath(self._root, ROOT_DIR)

        self._data_root = os.path.join(project_path, project_properties["data_root"])
        self._root_path: str = os.path.join(self._data_root, container_graph_path)

        self._socket.send_json(
            {"Container": {"Get": {"project": self._project, "container": self._root}}}
        )
        root = self._socket.recv_json()
        root = root["Ok"]
        if root is None:
            raise RuntimeError("Could not get root Container")

        root_properties = root["properties"]["Ok"]
        self._root_id: str = root_properties["rid"]
        
        analysis_id: OptStr = os.getenv(ANALYSIS_ID_KEY)
        if analysis_id is None:
            raise RuntimeError(f"{ANALYSIS_ID_KEY} not set")
        
        self._creator = {"Script": analysis_id}

        if chdir:
            analysis_root = project_properties["analysis_root"]
            if analysis_root is None:
                raise RuntimeError("Analysis root is not set, can not change directory")

            analysis_path = os.path.join(project_path, analysis_root)
            os.chdir(analysis_path)

    def _is_database_available(self) -> bool:
        """
        Check if a database is already running.

        Returns:
            bool: `True` if a database is already running, `False` otherwise.
        """
        s = socket.socket()
        try:
            s.bind((LOCALHOST, SYRE_PORT))
        except OSError as err:
            system = platform.system()
            if system == "Darwin":
                if err.errno != 48:
                    raise err
            elif system == "Linux":
                if err.errno != 98:
                    raise err
            elif system == "Windows":
                if (not hasattr(err, "winerror")) or (err.winerror != 10048):
                    raise err
            else:
                raise err
        else:
            # socket not bound, no chance of database running
            return False

        self._socket.send_json({"Config": "Id"})
        resp = self._socket.recv_json()
        return resp == "syre local database"

    @property
    def root(self) -> Container:
        """
        Returns the root Container.

        Returns:
            Container: Root Container.
        """
        self._socket.send_json(
            {
                "Container": {
                    "GetForAnalysis": {
                        "project": self._project,
                        "container": self._root,
                    }
                }
            }
        )
        root = self._socket.recv_json()
        root = root["Ok"]
        if root is None:
            raise RuntimeError("Could not get root Container")

        if "Err" in root:
            raise RuntimeError(f"Error getting root: {root['Err']}")
        root = root["Ok"]

        root = dict_to_container(root)
        root._db = self
        return root

    def find_containers(
        self,
        name: OptStr = None,
        type: OptStr = None,
        tags: OptTags = None,
        metadata: OptMetadata = None,
    ) -> Containers:
        """
        Find Containers matching the filter.

        Args:
            name (OptStr, optional): Name filter. Defaults to `None`.
            type (OptStr, optional): Type filter. Defaults to `None`.
            tags (OptTags, optional): Tags filter. Defaults to `None`.
            metadata (OptMetadata, optional): Metadata filter. Defaults to `None`.

        Returns:
            list[Container]: Containers matching the filter.
        """
        f = {}
        if name is not None:
            f["name"] = name
        if type is not None:
            f["kind"] = type
        if tags is not None:
            f["tags"] = tags
        else:
            f["tags"] = []
        if metadata is not None:
            f["metadata"] = [item for item in metadata.items()]
        else:
            f["metadata"] = []

        self._socket.send_json(
            {
                "Container": {
                    "Search": {"project": self._project, "root": self._root, "query": f}
                }
            }
        )
        containers = self._socket.recv_json()
        if "Err" in containers:
            raise RuntimeError(f"Error getting containers: {containers['Err']}")

        return list(
            map(
                lambda container: dict_to_container(container, db=self),
                containers["Ok"],
            )
        )

    def find_container(
        self,
        name: OptStr = None,
        type: OptStr = None,
        tags: OptTags = None,
        metadata: OptMetadata = None,
    ) -> Union[Container, None]:
        """
        Find a single Container matching the filter.

        Args:
            name (OptStr, optional): Name filter. Defaults to `None`.
            type (OptStr, optional): Type filter. Defaults to `None`.
            tags (OptTags, optional): Tags filter. Defaults to `None`.
            metadata (OptMetadata, optional): Metadata filter. Defaults to `None`.

        Returns:
            Union[Container, None]: A Contianer, or `None`.
        """
        containers = self.find_containers(
            name=name, type=type, tags=tags, metadata=metadata
        )
        if len(containers) == 0:
            return None

        return containers[0]

    def find_assets(
        self,
        name: OptStr = None,
        type: OptStr = None,
        tags: OptTags = None,
        metadata: OptMetadata = None,
    ) -> Assets:
        """
        Find Assets matching the filter.

        Args:
            name (OptStr, optional): Name filter. Defaults to `None`.
            type (OptStr, optional): Type filter. Defaults to `None`.
            tags (OptTags, optional): Tags filter. Defaults to `None`.
            metadata (OptMetadata, optional): Metadata filter. Defaults to `None`.

        Returns:
            list[Asset]: Assets matching the filter.
        """
        f = {}
        if name is not None:
            f["name"] = name
        if type is not None:
            f["kind"] = type
        if tags is not None:
            f["tags"] = tags
        else:
            f["tags"] = []
        if metadata is not None:
            f["metadata"] = [item for item in metadata.items()]
        else:
            f["metadata"] = []

        self._socket.send_json(
            {
                "Asset": {
                    "Search": {"project": self._project, "root": self._root, "query": f}
                }
            }
        )
        assets = self._socket.recv_json()
        if "Err" in assets:
            raise RuntimeError(f"Error getting assets: {assets['Err']}")

        return list(map(lambda asset: dict_to_asset(asset, db=self), assets["Ok"]))

    def find_asset(
        self,
        name: OptStr = None,
        type: OptStr = None,
        tags: OptTags = None,
        metadata: OptMetadata = None,
    ) -> Union[Asset, None]:
        """
        Find a single Asset matching the filter.

        Args:
            name (OptStr, optional): Name filter. Defaults to `None`.
            type (OptStr, optional): Type filter. Defaults to `None`.
            tags (OptTags, optional): Tags filter. Defaults to `None`.
            metadata (OptMetadata, optional): Metadata filter. Defaults to `None`.

        Returns:
            Union[Asset, None]: An Asset or `None`.
        """
        assets = self.find_assets(name=name, type=type, tags=tags, metadata=metadata)
        if len(assets) == 0:
            return None

        return assets[0]

    def add_asset(
        self,
        file: str,
        name: OptStr = None,
        type: OptStr = None,
        description: OptStr = None,
        tags: Tags = [],
        metadata: Metadata = {},
    ) -> str:
        """
        Adds an Asset to the project.

        Args:
            file (str): File name of the associated data. Use relative paths to place the Asset in a bucket.
            name (OptStr, optional): Name filter. Defaults to `None`.
            type (OptStr, optional): Type filter. Defaults to `None`.
            tags (OptTags, optional): Tags filter. Defaults to `None`.
            metadata (OptMetadata, optional): Metadata filter. Defaults to `None`.

        Returns:
            str: Path to save the Asset's file to.

        Example::

            >>> import syre
            >>> db = syre.Database()
            >>> path = db.add_asset("new_data.txt")
            >>> with open(path, "a+") as f:
            >>>     f.write("Some new data")
        """
        if os.path.isabs(file):
            raise ValueError("file must be a relative path")

        properties = {
            "created": datetime.now().strftime("%Y-%m-%dT%H:%M:%SZ"),
            "creator": self._creator,
            "name": name,
            "kind": type,
            "description": description,
            "tags": tags,
            "metadata": metadata,
        }
        asset = {"rid": str(uuid()), "properties": properties, "path": file}

        with open(assets_file_of(self._root_path), "r+") as f:
            assets: list = json.load(f)
            update = False
            dirty = False
            for stored_asset in assets:
                if stored_asset["path"] == asset["path"]:
                    update = True
                    if stored_asset["properties"] != asset["properties"]:
                        stored_asset["properties"] = asset["properties"]
                        dirty = True
                    break

            if not update:
                assets.append(asset)
                dirty = True

            if dirty:
                json_overwrite(assets, f)

        path = os.path.join(self._root_path, os.path.normpath(file))
        os.makedirs(
            os.path.dirname(path), exist_ok=True
        )  # ensure bucket directory exists
        return path

    def flag(
        self,
        resource: Union[Container, Asset],
        message: str,
        severity: FlagSeverity = FlagSeverity.Warning,
    ):
        """Add a flag to the resource.

        Args:
            resource (Union[Container, Asset]): Resource to flag.
            message (str): Message to display.
            severity (FlagSeverity): Flag's severity. Defaults to `FlagSeverity.Warning`.
        """
        resource_type = type(resource)
        if resource_type == Container:
            container_rid = resource._rid
        elif resource_type == Asset:
            container = resource.parent()
            container_rid = container._rid
        else:
            raise TypeError(
                f"`resource` must be a `Container` or `Asset`, found {resource_type}"
            )

        self._socket.send_json(
            {
                "Container": {
                    "SystemPathById": {
                        "project": self._project,
                        "container": container_rid,
                    }
                }
            }
        )
        container_path = self._socket.recv_json()
        if container_path is None:
            raise RuntimeError(f"Error setting flag: Could not get container path.")
        assert os.path.commonpath([container_path, self._data_root]) == self._data_root

        if resource_type == Container:
            resource_container_path = "/"
        elif resource_type == Asset:
            resource_container_path = os.path.relpath(
                resource.file, container_path
            )

        flags_path = flags_file_of(container_path)
        if (not os.path.exists(flags_path)) or (os.stat(flags_path).st_size == 0):
            flags = {}
        else:
            with open(flags_path) as f:
                flags = json.load(f)
                if type(flags) != dict:
                    raise RuntimeError("Invalid flags file.")

        flag = {
            "id": str(uuid()),
            "message": message,
            "severity": severity,
        }
        if resource_container_path in flags:
            flags[resource_container_path].append(flag)
        else:
            flags[resource_container_path] = [flag]

        with open(flags_path, "w+") as f:
            json_overwrite(flags, f)

    def clone(self) -> "Database":
        """Clones the Database.
        For use in multithreaded applications.

        Returns:
            Database: Clone of the Database.
        """
        clone = Database.__new__(Database)
        clone._ctx = self._ctx
        clone._socket = self._ctx.socket(zmq.REQ)  # zmq.Socket
        clone._root_path = self._root_path
        clone._root = self._root

        clone._socket.setsockopt(zmq.SNDTIMEO, SOCKET_TIMEOUT)
        clone._socket.setsockopt(zmq.RCVTIMEO, SOCKET_TIMEOUT)
        clone._socket.connect(f"tcp://{LOCALHOST}:{SYRE_PORT}")
        return clone

    def _active_user(self) -> OptStr:
        """
        Get the active user.

        Returns:
            OptStr: Active user.
        """
        self._socket.send_json({"State": "LocalConfig"})
        config = self._socket.recv_json()
        if "Err" in config:
            raise RuntimeError(f"Could not get local config: {config['Err']}")
        config = config["Ok"]

        return config["user"]


def windows_ensure_unc_path(path: str) -> str:
    """Ensures the path begins with the windows UNC identifier (\\\\?\\)

    Args:
        path (str): Path to modify.

    Returns:
        str: Path with UNC prefix.
    """
    UNC_PREFIX = "\\\\?\\"
    if path.startswith(UNC_PREFIX):
        return path
    else:
        return UNC_PREFIX + path


def ensure_root_path(path: str) -> str:
    """Ensures the path begins with the root directory (`/` on unix, `\\\\` on Windows).

    Args:
        path (str): Path to modify.

    Returns:
        str: Path beginning with root directory.

    Notes:
        If the path equals the current dir (`.`), the root path is returned.
    """
    if path.startswith(ROOT_DIR):
        return path
    elif path == os.path.curdir:
        return ROOT_DIR
    else:
        return ROOT_DIR + path


def json_overwrite(obj: Any, f: io.TextIOWrapper):
    """Overwrite a file's contents with the JSON serialization of the object."""
    f.seek(0)
    json.dump(obj, f, indent=4)
    f.truncate()
