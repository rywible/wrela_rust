from __future__ import annotations

import importlib.util
import pathlib
import tempfile
import unittest
from types import SimpleNamespace
from unittest import mock


SCRIPT_PATH = pathlib.Path(__file__).resolve().parents[1] / "run_claude_review.py"
SPEC = importlib.util.spec_from_file_location("run_claude_review", SCRIPT_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ClassifyClaudeStdoutTests(unittest.TestCase):
    def test_completed_review_extracts_text_and_payload(self) -> None:
        status, review_text, payload, parse_error = MODULE.classify_claude_stdout(
            '{"type":"result","subtype":"success","result":"Review body\\n"}'
        )

        self.assertEqual(status, "completed")
        self.assertEqual(review_text, "Review body\n")
        self.assertIsNone(parse_error)
        self.assertEqual(payload["type"], "result")

    def test_blank_result_is_classified_as_empty_output(self) -> None:
        status, review_text, payload, parse_error = MODULE.classify_claude_stdout(
            '{"type":"result","subtype":"success","result":"   "}'
        )

        self.assertEqual(status, "completed_empty_output")
        self.assertEqual(review_text, "")
        self.assertIsNone(parse_error)
        self.assertEqual(payload["subtype"], "success")

    def test_invalid_json_is_classified_separately(self) -> None:
        status, review_text, payload, parse_error = MODULE.classify_claude_stdout("{not-json}")

        self.assertEqual(status, "completed_invalid_output")
        self.assertEqual(review_text, "")
        self.assertIsNone(payload)
        self.assertIn("line 1, column 2", parse_error)

    def test_non_object_json_payload_is_classified_separately(self) -> None:
        status, review_text, payload, parse_error = MODULE.classify_claude_stdout("[1, 2, 3]")

        self.assertEqual(status, "completed_invalid_output")
        self.assertEqual(review_text, "")
        self.assertIsNone(payload)
        self.assertIn("Expected Claude JSON output to be an object", parse_error)

    def test_non_string_result_is_classified_as_invalid_output(self) -> None:
        status, review_text, payload, parse_error = MODULE.classify_claude_stdout(
            '{"type":"result","subtype":"success","result":123}'
        )

        self.assertEqual(status, "completed_invalid_output")
        self.assertEqual(review_text, "")
        self.assertEqual(payload["subtype"], "success")
        self.assertIn("Expected Claude JSON output 'result' to be a string", parse_error)

    def test_empty_stdout_stays_empty_output(self) -> None:
        status, review_text, payload, parse_error = MODULE.classify_claude_stdout("\n")

        self.assertEqual(status, "completed_empty_output")
        self.assertEqual(review_text, "")
        self.assertIsNone(payload)
        self.assertIsNone(parse_error)


class PrimaryModelNameTests(unittest.TestCase):
    def test_returns_only_model_key_when_present(self) -> None:
        self.assertEqual(
            MODULE.primary_model_name({"modelUsage": {"claude-opus-4-6[1m]": {"inputTokens": 1}}}),
            "claude-opus-4-6[1m]",
        )

    def test_returns_none_for_empty_model_usage(self) -> None:
        self.assertIsNone(MODULE.primary_model_name({"modelUsage": {}}))

    def test_returns_none_when_model_usage_missing(self) -> None:
        self.assertIsNone(MODULE.primary_model_name({}))

    def test_returns_none_when_model_usage_is_not_a_dict(self) -> None:
        self.assertIsNone(MODULE.primary_model_name({"modelUsage": []}))


class ExitCodeForStatusTests(unittest.TestCase):
    def test_statuses_map_to_expected_exit_codes(self) -> None:
        self.assertEqual(MODULE.exit_code_for_status("completed"), 0)
        self.assertEqual(MODULE.exit_code_for_status("completed_empty_output"), 3)
        self.assertEqual(MODULE.exit_code_for_status("exit_nonzero"), 4)
        self.assertEqual(MODULE.exit_code_for_status("timeout"), 5)
        self.assertEqual(MODULE.exit_code_for_status("completed_invalid_output"), 6)
        self.assertEqual(MODULE.exit_code_for_status("unexpected"), 1)


class RunClaudeAttemptTests(unittest.TestCase):
    def test_run_claude_attempt_classifies_successful_json_output(self) -> None:
        completed = SimpleNamespace(
            returncode=0,
            stdout='{"type":"result","subtype":"success","result":"Review text"}',
            stderr="",
        )

        with mock.patch.object(MODULE.subprocess, "run", return_value=completed):
            result = MODULE.run_claude_attempt(["claude"], pathlib.Path("."), 123)

        self.assertEqual(result["status"], "completed")
        self.assertEqual(result["review_text"], "Review text\n")
        self.assertEqual(result["summary"]["timeout_seconds"], 123)

    def test_run_claude_attempt_preserves_partial_timeout_output(self) -> None:
        timeout = MODULE.subprocess.TimeoutExpired(
            cmd=["claude"],
            timeout=60,
            output='{"type":"result","subtype":"success","result":"Partial review"}',
            stderr="",
        )

        with mock.patch.object(MODULE.subprocess, "run", side_effect=timeout):
            result = MODULE.run_claude_attempt(["claude"], pathlib.Path("."), 60)

        self.assertEqual(result["status"], "timeout")
        self.assertEqual(result["review_text"], "Partial review\n")
        self.assertTrue(result["summary"]["timed_out"])


class MainRetryFlowTests(unittest.TestCase):
    def make_attempt(
        self,
        *,
        status: str,
        review_text: str = "",
        raw_response: str = "",
        exit_code: int | None = 0,
        timed_out: bool = False,
    ) -> dict:
        return {
            "summary": {
                "status": status,
                "exit_code": exit_code,
                "timed_out": timed_out,
                "timeout_seconds": 600,
                "started_at_unix_ms": 1,
                "completed_at_unix_ms": 2,
                "stdout_bytes": len(review_text.encode("utf-8")),
                "raw_response_bytes": len(raw_response.encode("utf-8")),
                "stderr_bytes": 0,
            },
            "status": status,
            "exit_code": exit_code,
            "timed_out": timed_out,
            "raw_response": raw_response,
            "stderr": "",
            "review_text": review_text,
            "claude_payload": None,
            "parse_error": None,
        }

    def test_main_does_not_retry_after_completed_attempt(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = pathlib.Path(temp_dir)
            args = SimpleNamespace(
                base_ref="origin/main",
                timeout_seconds=600,
                run_id="single-attempt",
                reports_root="reports/process/claude-review",
                policy_path="docs/process/CODE_REVIEW_GUIDELINES.md",
                claude_binary="claude",
            )
            attempt = self.make_attempt(
                status="completed",
                review_text="Review text\n",
                raw_response='{"result":"Review text"}',
            )

            with (
                mock.patch.object(MODULE, "parse_args", return_value=args),
                mock.patch.object(MODULE, "repo_root", return_value=temp_root),
                mock.patch.object(MODULE, "current_git_value", return_value="main"),
                mock.patch.object(MODULE, "run_claude_attempt", return_value=attempt) as run_attempt,
                mock.patch.object(MODULE.time, "monotonic", side_effect=[0.0, 0.0]),
            ):
                exit_code = MODULE.main()

            self.assertEqual(exit_code, 0)
            self.assertEqual(run_attempt.call_count, 1)

    def test_main_retries_once_after_empty_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = pathlib.Path(temp_dir)
            args = SimpleNamespace(
                base_ref="origin/main",
                timeout_seconds=600,
                run_id="retry-attempt",
                reports_root="reports/process/claude-review",
                policy_path="docs/process/CODE_REVIEW_GUIDELINES.md",
                claude_binary="claude",
            )
            empty_attempt = self.make_attempt(status="completed_empty_output", raw_response='{"result":""}')
            completed_attempt = self.make_attempt(
                status="completed",
                review_text="Recovered review\n",
                raw_response='{"result":"Recovered review"}',
            )

            with (
                mock.patch.object(MODULE, "parse_args", return_value=args),
                mock.patch.object(MODULE, "repo_root", return_value=temp_root),
                mock.patch.object(MODULE, "current_git_value", return_value="main"),
                mock.patch.object(
                    MODULE,
                    "run_claude_attempt",
                    side_effect=[empty_attempt, completed_attempt],
                ) as run_attempt,
                mock.patch.object(MODULE.time, "monotonic", side_effect=[0.0, 0.0, 1.0]),
            ):
                exit_code = MODULE.main()

            result_path = (
                temp_root / "reports/process/claude-review/retry-attempt/result.json"
            )
            payload = MODULE.json.loads(result_path.read_text(encoding="utf-8"))
            self.assertEqual(exit_code, 0)
            self.assertEqual(run_attempt.call_count, 2)
            self.assertEqual(payload["attempt_count"], 2)
            self.assertEqual(payload["status"], "completed")
            self.assertEqual(payload["attempts"][0]["status"], "completed_empty_output")
            self.assertEqual(payload["attempts"][1]["status"], "completed")

    def test_main_skips_retry_when_timeout_budget_is_exhausted(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = pathlib.Path(temp_dir)
            args = SimpleNamespace(
                base_ref="origin/main",
                timeout_seconds=1,
                run_id="timeout-budget",
                reports_root="reports/process/claude-review",
                policy_path="docs/process/CODE_REVIEW_GUIDELINES.md",
                claude_binary="claude",
            )
            empty_attempt = self.make_attempt(status="completed_empty_output", raw_response='{"result":""}')

            with (
                mock.patch.object(MODULE, "parse_args", return_value=args),
                mock.patch.object(MODULE, "repo_root", return_value=temp_root),
                mock.patch.object(MODULE, "current_git_value", return_value="main"),
                mock.patch.object(MODULE, "run_claude_attempt", return_value=empty_attempt) as run_attempt,
                mock.patch.object(MODULE.time, "monotonic", side_effect=[0.0, 0.0, 2.0]),
            ):
                exit_code = MODULE.main()

            result_path = (
                temp_root / "reports/process/claude-review/timeout-budget/result.json"
            )
            payload = MODULE.json.loads(result_path.read_text(encoding="utf-8"))
            self.assertEqual(exit_code, 5)
            self.assertEqual(run_attempt.call_count, 1)
            self.assertEqual(payload["attempt_count"], 2)
            self.assertEqual(payload["status"], "timeout")
            self.assertEqual(payload["attempts"][0]["status"], "completed_empty_output")
            self.assertEqual(payload["attempts"][1]["status"], "timeout")


if __name__ == "__main__":
    unittest.main()
