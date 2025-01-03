# SPDX-FileCopyrightText: 2023-present Brian Carlsen <carlsen.bri@gmail.com>
#
# SPDX-License-Identifier: MIT
import sys

if sys.version_info < (3, 9):
    _LEGACY_ = True
else:
    _LEGACY_ = False

from .common import dev_mode
from .database import Database, Asset, Container, FlagSeverity
from .filter import filter