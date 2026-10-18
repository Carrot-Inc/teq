# The runs of the fixture by tests/interp-script.sh, in order over one home directory (`HOME` in an argument):
# `fixture <name> <script> <argument>...` against expected/<name>.expected (its stdout) and, where its status is
# not 0, expected/<name>.status and expected/<name>.stderr (the first line of its stderr), which Scala CLI 1.17.1's
# run of the script under scalac 3.8.4 wrote (`--record`); `refusal <name> <script> <argument>...` against
# expected/<name>.status and expected/<name>.stderr alone, teq's own refusal of a script's `//> using dep`
# (`src/scriptdeps.rs`), the repository's path written `<repo>`, which no Scala CLI run records; teq's own options
# (`--classpath`) follow the script.
fixture usage fixture-env.sc
fixture init fixture-env.sc init HOME
fixture add-api fixture-env.sc add HOME api 20000
fixture add-web fixture-env.sc add HOME web 20100
fixture list fixture-env.sc list HOME
fixture log fixture-env.sc log HOME
fixture probe fixture-env.sc probe
fixture race fixture-env.sc race
fixture teardown fixture-env.sc teardown HOME
fixture evicted evicted.sc
refusal missing-dep missing-dep.sc
refusal missing-dep-classpath missing-dep.sc --classpath ""
refusal wrong-version-classpath wrong-version.sc --classpath "$M2C/dev/zio/zio_3/2.1.25/zio_3-2.1.25.jar"
refusal missing-dep-platform-js platform-js.sc
refusal wrong-version wrong-version.sc
refusal wrong-version-tab wrong-version-tab.sc
refusal wrong-version-backtick wrong-version-backtick.sc
refusal ambiguous ambiguous.sc
refusal no-sources no-sources.sc
fixture no-sources-module no-sources.sc --module tools
products product-closure product-closure.sc shared/src
products product-source product-source.sc epsilon/src=-source:3.3
excluded excluded-typed name product-closure.sc shared/src shared/Broken.scala
excluded excluded-directive name product-closure.sc shared/src shared/Excluded.scala
excluded excluded-typed-dot dot product-closure.sc shared/src shared/Broken.scala
excluded excluded-directive-dot dot product-closure.sc shared/src shared/Excluded.scala
refusal product-wrong-version product-wrong-version.sc
fixture classifier-unquoted classifier-unquoted.sc --module tools
fixture classifier-quoted classifier-quoted.sc --module tools
refusal dep-parameter dep-parameter.sc --module tools
fixture dep-unicode-escape dep-unicode-escape.sc --module tools
refusal dep-unicode-invalid dep-unicode-invalid.sc --module tools
refusal lib-version lib-version.sc
refusal compile-only-version compile-only-version.sc
refusal key-empty key-empty.sc --module tools
refusal key-unclosed key-unclosed.sc --module tools
refusal key-quoted key-quoted.sc --module tools
refusal key-missing key-missing.sc --module tools
refusal source-after-lexer source-after-lexer.sc --module tools
fixture block-comment-code block-comment-code.sc --module tools
refusal outside OUTSIDE
refusal outside-classpath OUTSIDE --classpath ""
refusal no-sources-classpath no-sources.sc --classpath ""
refusal ambiguous-classpath ambiguous.sc --classpath ""
