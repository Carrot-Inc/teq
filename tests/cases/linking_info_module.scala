//> using platform js
//> using jsModuleKind es
//> using jsVersion 1.21.0
// A single file that imports from a JavaScript module is an ES module, and Scala.js's
// LinkingInfo.moduleKind says so.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport
import scala.scalajs.LinkingInfo

@js.native
@JSImport("node:path", "basename")
def basename(p: String): String = js.native

@main def run(): Unit =
  println(LinkingInfo.moduleKind == LinkingInfo.ModuleKind.ESModule)
  println(LinkingInfo.linkTimeIf(LinkingInfo.moduleKind == LinkingInfo.ModuleKind.NoModule)("script")("module"))
  println(basename("/a/b.txt"))
