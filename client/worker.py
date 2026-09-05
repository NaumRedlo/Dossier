#!/usr/bin/env python3

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from dossier.worker import run

if __name__ == "__main__":
    run()
