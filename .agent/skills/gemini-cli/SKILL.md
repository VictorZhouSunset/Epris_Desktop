---

name: gemini-cli

description: Use when writing code for configuring gemini-cli, this is the official gemini-cli configuration format

---



# Gemini CLI configuration



> \*\*Note on configuration format, 9/17/25:\*\* The format of the `settings.json`

> file has been updated to a new, more organized structure.

>

> - The new format will be supported in the stable release starting

>   \*\*\[09/10/25]\*\*.

> - Automatic migration from the old format to the new format will begin on

>   \*\*\[09/17/25]\*\*.

>

> For details on the previous format, please see the

> \[v1 Configuration documentation](./configuration-v1.md).



Gemini CLI offers several ways to configure its behavior, including environment

variables, command-line arguments, and settings files. This document outlines

the different configuration methods and available settings.



\## Configuration layers



Configuration is applied in the following order of precedence (lower numbers are

overridden by higher numbers):



1\.  \*\*Default values:\*\* Hardcoded defaults within the application.

2\.  \*\*System defaults file:\*\* System-wide default settings that can be

&nbsp;   overridden by other settings files.

3\.  \*\*User settings file:\*\* Global settings for the current user.

4\.  \*\*Project settings file:\*\* Project-specific settings.

5\.  \*\*System settings file:\*\* System-wide settings that override all other

&nbsp;   settings files.

6\.  \*\*Environment variables:\*\* System-wide or session-specific variables,

&nbsp;   potentially loaded from `.env` files.

7\.  \*\*Command-line arguments:\*\* Values passed when launching the CLI.



\## Settings files



Gemini CLI uses JSON settings files for persistent configuration. There are four

locations for these files:



> \*\*Tip:\*\* JSON-aware editors can use autocomplete and validation by pointing to

> the generated schema at `schemas/settings.schema.json` in this repository.

> When working outside the repo, reference the hosted schema at

> `https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json`.



\- \*\*System defaults file:\*\*

&nbsp; - \*\*Location:\*\* `/etc/gemini-cli/system-defaults.json` (Linux),

&nbsp;   `C:\\ProgramData\\gemini-cli\\system-defaults.json` (Windows) or

&nbsp;   `/Library/Application Support/GeminiCli/system-defaults.json` (macOS). The

&nbsp;   path can be overridden using the `GEMINI\_CLI\_SYSTEM\_DEFAULTS\_PATH`

&nbsp;   environment variable.

&nbsp; - \*\*Scope:\*\* Provides a base layer of system-wide default settings. These

&nbsp;   settings have the lowest precedence and are intended to be overridden by

&nbsp;   user, project, or system override settings.

\- \*\*User settings file:\*\*

&nbsp; - \*\*Location:\*\* `~/.gemini/settings.json` (where `~` is your home directory).

&nbsp; - \*\*Scope:\*\* Applies to all Gemini CLI sessions for the current user. User

&nbsp;   settings override system defaults.

\- \*\*Project settings file:\*\*

&nbsp; - \*\*Location:\*\* `.gemini/settings.json` within your project's root directory.

&nbsp; - \*\*Scope:\*\* Applies only when running Gemini CLI from that specific project.

&nbsp;   Project settings override user settings and system defaults.

\- \*\*System settings file:\*\*

&nbsp; - \*\*Location:\*\* `/etc/gemini-cli/settings.json` (Linux),

&nbsp;   `C:\\ProgramData\\gemini-cli\\settings.json` (Windows) or

&nbsp;   `/Library/Application Support/GeminiCli/settings.json` (macOS). The path can

&nbsp;   be overridden using the `GEMINI\_CLI\_SYSTEM\_SETTINGS\_PATH` environment

&nbsp;   variable.

&nbsp; - \*\*Scope:\*\* Applies to all Gemini CLI sessions on the system, for all users.

&nbsp;   System settings act as overrides, taking precedence over all other settings

&nbsp;   files. May be useful for system administrators at enterprises to have

&nbsp;   controls over users' Gemini CLI setups.



\*\*Note on environment variables in settings:\*\* String values within your

`settings.json` and `gemini-extension.json` files can reference environment

variables using either `$VAR\_NAME` or `${VAR\_NAME}` syntax. These variables will

be automatically resolved when the settings are loaded. For example, if you have

an environment variable `MY\_API\_TOKEN`, you could use it in `settings.json` like

this: `"apiKey": "$MY\_API\_TOKEN"`. Additionally, each extension can have its own

`.env` file in its directory, which will be loaded automatically.



> \*\*Note for Enterprise Users:\*\* For guidance on deploying and managing Gemini

> CLI in a corporate environment, please see the

> \[Enterprise Configuration](../cli/enterprise.md) documentation.



\### The `.gemini` directory in your project



In addition to a project settings file, a project's `.gemini` directory can

contain other project-specific files related to Gemini CLI's operation, such as:



\- \[Custom sandbox profiles](#sandboxing) (e.g.,

&nbsp; `.gemini/sandbox-macos-custom.sb`, `.gemini/sandbox.Dockerfile`).



\### Available settings in `settings.json`



Settings are organized into categories. All settings should be placed within

their corresponding top-level category object in your `settings.json` file.



<!-- SETTINGS-AUTOGEN:START -->



\#### `general`



\- \*\*`general.previewFeatures`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable preview features (e.g., preview models).

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`general.preferredEditor`\*\* (string):

&nbsp; - \*\*Description:\*\* The preferred editor to open files in.

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`general.vimMode`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable Vim keybindings

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`general.enableAutoUpdate`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable automatic updates.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`general.enableAutoUpdateNotification`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable update notification prompts.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`general.checkpointing.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable session checkpointing for recovery

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`general.enablePromptCompletion`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable AI-powered prompt completion suggestions while

&nbsp;   typing.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`general.retryFetchErrors`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Retry on "exception TypeError: fetch failed sending

&nbsp;   request" errors.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`general.debugKeystrokeLogging`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable debug logging of keystrokes to the console.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`general.sessionRetention.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable automatic session cleanup

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`general.sessionRetention.maxAge`\*\* (string):

&nbsp; - \*\*Description:\*\* Maximum age of sessions to keep (e.g., "30d", "7d", "24h",

&nbsp;   "1w")

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`general.sessionRetention.maxCount`\*\* (number):

&nbsp; - \*\*Description:\*\* Alternative: Maximum number of sessions to keep (most

&nbsp;   recent)

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`general.sessionRetention.minRetention`\*\* (string):

&nbsp; - \*\*Description:\*\* Minimum retention period (safety limit, defaults to "1d")

&nbsp; - \*\*Default:\*\* `"1d"`



\#### `output`



\- \*\*`output.format`\*\* (enum):

&nbsp; - \*\*Description:\*\* The format of the CLI output. Can be `text` or `json`.

&nbsp; - \*\*Default:\*\* `"text"`

&nbsp; - \*\*Values:\*\* `"text"`, `"json"`



\#### `ui`



\- \*\*`ui.theme`\*\* (string):

&nbsp; - \*\*Description:\*\* The color theme for the UI. See the CLI themes guide for

&nbsp;   available options.

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`ui.customThemes`\*\* (object):

&nbsp; - \*\*Description:\*\* Custom theme definitions.

&nbsp; - \*\*Default:\*\* `{}`



\- \*\*`ui.hideWindowTitle`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the window title bar

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`ui.showStatusInTitle`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show Gemini CLI model thoughts in the terminal window title

&nbsp;   during the working phase

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.dynamicWindowTitle`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Update the terminal window title with current status icons

&nbsp;   (Ready: ◇, Action Required: ✋, Working: ✦)

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`ui.showHomeDirectoryWarning`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show a warning when running Gemini CLI in the home

&nbsp;   directory.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`ui.hideTips`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide helpful tips in the UI

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.hideBanner`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the application banner

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.hideContextSummary`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the context summary (GEMINI.md, MCP servers) above the

&nbsp;   input.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.footer.hideCWD`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the current working directory path in the footer.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.footer.hideSandboxStatus`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the sandbox status indicator in the footer.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.footer.hideModelInfo`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the model name and context usage in the footer.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.footer.hideContextPercentage`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hides the context window remaining percentage.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`ui.hideFooter`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Hide the footer from the UI

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.showMemoryUsage`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Display memory usage information in the UI

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.showLineNumbers`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show line numbers in the chat.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`ui.showCitations`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show citations for generated text in the chat.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.showModelInfoInChat`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show the model name in the chat for each model turn.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`ui.useFullWidth`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Use the entire width of the terminal for output.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`ui.useAlternateBuffer`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Use an alternate screen buffer for the UI, preserving shell

&nbsp;   history.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`ui.incrementalRendering`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable incremental rendering for the UI. This option will

&nbsp;   reduce flickering but may cause rendering artifacts. Only supported when

&nbsp;   useAlternateBuffer is enabled.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`ui.customWittyPhrases`\*\* (array):

&nbsp; - \*\*Description:\*\* Custom witty phrases to display during loading. When

&nbsp;   provided, the CLI cycles through these instead of the defaults.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`ui.accessibility.enableLoadingPhrases`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable loading phrases during operations.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`ui.accessibility.screenReader`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Render output in plain-text to be more screen reader

&nbsp;   accessible

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `ide`



\- \*\*`ide.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable IDE integration mode.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`ide.hasSeenNudge`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Whether the user has seen the IDE integration nudge.

&nbsp; - \*\*Default:\*\* `false`



\#### `privacy`



\- \*\*`privacy.usageStatisticsEnabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable collection of usage statistics

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `model`



\- \*\*`model.name`\*\* (string):

&nbsp; - \*\*Description:\*\* The Gemini model to use for conversations.

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`model.maxSessionTurns`\*\* (number):

&nbsp; - \*\*Description:\*\* Maximum number of user/model/tool turns to keep in a

&nbsp;   session. -1 means unlimited.

&nbsp; - \*\*Default:\*\* `-1`



\- \*\*`model.summarizeToolOutput`\*\* (object):

&nbsp; - \*\*Description:\*\* Enables or disables summarization of tool output. Configure

&nbsp;   per-tool token budgets (for example {"run\_shell\_command": {"tokenBudget":

&nbsp;   2000}}). Currently only the run\_shell\_command tool supports summarization.

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`model.compressionThreshold`\*\* (number):

&nbsp; - \*\*Description:\*\* The fraction of context usage at which to trigger context

&nbsp;   compression (e.g. 0.2, 0.3).

&nbsp; - \*\*Default:\*\* `0.5`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`model.skipNextSpeakerCheck`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Skip the next speaker check.

&nbsp; - \*\*Default:\*\* `true`



\#### `modelConfigs`



\- \*\*`modelConfigs.aliases`\*\* (object):

&nbsp; - \*\*Description:\*\* Named presets for model configs. Can be used in place of a

&nbsp;   model name and can inherit from other aliases using an `extends` property.

&nbsp; - \*\*Default:\*\*



&nbsp;   ```json

&nbsp;   {

&nbsp;     "base": {

&nbsp;       "modelConfig": {

&nbsp;         "generateContentConfig": {

&nbsp;           "temperature": 0,

&nbsp;           "topP": 1

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-base": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "generateContentConfig": {

&nbsp;           "thinkingConfig": {

&nbsp;             "includeThoughts": true

&nbsp;           },

&nbsp;           "temperature": 1,

&nbsp;           "topP": 0.95,

&nbsp;           "topK": 64

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-base-2.5": {

&nbsp;       "extends": "chat-base",

&nbsp;       "modelConfig": {

&nbsp;         "generateContentConfig": {

&nbsp;           "thinkingConfig": {

&nbsp;             "thinkingBudget": 8192

&nbsp;           }

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-base-3": {

&nbsp;       "extends": "chat-base",

&nbsp;       "modelConfig": {

&nbsp;         "generateContentConfig": {

&nbsp;           "thinkingConfig": {

&nbsp;             "thinkingLevel": "HIGH"

&nbsp;           }

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "gemini-3-pro-preview": {

&nbsp;       "extends": "chat-base-3",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-3-pro-preview"

&nbsp;       }

&nbsp;     },

&nbsp;     "gemini-3-flash-preview": {

&nbsp;       "extends": "chat-base-3",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-3-flash-preview"

&nbsp;       }

&nbsp;     },

&nbsp;     "gemini-2.5-pro": {

&nbsp;       "extends": "chat-base-2.5",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-pro"

&nbsp;       }

&nbsp;     },

&nbsp;     "gemini-2.5-flash": {

&nbsp;       "extends": "chat-base-2.5",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash"

&nbsp;       }

&nbsp;     },

&nbsp;     "gemini-2.5-flash-lite": {

&nbsp;       "extends": "chat-base-2.5",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite"

&nbsp;       }

&nbsp;     },

&nbsp;     "gemini-2.5-flash-base": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash"

&nbsp;       }

&nbsp;     },

&nbsp;     "classifier": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite",

&nbsp;         "generateContentConfig": {

&nbsp;           "maxOutputTokens": 1024,

&nbsp;           "thinkingConfig": {

&nbsp;             "thinkingBudget": 512

&nbsp;           }

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "prompt-completion": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite",

&nbsp;         "generateContentConfig": {

&nbsp;           "temperature": 0.3,

&nbsp;           "maxOutputTokens": 16000,

&nbsp;           "thinkingConfig": {

&nbsp;             "thinkingBudget": 0

&nbsp;           }

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "edit-corrector": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite",

&nbsp;         "generateContentConfig": {

&nbsp;           "thinkingConfig": {

&nbsp;             "thinkingBudget": 0

&nbsp;           }

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "summarizer-default": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite",

&nbsp;         "generateContentConfig": {

&nbsp;           "maxOutputTokens": 2000

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "summarizer-shell": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite",

&nbsp;         "generateContentConfig": {

&nbsp;           "maxOutputTokens": 2000

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "web-search": {

&nbsp;       "extends": "gemini-2.5-flash-base",

&nbsp;       "modelConfig": {

&nbsp;         "generateContentConfig": {

&nbsp;           "tools": \[

&nbsp;             {

&nbsp;               "googleSearch": {}

&nbsp;             }

&nbsp;           ]

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "web-fetch": {

&nbsp;       "extends": "gemini-2.5-flash-base",

&nbsp;       "modelConfig": {

&nbsp;         "generateContentConfig": {

&nbsp;           "tools": \[

&nbsp;             {

&nbsp;               "urlContext": {}

&nbsp;             }

&nbsp;           ]

&nbsp;         }

&nbsp;       }

&nbsp;     },

&nbsp;     "web-fetch-fallback": {

&nbsp;       "extends": "gemini-2.5-flash-base",

&nbsp;       "modelConfig": {}

&nbsp;     },

&nbsp;     "loop-detection": {

&nbsp;       "extends": "gemini-2.5-flash-base",

&nbsp;       "modelConfig": {}

&nbsp;     },

&nbsp;     "loop-detection-double-check": {

&nbsp;       "extends": "base",

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-pro"

&nbsp;       }

&nbsp;     },

&nbsp;     "llm-edit-fixer": {

&nbsp;       "extends": "gemini-2.5-flash-base",

&nbsp;       "modelConfig": {}

&nbsp;     },

&nbsp;     "next-speaker-checker": {

&nbsp;       "extends": "gemini-2.5-flash-base",

&nbsp;       "modelConfig": {}

&nbsp;     },

&nbsp;     "chat-compression-3-pro": {

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-3-pro-preview"

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-compression-3-flash": {

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-3-flash-preview"

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-compression-2.5-pro": {

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-pro"

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-compression-2.5-flash": {

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash"

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-compression-2.5-flash-lite": {

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-flash-lite"

&nbsp;       }

&nbsp;     },

&nbsp;     "chat-compression-default": {

&nbsp;       "modelConfig": {

&nbsp;         "model": "gemini-2.5-pro"

&nbsp;       }

&nbsp;     }

&nbsp;   }

&nbsp;   ```



\- \*\*`modelConfigs.customAliases`\*\* (object):

&nbsp; - \*\*Description:\*\* Custom named presets for model configs. These are merged

&nbsp;   with (and override) the built-in aliases.

&nbsp; - \*\*Default:\*\* `{}`



\- \*\*`modelConfigs.customOverrides`\*\* (array):

&nbsp; - \*\*Description:\*\* Custom model config overrides. These are merged with (and

&nbsp;   added to) the built-in overrides.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`modelConfigs.overrides`\*\* (array):

&nbsp; - \*\*Description:\*\* Apply specific configuration overrides based on matches,

&nbsp;   with a primary key of model (or alias). The most specific match will be

&nbsp;   used.

&nbsp; - \*\*Default:\*\* `\[]`



\#### `agents`



\- \*\*`agents.overrides`\*\* (object):

&nbsp; - \*\*Description:\*\* Override settings for specific agents, e.g. to disable the

&nbsp;   agent, set a custom model config, or run config.

&nbsp; - \*\*Default:\*\* `{}`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `context`



\- \*\*`context.fileName`\*\* (string | string\[]):

&nbsp; - \*\*Description:\*\* The name of the context file or files to load into memory.

&nbsp;   Accepts either a single string or an array of strings.

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`context.importFormat`\*\* (string):

&nbsp; - \*\*Description:\*\* The format to use when importing memory.

&nbsp; - \*\*Default:\*\* `undefined`



\- \*\*`context.discoveryMaxDirs`\*\* (number):

&nbsp; - \*\*Description:\*\* Maximum number of directories to search for memory.

&nbsp; - \*\*Default:\*\* `200`



\- \*\*`context.includeDirectories`\*\* (array):

&nbsp; - \*\*Description:\*\* Additional directories to include in the workspace context.

&nbsp;   Missing directories will be skipped with a warning.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`context.loadMemoryFromIncludeDirectories`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Controls how /memory refresh loads GEMINI.md files. When

&nbsp;   true, include directories are scanned; when false, only the current

&nbsp;   directory is used.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`context.fileFiltering.respectGitIgnore`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Respect .gitignore files when searching.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`context.fileFiltering.respectGeminiIgnore`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Respect .geminiignore files when searching.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`context.fileFiltering.enableRecursiveFileSearch`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable recursive file search functionality when completing

&nbsp;   @ references in the prompt.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`context.fileFiltering.enableFuzzySearch`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable fuzzy search when searching for files.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `tools`



\- \*\*`tools.sandbox`\*\* (boolean | string):

&nbsp; - \*\*Description:\*\* Sandbox execution environment. Set to a boolean to enable

&nbsp;   or disable the sandbox, or provide a string path to a sandbox profile.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.shell.enableInteractiveShell`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Use node-pty for an interactive shell experience. Fallback

&nbsp;   to child\_process still applies.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.shell.pager`\*\* (string):

&nbsp; - \*\*Description:\*\* The pager command to use for shell output. Defaults to

&nbsp;   `cat`.

&nbsp; - \*\*Default:\*\* `"cat"`



\- \*\*`tools.shell.showColor`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show color in shell output.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`tools.shell.inactivityTimeout`\*\* (number):

&nbsp; - \*\*Description:\*\* The maximum time in seconds allowed without output from the

&nbsp;   shell command. Defaults to 5 minutes.

&nbsp; - \*\*Default:\*\* `300`



\- \*\*`tools.shell.enableShellOutputEfficiency`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable shell output efficiency optimizations for better

&nbsp;   performance.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`tools.autoAccept`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Automatically accept and execute tool calls that are

&nbsp;   considered safe (e.g., read-only operations).

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`tools.approvalMode`\*\* (enum):

&nbsp; - \*\*Description:\*\* The default approval mode for tool execution. 'default'

&nbsp;   prompts for approval, 'auto\_edit' auto-approves edit tools, and 'plan' is

&nbsp;   read-only mode. 'yolo' is not supported yet.

&nbsp; - \*\*Default:\*\* `"default"`

&nbsp; - \*\*Values:\*\* `"default"`, `"auto\_edit"`, `"plan"`



\- \*\*`tools.core`\*\* (array):

&nbsp; - \*\*Description:\*\* Restrict the set of built-in tools with an allowlist. Match

&nbsp;   semantics mirror tools.allowed; see the built-in tools documentation for

&nbsp;   available names.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.allowed`\*\* (array):

&nbsp; - \*\*Description:\*\* Tool names that bypass the confirmation dialog. Useful for

&nbsp;   trusted commands (for example \["run\_shell\_command(git)",

&nbsp;   "run\_shell\_command(npm test)"]). See shell tool command restrictions for

&nbsp;   matching details.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.exclude`\*\* (array):

&nbsp; - \*\*Description:\*\* Tool names to exclude from discovery.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.discoveryCommand`\*\* (string):

&nbsp; - \*\*Description:\*\* Command to run for tool discovery.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.callCommand`\*\* (string):

&nbsp; - \*\*Description:\*\* Defines a custom shell command for invoking discovered

&nbsp;   tools. The command must take the tool name as the first argument, read JSON

&nbsp;   arguments from stdin, and emit JSON results on stdout.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.useRipgrep`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Use ripgrep for file content search instead of the fallback

&nbsp;   implementation. Provides faster search performance.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`tools.enableToolOutputTruncation`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable truncation of large tool outputs.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.truncateToolOutputThreshold`\*\* (number):

&nbsp; - \*\*Description:\*\* Truncate tool output if it is larger than this many

&nbsp;   characters. Set to -1 to disable.

&nbsp; - \*\*Default:\*\* `4000000`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.truncateToolOutputLines`\*\* (number):

&nbsp; - \*\*Description:\*\* The number of lines to keep when truncating tool output.

&nbsp; - \*\*Default:\*\* `1000`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.disableLLMCorrection`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Disable LLM-based error correction for edit tools. When

&nbsp;   enabled, tools will fail immediately if exact string matches are not found,

&nbsp;   instead of attempting to self-correct.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`tools.enableHooks`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enables the hooks system experiment. When disabled, the

&nbsp;   hooks system is completely deactivated regardless of other settings.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `mcp`



\- \*\*`mcp.serverCommand`\*\* (string):

&nbsp; - \*\*Description:\*\* Command to start an MCP server.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`mcp.allowed`\*\* (array):

&nbsp; - \*\*Description:\*\* A list of MCP servers to allow.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`mcp.excluded`\*\* (array):

&nbsp; - \*\*Description:\*\* A list of MCP servers to exclude.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `useWriteTodos`



\- \*\*`useWriteTodos`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable the write\_todos tool.

&nbsp; - \*\*Default:\*\* `true`



\#### `security`



\- \*\*`security.disableYoloMode`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Disable YOLO mode, even if enabled by a flag.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.enablePermanentToolApproval`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable the "Allow for all future sessions" option in tool

&nbsp;   confirmation dialogs.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`security.blockGitExtensions`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Blocks installing and loading extensions from Git.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.folderTrust.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Setting to track whether Folder trust is enabled.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.environmentVariableRedaction.allowed`\*\* (array):

&nbsp; - \*\*Description:\*\* Environment variables to always allow (bypass redaction).

&nbsp; - \*\*Default:\*\* `\[]`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.environmentVariableRedaction.blocked`\*\* (array):

&nbsp; - \*\*Description:\*\* Environment variables to always redact.

&nbsp; - \*\*Default:\*\* `\[]`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.environmentVariableRedaction.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable redaction of environment variables that may contain

&nbsp;   secrets.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.auth.selectedType`\*\* (string):

&nbsp; - \*\*Description:\*\* The currently selected authentication type.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.auth.enforcedType`\*\* (string):

&nbsp; - \*\*Description:\*\* The required auth type. If this does not match the selected

&nbsp;   auth type, the user will be prompted to re-authenticate.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`security.auth.useExternal`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Whether to use an external authentication flow.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `advanced`



\- \*\*`advanced.autoConfigureMemory`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Automatically configure Node.js memory limits

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`advanced.dnsResolutionOrder`\*\* (string):

&nbsp; - \*\*Description:\*\* The DNS resolution order.

&nbsp; - \*\*Default:\*\* `undefined`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`advanced.excludedEnvVars`\*\* (array):

&nbsp; - \*\*Description:\*\* Environment variables to exclude from project context.

&nbsp; - \*\*Default:\*\*



&nbsp;   ```json

&nbsp;   \["DEBUG", "DEBUG\_MODE"]

&nbsp;   ```



\- \*\*`advanced.bugCommand`\*\* (object):

&nbsp; - \*\*Description:\*\* Configuration for the bug report command.

&nbsp; - \*\*Default:\*\* `undefined`



\#### `experimental`



\- \*\*`experimental.enableAgents`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable local and remote subagents. Warning: Experimental

&nbsp;   feature, uses YOLO mode for subagents

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.extensionManagement`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable extension management features.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.extensionConfig`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable requesting and fetching of extension settings.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.enableEventDrivenScheduler`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enables event-driven scheduler within the CLI session.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.extensionReloading`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enables extension loading/unloading within the CLI session.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.jitContext`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable Just-In-Time (JIT) context loading.

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.skills`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable Agent Skills (experimental).

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\- \*\*`experimental.useOSC52Paste`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Use OSC 52 sequence for pasting instead of clipboardy

&nbsp;   (useful for remote sessions).

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`experimental.plan`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Enable planning features (Plan Mode and tools).

&nbsp; - \*\*Default:\*\* `false`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `skills`



\- \*\*`skills.disabled`\*\* (array):

&nbsp; - \*\*Description:\*\* List of disabled skills.

&nbsp; - \*\*Default:\*\* `\[]`

&nbsp; - \*\*Requires restart:\*\* Yes



\#### `hooksConfig`



\- \*\*`hooksConfig.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Canonical toggle for the hooks system. When disabled, no

&nbsp;   hooks will be executed.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`hooksConfig.disabled`\*\* (array):

&nbsp; - \*\*Description:\*\* List of hook names (commands) that should be disabled.

&nbsp;   Hooks in this list will not execute even if configured.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooksConfig.notifications`\*\* (boolean):

&nbsp; - \*\*Description:\*\* Show visual indicators when hooks are executing.

&nbsp; - \*\*Default:\*\* `true`



\#### `hooks`



\- \*\*`hooks.BeforeTool`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute before tool execution. Can intercept,

&nbsp;   validate, or modify tool calls.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.AfterTool`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute after tool execution. Can process

&nbsp;   results, log outputs, or trigger follow-up actions.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.BeforeAgent`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute before agent loop starts. Can set up

&nbsp;   context or initialize resources.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.AfterAgent`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute after agent loop completes. Can perform

&nbsp;   cleanup or summarize results.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.Notification`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute on notification events (errors,

&nbsp;   warnings, info). Can log or alert on specific conditions.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.SessionStart`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute when a session starts. Can initialize

&nbsp;   session-specific resources or state.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.SessionEnd`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute when a session ends. Can perform cleanup

&nbsp;   or persist session data.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.PreCompress`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute before chat history compression. Can

&nbsp;   back up or analyze conversation before compression.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.BeforeModel`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute before LLM requests. Can modify prompts,

&nbsp;   inject context, or control model parameters.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.AfterModel`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute after LLM responses. Can process

&nbsp;   outputs, extract information, or log interactions.

&nbsp; - \*\*Default:\*\* `\[]`



\- \*\*`hooks.BeforeToolSelection`\*\* (array):

&nbsp; - \*\*Description:\*\* Hooks that execute before tool selection. Can filter or

&nbsp;   prioritize available tools dynamically.

&nbsp; - \*\*Default:\*\* `\[]`



\#### `admin`



\- \*\*`admin.secureModeEnabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* If true, disallows yolo mode from being used.

&nbsp; - \*\*Default:\*\* `false`



\- \*\*`admin.extensions.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* If false, disallows extensions from being installed or

&nbsp;   used.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`admin.mcp.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* If false, disallows MCP servers from being used.

&nbsp; - \*\*Default:\*\* `true`



\- \*\*`admin.skills.enabled`\*\* (boolean):

&nbsp; - \*\*Description:\*\* If false, disallows agent skills from being used.

&nbsp; - \*\*Default:\*\* `true`

&nbsp; <!-- SETTINGS-AUTOGEN:END -->



\#### `mcpServers`



Configures connections to one or more Model-Context Protocol (MCP) servers for

discovering and using custom tools. Gemini CLI attempts to connect to each

configured MCP server to discover available tools. If multiple MCP servers

expose a tool with the same name, the tool names will be prefixed with the

server alias you defined in the configuration (e.g.,

`serverAlias\_\_actualToolName`) to avoid conflicts. Note that the system might

strip certain schema properties from MCP tool definitions for compatibility. At

least one of `command`, `url`, or `httpUrl` must be provided. If multiple are

specified, the order of precedence is `httpUrl`, then `url`, then `command`.



\- \*\*`mcpServers.<SERVER\_NAME>`\*\* (object): The server parameters for the named

&nbsp; server.

&nbsp; - `command` (string, optional): The command to execute to start the MCP server

&nbsp;   via standard I/O.

&nbsp; - `args` (array of strings, optional): Arguments to pass to the command.

&nbsp; - `env` (object, optional): Environment variables to set for the server

&nbsp;   process.

&nbsp; - `cwd` (string, optional): The working directory in which to start the

&nbsp;   server.

&nbsp; - `url` (string, optional): The URL of an MCP server that uses Server-Sent

&nbsp;   Events (SSE) for communication.

&nbsp; - `httpUrl` (string, optional): The URL of an MCP server that uses streamable

&nbsp;   HTTP for communication.

&nbsp; - `headers` (object, optional): A map of HTTP headers to send with requests to

&nbsp;   `url` or `httpUrl`.

&nbsp; - `timeout` (number, optional): Timeout in milliseconds for requests to this

&nbsp;   MCP server.

&nbsp; - `trust` (boolean, optional): Trust this server and bypass all tool call

&nbsp;   confirmations.

&nbsp; - `description` (string, optional): A brief description of the server, which

&nbsp;   may be used for display purposes.

&nbsp; - `includeTools` (array of strings, optional): List of tool names to include

&nbsp;   from this MCP server. When specified, only the tools listed here will be

&nbsp;   available from this server (allowlist behavior). If not specified, all tools

&nbsp;   from the server are enabled by default.

&nbsp; - `excludeTools` (array of strings, optional): List of tool names to exclude

&nbsp;   from this MCP server. Tools listed here will not be available to the model,

&nbsp;   even if they are exposed by the server. \*\*Note:\*\* `excludeTools` takes

&nbsp;   precedence over `includeTools` - if a tool is in both lists, it will be

&nbsp;   excluded.



\#### `telemetry`



Configures logging and metrics collection for Gemini CLI. For more information,

see \[Telemetry](../cli/telemetry.md).



\- \*\*Properties:\*\*

&nbsp; - \*\*`enabled`\*\* (boolean): Whether or not telemetry is enabled.

&nbsp; - \*\*`target`\*\* (string): The destination for collected telemetry. Supported

&nbsp;   values are `local` and `gcp`.

&nbsp; - \*\*`otlpEndpoint`\*\* (string): The endpoint for the OTLP Exporter.

&nbsp; - \*\*`otlpProtocol`\*\* (string): The protocol for the OTLP Exporter (`grpc` or

&nbsp;   `http`).

&nbsp; - \*\*`logPrompts`\*\* (boolean): Whether or not to include the content of user

&nbsp;   prompts in the logs.

&nbsp; - \*\*`outfile`\*\* (string): The file to write telemetry to when `target` is

&nbsp;   `local`.

&nbsp; - \*\*`useCollector`\*\* (boolean): Whether to use an external OTLP collector.



\### Example `settings.json`



Here is an example of a `settings.json` file with the nested structure, new as

of v0.3.0:



```json

{

&nbsp; "general": {

&nbsp;   "vimMode": true,

&nbsp;   "preferredEditor": "code",

&nbsp;   "sessionRetention": {

&nbsp;     "enabled": true,

&nbsp;     "maxAge": "30d",

&nbsp;     "maxCount": 100

&nbsp;   }

&nbsp; },

&nbsp; "ui": {

&nbsp;   "theme": "GitHub",

&nbsp;   "hideBanner": true,

&nbsp;   "hideTips": false,

&nbsp;   "customWittyPhrases": \[

&nbsp;     "You forget a thousand things every day. Make sure this is one of ’em",

&nbsp;     "Connecting to AGI"

&nbsp;   ]

&nbsp; },

&nbsp; "tools": {

&nbsp;   "sandbox": "docker",

&nbsp;   "discoveryCommand": "bin/get\_tools",

&nbsp;   "callCommand": "bin/call\_tool",

&nbsp;   "exclude": \["write\_file"]

&nbsp; },

&nbsp; "mcpServers": {

&nbsp;   "mainServer": {

&nbsp;     "command": "bin/mcp\_server.py"

&nbsp;   },

&nbsp;   "anotherServer": {

&nbsp;     "command": "node",

&nbsp;     "args": \["mcp\_server.js", "--verbose"]

&nbsp;   }

&nbsp; },

&nbsp; "telemetry": {

&nbsp;   "enabled": true,

&nbsp;   "target": "local",

&nbsp;   "otlpEndpoint": "http://localhost:4317",

&nbsp;   "logPrompts": true

&nbsp; },

&nbsp; "privacy": {

&nbsp;   "usageStatisticsEnabled": true

&nbsp; },

&nbsp; "model": {

&nbsp;   "name": "gemini-1.5-pro-latest",

&nbsp;   "maxSessionTurns": 10,

&nbsp;   "summarizeToolOutput": {

&nbsp;     "run\_shell\_command": {

&nbsp;       "tokenBudget": 100

&nbsp;     }

&nbsp;   }

&nbsp; },

&nbsp; "context": {

&nbsp;   "fileName": \["CONTEXT.md", "GEMINI.md"],

&nbsp;   "includeDirectories": \["path/to/dir1", "~/path/to/dir2", "../path/to/dir3"],

&nbsp;   "loadFromIncludeDirectories": true,

&nbsp;   "fileFiltering": {

&nbsp;     "respectGitIgnore": false

&nbsp;   }

&nbsp; },

&nbsp; "advanced": {

&nbsp;   "excludedEnvVars": \["DEBUG", "DEBUG\_MODE", "NODE\_ENV"]

&nbsp; }

}

```



\## Shell history



The CLI keeps a history of shell commands you run. To avoid conflicts between

different projects, this history is stored in a project-specific directory

within your user's home folder.



\- \*\*Location:\*\* `~/.gemini/tmp/<project\_hash>/shell\_history`

&nbsp; - `<project\_hash>` is a unique identifier generated from your project's root

&nbsp;   path.

&nbsp; - The history is stored in a file named `shell\_history`.



\## Environment variables and `.env` files



Environment variables are a common way to configure applications, especially for

sensitive information like API keys or for settings that might change between

environments. For authentication setup, see the

\[Authentication documentation](./authentication.md) which covers all available

authentication methods.



The CLI automatically loads environment variables from an `.env` file. The

loading order is:



1\.  `.env` file in the current working directory.

2\.  If not found, it searches upwards in parent directories until it finds an

&nbsp;   `.env` file or reaches the project root (identified by a `.git` folder) or

&nbsp;   the home directory.

3\.  If still not found, it looks for `~/.env` (in the user's home directory).



\*\*Environment variable exclusion:\*\* Some environment variables (like `DEBUG` and

`DEBUG\_MODE`) are automatically excluded from being loaded from project `.env`

files to prevent interference with gemini-cli behavior. Variables from

`.gemini/.env` files are never excluded. You can customize this behavior using

the `advanced.excludedEnvVars` setting in your `settings.json` file.



\- \*\*`GEMINI\_API\_KEY`\*\*:

&nbsp; - Your API key for the Gemini API.

&nbsp; - One of several available \[authentication methods](./authentication.md).

&nbsp; - Set this in your shell profile (e.g., `~/.bashrc`, `~/.zshrc`) or an `.env`

&nbsp;   file.

\- \*\*`GEMINI\_MODEL`\*\*:

&nbsp; - Specifies the default Gemini model to use.

&nbsp; - Overrides the hardcoded default

&nbsp; - Example: `export GEMINI\_MODEL="gemini-3-flash-preview"`

\- \*\*`GOOGLE\_API\_KEY`\*\*:

&nbsp; - Your Google Cloud API key.

&nbsp; - Required for using Vertex AI in express mode.

&nbsp; - Ensure you have the necessary permissions.

&nbsp; - Example: `export GOOGLE\_API\_KEY="YOUR\_GOOGLE\_API\_KEY"`.

\- \*\*`GOOGLE\_CLOUD\_PROJECT`\*\*:

&nbsp; - Your Google Cloud Project ID.

&nbsp; - Required for using Code Assist or Vertex AI.

&nbsp; - If using Vertex AI, ensure you have the necessary permissions in this

&nbsp;   project.

&nbsp; - \*\*Cloud Shell note:\*\* When running in a Cloud Shell environment, this

&nbsp;   variable defaults to a special project allocated for Cloud Shell users. If

&nbsp;   you have `GOOGLE\_CLOUD\_PROJECT` set in your global environment in Cloud

&nbsp;   Shell, it will be overridden by this default. To use a different project in

&nbsp;   Cloud Shell, you must define `GOOGLE\_CLOUD\_PROJECT` in a `.env` file.

&nbsp; - Example: `export GOOGLE\_CLOUD\_PROJECT="YOUR\_PROJECT\_ID"`.

\- \*\*`GOOGLE\_APPLICATION\_CREDENTIALS`\*\* (string):

&nbsp; - \*\*Description:\*\* The path to your Google Application Credentials JSON file.

&nbsp; - \*\*Example:\*\*

&nbsp;   `export GOOGLE\_APPLICATION\_CREDENTIALS="/path/to/your/credentials.json"`

\- \*\*`OTLP\_GOOGLE\_CLOUD\_PROJECT`\*\*:

&nbsp; - Your Google Cloud Project ID for Telemetry in Google Cloud

&nbsp; - Example: `export OTLP\_GOOGLE\_CLOUD\_PROJECT="YOUR\_PROJECT\_ID"`.

\- \*\*`GEMINI\_TELEMETRY\_ENABLED`\*\*:

&nbsp; - Set to `true` or `1` to enable telemetry. Any other value is treated as

&nbsp;   disabling it.

&nbsp; - Overrides the `telemetry.enabled` setting.

\- \*\*`GEMINI\_TELEMETRY\_TARGET`\*\*:

&nbsp; - Sets the telemetry target (`local` or `gcp`).

&nbsp; - Overrides the `telemetry.target` setting.

\- \*\*`GEMINI\_TELEMETRY\_OTLP\_ENDPOINT`\*\*:

&nbsp; - Sets the OTLP endpoint for telemetry.

&nbsp; - Overrides the `telemetry.otlpEndpoint` setting.

\- \*\*`GEMINI\_TELEMETRY\_OTLP\_PROTOCOL`\*\*:

&nbsp; - Sets the OTLP protocol (`grpc` or `http`).

&nbsp; - Overrides the `telemetry.otlpProtocol` setting.

\- \*\*`GEMINI\_TELEMETRY\_LOG\_PROMPTS`\*\*:

&nbsp; - Set to `true` or `1` to enable or disable logging of user prompts. Any other

&nbsp;   value is treated as disabling it.

&nbsp; - Overrides the `telemetry.logPrompts` setting.

\- \*\*`GEMINI\_TELEMETRY\_OUTFILE`\*\*:

&nbsp; - Sets the file path to write telemetry to when the target is `local`.

&nbsp; - Overrides the `telemetry.outfile` setting.

\- \*\*`GEMINI\_TELEMETRY\_USE\_COLLECTOR`\*\*:

&nbsp; - Set to `true` or `1` to enable or disable using an external OTLP collector.

&nbsp;   Any other value is treated as disabling it.

&nbsp; - Overrides the `telemetry.useCollector` setting.

\- \*\*`GOOGLE\_CLOUD\_LOCATION`\*\*:

&nbsp; - Your Google Cloud Project Location (e.g., us-central1).

&nbsp; - Required for using Vertex AI in non-express mode.

&nbsp; - Example: `export GOOGLE\_CLOUD\_LOCATION="YOUR\_PROJECT\_LOCATION"`.

\- \*\*`GEMINI\_SANDBOX`\*\*:

&nbsp; - Alternative to the `sandbox` setting in `settings.json`.

&nbsp; - Accepts `true`, `false`, `docker`, `podman`, or a custom command string.

\- \*\*`GEMINI\_SYSTEM\_MD`\*\*:

&nbsp; - Replaces the built‑in system prompt with content from a Markdown file.

&nbsp; - `true`/`1`: Use project default path `./.gemini/system.md`.

&nbsp; - Any other string: Treat as a path (relative/absolute supported, `~`

&nbsp;   expands).

&nbsp; - `false`/`0` or unset: Use the built‑in prompt. See

&nbsp;   \[System Prompt Override](../cli/system-prompt.md).

\- \*\*`GEMINI\_WRITE\_SYSTEM\_MD`\*\*:

&nbsp; - Writes the current built‑in system prompt to a file for review.

&nbsp; - `true`/`1`: Write to `./.gemini/system.md`. Otherwise treat the value as a

&nbsp;   path.

&nbsp; - Run the CLI once with this set to generate the file.

\- \*\*`SEATBELT\_PROFILE`\*\* (macOS specific):

&nbsp; - Switches the Seatbelt (`sandbox-exec`) profile on macOS.

&nbsp; - `permissive-open`: (Default) Restricts writes to the project folder (and a

&nbsp;   few other folders, see

&nbsp;   `packages/cli/src/utils/sandbox-macos-permissive-open.sb`) but allows other

&nbsp;   operations.

&nbsp; - `strict`: Uses a strict profile that declines operations by default.

&nbsp; - `<profile\_name>`: Uses a custom profile. To define a custom profile, create

&nbsp;   a file named `sandbox-macos-<profile\_name>.sb` in your project's `.gemini/`

&nbsp;   directory (e.g., `my-project/.gemini/sandbox-macos-custom.sb`).

\- \*\*`DEBUG` or `DEBUG\_MODE`\*\* (often used by underlying libraries or the CLI

&nbsp; itself):

&nbsp; - Set to `true` or `1` to enable verbose debug logging, which can be helpful

&nbsp;   for troubleshooting.

&nbsp; - \*\*Note:\*\* These variables are automatically excluded from project `.env`

&nbsp;   files by default to prevent interference with gemini-cli behavior. Use

&nbsp;   `.gemini/.env` files if you need to set these for gemini-cli specifically.

\- \*\*`NO\_COLOR`\*\*:

&nbsp; - Set to any value to disable all color output in the CLI.

\- \*\*`CLI\_TITLE`\*\*:

&nbsp; - Set to a string to customize the title of the CLI.

\- \*\*`CODE\_ASSIST\_ENDPOINT`\*\*:

&nbsp; - Specifies the endpoint for the code assist server.

&nbsp; - This is useful for development and testing.



\### Environment variable redaction



To prevent accidental leakage of sensitive information, Gemini CLI automatically

redacts potential secrets from environment variables when executing tools (such

as shell commands). This "best effort" redaction applies to variables inherited

from the system or loaded from `.env` files.



\*\*Default Redaction Rules:\*\*



\- \*\*By Name:\*\* Variables are redacted if their names contain sensitive terms

&nbsp; like `TOKEN`, `SECRET`, `PASSWORD`, `KEY`, `AUTH`, `CREDENTIAL`, `PRIVATE`, or

&nbsp; `CERT`.

\- \*\*By Value:\*\* Variables are redacted if their values match known secret

&nbsp; patterns, such as:

&nbsp; - Private keys (RSA, OpenSSH, PGP, etc.)

&nbsp; - Certificates

&nbsp; - URLs containing credentials

&nbsp; - API keys and tokens (GitHub, Google, AWS, Stripe, Slack, etc.)

\- \*\*Specific Blocklist:\*\* Certain variables like `CLIENT\_ID`, `DB\_URI`,

&nbsp; `DATABASE\_URL`, and `CONNECTION\_STRING` are always redacted by default.



\*\*Allowlist (Never Redacted):\*\*



\- Common system variables (e.g., `PATH`, `HOME`, `USER`, `SHELL`, `TERM`,

&nbsp; `LANG`).

\- Variables starting with `GEMINI\_CLI\_`.

\- GitHub Action specific variables.



\*\*Configuration:\*\*



You can customize this behavior in your `settings.json` file:



\- \*\*`security.allowedEnvironmentVariables`\*\*: A list of variable names to

&nbsp; \_never\_ redact, even if they match sensitive patterns.

\- \*\*`security.blockedEnvironmentVariables`\*\*: A list of variable names to

&nbsp; \_always\_ redact, even if they don't match sensitive patterns.



```json

{

&nbsp; "security": {

&nbsp;   "allowedEnvironmentVariables": \["MY\_PUBLIC\_KEY", "NOT\_A\_SECRET\_TOKEN"],

&nbsp;   "blockedEnvironmentVariables": \["INTERNAL\_IP\_ADDRESS"]

&nbsp; }

}

```



\## Command-line arguments



Arguments passed directly when running the CLI can override other configurations

for that specific session.



\- \*\*`--model <model\_name>`\*\* (\*\*`-m <model\_name>`\*\*):

&nbsp; - Specifies the Gemini model to use for this session.

&nbsp; - Example: `npm start -- --model gemini-3-pro-preview`

\- \*\*`--prompt <your\_prompt>`\*\* (\*\*`-p <your\_prompt>`\*\*):

&nbsp; - Used to pass a prompt directly to the command. This invokes Gemini CLI in a

&nbsp;   non-interactive mode.

&nbsp; - For scripting examples, use the `--output-format json` flag to get

&nbsp;   structured output.

\- \*\*`--prompt-interactive <your\_prompt>`\*\* (\*\*`-i <your\_prompt>`\*\*):

&nbsp; - Starts an interactive session with the provided prompt as the initial input.

&nbsp; - The prompt is processed within the interactive session, not before it.

&nbsp; - Cannot be used when piping input from stdin.

&nbsp; - Example: `gemini -i "explain this code"`

\- \*\*`--output-format <format>`\*\*:

&nbsp; - \*\*Description:\*\* Specifies the format of the CLI output for non-interactive

&nbsp;   mode.

&nbsp; - \*\*Values:\*\*

&nbsp;   - `text`: (Default) The standard human-readable output.

&nbsp;   - `json`: A machine-readable JSON output.

&nbsp;   - `stream-json`: A streaming JSON output that emits real-time events.

&nbsp; - \*\*Note:\*\* For structured output and scripting, use the

&nbsp;   `--output-format json` or `--output-format stream-json` flag.

\- \*\*`--sandbox`\*\* (\*\*`-s`\*\*):

&nbsp; - Enables sandbox mode for this session.

\- \*\*`--debug`\*\* (\*\*`-d`\*\*):

&nbsp; - Enables debug mode for this session, providing more verbose output. Open the

&nbsp;   debug console with F12 to see the additional logging.



\- \*\*`--help`\*\* (or \*\*`-h`\*\*):

&nbsp; - Displays help information about command-line arguments.

\- \*\*`--yolo`\*\*:

&nbsp; - Enables YOLO mode, which automatically approves all tool calls.

\- \*\*`--approval-mode <mode>`\*\*:

&nbsp; - Sets the approval mode for tool calls. Available modes:

&nbsp;   - `default`: Prompt for approval on each tool call (default behavior)

&nbsp;   - `auto\_edit`: Automatically approve edit tools (replace, write\_file) while

&nbsp;     prompting for others

&nbsp;   - `yolo`: Automatically approve all tool calls (equivalent to `--yolo`)

&nbsp;   - `plan`: Read-only mode for tool calls (requires experimental planning to

&nbsp;     be enabled).

&nbsp;     > \*\*Note:\*\* This mode is currently under development and not yet fully

&nbsp;     > functional.

&nbsp; - Cannot be used together with `--yolo`. Use `--approval-mode=yolo` instead of

&nbsp;   `--yolo` for the new unified approach.

&nbsp; - Example: `gemini --approval-mode auto\_edit`

\- \*\*`--allowed-tools <tool1,tool2,...>`\*\*:

&nbsp; - A comma-separated list of tool names that will bypass the confirmation

&nbsp;   dialog.

&nbsp; - Example: `gemini --allowed-tools "ShellTool(git status)"`

\- \*\*`--extensions <extension\_name ...>`\*\* (\*\*`-e <extension\_name ...>`\*\*):

&nbsp; - Specifies a list of extensions to use for the session. If not provided, all

&nbsp;   available extensions are used.

&nbsp; - Use the special term `gemini -e none` to disable all extensions.

&nbsp; - Example: `gemini -e my-extension -e my-other-extension`

\- \*\*`--list-extensions`\*\* (\*\*`-l`\*\*):

&nbsp; - Lists all available extensions and exits.

\- \*\*`--resume \[session\_id]`\*\* (\*\*`-r \[session\_id]`\*\*):

&nbsp; - Resume a previous chat session. Use "latest" for the most recent session,

&nbsp;   provide a session index number, or provide a full session UUID.

&nbsp; - If no session\_id is provided, defaults to "latest".

&nbsp; - Example: `gemini --resume 5` or `gemini --resume latest` or

&nbsp;   `gemini --resume a1b2c3d4-e5f6-7890-abcd-ef1234567890` or `gemini --resume`

&nbsp; - See \[Session Management](../cli/session-management.md) for more details.

\- \*\*`--list-sessions`\*\*:

&nbsp; - List all available chat sessions for the current project and exit.

&nbsp; - Shows session indices, dates, message counts, and preview of first user

&nbsp;   message.

&nbsp; - Example: `gemini --list-sessions`

\- \*\*`--delete-session <identifier>`\*\*:

&nbsp; - Delete a specific chat session by its index number or full session UUID.

&nbsp; - Use `--list-sessions` first to see available sessions, their indices, and

&nbsp;   UUIDs.

&nbsp; - Example: `gemini --delete-session 3` or

&nbsp;   `gemini --delete-session a1b2c3d4-e5f6-7890-abcd-ef1234567890`

\- \*\*`--include-directories <dir1,dir2,...>`\*\*:

&nbsp; - Includes additional directories in the workspace for multi-directory

&nbsp;   support.

&nbsp; - Can be specified multiple times or as comma-separated values.

&nbsp; - 5 directories can be added at maximum.

&nbsp; - Example: `--include-directories /path/to/project1,/path/to/project2` or

&nbsp;   `--include-directories /path/to/project1 --include-directories /path/to/project2`

\- \*\*`--screen-reader`\*\*:

&nbsp; - Enables screen reader mode, which adjusts the TUI for better compatibility

&nbsp;   with screen readers.

\- \*\*`--version`\*\*:

&nbsp; - Displays the version of the CLI.

\- \*\*`--experimental-acp`\*\*:

&nbsp; - Starts the agent in ACP mode.

\- \*\*`--allowed-mcp-server-names`\*\*:

&nbsp; - Allowed MCP server names.

\- \*\*`--fake-responses`\*\*:

&nbsp; - Path to a file with fake model responses for testing.

\- \*\*`--record-responses`\*\*:

&nbsp; - Path to a file to record model responses for testing.



\## Context files (hierarchical instructional context)



While not strictly configuration for the CLI's \_behavior\_, context files

(defaulting to `GEMINI.md` but configurable via the `context.fileName` setting)

are crucial for configuring the \_instructional context\_ (also referred to as

"memory") provided to the Gemini model. This powerful feature allows you to give

project-specific instructions, coding style guides, or any relevant background

information to the AI, making its responses more tailored and accurate to your

needs. The CLI includes UI elements, such as an indicator in the footer showing

the number of loaded context files, to keep you informed about the active

context.



\- \*\*Purpose:\*\* These Markdown files contain instructions, guidelines, or context

&nbsp; that you want the Gemini model to be aware of during your interactions. The

&nbsp; system is designed to manage this instructional context hierarchically.



\### Example context file content (e.g., `GEMINI.md`)



Here's a conceptual example of what a context file at the root of a TypeScript

project might contain:



```markdown

\# Project: My Awesome TypeScript Library



\## General Instructions:



\- When generating new TypeScript code, please follow the existing coding style.

\- Ensure all new functions and classes have JSDoc comments.

\- Prefer functional programming paradigms where appropriate.

\- All code should be compatible with TypeScript 5.0 and Node.js 20+.



\## Coding Style:



\- Use 2 spaces for indentation.

\- Interface names should be prefixed with `I` (e.g., `IUserService`).

\- Private class members should be prefixed with an underscore (`\_`).

\- Always use strict equality (`===` and `!==`).



\## Specific Component: `src/api/client.ts`



\- This file handles all outbound API requests.

\- When adding new API call functions, ensure they include robust error handling

&nbsp; and logging.

\- Use the existing `fetchWithRetry` utility for all GET requests.



\## Regarding Dependencies:



\- Avoid introducing new external dependencies unless absolutely necessary.

\- If a new dependency is required, please state the reason.

```



This example demonstrates how you can provide general project context, specific

coding conventions, and even notes about particular files or components. The

more relevant and precise your context files are, the better the AI can assist

you. Project-specific context files are highly encouraged to establish

conventions and context.



\- \*\*Hierarchical loading and precedence:\*\* The CLI implements a sophisticated

&nbsp; hierarchical memory system by loading context files (e.g., `GEMINI.md`) from

&nbsp; several locations. Content from files lower in this list (more specific)

&nbsp; typically overrides or supplements content from files higher up (more

&nbsp; general). The exact concatenation order and final context can be inspected

&nbsp; using the `/memory show` command. The typical loading order is:

&nbsp; 1.  \*\*Global context file:\*\*

&nbsp;     - Location: `~/.gemini/<configured-context-filename>` (e.g.,

&nbsp;       `~/.gemini/GEMINI.md` in your user home directory).

&nbsp;     - Scope: Provides default instructions for all your projects.

&nbsp; 2.  \*\*Project root and ancestors context files:\*\*

&nbsp;     - Location: The CLI searches for the configured context file in the

&nbsp;       current working directory and then in each parent directory up to either

&nbsp;       the project root (identified by a `.git` folder) or your home directory.

&nbsp;     - Scope: Provides context relevant to the entire project or a significant

&nbsp;       portion of it.

&nbsp; 3.  \*\*Sub-directory context files (contextual/local):\*\*

&nbsp;     - Location: The CLI also scans for the configured context file in

&nbsp;       subdirectories \_below\_ the current working directory (respecting common

&nbsp;       ignore patterns like `node\_modules`, `.git`, etc.). The breadth of this

&nbsp;       search is limited to 200 directories by default, but can be configured

&nbsp;       with the `context.discoveryMaxDirs` setting in your `settings.json`

&nbsp;       file.

&nbsp;     - Scope: Allows for highly specific instructions relevant to a particular

&nbsp;       component, module, or subsection of your project.

\- \*\*Concatenation and UI indication:\*\* The contents of all found context files

&nbsp; are concatenated (with separators indicating their origin and path) and

&nbsp; provided as part of the system prompt to the Gemini model. The CLI footer

&nbsp; displays the count of loaded context files, giving you a quick visual cue

&nbsp; about the active instructional context.

\- \*\*Importing content:\*\* You can modularize your context files by importing

&nbsp; other Markdown files using the `@path/to/file.md` syntax. For more details,

&nbsp; see the \[Memory Import Processor documentation](../core/memport.md).

\- \*\*Commands for memory management:\*\*

&nbsp; - Use `/memory refresh` to force a re-scan and reload of all context files

&nbsp;   from all configured locations. This updates the AI's instructional context.

&nbsp; - Use `/memory show` to display the combined instructional context currently

&nbsp;   loaded, allowing you to verify the hierarchy and content being used by the

&nbsp;   AI.

&nbsp; - See the \[Commands documentation](../cli/commands.md#memory) for full details

&nbsp;   on the `/memory` command and its sub-commands (`show` and `refresh`).



By understanding and utilizing these configuration layers and the hierarchical

nature of context files, you can effectively manage the AI's memory and tailor

the Gemini CLI's responses to your specific needs and projects.



\## Sandboxing



The Gemini CLI can execute potentially unsafe operations (like shell commands

and file modifications) within a sandboxed environment to protect your system.



Sandboxing is disabled by default, but you can enable it in a few ways:



\- Using `--sandbox` or `-s` flag.

\- Setting `GEMINI\_SANDBOX` environment variable.

\- Sandbox is enabled when using `--yolo` or `--approval-mode=yolo` by default.



By default, it uses a pre-built `gemini-cli-sandbox` Docker image.



For project-specific sandboxing needs, you can create a custom Dockerfile at

`.gemini/sandbox.Dockerfile` in your project's root directory. This Dockerfile

can be based on the base sandbox image:



```dockerfile

FROM gemini-cli-sandbox



\# Add your custom dependencies or configurations here

\# For example:

\# RUN apt-get update \&\& apt-get install -y some-package

\# COPY ./my-config /app/my-config

```



When `.gemini/sandbox.Dockerfile` exists, you can use `BUILD\_SANDBOX`

environment variable when running Gemini CLI to automatically build the custom

sandbox image:



```bash

BUILD\_SANDBOX=1 gemini -s

```



\## Usage statistics



To help us improve the Gemini CLI, we collect anonymized usage statistics. This

data helps us understand how the CLI is used, identify common issues, and

prioritize new features.



\*\*What we collect:\*\*



\- \*\*Tool calls:\*\* We log the names of the tools that are called, whether they

&nbsp; succeed or fail, and how long they take to execute. We do not collect the

&nbsp; arguments passed to the tools or any data returned by them.

\- \*\*API requests:\*\* We log the Gemini model used for each request, the duration

&nbsp; of the request, and whether it was successful. We do not collect the content

&nbsp; of the prompts or responses.

\- \*\*Session information:\*\* We collect information about the configuration of the

&nbsp; CLI, such as the enabled tools and the approval mode.



\*\*What we DON'T collect:\*\*



\- \*\*Personally identifiable information (PII):\*\* We do not collect any personal

&nbsp; information, such as your name, email address, or API keys.

\- \*\*Prompt and response content:\*\* We do not log the content of your prompts or

&nbsp; the responses from the Gemini model.

\- \*\*File content:\*\* We do not log the content of any files that are read or

&nbsp; written by the CLI.



\*\*How to opt out:\*\*



You can opt out of usage statistics collection at any time by setting the

`usageStatisticsEnabled` property to `false` under the `privacy` category in

your `settings.json` file:



```json

{

&nbsp; "privacy": {

&nbsp;   "usageStatisticsEnabled": false

&nbsp; }

}

```

