// expect: 14:5: error: value foo_= is not a member of E
// expect: 16:5: error: value foo_= is not a member of E
// expect: 2 errors found
// An assignment operator's name starts with an operator character (`NameOps.isOpAssignmentName`): `foo_=` is the
// name of a member, and nothing is assigned through `foo_`. scalac tells this one line (E008).
class E:
  def foo_(n: Int): E = { println("foo_"); this }
class C:
  def apply(i: Int): E = new E
  def update(i: Int, e: E): Unit = println("update")
object Test:
  def main(args: Array[String]): Unit =
    val c = new C
    c(0) foo_= 1
    var e = new E
    e foo_= 1
