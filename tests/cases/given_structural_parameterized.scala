// A structural given with parameters is a class beside its given method, which answers an
// instance of the class (dotty's `Parsers.givenDef` `TypeDef` and the desugaring's
// `def t(using n: Int): t = new t(using n)`): the members its body adds are the result's, in
// 3.6's syntax and in the old `with` form.
trait Parent(val count: Int)
trait Show[A]:
  def show(a: A): String
object New:
  given t: (n: Int) => Parent(n):
    def extra = n + 1
object Old:
  given t(using n: Int): Parent(n) with
    def extra = n + 2
object Generic:
  given showAll: [A] => (k: Int) => Show[A]:
    def show(a: A) = s"$a#$k"
    def tag = s"k$k"
@main def run(): Unit =
  given Int = 3
  println(New.t.extra + " " + New.t.count)
  println(Old.t.extra + " " + Old.t.count)
  println(Generic.showAll[String].show("x") + " " + Generic.showAll[Int].tag)
