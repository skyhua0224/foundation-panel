#!/usr/bin/env python3
"""Print the files a run changed, one per line.

Pull requests use the REST file list (paginated); a push compares before..after. Anything the
API cannot answer prints nothing, and classify-changes.py then runs everything. A new branch
(a "before" of all zeroes) is therefore a full run by design.
"""

import argparse
import os
import subprocess
import sys


def gh_output(args):
    result = subprocess.run(
        ["gh", "api", "--paginate", *args],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(result.stderr.strip(), file=sys.stderr)
        return []
    return [line for line in result.stdout.splitlines() if line]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", default=os.environ.get("EVENT", ""))
    parser.add_argument("--repo", default=os.environ.get("REPO", ""))
    parser.add_argument("--pr", default=os.environ.get("PR_NUMBER", ""))
    parser.add_argument("--before", default=os.environ.get("BEFORE", ""))
    parser.add_argument("--after", default=os.environ.get("AFTER", ""))
    args = parser.parse_args()

    if args.event == "pull_request" and args.repo and args.pr:
        files = gh_output([f"repos/{args.repo}/pulls/{args.pr}/files?per_page=100", "--jq", ".[].filename"])
    elif args.event == "push" and args.repo and args.after and set(args.before) != {"0"}:
        files = gh_output([f"repos/{args.repo}/compare/{args.before}...{args.after}", "--jq", ".files[].filename"])
    else:
        files = []

    for name in files:
        print(name)


if __name__ == "__main__":
    main()

