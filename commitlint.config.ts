// refer to: https://commitlint.js.org/#/reference-configuration?id=shareable-configuration
// Rule: https://karma-runner.github.io/6.4/dev/git-commit-msg.html
// Rule: https://github.com/conventional-changelog/commitlint/tree/master/@commitlint/config-conventional

// NOTE: the extends must consistent with `.pre-commit-config.yaml`

import type { UserConfig } from "@commitlint/types";

const config: UserConfig = {
    extends: ["@commitlint/config-conventional"],
};

export default config;
