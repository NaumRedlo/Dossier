# `dossier` — the Python bridge

Six modules and a program:

| | |
|---|---|
| `settings.py` | ten environment variables, each with a default that works |
| `build.py` | which build the engine is, so two machines can agree |
| `runner.py` | run the engine: judge a replay, render a video, cut a reel |
| `maps.py` | put the map a replay was played on onto disk |
| `skins.py` | unpack, keep and prune `.osk` skins |
| `machine.py` | how hard this machine may work right now |
| `log.py` | name the loggers, and let the host decide where they go |
| `worker.py` | the render client — poll a bot for jobs and answer them |

Two things use it: the bot at [NaumRedlo/1984](https://github.com/NaumRedlo/1984),
which takes render requests, and the render client, which answers them on
somebody else's machine.

## Running the client

From a built checkout, with nothing installed but the two dependencies:

```
python client/worker.py --check
python client/worker.py
```

See the repository README for the whole of it.

## Installing it as a library

```
pip install "dossier @ git+https://github.com/NaumRedlo/Dossier@v0.1.0#subdirectory=client"
```

The engine is a separate thing — a compiled binary this package looks for and
runs. Point `DOSSIER_BIN` at it when it is not in the checkout beside this
package.
