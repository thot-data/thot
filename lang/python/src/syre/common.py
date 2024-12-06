import os

APP_DIR = ".syre"
ASSETS_FILE = "assets.json"

PROJECT_ID_KEY = "SYRE_PROJECT_ID"
CONTAINER_ID_KEY = "SYRE_CONTAINER_ID"

def dev_mode() -> bool:
    """
    Returns if the script is running in dev mode.
    
    Returns:
        bool: If the database is running in dev mode.
    """
    return os.getenv(CONTAINER_ID_KEY) is None



def assets_file_of(base_path: str) -> str:
    """Returns the path to the container file of the base path.
    """
    return os.path.join(base_path, APP_DIR, ASSETS_FILE)