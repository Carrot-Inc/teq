package dev.teq.mill

/** The inputs mill-teq records, as teq's reader finds them (`task::export::mill_definition_files`, whose test
  * `a_mill_builds_inputs_and_their_changes` holds the same tree). */
class InputsSuite extends munit.FunSuite:
  test("a Mill build's definition files as Mill's FileImportGraph.walkBuildFiles finds its scripts") {
    val root = os.temp.dir()
    for (f, t) <- Seq(
        "build.mill" -> "//| mill-version: 1.0.6", "helper.mill" -> "x", "build.sbt" -> "not this build's",
        "core/package.mill" -> "x", "core/Deps.mill" -> "x", "core/js/package.mill" -> "x", "core/plain/Unreached.mill" -> "x",
        "docs/nested/package.mill" -> "x", "out/core/package.mill" -> "x", "mill-build/build.mill" -> "x",
        "mill-build/src/Plugin.scala" -> "x", "mill-build/.bsp/x.json" -> "{}", ".mill-version" -> "1.0.6",
        ".config/mill-jvm-version" -> "17", "teq.mill.json" -> "{}", ".tyrian-version" -> "0.14.0", "notes.txt" -> "n",
      )
    do os.write(root / os.RelPath(f), t, createFolders = true)
    val inputs = Inputs.of(root, Seq(".tyrian-version", "absent.txt"))
    assertEquals(inputs.files.map(_._1), Seq(".config/mill-jvm-version", ".mill-version", ".tyrian-version", "build.mill", "core/Deps.mill",
      "core/js/package.mill", "core/package.mill", "helper.mill", "mill-build/build.mill", "mill-build/src/Plugin.scala", "teq.mill.json"))
    assertEquals(inputs.declared, Seq(".tyrian-version", "absent.txt"))
    os.remove(root / "build.mill")
    assertEquals(Inputs.definitionFiles(root), Nil)
  }

  test("Mill 1.1's YAML scripts as well as 1.0's, and a build.mill.scala build's own (teq's a_mill_builds_yaml_and_scala_scripts)") {
    val root = os.temp.dir()
    for f <- Seq("build.mill.yaml", "helper.mill.yaml", "a/package.mill.yaml", "a/Deps.mill", "a/b/package.mill", "c/package.mill.scala", "d/stray.sc", "x.sc") do
      os.write(root / os.RelPath(f), "x", createFolders = true)
    assertEquals(Inputs.definitionFiles(root).map(_._1), Seq("a/Deps.mill", "a/b/package.mill", "a/package.mill.yaml", "build.mill.yaml", "helper.mill.yaml"))
    os.remove(root / "build.mill.yaml")
    os.write(root / "build.mill.scala", "x")
    assertEquals(Inputs.definitionFiles(root).map(_._1), Seq("build.mill.scala", "c/package.mill.scala"))
  }

  test("a build that allows nested build files, by its header or its YAML") {
    val root = os.temp.dir()
    os.write(root / "build.mill", "//| mill-version: 1.1.10\n//| mill-allow-nested-build-mill: true\npackage build\n")
    assert(Inputs.allowsNestedBuilds(root))
    os.write.over(root / "build.mill", "//| mill-version: 1.1.10\npackage build\n// mill-allow-nested-build-mill: true\n")
    assert(!Inputs.allowsNestedBuilds(root))
    os.write(root / "build.mill.yaml", "mill-version: 1.1.10\nmill-allow-nested-build-mill: true\n")
    assert(Inputs.allowsNestedBuilds(root))
  }

  test("the nested-build header read as Mill reads it: YAML, its comments, quotes and flow style, a `#!` line before it") {
    val root = os.temp.dir()
    def allows(header: String) =
      os.write.over(root / "build.mill", header + "package build\n")
      Inputs.allowsNestedBuilds(root)
    assert(allows("//| mill-allow-nested-build-mill: true # nested packages\n"))
    assert(allows("#!/usr/bin/env mill\n//| mill-version: 1.1.10\n//| \"mill-allow-nested-build-mill\": \"true\"\n"))
    assert(allows("//| {mill-version: 1.1.10, mill-allow-nested-build-mill: True}\n"))
    assert(allows("//| mill-allow-nested-build-mill:\n//|   true\n"))
    assert(!allows("//| mill-allow-nested-build-mill: false # not yet\n"))
    assert(!allows("//| mill-version: 1.1.10 # mill-allow-nested-build-mill: true\n"))
    os.write(root / "build.mill.yaml", "mill-allow-nested-build-mill: true # nested packages\n")
    assert(Inputs.allowsNestedBuilds(root))
  }
