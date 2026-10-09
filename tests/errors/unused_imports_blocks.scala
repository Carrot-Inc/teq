// Imports in blocks and class bodies; the language import is never reported; `@unused`'s own
// import is used by the annotation.
import scala.annotation.unused
import scala.language.implicitConversions
object P { val p = 1; val q = 2 }
object Use {
  def f = {
    import P.p
    import P.q
    p
  }
  def g(@unused x: Int) = 0
}
class C {
  import P.q
  def h = 1
}

// teq: --werror --wunused imports
// expect: unused_imports_blocks.scala:9:14: warning: unused import
// expect: unused_imports_blocks.scala:15:12: warning: unused import
