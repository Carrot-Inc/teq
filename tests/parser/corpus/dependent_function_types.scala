// Function types whose result names a parameter, `(c: Ctx) => c.T` and `(c: Ctx) ?=> List[c.T] =>
// Int`: a lambda typed against one sees its own parameter in the result, an unannotated lambda and
// an eta-expanded method infer one, and a call's result has the argument's path, `IntCtx.T` for
// `fn(IntCtx)`, which is `Int`, the argument passed by position or by the parameter's name.
// Where a plain function type is asked for, the result is the least type naming no parameter:
// `Ctx#T`, `List[Nothing]` in a parameter, `Box[?]` in an invariant position.
trait Ctx:
  type T
  def make: T
  def show(t: T): String
object IntCtx extends Ctx:
  type T = Int
  def make: Int = 7
  def show(t: Int): String = s"int $t"
object StrCtx extends Ctx:
  type T = String
  def make: String = "s"
  def show(t: String): String = s"str $t"
class Box[A](val a: A)
trait C:
  type M
  val m: M

object Main:
  type DF = (x: C) => x.M
  type IDF = (x: C) ?=> x.M
  type F[S, T] = (x: S) => Option[x.type & T]

  def f(fn: (c: Ctx) => c.T): Any = fn(IntCtx)
  def g(fn: (c: Ctx) ?=> List[c.T] => Int): Int = fn(using IntCtx)(List(1, 2, 3))
  def applied(fn: (c: Ctx) => c.T)(c: Ctx): c.T = fn(c)
  def roundTrip(fn: (c: Ctx) => c.T => String): (String, String) = (fn(IntCtx)(IntCtx.make), fn(StrCtx)(StrCtx.make))
  def the[T](using ev: T): ev.type = ev
  def within(using c: Ctx)(f: (c: Ctx) ?=> c.T): String = c.show(f)
  def identity[T]: F[T, T] = Some(_)
  def depmeth(x: C) = x.m

  val c: C = new C { type M = Int; val m = 3 }
  given C = c
  val depfun1: DF = (x: C) => x.m
  val depfun2 = depmeth
  val depfun3: DF = depfun2
  val ifun: IDF = the[C].m

  def main(args: Array[String]): Unit =
    println(f(c => c.make))
    println(g(xs => xs.size))
    val inferred = (c: Ctx) => c.make
    val typed: (c: Ctx) => c.T = inferred
    val n: Int = inferred(IntCtx)
    val viaApply: Int = typed.apply(IntCtx)
    val viaName: Int = typed(c = IntCtx)
    val left: Ctx = StrCtx
    val l: left.T = typed(left)
    println(s"$n $viaApply $viaName ${left.show(l)} ${applied(typed)(IntCtx) + 1}")
    val anon: Int = typed(new Ctx { type T = Int; def make = 5; def show(t: Int) = "" })
    println(anon)
    val renamed: (d: Ctx) => d.T = typed
    val proj: Ctx => Ctx#T = typed
    println(s"${renamed(IntCtx) + 1} ${proj(StrCtx)}")
    val lst: (c: Ctx) => List[c.T] => Int = c => xs => xs.size
    val nothingList: Ctx => List[Nothing] => Int = lst
    val bx: (c: Ctx) => Box[c.T] = c => Box(c.make)
    val anyBox: Ctx => Box[?] = bx
    println(s"${nothingList(IntCtx)(Nil)} ${bx(IntCtx).a + 1} ${anyBox(StrCtx).a}")
    val pn: Ctx => Nothing = _ => throw new Exception("never")
    val fromPlain: (c: Ctx) => c.T = pn
    val pi: Int => Int = x => x + 1
    val named: (x: Int) => Int = pi
    val back: Int => Int = named
    println(s"${named(3) + back(4)} ${fromPlain.hashCode == pn.hashCode}")
    val two: (a: Ctx, b: Ctx) => (a.T, b.T) = (x, y) => (x.make, y.make)
    val pair: (Int, String) = two(IntCtx, StrCtx)
    println(pair)
    println(roundTrip(c => t => c.show(t)))
    println(within(using StrCtx)(the[Ctx].make))
    println(within(using IntCtx)((c: Ctx) ?=> c.make))
    val y: Int = depfun1(c.asInstanceOf[C { type M = Int }])
    val z: Option[1] = identity(1: 1)
    println(s"$y ${depfun3(c)} ${ifun(using c)} $z")
    println(depfun2(new C { type M = String; val m = "anon" }).length)
