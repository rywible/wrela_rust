# Scenario Catalog

`PR-003` introduces the first canonical smoke scenarios for the headless runner.

Current scenarios:

- `scenarios/smoke/startup.ron`: bootstrap smoke pass for deterministic startup execution.
- `scenarios/smoke/assertion_failure.ron`: forced assertion failure used to prove the terminal report still lands on failure.

Scenario files are authored in RON and deserialize into `wr_tools_harness::ScenarioRequest`.
