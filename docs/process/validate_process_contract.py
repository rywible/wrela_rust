#!/usr/bin/env python3

import json
import pathlib
import re
import sys


def main() -> int:
    repo_root = pathlib.Path(__file__).resolve().parents[2]
    manifest_path = repo_root / "docs/process/process_contract.json"
    manifest = json.loads(manifest_path.read_text())

    checks = []

    def add_check(name: str, ok: bool, details: str, path: str | None = None) -> None:
        item = {"name": name, "ok": ok, "details": details}
        if path is not None:
            item["path"] = path
        checks.append(item)

    for rel_path in manifest["bootstrap"]["required_paths"]:
        path = repo_root / rel_path
        add_check(
            "required_path_exists",
            path.exists(),
            "required path present" if path.exists() else "required path missing",
            rel_path,
        )

    issue_dir = repo_root / manifest["issue_templates"]["directory"]
    legacy_issue_dir = repo_root / manifest["issue_templates"]["forbidden_legacy_directory"]
    add_check(
        "issue_template_directory_present",
        issue_dir.is_dir(),
        "GitHub issue template directory present"
        if issue_dir.is_dir()
        else "GitHub issue template directory missing",
        manifest["issue_templates"]["directory"],
    )
    add_check(
        "legacy_issue_template_directory_absent",
        not legacy_issue_dir.exists(),
        "legacy issue template directory absent"
        if not legacy_issue_dir.exists()
        else "legacy issue template directory still exists",
        manifest["issue_templates"]["forbidden_legacy_directory"],
    )

    for rel_path in manifest["issue_templates"]["required_files"]:
        path = repo_root / rel_path
        add_check(
            "issue_template_file_exists",
            path.is_file(),
            "issue template file present"
            if path.is_file()
            else "issue template file missing",
            rel_path,
        )

    backlog_path = repo_root / manifest["backlog"]["path"]
    try:
        backlog = json.loads(backlog_path.read_text())
        backlog_ok = isinstance(backlog.get("prs"), list)
        add_check(
            "backlog_json_parses",
            backlog_ok,
            "backlog JSON parsed with prs list"
            if backlog_ok
            else "backlog JSON missing prs list",
            manifest["backlog"]["path"],
        )
    except Exception as exc:
        backlog = {"prs": []}
        add_check(
            "backlog_json_parses",
            False,
            f"failed to parse backlog JSON: {exc}",
            manifest["backlog"]["path"],
        )

    pr_id_pattern = re.compile(manifest["backlog"]["pr_id_pattern"])
    task_filename_pattern = re.compile(manifest["backlog"]["task_filename_pattern"])

    backlog_ids = []
    for pr in backlog.get("prs", []):
        pr_id = pr.get("id", "")
        backlog_ids.append(pr_id)
        add_check(
            "backlog_pr_id_shape",
            bool(pr_id_pattern.match(pr_id)),
            "backlog PR id matches expected pattern"
            if pr_id_pattern.match(pr_id)
            else "backlog PR id does not match expected pattern",
            f"{manifest['backlog']['path']}::{pr_id or '<missing>'}",
        )

    pending_dir = repo_root / manifest["backlog"]["pending_dir"]
    completed_dir = repo_root / manifest["backlog"]["completed_dir"]

    def collect_task_ids(directory: pathlib.Path, label: str) -> tuple[set[str], list[str]]:
        ids = set()
        invalid_files = []
        for path in sorted(directory.glob("*.md")):
            match = task_filename_pattern.match(path.name)
            if not match:
                invalid_files.append(str(path.relative_to(repo_root)))
                add_check(
                    "task_filename_shape",
                    False,
                    "task file name does not match expected pattern",
                    str(path.relative_to(repo_root)),
                )
                continue
            ids.add(match.group(1))
            add_check(
                "task_filename_shape",
                True,
                f"{label} task file name matches expected pattern",
                str(path.relative_to(repo_root)),
            )
        return ids, invalid_files

    pending_ids, _ = collect_task_ids(pending_dir, "pending")
    completed_ids, _ = collect_task_ids(completed_dir, "completed")

    duplicate_ids = sorted(pending_ids & completed_ids)
    add_check(
        "task_ids_not_duplicated_across_status_dirs",
        not duplicate_ids,
        "no task ids duplicated across pending and completed"
        if not duplicate_ids
        else f"duplicate task ids across pending and completed: {duplicate_ids}",
        "roadmap/pending_tasks + roadmap/completed_tasks",
    )

    all_task_ids = pending_ids | completed_ids
    backlog_id_set = set(backlog_ids)
    missing_ids = sorted(backlog_id_set - all_task_ids)
    extra_ids = sorted(all_task_ids - backlog_id_set)

    add_check(
        "all_backlog_ids_have_task_files",
        not missing_ids,
        "every backlog id has a task file"
        if not missing_ids
        else f"missing task files for backlog ids: {missing_ids}",
        "roadmap/pending_tasks + roadmap/completed_tasks",
    )
    add_check(
        "task_dirs_do_not_contain_unknown_ids",
        not extra_ids,
        "task directories contain only known backlog ids"
        if not extra_ids
        else f"task directories contain ids missing from backlog: {extra_ids}",
        "roadmap/pending_tasks + roadmap/completed_tasks",
    )

    for rel_path in manifest["content_rules"]["files_checked_for_forbidden_strings"]:
        path = repo_root / rel_path
        text = path.read_text()
        for forbidden in manifest["content_rules"]["forbidden_strings"]:
            add_check(
                "forbidden_legacy_string_absent",
                forbidden not in text,
                "forbidden legacy string absent"
                if forbidden not in text
                else f"forbidden legacy string found: {forbidden}",
                rel_path,
            )

    failures = [check for check in checks if not check["ok"]]
    result = {
        "ok": not failures,
        "repo_root": str(repo_root),
        "manifest_path": str(manifest_path.relative_to(repo_root)),
        "summary": {
            "passed": len(checks) - len(failures),
            "failed": len(failures),
            "backlog_ids": len(backlog_id_set),
            "pending_task_ids": len(pending_ids),
            "completed_task_ids": len(completed_ids),
        },
        "checks": checks,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
