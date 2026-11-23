#!/usr/bin/env python3
"""Measure one unit with many satellites, as their browsers would hold it.

    scripts/measure-screens.py <container> <port> <screens> <minutes> [presses a minute]

The unit runs in the container named, started beforehand as a demo so
that it hands vouchers out with no UniFi controller, and is reached on
the loopback address at the port:

    docker run -d --name usher-measured --cpus 2 -p 127.0.0.1:8443:8443 \
      -e USHER_DEMO=1 ghcr.io/foonerd/unifi-evo-usher:<version>
    scripts/measure-screens.py usher-measured 8443 1000 5

The program sets the unit up, adopts <screens> satellites, twenty to a
floor, and has every one of them exchange its secret for a session at
once, as after a power cut. Then every screen says that it is there
every thirty seconds, give or take ten, on a connection it keeps, as a
satellite's page does, for <minutes>; meanwhile screens chosen at random
press their button (sixty presses a minute unless another number is
given). Last, the container is started again under them, and the time is
taken until every screen is answered once more. It prints how long each
kind of asking took, and what the container used, as docker stats says
it.

Every connection is asked on the moment it is opened, as a screen asks
as soon as it is joined: a unit closes a connection that asks nothing
within ten seconds, and lets sixty-four of those wait at once, the rest
in turn. A screen that joins in a crowd waits its turn, and the time it
took is counted from when it began to ask.

docs/deploying.md has what this measured. It needs Python 3 and docker,
and nothing else. The unit's certificate is not checked: this measures a
unit on the loopback address, and is no client to copy.
"""
import asyncio
import base64
import json
import random
import resource
import ssl
import statistics
import sys
import time

NAME, PORT, SCREENS, MINUTES = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), float(sys.argv[4])
PRESSES = float(sys.argv[5]) if len(sys.argv) > 5 else 60.0
HOST = "127.0.0.1"
PASSWORD = "a measuring long password"
soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
resource.setrlimit(resource.RLIMIT_NOFILE, (min(hard, 65536), hard))
TLS = ssl.create_default_context()
TLS.check_hostname = False
TLS.verify_mode = ssl.CERT_NONE


class Line:
    """One kept connection, asked one thing at a time."""

    def __init__(self):
        self.reader = self.writer = None
        self.lock = asyncio.Lock()

    async def ask(self, method, path, body=None, bearer=None):
        async with self.lock:
            for again in (False, True):
                try:
                    if self.writer is None:
                        self.reader, self.writer = await asyncio.wait_for(
                            asyncio.open_connection(HOST, PORT, ssl=TLS), 20)
                    sent = b"" if body is None else json.dumps(body).encode()
                    head = [f"{method} {path} HTTP/1.1", f"Host: {HOST}:{PORT}", f"Content-Length: {len(sent)}"]
                    if body is not None:
                        head.append("Content-Type: application/json")
                    if bearer:
                        head.append(f"Authorization: Bearer {bearer}")
                    self.writer.write(("\r\n".join(head) + "\r\n\r\n").encode() + sent)
                    await self.writer.drain()
                    status = int((await asyncio.wait_for(self.reader.readline(), 40)).split()[1])
                    length, closing = 0, False
                    while True:
                        line = (await self.reader.readline()).decode("latin-1").strip()
                        if not line:
                            break
                        name, _, value = line.partition(":")
                        if name.lower() == "content-length":
                            length = int(value)
                        if name.lower() == "connection" and value.strip().lower() == "close":
                            closing = True
                    said = await self.reader.readexactly(length) if length else b""
                    if closing:
                        await self.drop()
                    try:
                        return status, json.loads(said) if said else None
                    except ValueError:
                        return status, None
                except (OSError, asyncio.IncompleteReadError, asyncio.TimeoutError, IndexError, ValueError):
                    await self.drop()
                    if again:
                        return 0, None
        return 0, None

    async def drop(self):
        if self.writer is not None:
            try:
                self.writer.close()
            except OSError:
                pass
        self.reader = self.writer = None


def spread(name, took):
    if not took:
        print(f"  {name}: none")
        return
    took = sorted(took)
    at = lambda share: took[min(len(took) - 1, int(len(took) * share))] * 1000
    print(f"  {name}: {len(took)} asked; median {at(0.5):.1f} ms, 95 in 100 within {at(0.95):.1f} ms, "
          f"99 in 100 within {at(0.99):.1f} ms, slowest {took[-1] * 1000:.1f} ms")


async def stats():
    run = await asyncio.create_subprocess_exec(
        "docker", "stats", "--no-stream", "--format", "{{.CPUPerc}} {{.MemUsage}}", NAME,
        stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.DEVNULL)
    out, _ = await run.communicate()
    try:
        cpu, mem = out.decode().split()[0:2]
        value = float(mem.rstrip("GMKiB"))
        mib = value * 1024 if mem.endswith("GiB") else value if mem.endswith("MiB") else value / 1024
        return float(cpu.rstrip("%")), mib
    except (ValueError, IndexError):
        return None


async def watch(into, until):
    while not until.is_set():
        seen = await stats()
        if seen:
            into.append(seen)
        try:
            await asyncio.wait_for(until.wait(), 5)
        except asyncio.TimeoutError:
            pass


def used(name, seen):
    if seen:
        print(f"  {name}: processor {statistics.mean(c for c, _ in seen):.1f}% of one core on average, "
              f"{max(c for c, _ in seen):.1f}% at most; memory {max(m for _, m in seen):.0f} MiB at most "
              f"({len(seen)} looks)")


async def main():
    admin = Line()
    for _ in range(120):
        status, state = await admin.ask("GET", "/usher/v1/state")
        if status == 200:
            break
        await asyncio.sleep(0.5)
    assert status == 200 and state.get("demo") is True, f"no demo unit at the port: {status} {state}"
    idle = await stats()
    print(f"the unit before anything: {idle}")
    if not state.get("set_up"):
        status, _ = await admin.ask("POST", "/usher/v1/set_up", {"name": "Anna", "password": PASSWORD})
        assert status == 200, status
    status, said = await admin.ask("POST", "/usher/v1/sign_in", {"name": "Anna", "password": PASSWORD})
    assert status == 200, status
    bearer = said["token"]

    # Adopting: a screen is given a code, and the administrator adopts it by it.
    print(f"adopting {SCREENS} screens")
    seen, done = [], asyncio.Event()
    looking = asyncio.create_task(watch(seen, done))
    secrets, took = [], []
    begun = time.monotonic()
    for n in range(SCREENS):
        floor, screen = n // 20 + 1, n % 20 + 1
        status, given = await admin.ask("POST", "/usher/v1/satellite_code", {})
        assert status == 200, f"no code for screen {n}: {status} {given}"
        started = time.monotonic()
        status, adopted = await admin.ask("POST", "/usher/v1/adopt_satellite", {
            "code": given["code"], "name": f"Floor {floor:02d}, screen {screen:02d}", "place": f"Floor {floor}"}, bearer)
        took.append(time.monotonic() - started)
        assert status == 200, f"screen {n} not adopted: {status} {adopted}"
        secrets.append(given["secret"])
    done.set(); await looking
    print(f"  all adopted in {time.monotonic() - begun:.1f} s")
    spread("an adoption, first hundred", took[:100]); spread("an adoption, last hundred", took[-100:])
    used("while adopting", seen)

    # Every screen exchanges its secret for a session, all at once, as after a power cut.
    print("every screen signs in, all at once")
    lines = [Line() for _ in secrets]
    tokens = [None] * len(secrets)
    took = []
    seen, done = [], asyncio.Event()
    looking = asyncio.create_task(watch(seen, done))

    async def sign_in(n):
        started = time.monotonic()
        for _ in range(8):
            status, said = await lines[n].ask("POST", "/usher/v1/satellite_sign_in", {"secret": secrets[n]})
            if status == 200 and said and said.get("token"):
                tokens[n] = said["token"]
                took.append(time.monotonic() - started)
                return True
            await asyncio.sleep(random.uniform(0.5, 2))
        return False

    begun = time.monotonic()
    held = await asyncio.gather(*(sign_in(n) for n in range(len(secrets))))
    done.set(); await looking
    print(f"  {sum(held)} of {len(held)} have a session after {time.monotonic() - begun:.1f} s")
    spread("a sign-in, from asking to session", took)
    used("while signing in", seen)

    # The steady state: every screen says it is there, on the connection it keeps.
    print(f"{MINUTES:g} minutes of every screen saying it is there, and {PRESSES:g} presses a minute")
    there, pressed, refused, lost = [], [], [], [0]
    seen, done = [], asyncio.Event()
    looking = asyncio.create_task(watch(seen, done))
    ending = time.monotonic() + MINUTES * 60
    last_ok = [0.0] * len(secrets)

    async def screen(n, until, at_once=False):
        # In the steady state the screens are spread over the half minute;
        # after a start each asks at once, as a page does that lost its unit.
        await asyncio.sleep(random.uniform(0, 1) if at_once else random.uniform(0, 30))
        while time.monotonic() < until():
            started = time.monotonic()
            status, _ = await lines[n].ask("GET", "/usher/v1/satellite", None, tokens[n])
            if status == 200:
                there.append(time.monotonic() - started)
                last_ok[n] = time.monotonic()
            elif status == 401:
                await sign_in(n)
                continue
            else:
                lost[0] += 1
                await asyncio.sleep(random.uniform(1, 5))
                continue
            await asyncio.sleep(30 + random.uniform(-10, 10))

    async def presses(until):
        asked = {"shelf": "issue.counter", "request_type": "hand_default",
                 "payload_b64": base64.b64encode(b"{}").decode()}
        while time.monotonic() < until():
            await asyncio.sleep(random.expovariate(PRESSES / 60))
            n = random.randrange(len(secrets))
            started = time.monotonic()
            status, said = await lines[n].ask("POST", "/api/v1/request", asked, tokens[n])
            (pressed if status == 200 else refused).append(time.monotonic() - started)

    await asyncio.gather(*(screen(n, lambda: ending) for n in range(len(secrets))), presses(lambda: ending))
    done.set(); await looking
    spread("saying it is there", there); spread("a voucher at a press", pressed)
    print(f"  presses not answered with a voucher: {len(refused)}; askings that failed: {lost[0]}")
    used("in the steady state", seen)

    # The unit is started again under them: how long until every screen is answered once more.
    print("the unit is started again with every screen connected")
    restart = await asyncio.create_subprocess_exec("docker", "restart", NAME, stdout=asyncio.subprocess.DEVNULL)
    await restart.wait()
    begun = time.monotonic()
    for n in range(len(last_ok)):
        last_ok[n] = 0.0
    ending = begun + 180
    seen, done = [], asyncio.Event()
    looking = asyncio.create_task(watch(seen, done))
    tasks = [asyncio.create_task(screen(n, lambda: ending, True)) for n in range(len(secrets))]
    back = None
    while time.monotonic() < ending:
        await asyncio.sleep(1)
        if all(at > begun for at in last_ok):
            back = time.monotonic() - begun
            break
    ending = 0
    for task in tasks:
        task.cancel()
    done.set(); await looking
    answered = sum(1 for at in last_ok if at > begun)
    print(f"  {answered} of {len(last_ok)} screens answered again" + (f", the last after {back:.0f} s" if back else " within 180 s"))
    used("after the start", seen)


asyncio.run(main())
