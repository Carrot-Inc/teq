// A value-position `if` or `match` whose branch that jumps first to the join is `null`, where an `Any`, an `AnyRef`
// or an `Object` is wanted: the join's frame takes the value at the node's type, not the `null` the branch left (the
// verifier refused the class whose other branch then brought a `String`); a `String` wanted, a `null` in the second
// branch and two classes are the controls.
trait Shape
class Square extends Shape { override def toString = "square" }

def any(x: Any): String = "any(" + x + ")"
def ref(x: AnyRef): String = "ref(" + x + ")"
def obj(x: Object): String = "obj(" + x + ")"
def str(x: String): String = "str(" + x + ")"
def shape(x: Shape): String = "shape(" + x + ")"

class Holder(n: Int, s: String):
  val field: Any = if n == 1 then null else s
  def argument: String = any(if n == 1 then null else s)
  def anyRef: String = ref(if n == 1 then null else s)
  def objectWanted: String = obj(if n == 1 then null else s)
  def local: String =
    val x: Any = if n == 1 then null else s
    any(x)
  def matched: String = any(n match { case 1 => null; case 2 => "two"; case _ => s })
  def chained: String = any(if n == 1 then null else if n == 2 then "two" else s)
  def nested: String = any(n match { case 1 | 2 => if n == 1 then null else "two"; case _ => s })
  def element: String =
    val a = new Array[Any](1)
    a(0) = if n == 1 then null else s
    String.valueOf(a(0))
  def equal: Boolean = (if n == 1 then null else s: Any) == s
  def secondNull: String = any(if n == 1 then s else null)
  def stringWanted: String = str(if n == 1 then null else s)
  def traitWanted: String = shape(if n == 1 then null else new Square)
  def twoClasses: String = any(if n == 1 then "a" else List(n))

@main def main(): Unit =
  for n <- 0 to 2 do
    val h = Holder(n, "s" + n)
    println(List(h.field, h.argument, h.anyRef, h.objectWanted, h.local, h.matched, h.chained, h.nested, h.element,
      h.equal, h.secondNull, h.stringWanted, h.traitWanted, h.twoClasses).mkString(" "))
