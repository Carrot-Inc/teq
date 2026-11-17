// expect: 32:5: error: value += is not a member of Int
// expect: 35:5: error: value += is not a member of Int
// expect: 37:5: error: value += is not a member of Int
// expect: 39:5: error: value += is not a member of Int
// expect: 40:5: error: value += is not a member of Int
// expect: 5 errors found
// Where an assignment operator's application as written fails and its left side, typed again by itself, is no
// assignment's target, nothing stands in its place and the first failure is told: an `apply` that is an
// extension's, one with a using clause, a structural type's, arguments named out of the parameters' order (bound
// in a block), a method's result. scalac tells these five lines.
import scala.reflect.Selectable.reflectiveSelectable
object O:
  opaque type C = Array[Int]
  def make(): C = Array(10)
  extension (c: C)
    def apply(i: Int): Int = c(i)
    def update(i: Int, v: Int): Unit = c(i) = v
class U:
  def apply(i: Int)(using s: String): Int = 10
  def update(i: Int, v: Int)(using s: String): Unit = ()
class P:
  def apply(i: Int): Int = 10
  def update(i: Int, v: Int): Unit = ()
class N:
  def apply(i: Int, j: Int): Int = i + j
  def update(i: Int, j: Int, v: Int): Unit = ()
  def at(i: Int): Int = i
object Test:
  import O.*
  def main(args: Array[String]): Unit =
    val c = make()
    c(0) += 2
    given String = "given"
    val u = new U
    u(0) += 2
    val s: { def apply(i: Int): Int; def update(i: Int, v: Int): Unit } = new P
    s(0) += 2
    val n = new N
    n(j = n.at(2), i = n.at(1)) += 4
    n.at(0) += 1
