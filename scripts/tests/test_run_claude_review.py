from __future__ import annotations

import importlib.util
import pathlib
import unittest


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


if __name__ == "__main__":
    unittest.main()
