// A type pattern `case x: C[_]` binds `x` to `C[?]`, each wildcard within the parameter's
// declared bounds: a member returning the parameter is seen at that bound, a member taking
// it accepts what the same binder gives, and the binder passes where a `C[?]` is expected.
trait Foo:
  def bar: Int
class F1(n: Int) extends Foo:
  def bar = n
  override def toString = "F1(" + n + ")"
class Box[D <: Foo](val d: D):
  def get: D = d
  def put(x: D): Int = x.bar + get.bar
  def twice(other: Box[D]): Int = other.get.bar * 2
class Pair[K, V](val k: K, val v: V):
  def key: K = k
  def value: V = v
  def swap: Pair[V, K] = new Pair(v, k)
class Wide[A, B <: Foo](val a: A, val b: B):
  def both: (A, B) = (a, b)

object Main:
  def take(b: Box[?]): Int = b.get.bar
  def takePair(p: Pair[?, ?]): String = p.key.toString + "/" + p.value
  def show(x: Any): String = x match
    case b: Box[_] => "box " + b.get.bar + " " + take(b) + " " + b.put(b.get) + " " + b.twice(b)
    case p: Pair[_, _] => "pair " + p.key + " " + p.value + " " + takePair(p) + " " + p.swap.key
    case w: Wide[_, _] => "wide " + w.a + " " + w.b.bar + " " + w.both
    case _ => "other"
  def main(args: Array[String]): Unit =
    println(show(new Box(new F1(7))))
    println(show(new Pair("a", 1)))
    println(show(new Wide(2.5, new F1(3))))
    println(show(3))
    val boxes: List[Box[?]] = List(new Box(new F1(1)), new Box(new F1(2)))
    println(boxes.map(b => b.get.bar + take(b)).sum)
