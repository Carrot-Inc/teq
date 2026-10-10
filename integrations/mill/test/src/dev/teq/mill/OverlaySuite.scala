package dev.teq.mill

import dev.teq.sbt.TeqCommand

class OverlaySuite extends munit.FunSuite:
  test("an overlay's fields, the defaults where it says nothing") {
    val overlay = Overlay.parse("""{"buildTool": true, "teq": "0.1.8", "inputs": [".tyrian-version"],
      |"modules": {"app": {"generators": [{"run": ["sh", "gen.sh"], "inputs": ["labels.txt"]}], "runAliases": {"main": "app.Main"}, "test": "app.it"}}}""".stripMargin)
    val o = overlay.fold(fail(_), identity)
    assertEquals((o.buildTool, o.teq, o.inputs, o.allowsSnapshots), (true, Some("0.1.8"), Seq(".tyrian-version"), false))
    assertEquals(o.module("app").generators, Seq(TeqCommand(Seq("sh", "gen.sh"), Seq("labels.txt"))))
    assertEquals((o.module("app").runAliases, o.module("app").test), (Map("main" -> "app.Main"), Some("app.it")))
    assertEquals(o.module("other"), Overlay.Module())
    assertEquals(Overlay.parse("{}").map(_.allowsSnapshots), Right(true))
  }

  test("what is no overlay, every problem named") {
    assertEquals(
      Overlay.parse("""{"buildTool": "yes", "modules": {"app": {"generators": [{"inputs": []}], "mainclasses": []}}, "extra": 1}"""),
      Left("the overlay has no field extra (it takes buildTool, inputs, modules, releases, snapshots, teq); modules.app has no field mainclasses (it takes devCommand, generators, mainClasses, runAliases, test); modules.app.generators[0] has no run; buildTool is not true or false")
    )
  }

  test("an input is a path relative to the build's root") {
    assertEquals(Overlay.parse("""{"inputs": ["/etc/hosts", "ok.txt"]}"""), Left("inputs names /etc/hosts, which is no path relative to the build's root"))
  }
