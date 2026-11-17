// expect: 15:5: error: member-late-error
// expect: 1 error found
// An inline operator whose expansion fails is an error of a later phase than the application's typing: the
// first try of the assignment operator's application stands, and the assignment form (which would build here) is
// not tried. scalac tells this one line.
class E:
  inline def +=(n: Int): Unit = scala.compiletime.error("member-late-error")
  def +(n: Int): E = { println("plus"); this }
class C:
  def apply(i: Int): E = new E
  def update(i: Int, e: E): Unit = println("update")
object Test:
  def main(args: Array[String]): Unit =
    val c = new C
    c(0) += 1
