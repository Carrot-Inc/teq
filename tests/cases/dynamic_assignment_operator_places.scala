// An assignment operator on a value whose class derives from `scala.Dynamic` is the application as written first,
// whatever the left side is: `l op= r` is `l.applyDynamic("op=")(r)` where the class's dynamic method takes the
// call, through a getter with a setter (a member's or an extension's, selected or bare), an array's element, an
// `apply` and `update` pair and a mutable map as on a `var`. Where the dynamic method does not take the call
// (`Strict`'s takes a `String`), the assignment `l = l op r` by the class's own operator is what is left.
import scala.language.dynamics
import scala.collection.mutable

class Shown(val text: String):
  def :=(v: Int): String = text + ":=" + v
class Typed extends Dynamic:
  def applyDynamic(name: String)(args: Any*): Shown = new Shown(name + args.mkString("(", ",", ")"))

class Counter(val n: Int) extends Dynamic:
  def +(k: Int): Counter = { println(s"plus($k)"); new Counter(n + k) }
  def applyDynamic(name: String)(args: Any*): Unit = println(s"apply($name)(${args.mkString(",")})")

class Strict(val n: Int) extends Dynamic:
  def +(k: Int): Strict = { println(s"plus($k)"); new Strict(n + k) }
  def applyDynamic(name: String)(arg: String): Unit = println(s"apply($name)($arg)")

class Holder:
  private var c = new Counter(1)
  def cur: Counter = c
  def cur_=(x: Counter): Unit = c = x
  def bump(): Unit = cur += 2
  private var s = new Strict(1)
  def strict: Strict = s
  def strict_=(x: Strict): Unit = s = x
  def step(): Unit = strict += 2

class Wide:
  var c = new Counter(1)
  def cur: Counter = c
extension (w: Wide) def cur_=(x: Counter): Unit = w.c = x

class Slots[T](private var c: T):
  def apply(i: Int): T = c
  def update(i: Int, x: T): Unit = c = x

@main def run(): Unit =
  val a = new Typed
  println(a.slot("x") := 1)
  val h = new Holder
  h.cur += 1
  println(h.cur.n)
  h.bump()
  println(h.cur.n)
  val w = new Wide
  w.cur += 3
  println(w.cur.n)
  val xs = Array(new Counter(1))
  xs(0) += 4
  println(xs(0).n)
  val s = new Slots(new Counter(1))
  s(0) += 5
  println(s(0).n)
  val m = mutable.Map("k" -> new Counter(1))
  m("k") += 6
  println(m("k").n)
  var v = new Strict(1)
  v += 7
  println(v.n)
  h.strict += 1
  println(h.strict.n)
  h.step()
  println(h.strict.n)
  val ys = Array(new Strict(1))
  ys(0) += 4
  println(ys(0).n)
  val t = new Slots(new Strict(1))
  t(0) += 5
  println(t(0).n)
  val k = mutable.Map("k" -> new Strict(1))
  k("k") += 6
  println(k("k").n)
