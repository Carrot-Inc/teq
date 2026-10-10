package dev.teq.sbt

import java.nio.file.Files

import org.scalajs.jsenv.nodejs.NodeJSEnv

class TestJsSuite extends munit.FunSuite {
  private def root(files: String*) = {
    val dir = Files.createTempDirectory("teq-testjs").toFile
    files.foreach(f => Files.write(dir.toPath.resolve(f), "{}".getBytes("UTF-8")))
    dir
  }

  private def envOf(js: TestJs): Map[String, Json.Value] = js.fields.get("jsEnv") match {
    case Some(Json.Obj(env)) => env
    case other => fail(s"no jsEnv: $other")
  }

  test("a NodeJSEnv's executable, arguments, variables and source maps, read from its configuration") {
    val env = new NodeJSEnv(NodeJSEnv.Config().withArgs(List("--max-old-space-size=300")).withEnv(Map("A" -> "1")).withSourceMap(false))
    val js = TestJs.of("CommonJSModule", env, root(), Nil)
    assertEquals(js.refusals, Nil)
    assertEquals(js.fields.get("moduleKind"), Some(Json.Str("CommonJSModule")))
    val fields = envOf(js)
    assertEquals(fields("kind"), Json.Str("node"))
    assertEquals(fields("executable"), Json.Str("node"))
    assertEquals(fields("args"), Json.Arr(Seq(Json.Str("--max-old-space-size=300"))))
    assertEquals(fields("env"), Json.Obj(Map("A" -> Json.Str("1"))))
    assertEquals(fields("sourceMap"), Json.Str("disable"))
    assertEquals(envOf(TestJs.of("NoModule", new NodeJSEnv(), root(), Nil))("sourceMap"), Json.Str("enableIfAvailable"))
  }

  test("another environment is recorded by its kind, which teq test refuses, and earlier refusals kept") {
    val other = TestJs.of("NoModule", new Object, root(), Seq("earlier"))
    assertEquals(other.refusals, Seq("earlier"))
    assertEquals(envOf(other)("kind"), Json.Str("object"))
  }

  test("a JSDOMNodeJSEnv's configuration, jsdom installed by the root's lockfile") {
    val env = new org.scalajs.jsenv.jsdomnodejs.JSDOMNodeJSEnv(new org.scalajs.jsenv.jsdomnodejs.JSDOMNodeJSEnv.Config("node", Nil, Map.empty))
    val js = TestJs.of("NoModule", env, root("package-lock.json"), Nil)
    assertEquals(js.refusals, Nil)
    val fields = envOf(js)
    assertEquals(fields("kind"), Json.Str("jsdom"))
    assertEquals(fields("packageRoot"), Json.Str("."))
    assertEquals(fields("lockfile"), Json.Str("package-lock.json"))
    // No lockfile at the root: none recorded, which teq test refuses.
    assertEquals(envOf(TestJs.of("NoModule", env, root(), Nil)).get("lockfile"), None)
  }

  test("the inputs a build adds before the test link are refusals, sbt-scalajs's input of the link none") {
    assertEquals(TestJs.inputRefusals(Nil, Nil), Nil)
    assert(TestJs.fromScalaJS("ScalaJSPluginInternal.scala"))
    assert(TestJs.fromScalaJS("(org.scalajs.sbtplugin.ScalaJSPluginInternal.scalaJSConfigSettings) ScalaJSPluginInternal.scala"))
    assert(!TestJs.fromScalaJS("/work/build.sbt"))
    assertEquals(TestJs.inputRefusals(Seq("the script setup.js"), Nil), Seq("jsEnvInput loads the script setup.js before the test link, which teq's run does not"))
    assertEquals(TestJs.inputRefusals(Nil, Seq("build.sbt:12")), Seq("Test / jsEnvInput is set at build.sbt:12 beside sbt-scalajs's input of the link: what it adds loads before the test link, which teq's run does not"))
  }
}
