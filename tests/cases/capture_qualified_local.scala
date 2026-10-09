// A local named like a definition of another package that its block reads through the package
// (`p.x()`, written `x()`) is named apart from it, also where the read comes first in the
// block, which a `const` declared later in it would hold in its temporal dead zone.
package p:
  def x(): Int = 7
  def y(): Int = 8

package q:
  @main def main(): Unit =
    val x = p.x()
    println(x)
    println(p.y())
    val y = 1
    println(y)
