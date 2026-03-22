# Report Artifacts

Machine-readable reports and terminal result bundles should land here in stable, discoverable paths.

Harness contract artifacts created in bootstrap mode live under:

- `reports/harness/<command>/<run_id>/terminal_report.json`

The verification stack also writes stable sibling artifacts inside the same run directory, including:

- `verify_steps.json`
- `trace.jsonl`
- `nextest-junit.xml`
- copied Criterion `estimates.json` files for the selected benchmark group
