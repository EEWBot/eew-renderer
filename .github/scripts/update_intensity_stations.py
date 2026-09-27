#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import html.parser
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import unicodedata
from urllib.parse import urljoin, urlparse
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
import zipfile
from datetime import datetime
from zoneinfo import ZoneInfo


RELEASE_API = "https://api.github.com/repos/EEWBot/jma-station-master/releases/latest"
STATIONS_URL = "https://www.data.jma.go.jp/svd/eqev/data/intens-st/stations.json"
CODE_TABLE_PAGE = "https://xml.kishou.go.jp/tec_material.html"
MAX_DOWNLOAD_BYTES = 25 * 1024 * 1024


def download(url: str, *, token: str | None = None) -> bytes:
    headers = {"User-Agent": "eew-renderer-station-updater"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
        headers["Accept"] = "application/vnd.github+json"
    request = Request(url, headers=headers)
    for attempt in range(4):
        try:
            with urlopen(request, timeout=60) as response:
                data = response.read(MAX_DOWNLOAD_BYTES + 1)
            if len(data) > MAX_DOWNLOAD_BYTES:
                raise ValueError(f"download exceeds {MAX_DOWNLOAD_BYTES} bytes: {url}")
            return data
        except HTTPError as error:
            if error.code not in (408, 429, 500, 502, 503, 504) or attempt == 3:
                raise
        except (URLError, TimeoutError, ConnectionError):
            if attempt == 3:
                raise
        delay = (5, 10, 20)[attempt]
        print(f"Retrying download after {delay}s: {url}", file=sys.stderr, flush=True)
        time.sleep(delay)
    raise AssertionError("download retry loop ended unexpectedly")


class CodeTableLinkParser(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.href: str | None = None
        self.label: list[str] = []
        self.matches: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag == "a":
            self.href = dict(attrs).get("href")
            self.label = []

    def handle_data(self, data: str) -> None:
        if self.href is not None:
            self.label.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag == "a" and self.href is not None:
            if "個別コード表" in "".join(self.label) and self.href.lower().endswith(".zip"):
                self.matches.append(self.href)
            self.href = None
            self.label = []


def code_table_url(page: bytes) -> str:
    parser = CodeTableLinkParser()
    parser.feed(page.decode("utf-8"))
    if len(parser.matches) != 1:
        raise ValueError(f"expected one individual code-table ZIP link, got {len(parser.matches)}")
    url = urljoin(CODE_TABLE_PAGE, parser.matches[0])
    parsed = urlparse(url)
    if parsed.scheme != "https" or parsed.hostname != "xml.kishou.go.jp":
        raise ValueError(f"unexpected code-table URL: {url}")
    return url


def code_table_xlsx(archive: bytes) -> bytes:
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        matches = []
        for member in package.infolist():
            name = member.filename
            if not member.flag_bits & 0x800:
                try:
                    name = name.encode("cp437").decode("cp932")
                except (UnicodeEncodeError, UnicodeDecodeError):
                    pass
            name = unicodedata.normalize("NFC", name)
            if "地震火山関連" in name and name.lower().endswith(".xlsx"):
                matches.append(member)
        if len(matches) != 1:
            raise ValueError(f"expected one earthquake/volcano XLSX, got {len(matches)}")
        return package.read(matches[0])


def release_assets(release: dict) -> tuple[str, dict, dict]:
    tag = release["tag_name"]
    if not isinstance(tag, str) or not tag.startswith("v") or "/" in tag:
        raise ValueError(f"unexpected release tag: {tag!r}")
    assets = {item["name"]: item for item in release["assets"]}
    archive_name = f"jma-station-master-{tag}-linux-x86_64.tar.gz"
    return tag, assets[archive_name], assets["SHA256SUMS"]


def download_asset(asset: dict) -> bytes:
    url = asset["browser_download_url"]
    parsed = urlparse(url)
    if parsed.scheme != "https" or parsed.hostname != "github.com" or not parsed.path.startswith(
        "/EEWBot/jma-station-master/releases/download/"
    ):
        raise ValueError(f"unexpected release asset URL: {url}")
    data = download(url)
    digest = asset.get("digest")
    if isinstance(digest, str) and digest.startswith("sha256:"):
        if hashlib.sha256(data).hexdigest() != digest.removeprefix("sha256:"):
            raise ValueError(f"GitHub asset digest mismatch: {asset['name']}")
    return data


def verify_archive(archive: bytes, archive_name: str, sums: bytes) -> None:
    matches = []
    for line in sums.decode("utf-8").splitlines():
        parts = line.split(maxsplit=1)
        if len(parts) == 2 and parts[1].lstrip("*") == archive_name:
            matches.append(parts[0])
    if len(matches) != 1 or hashlib.sha256(archive).hexdigest() != matches[0]:
        raise ValueError(f"SHA256SUMS mismatch: {archive_name}")


def extract_cli(archive: bytes, destination: Path) -> None:
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as package:
        member = package.getmember("jma-station-master")
        if not member.isfile():
            raise ValueError("release archive does not contain a CLI file")
        source = package.extractfile(member)
        if source is None:
            raise ValueError("release CLI could not be read")
        destination.write_bytes(source.read())
    destination.chmod(0o755)


def master_mode(asset: Path) -> str:
    data = json.loads(asset.read_text(encoding="utf-8"))
    if isinstance(data, list) and data and all(isinstance(item, dict) for item in data):
        return "init"
    if (
        isinstance(data, dict)
        and data.get("schema_version") == 1
        and data.get("source_kind") == "jma_public"
        and isinstance(data.get("stations"), list)
        and isinstance(data.get("releases"), list)
        and data["releases"]
    ):
        return "update"
    raise ValueError(f"unsupported station master shape: {asset}")


def report_has_changes(report: dict) -> bool:
    fields = (
        "stations_appended",
        "metadata_revised",
        "lifecycle_activated",
        "lifecycle_deactivated",
    )
    counts = [report[name] for name in fields]
    scopes = report["scopes"]
    if not isinstance(scopes, list):
        raise ValueError("report scopes must be a list")
    counts.extend(count for scope in scopes for count in (scope["enabled"], scope["disabled"]))
    if any(type(count) is not int or count < 0 for count in counts):
        raise ValueError("report has invalid change counts")
    return any(counts)


def validate_master(path: Path, release_id: str, report: dict) -> None:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not (
        isinstance(data, dict)
        and data.get("schema_version") == 1
        and data.get("source_kind") == "jma_public"
        and isinstance(data.get("stations"), list)
        and data["stations"]
        and isinstance(data.get("releases"), list)
        and data["releases"]
        and data["releases"][-1].get("id") == release_id
        and len(data["stations"]) == report["stations_total"]
    ):
        raise ValueError("CLI output is not the expected station master")


def pr_body(report: dict, tag: str, code_url: str, run_url: str) -> str:
    preface = (
        "## Sources\n\n"
        f"- CLI: https://github.com/EEWBot/jma-station-master/releases/tag/{tag}\n"
        f"- Stations: {STATIONS_URL}\n"
        f"- Code table: {code_url}\n\n"
        "`release-id` と `effective-from` は変更検出時の日本時間です。"
        "気象庁が定めた正式な施行日時とは異なる可能性があります。\n\n"
        "## Update report\n\n"
    )
    rendered = json.dumps(report, ensure_ascii=False, indent=2)
    if len(preface) + len(rendered) < 55_000:
        return f"{preface}```json\n{rendered}\n```\n"
    return (
        f"{preface}Full report: {run_url} (workflow artifact `intensity-stations-report`).\n\n"
        f"- Release: {report['release_id']}\n"
        f"- Stations appended: {report['stations_appended']}\n"
        f"- Metadata revised: {report['metadata_revised']}\n"
        f"- Warnings: {len(report['warnings'])}\n"
    )


def update(asset: Path, report_out: Path, body_out: Path, github_output: Path) -> bool:
    release = json.loads(download(RELEASE_API, token=os.environ.get("GH_TOKEN")))
    tag, cli_asset, sums_asset = release_assets(release)
    cli_archive = download_asset(cli_asset)
    sums = download_asset(sums_asset)
    verify_archive(cli_archive, cli_asset["name"], sums)

    stations = download(STATIONS_URL)
    raw_stations = json.loads(stations)
    if not isinstance(raw_stations, list) or not raw_stations:
        raise ValueError("public stations.json is empty or not an array")
    code_url = code_table_url(download(CODE_TABLE_PAGE))
    spreadsheet = code_table_xlsx(download(code_url))

    mode = master_mode(asset)
    detected_at = datetime.now(ZoneInfo("Asia/Tokyo"))
    release_id = detected_at.strftime("%Y%m%dT%H%M%S%f%z")
    effective_from = detected_at.isoformat(timespec="microseconds")

    with tempfile.TemporaryDirectory(prefix="intensity-stations-") as temporary:
        work = Path(temporary)
        cli = work / "jma-station-master"
        extract_cli(cli_archive, cli)
        stations_path = work / "stations.json"
        stations_path.write_bytes(stations)
        table_path = work / "code-table.xlsx"
        table_path.write_bytes(spreadsheet)
        output = work / "station-master.json"
        report_path = work / "report.json"
        command = [
            str(cli),
            mode,
            "--stations-json",
            str(stations_path),
            "--code-table-xlsx",
            str(table_path),
            "--release-id",
            release_id,
            "--effective-from",
            effective_from,
            "--output",
            str(output),
            "--report",
            str(report_path),
        ]
        if mode == "update":
            command.extend(("--previous", str(asset)))
        subprocess.run(command, check=True)

        report = json.loads(report_path.read_text(encoding="utf-8"))
        validate_master(output, release_id, report)
        changed = mode == "init" or report_has_changes(report)
        report_out.write_bytes(report_path.read_bytes())
        if changed:
            run_url = (
                f"https://github.com/{os.environ['GITHUB_REPOSITORY']}/actions/runs/"
                f"{os.environ['GITHUB_RUN_ID']}"
            )
            body_out.write_text(pr_body(report, tag, code_url, run_url), encoding="utf-8")
            replacement = asset.with_name(f".{asset.name}.tmp")
            try:
                shutil.copyfile(output, replacement)
                os.replace(replacement, asset)
            finally:
                replacement.unlink(missing_ok=True)

    with github_output.open("a", encoding="utf-8") as outputs:
        outputs.write(f"changed={'true' if changed else 'false'}\n")
    return changed


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--asset", required=True, type=Path)
    parser.add_argument("--report-out", required=True, type=Path)
    parser.add_argument("--body-out", required=True, type=Path)
    parser.add_argument("--github-output", required=True, type=Path)
    args = parser.parse_args()
    update(args.asset, args.report_out, args.body_out, args.github_output)


if __name__ == "__main__":
    main()
