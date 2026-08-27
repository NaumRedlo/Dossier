#!/usr/bin/env python3
"""Start the render client from a plain checkout.

The client itself is `dossier/worker.py`, inside the package, so that it can be
imported — by its own tests, by the bot's, and by whatever builds the `.exe`
that people who do not have Python download instead. This file is what makes
`python client/worker.py` go on working for everybody who was told to type
exactly that.

Installed rather than cloned, there is a `dossier-worker` command, and
`python -m dossier.worker` works either way.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from dossier.worker import run  # noqa: E402

if __name__ == "__main__":
    run()
