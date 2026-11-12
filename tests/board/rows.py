"""The rows of the board for dotty's own suites at the tag 3.8.4: one per directory under tests/ that dotty's test
sources compile or run there, a fixed list drawn from those sources. Each
row: the name of its list, the path under tests/, its kind, where dotty's sources name it, scalac's flags for it,
and how its tests are named:

  ("files-in-dir", path, include list, exclude list, flags)   compileFilesInDir: every source file and every
                                                              directory holding one, after the filter
                                                              (a file of compiler/test/dotc)
  ("file", path, flags[, name])                               compileFile
  ("dir", path, flags[, name])                                compileDir: the directory's files as one program
  ("list", name, [paths], flags)                              compileList
  ("chain", name, [steps])                                    compilations each into a directory of its own, a
                                                              step against the steps it names (`against`) and
                                                              the class path it names
  ("patmat", path, exclude list)                              the tag's PatmatExhaustivityTest: every .scala
                                                              file and every directory
  ("each", path) / ("entries", path) / ("units", path, names) the tests of a row no compiler runs from the
                                                              checkout, named as its own runner walks them

A row with `cannot` is listed test by test and never run: the reason is its class.

Left out, since they run the tests of a row once more under an option of scalac's own
machinery: the lazy-vals allowlists (CompilationTests.scala:48, :183), posTwice's files of tests/pos
(:104-136), pickling (:201-202), parallelBackend (:378-407), the idempotency of tests/pos, best-effort over
tests/neg, from TASTy (FromTastyTests.scala), and neg/i7575.scala a second time without language features
(:164; the default options name none).
"""

C = "compiler/test/dotty/tools/dotc/CompilationTests.scala"
B = "compiler/test/dotty/tools/dotc/BootstrappedOnlyCompilationTests.scala"
J = "sjs-compiler-tests/test/scala/dotty/tools/dotc/ScalaJSCompilationTests.scala"
T = "compiler/test/dotty/tools/"

# TestConfiguration.defaultOptions at the tag (compiler/test/dotty/tools/vulpix/TestConfiguration.scala:14-80)
D = ["-indent", "-Yno-deep-subtypes", "-Yno-double-bindings", "-Yforce-sbt-phases", "-Xsemanticdb", "-Xverify-signatures",
     "-pagewidth", "120", "-color:never", "-Xtarget", "17", "-Ycheck:all", "-Wconf:id=E222:s"]
DEEP = [f for f in D if f != "-Yno-deep-subtypes"]
NOYCHECK = [f for f in D if f != "-Ycheck:all"]
UNINDENT = ["-no-indent"] + D[1:]
CC = ["-language:experimental.captureChecking", "-language:experimental.separationChecking"]
NULLS = D + ["-Yexplicit-nulls"]

COMPILER = "the compiler as a library: the test is compiled against scalac's own classes"
STAGING = "the compiler as a library: scala.quoted.staging compiles at run time"
INSPECTOR = "the compiler as a library: the TASTy inspector"
PLUGIN = "scalac's plugin API"
DEBUGGER = "a JVM debugger session over scalac's expression compiler"
COVERAGE = "scalac's coverage instrumentation: the statements it records are the expectation"
PRINTER = "scalac's tree printer: the output of -Xprint is the expectation"
SEMANTICDB = "scalac's SemanticDB of the corpus printed by dotty's own tool is the expectation"
BESTEFFORT = "scalac's best-effort TASTy"
FRAMEWORK = "a test of dotty's test framework: the file must fail its kind"
SBT = "an sbt build"
BYTES = "a program that compares scalac's class files and TASTy of two compilations"


class Row:
    def __init__(self, name, path, kind, source, flags, entries, cannot=None, platform="jvm"):
        self.name, self.path, self.kind, self.source = name, path, kind, source
        self.flags, self.entries, self.cannot, self.platform = flags, entries, cannot, platform


def in_dir(path, flags=None, include=None, exclude=None):
    return ("files-in-dir", path, include, exclude, flags)


def rewrites():
    def f(name, *flags):
        return ("file", f"rewrites/{name}.scala", D + list(flags))

    def u(name, *flags):
        return ("file", f"rewrites/{name}.scala", UNINDENT + list(flags))
    return [
        f("rewrites", "-source", "3.0-migration", "-rewrite", "-indent"),
        f("rewrites3x", "-rewrite", "-source", "future-migration"),
        f("rewrites3x-fatal-warnings", "-rewrite", "-source", "future-migration", "-Werror"),
        f("i21394", "-rewrite", "-source", "future-migration"),
        f("uninitialized-var", "-rewrite", "-source", "future-migration"),
        f("with-type-operator", "-rewrite", "-source", "future-migration"),
        f("private-this", "-rewrite", "-source", "future-migration"),
        f("alphanumeric-infix-operator", "-rewrite", "-source", "future-migration"),
        f("filtering-fors", "-rewrite", "-source", "3.2-migration"),
        f("refutable-pattern-bindings-old", "-rewrite", "-source", "3.2-migration"),
        f("refutable-pattern-bindings", "-rewrite", "-source", "3.8-migration"),
        f("i8982", "-indent", "-rewrite"),
        f("i9632", "-indent", "-rewrite"),
        f("i11895", "-indent", "-rewrite"),
        u("i12340", "-rewrite"),
        u("i17187", "-rewrite"),
        u("i17399", "-rewrite"),
        f("i20002", "-indent", "-rewrite"),
        ("dir", "rewrites/annotation-named-pararamters", D + ["-rewrite", "-source:3.6-migration"]),
        u("i21418", "-rewrite", "-source:3.5-migration"),
        f("infix-named-args", "-rewrite", "-source:3.7-migration"),
        f("ambiguous-named-tuple-assignment", "-rewrite", "-source:3.6-migration"),
        f("i21382", "-indent", "-rewrite"),
        f("unused", "-rewrite", "-Wunused:all"),
        f("i22440", "-rewrite"),
        f("i22731", "-rewrite", "-source:3.7-migration"),
        f("i22731b", "-rewrite", "-source:3.7-migration"),
        f("implicit-to-given", "-rewrite", "-Yimplicit-to-given"),
        f("i22792", "-rewrite"),
        f("i23449", "-rewrite", "-source:3.4-migration"),
        f("i24103", "-rewrite", "-source:3.4-migration"),
        f("i24103b", "-rewrite", "-source:3.4-migration"),
        f("i24213", "-rewrite", "-source:3.4-migration"),
    ]


def rows():
    init = D + ["-Wsafe-init", "-Werror"]
    tasty_error = D + ["-Wsafe-init"]
    lint = D + ["-Wsafe-init", "-Wunused:all", "-Wshadow:private-shadow", "-Wshadow:type-parameter-shadow"]
    sp = ["-sourcepath", "tests/pos-special/sourcepath"]
    te = "init/tasty-error/"
    return [
        Row("pos", "pos", "pos", f"{C}:36-37", D + ["-Wsafe-init"],
            [in_dir("pos", lint, include="pos-linting.allowlist"), in_dir("pos", exclude="pos-linting.allowlist")]),
        Row("pos-deep-subtype", "pos-deep-subtype", "pos", f"{C}:38", DEEP, [in_dir("pos-deep-subtype")]),
        Row("pos-special-sourcepath", "pos-special/sourcepath/outer", "pos", f"{C}:39-40", D + sp,
            [in_dir("pos-special/sourcepath/outer"), ("file", "pos-special/sourcepath/outer/nested/Test4.scala", None)]),
        Row("pos-scala2", "pos-scala2", "pos", f"{C}:41", D + ["-source", "3.0-migration"], [in_dir("pos-scala2")]),
        Row("pos-custom-args-captures", "pos-custom-args/captures", "pos", f"{C}:42", D + CC, [in_dir("pos-custom-args/captures")]),
        Row("pos-special", "pos-special", "pos", f"{C}:43-46, :49, :52", D, [
            ("file", "pos-special/utf8encoded.scala", D + ["-encoding", "UTF8"]),
            ("file", "pos-special/utf16encoded.scala", D + ["-encoding", "UTF16"]),
            ("dir", "pos-special/i18589", NOYCHECK + ["-Wsafe-init"]),
            ("dir", "pos-special/i24547", NOYCHECK),
            ("dir", "pos-special/java-param-names", D),
            ("dir", "pos-special/stdlib", DEEP)]),
        Row("rewrites", "rewrites", "rewrite", f"{C}:63-95", D, rewrites()),
        Row("pos-java-interop", "pos-java-interop", "pos", f"{C}:102", D, [in_dir("pos-java-interop")]),
        Row("pos-java-interop-separate", "pos-java-interop-separate", "pos", f"{C}:103", D, [in_dir("pos-java-interop-separate")]),
        Row("warn", "warn", "warn", f"{C}:145", D, [in_dir("warn")]),
        Row("neg", "neg", "neg", f"{C}:155", D, [in_dir("neg", exclude="neg-scala2-library-tasty.excludelist")]),
        Row("neg-deep-subtype", "neg-deep-subtype", "neg", f"{C}:156", DEEP, [in_dir("neg-deep-subtype")]),
        Row("neg-custom-args-captures", "neg-custom-args/captures", "neg", f"{C}:157", D + CC + ["-source", "3.8"],
            [in_dir("neg-custom-args/captures")]),
        Row("neg-custom-args", "neg-custom-args", "neg", f"{C}:158-165", D, [
            ("file", "neg-custom-args/sourcepath/outer/nested/Test1.scala", D + ["-sourcepath", "tests/neg-custom-args/sourcepath"], "sourcepath/outer/nested/Test1.scala"),
            ("dir", "neg-custom-args/sourcepath2/hi", D + ["-sourcepath", "tests/neg-custom-args/sourcepath2", "-Werror"], "sourcepath2/hi"),
            ("list", "toplevel-samesource", ["neg-custom-args/toplevel-samesource/S.scala", "neg-custom-args/toplevel-samesource/nested/S.scala"], D),
            ("chain", "i20491/Test.scala", [{"files": ["neg-custom-args/i20491/Test.scala"], "classpath": ["neg-custom-args/i20491/cp"]}])]),
        Row("fuzzy", "fuzzy", "nocrash", f"{C}:171", D, [in_dir("fuzzy")]),
        Row("run", "run", "run", f"{C}:179", D + ["-Wsafe-init"], [in_dir("run")]),
        Row("run-deep-subtype", "run-deep-subtype", "run", f"{C}:180", DEEP, [in_dir("run-deep-subtype")]),
        Row("run-custom-args-captures", "run-custom-args/captures", "run", f"{C}:181", DEEP + CC + ["-source", "3.8"],
            [in_dir("run-custom-args/captures")]),
        Row("generic-java-signatures", "generic-java-signatures", "run", f"{C}:192", D, [in_dir("generic-java-signatures")]),
        Row("patmat", "patmat", "patmat", T + "dotc/transform/PatmatExhaustivityTest.scala:20, :65-84",
            ["-pagewidth", "80", "-color:never", "-Ystop-after:explicitSelf", "-Ycheck-constraint-deps"],
            [("patmat", "patmat", "patmat-exhaustivity-scala2-library-tasty.excludelist")]),
        Row("explicit-nulls-neg", "explicit-nulls/neg", "neg", f"{C}:219", NULLS,
            [in_dir("explicit-nulls/neg", exclude="neg-explicit-nulls-scala2-library-tasty.excludelist")]),
        Row("explicit-nulls-flexible-types-common-neg", "explicit-nulls/flexible-types-common", "neg", f"{C}:220",
            NULLS + ["-Yno-flexible-types"], [in_dir("explicit-nulls/flexible-types-common")]),
        Row("explicit-nulls-unsafe-common-neg", "explicit-nulls/unsafe-common", "neg", f"{C}:221", NULLS + ["-Yno-flexible-types"],
            [in_dir("explicit-nulls/unsafe-common", exclude="neg-explicit-nulls-scala2-library-tasty.excludelist")]),
        Row("explicit-nulls-pos", "explicit-nulls/pos", "pos", f"{C}:239", NULLS, [in_dir("explicit-nulls/pos")]),
        Row("explicit-nulls-flexible-types-common-pos", "explicit-nulls/flexible-types-common", "pos", f"{C}:240", NULLS,
            [in_dir("explicit-nulls/flexible-types-common")]),
        Row("explicit-nulls-unsafe-common-pos", "explicit-nulls/unsafe-common", "pos", f"{C}:241",
            NULLS + ["-language:unsafeNulls", "-Yno-flexible-types"], [in_dir("explicit-nulls/unsafe-common")]),
        Row("explicit-nulls-warn", "explicit-nulls/warn", "warn", f"{C}:258", NULLS, [in_dir("explicit-nulls/warn")]),
        Row("explicit-nulls-run", "explicit-nulls/run", "run", f"{C}:264", NULLS, [in_dir("explicit-nulls/run")]),
        Row("init-global-warn", "init-global/warn", "warn", f"{C}:272", D + ["-Ysafe-init-global"],
            [in_dir("init-global/warn", exclude="neg-init-global-scala2-library-tasty.excludelist")]),
        Row("init-global-pos", "init-global/pos", "pos", f"{C}:273", D + ["-Ysafe-init-global", "-Werror"],
            [in_dir("init-global/pos", exclude="pos-init-global-scala2-library-tasty.excludelist")]),
        Row("init-global-special", "init-global/special/tastySource", "warn", f"{C}:279-291", D + ["-Ysafe-init-global"], [
            ("chain", "tastySource", [
                {"files": ["init-global/special/tastySource/A.scala"], "flags": D + ["-Ysafe-init-global", "-sourceroot", "tests/init-global/special"]},
                {"files": ["init-global/special/tastySource/B.scala"], "against": [0]}])]),
        Row("init-neg", "init/neg", "neg", f"{C}:298", init, [in_dir("init/neg")]),
        Row("init-warn", "init/warn", "warn", f"{C}:299", D + ["-Wsafe-init"], [in_dir("init/warn")]),
        Row("init-pos", "init/pos", "pos", f"{C}:301", init, [in_dir("init/pos")]),
        Row("init-crash", "init/crash", "pos", f"{C}:303", D + ["-Wsafe-init"], [in_dir("init/crash")]),
        Row("init-special", "init/special/i12128", "pos", f"{C}:311-324", D + ["-Wsafe-init"], [
            ("chain", "i12128", [
                {"files": ["init/special/i12128/Reflect_1.scala"]},
                {"files": ["init/special/i12128/Macro_2.scala"], "against": [0]},
                {"files": ["init/special/i12128/Test_3.scala"], "against": [1], "flags": init}])]),
        Row("init-tasty-error", "init/tasty-error", "neg", f"{C}:330-374", tasty_error, [
            ("chain", "val-or-defdef", [
                {"files": [te + "val-or-defdef/v1/A.scala"]},
                {"files": [te + "val-or-defdef/v1/B.scala"], "against": [0]},
                {"files": [te + "val-or-defdef/v0/A.scala"]},
                {"files": [te + "val-or-defdef/Main.scala"], "against": [2, 1]}]),
            ("chain", "typedef", [
                {"files": [te + "typedef/C.scala"]},
                {"files": [te + "typedef/v1/A.scala"], "against": [0]},
                {"files": [te + "typedef/v1/B.scala"], "against": [0, 1]},
                {"files": [te + "typedef/v0/A.scala"], "against": [0]},
                {"files": [te + "typedef/Main.scala"], "against": [0, 3, 2]}])]),
        Row("bench", "bench", "pos", f"{B}:28", DEEP, [in_dir("bench")]),
        Row("pos-macros", "pos-macros", "pos", f"{B}:29", D + ["-Xcheck-macros"], [in_dir("pos-macros")]),
        Row("pos-with-compiler", "pos-with-compiler", "pos", f"{B}:36", D, [in_dir("pos-with-compiler")], COMPILER),
        Row("pos-staging", "pos-staging", "pos", f"{B}:37", D, [in_dir("pos-staging")], STAGING),
        Row("neg-macros", "neg-macros", "neg", f"{B}:98", D + ["-Xcheck-macros"], [in_dir("neg-macros")]),
        Row("neg-with-compiler", "neg-with-compiler", "neg", f"{B}:105", D, [in_dir("neg-with-compiler")], COMPILER),
        Row("neg-staging", "neg-staging", "neg", f"{B}:106", D, [in_dir("neg-staging")], STAGING),
        Row("run-macros", "run-macros", "run", f"{B}:114", D + ["-Xcheck-macros"],
            [in_dir("run-macros", exclude="run-macros-scala2-library-tasty.excludelist")]),
        Row("run-with-compiler", "run-with-compiler", "run", f"{B}:121", D, [in_dir("run-with-compiler")], COMPILER),
        Row("run-staging", "run-staging", "run", f"{B}:122", D, [in_dir("run-staging")], STAGING),
        Row("run-tasty-inspector", "run-tasty-inspector", "run", f"{B}:123", D, [in_dir("run-tasty-inspector")], INSPECTOR),
        Row("old-tasty-interpreter-prototype", "old-tasty-interpreter-prototype", "run", f"{B}:127", D,
            [("dir", "old-tasty-interpreter-prototype", None)], INSPECTOR),
        Row("run-bootstrapped", "run-bootstrapped", "run", f"{B}:146", D, [in_dir("run-bootstrapped")], COMPILER),
        Row("plugins-neg", "plugins/neg", "neg", f"{B}:206", D, [in_dir("plugins/neg")], PLUGIN),
        Row("plugins-custom", "plugins/custom/analyzer", "pos", f"{B}:207", D, [("dir", "plugins/custom/analyzer", None)], PLUGIN),
        Row("plugins-run", "plugins/run", "run", f"{B}:208", D, [in_dir("plugins/run")], PLUGIN),
        # -scalajs is the row's target, JavaScript, which `teq compiler check` types for without a flag
        Row("neg-scalajs", "neg-scalajs", "neg", f"{J}:25", D, [in_dir("neg-scalajs")], platform="scala-js"),
        Row("run-scalajs", "run", "run-js", f"{J}:32", D, [in_dir("run")], platform="scala-js"),
        Row("debug", "debug", "run", T + "debug/DebugTests.scala:21", D, [in_dir("debug")], DEBUGGER),
        Row("debug-custom-args", "debug-custom-args", "run", T + "debug/DebugTests.scala:20", NULLS,
            [("file", "debug-custom-args/eval-explicit-nulls.scala", None)], DEBUGGER),
        Row("coverage-pos", "coverage/pos", "pos", T + "dotc/coverage/CoverageTests.scala:33", D, [("entries", "coverage/pos")], COVERAGE),
        Row("coverage-run", "coverage/run", "run", T + "dotc/coverage/CoverageTests.scala:37", D, [("entries", "coverage/run")], COVERAGE),
        Row("printing", "printing", "pos", T + "dotc/printing/PrintingTest.scala:71", D, [("each", "printing")], PRINTER),
        Row("printing-posttyper", "printing/posttyper", "pos", T + "dotc/printing/PrintingTest.scala:74", D, [("each", "printing/posttyper")], PRINTER),
        Row("printing-untyped", "printing/untyped", "pos", T + "dotc/printing/PrintingTest.scala:77", D, [("each", "printing/untyped")], PRINTER),
        Row("printing-transformed", "printing/transformed", "pos", T + "dotc/printing/PrintingTest.scala:80", D, [("each", "printing/transformed")], PRINTER),
        Row("printing-getters", "printing/getters", "pos", T + "dotc/printing/PrintingTest.scala:83", D, [("each", "printing/getters")], PRINTER),
        Row("printing-inlining", "printing/inlining", "pos", T + "dotc/printing/PrintingTest.scala:86", D, [("each", "printing/inlining")], PRINTER),
        Row("semanticdb", "semanticdb", "pos", T + "dotc/semanticdb/SemanticdbTests.scala:57-63", D, [("entries", "semanticdb/expect")], SEMANTICDB),
        Row("best-effort", "best-effort", "nocrash", T + "dotc/BestEffortOptionsTests.scala:39", D, [("entries", "best-effort")], BESTEFFORT),
        Row("vulpix-tests-meta-pos", "vulpix-tests/meta/pos", "pos", T + "vulpix/VulpixMetaTests.scala:20", D, [in_dir("vulpix-tests/meta/pos")], FRAMEWORK),
        Row("vulpix-tests-meta-neg", "vulpix-tests/meta/neg", "neg", T + "vulpix/VulpixMetaTests.scala:21", D, [in_dir("vulpix-tests/meta/neg")], FRAMEWORK),
        Row("vulpix-tests-meta-run", "vulpix-tests/meta/run", "run", T + "vulpix/VulpixMetaTests.scala:22", D, [in_dir("vulpix-tests/meta/run")], FRAMEWORK),
        Row("vulpix-tests-unit", "vulpix-tests/unit", "pos", T + "vulpix/VulpixUnitTests.scala:17-91", D, [("each", "vulpix-tests/unit")], FRAMEWORK),
        Row("sjs-junit", "sjs-junit", "run", "project/Build.scala:2194", [],
            [("units", "sjs-junit", ["test", "test-require-async-await", "test-require-multi-modules"])], SBT + " with Scala.js's JUnit runner"),
        Row("cmdTest-sbt-tests", "cmdTest-sbt-tests", "run", "project/scripts/bootstrappedOnlyCmdTests:125, :137", [],
            [("units", "cmdTest-sbt-tests", ["sourcepath-with-inline", "sourcepath-with-inline-api-hash"])], SBT + " driven by a script"),
        Row("idempotency", "idempotency", "run", T + "dotc/IdempotencyTests.scala:52-60", D,
            [("units", "idempotency", ["CheckOrderIdempotency", "CheckPosIdempotency"])], BYTES),
        Row("order-idempotency", "order-idempotency", "pos", T + "dotc/IdempotencyTests.scala:40-46", D,
            [("units", "order-idempotency", ["listTest"])], BYTES),
    ]
