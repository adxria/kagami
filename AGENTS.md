# Testing judgment

Every automated test should have a clear purpose: protecting meaningful behavior or a consequential failure. Choose coverage based on risk, not test counts or implementation details.

Keep tests focused and readable. Decide whether to add, extend, reorganize, or omit a test based on clarity and purpose; avoid both redundant tests and oversized scenarios.

Do not add tests for cosmetic changes, trivial implementation details, or redundant permutations. Use visual inspection for styling.

Run checks appropriate to the change, and expand validation only when there is a concrete unresolved concern.

## Windows test build

After app or packaging changes, run `npm run build:windows:test` before handing off and link `target/test-profile/x86_64-pc-windows-msvc/release/kagami.exe` in your final response. Use this executable for manual checks. It uses the `app.kagami.test` data profile so agents and humans can review changes without touching their regular kagami data.
