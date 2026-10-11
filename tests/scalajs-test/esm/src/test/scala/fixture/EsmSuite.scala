package fixture

import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("node:path", JSImport.Namespace)
object NodePath extends js.Object:
  def basename(path: String): String = js.native

class EsmSuite extends munit.FunSuite:
  test("an imported module") { assertEquals(NodePath.basename("/a/b.txt"), "b.txt") }
