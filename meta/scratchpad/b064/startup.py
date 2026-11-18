import json, pathlib, subprocess, sqlite3
root = pathlib.Path.cwd() / 'scratch'
messages = '\n'.join(json.dumps(x) for x in [
 {'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'review','version':'0'}}},
 {'jsonrpc':'2.0','method':'notifications/initialized'},
]) + '\n'
for flags in [[], ['--allow-edits']]:
    path = root / ('cli-empty-write.sqlite' if flags else 'cli-empty-read.sqlite')
    path.write_bytes(b'')
    p = subprocess.run(['target/probe/debug/sophia-mcp','--store',str(path),*flags], input=messages, text=True, capture_output=True, timeout=15)
    print(f'flags={flags} exit={p.returncode} size={path.stat().st_size} initialized={"serverInfo" in p.stdout} stderr={p.stderr.strip()!r}')
    with sqlite3.connect(f'file:{path}?mode=ro', uri=True) as c:
        print('tables=', c.execute("select count(*) from sqlite_master where type='table'").fetchone()[0])
