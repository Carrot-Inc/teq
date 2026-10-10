// jars: munit munit-diff
//> using platform js
//> using jsVersion 1.22.0
//> using dep org.scalameta::munit::1.3.4
// A library body's cast to a JavaScript type, which Scala.js leaves untested: munit reads a file through
// node's `fs.readFileSync` and casts the Buffer it returns to `js.Array[Int]`, an alias of scala's
// `Array` in teq's std. munit finds `fs` through a global `require`, which node defines for the program
// it reads on stdin (Scala.js's NodeJSEnv) and not for a file it runs.
import munit.internal.io.PlatformIO.{Files, Paths}
import scala.scalajs.js

@main def run(): Unit =
  val global = js.Dynamic.global.globalThis
  if js.typeOf(global.require) == "undefined" then
    global.updateDynamic("require")(((m: String) => js.Dynamic.global.process.getBuiltinModule(m)): js.Function1[String, js.Dynamic])
  val lines = Files.readAllLines(Paths.get("tests/classpath/js/munit_read_file.scala"))
  println(lines.size())
  println(lines.get(0))
