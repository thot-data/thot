from typing import Union

from syre import _LEGACY_
from .types import OptStr, Tags, Metadata, Properties
from .common import dev_mode

OptDatabase = Union['Database', None]
OptContainer = Union['Container', None]
if _LEGACY_:
    from typing import List
    Assets = List['Asset']
    ContainerList = List['Container']
else:
    Assets = list['Asset']
    ContainerList = list['Container']
    
class Container:
    """
    A Container.
    """
    def __init__(
        self,
        rid: str,
        name: OptStr = None,
        type: OptStr = None,
        description: OptStr = None,
        tags: Tags = [],
        metadata: Metadata = {},
        assets: Assets = [],
        db: OptDatabase = None
    ):
        """
        Create a new Container.
        """
        self._rid: str = rid
        self._name: OptStr = name
        self._type: OptStr = type
        self._description: OptStr = description
        self._tags: Tags = tags
        self._metadata: Metadata = metadata
        self._assets: Assets = assets
        
        self._db: OptDatabase = db
        self._parent: OptContainer = None
        self._parent_set: bool = False
    
    @property
    def name(self) -> OptStr:
        """
        Returns:
            OptStr: Container's name.
        """
        return self._name
    
    @property
    def type(self) -> OptStr:
        """
        Returns:
            OptStr: Container's type.
        """
        return self._type
    
    @property
    def description(self) -> OptStr:
        """
        Returns:
            OptStr: Container's description.
        """
        return self._description
    
    @property
    def tags(self) -> Tags:
        """
        Returns:
            Tags: Container's tags.
        """
        return self._tags
    
    @property
    def metadata(self) -> Metadata:
        """
        Returns:
            Metadata: Container's metadata.
        """
        return self._metadata
        
    @property
    def assets(self) -> Assets:
        """
        Returns:
            list[Asset]: Container's Assets.
        """
        if self._db is None or not dev_mode():
            return self._assets
        
        self._db._socket.send_json({"Container": {"GetByIdForAnalysis": {"project": self._db._project, "container": self._rid}}})
        container = self._db._socket.recv_json()
        if container is None:
            raise RuntimeError("Could not get Container")

        if "Err" in container:
            raise RuntimeError(f"Error getting container: {container['Err']}")
        container = container["Ok"]
        
        container = dict_to_container(container, db = self._db)
        self._assets = container._assets
        return self._assets
    
    def children(self) -> ContainerList:
        """
        Returns:
            ContainerList: Container's children.
        """
        self._db._socket.send_json({"Graph": {"Children": self._rid}})
        child_ids = self._db._socket.recv_json()
        children = []
        for cid in child_ids:
            self._db._socket.send_json({"Container": {"GetWithMetadata": cid}})
            child = self._db._socket.recv_json()
            if child is None:
                raise RuntimeError("Could not get child Container")
            
            children.append(child)
            
        children = list(map(
            lambda child: dict_to_container(child, db = self._db),
            children
        ))
        
        for child in children:
            child._set_parent(self)
        
        return children
    
    def parent(self) -> OptContainer:
        """
        Returns:
            OptContainer: Container's parent or `None` if the
            Container is the root of the current graph.
        """
        if self._db is None:
            raise RuntimeError('No database connector')
        
        if self._parent_set and not dev_mode():
            return self._parent
        
        if self._rid == self._db._root:
            self._set_parent(None)
            return None
            
        self._db._socket.send_json({"Graph": {"Parent": self._rid}})
        parent = self._db._socket.recv_json()
        if parent is None:
            self._set_parent(None)
            return None
        
        self._db._socket.send_json({"Container": {"GetWithMetadata": parent}})
        parent = self._db._socket.recv_json()
        if parent is None:
            raise RuntimeError("Could not get container parent")
        
        parent = dict_to_container(parent, db = self._db)
        self._set_parent(parent)
        return parent
    
    def _set_parent(self, parent: OptContainer):
        """Set the Container's parent

        Args:
            parent (OptContainer): The Container's parent.
            `None` represents the root of the current tree.
        """
        self._parent = parent
        self._parent_set = True

class Asset:
    """
    An Asset.
    """
    def __init__(
        self,
        rid: str,
        file: str,
        name: OptStr = None,
        description: OptStr = None,
        type: OptStr = None,
        tags: Tags = [],
        metadata: Metadata = {},
        db: OptDatabase = None,
        parent: OptContainer = None,
    ):
        """
        Create a new Asset.
        """
        self._rid: str = rid
        self._file: str = file
        self._name: OptStr = name
        self._type: OptStr = type
        self._description: OptStr = description
        self._tags: Tags = tags
        self._metadata: Metadata = metadata
        self._db = db
        self._parent = parent
    
    @property
    def name(self) -> OptStr:
        """
        Returns:
            OptStr: Asset's name.
        """
        return self._name
            
    @property
    def type(self) -> OptStr:
        """
        Returns:
            OptStr: Asset's type.
        """
        return self._type
    
    @property
    def description(self) -> OptStr:
        """
        Returns:
            OptStr: Asset's description.
        """
        return self._description
    
    @property
    def tags(self) -> Tags:
        """
        Returns:
            Tags: Asset's tags.
        """
        return self._tags
    
    @property
    def metadata(self) -> Metadata:
        """
        Returns:
            Metadata: Asset's metadata.
        """
        return self._metadata
        
    @property
    def file(self) -> str:
        """
        Returns:
            str: Asset's file path.
        """
        return self._file
    
    def parent(self) -> 'Container':
        """
        Returns:
            Asset's Container.
        """
        if self._parent is not None:
            return self._parent
            
        if self._db is None:
            raise RuntimeError("No database connector")
        
        self._db._socket.send_json({"Asset": {"Parent": self._rid}})
        parent = self._db._socket.recv_json()
        if parent is None:
            return None
        
        self._db._socket.send_json({"Container": {"GetWithMetadata": parent["rid"]}})
        parent = self._db._socket.recv_json()
        if parent is None:
            raise RuntimeError("Parent Container could not be retrieved")
        
        return dict_to_container(parent, db = self._db)            

def dict_to_container(d: Properties, db: OptDatabase = None) -> Container:
    """
    Converts a dictionary to a Container.

    Args:
        d (dict[str, Any]): Dictionary to convert.

    Returns:
        Container: Container that the JSON represented.
    """
    container = Container(
        d["rid"],
        name = d["properties"]["name"],
        type = d["properties"]["kind"],
        description = d["properties"]["description"],
        tags = d["properties"]["tags"],
        metadata = d["properties"]["metadata"],
        db = db
    )
    
    container._assets = list(map(
        lambda asset: dict_to_asset(asset, db = db, parent = container),
        d["assets"]
    ))
    
    return container

def dict_to_asset(d: Properties, db: OptDatabase = None, parent: OptContainer = None) -> Asset:
    """
    Converts a dictionary to an Asset.

    Args:
        d (dict[str, Any]): Dictionary to convert.

    Returns:
        Asset: Asset that the JSON represented.
    """
    return Asset(
        d["rid"],
        d["path"],
        name = d["properties"]["name"],
        type = d["properties"]["kind"],
        description = d["properties"]["description"],
        tags = d["properties"]["tags"],
        metadata = d["properties"]["metadata"],
        db = db,
        parent = parent
    )
