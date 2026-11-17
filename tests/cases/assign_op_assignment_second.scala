// Where an assignment operator's application as written fails, the left side is typed again by itself and the
// assignment `l = l op r` is made by what it is (`Applications.typedOpAssign`, `Typer.typedAssign`): a member
// that does not take the argument is no reason to refuse; an `apply` is written through `update` with its
// inserted default, its named arguments in order, its written type arguments, the alternative that was selected
// (whose by-name parameter is not bound), a function literal typed by its parameter, `super`'s and a written
// `.apply`; and what the index defines is defined once more for the second typing.
object Test:
  var log = List.empty[String]
  def mark[A](s: String, a: A): A = { log ::= s; a }
  def flush(): Unit = { println(log.reverse.mkString(" ")); log = Nil }

  class E(val n: Int):
    def +=(s: String): Unit = mark("member", ())
    def +(i: Int): E = mark("plus", new E(n + i))
  class Two(val n: Int):
    def +=(s: String): Unit = mark("m1", ())
    def +=(b: Boolean): Unit = mark("m2", ())
    def +(i: Int): Two = mark("plus", new Two(n + i))
  class Cell:
    var e = new E(0)
    def apply(i: Int): E = mark("apply", e)
    def update(i: Int, v: E): Unit = { mark("update", ()); e = v }
  class Prop:
    private var v = new E(0)
    def x: E = mark("get", v)
    def x_=(e: E): Unit = { mark("set", ()); v = e }

  class Defaulted:
    def apply(i: Int, j: Int = mark("default", 1)): Int = mark("apply", 10)
    def update(i: Int, j: Int, v: Int): Unit = mark("update:" + i + ":" + j + ":" + v, ())
  class Poly:
    def apply[A](i: Int): Int = mark("apply", 10)
    def update[A](i: Int, v: Int): Unit = mark("update:" + v, ())
  class ByName:
    def apply(i: => Int): Int = { mark("apply-int", ()); i; i; 10 }
    def apply(s: String): Int = mark("apply-string", 20)
    def update(i: => Int, v: Int): Unit = { mark("update:" + v, ()); i; () }
  class PolyByName:
    def apply[T](i: => Int): Int = { mark("papply", ()); i + i }
    def update[T](i: => Int, v: Int): Unit = { mark("pupdate:" + v, ()); i; () }
  class PolyInline:
    inline def apply[T](inline i: Int): Int = { mark("iapply", ()); i + i }
    inline def update[T](inline i: Int, v: Int): Unit = { mark("iupdate:" + v, ()); i; () }
  class Fun:
    def apply(f: Int => Int): Int = mark("apply:" + f(1), 10)
    def update(f: Int => Int, v: Int): Unit = mark("update:" + f(2) + ":" + v, ())
  class Ctx:
    def apply(f: String ?=> Int): Int = mark("apply:" + f(using "ab"), 10)
    def update(f: String ?=> Int, v: Int): Unit = mark("update:" + f(using "abc") + ":" + v, ())
  class Base:
    def apply(i: Int): Int = mark("base-apply", 10)
    def update(i: Int, v: Int): Unit = mark("base-update:" + v, ())
  class Child extends Base:
    override def apply(i: Int): Int = mark("child-apply", 20)
    override def update(i: Int, v: Int): Unit = mark("child-update:" + v, ())
    def go(): Unit = super.apply(mark("index", 0)) += mark("rhs", 2)

  def f(i: Int): Int = i
  def index(): Int = mark("index", 0)

  def main(args: Array[String]): Unit =
    // the member does not take the argument: the assignment form, on a var, an element, a setter
    var e = new E(0)
    e += 2; flush()
    val c = new Cell
    c(0) += 2; flush()
    val p = new Prop
    p.x += 2; flush()
    println(e.n + c.e.n + p.x.n); log = Nil
    // an overloaded operator none of whose alternatives fits, an application in its right side
    var t = new Two(0)
    t += f(1); flush()
    var n = 1
    n += f(1)
    println(t.n + n)
    // the typed application lowered
    val d = new Defaulted
    d(mark("i", 0)) += mark("rhs", 5); flush()
    d(j = mark("j", 2), i = mark("i", 1)).toString; flush()
    d(i = mark("i", 1), j = mark("j", 2)) += mark("rhs", 7); flush()
    d(j = 2, i = 1) += mark("rhs", 8); flush()
    val poly = new Poly
    poly[Int](mark("index", 0)) += mark("rhs", 2); flush()
    val b = new ByName
    b(index()) += mark("rhs", 2); flush()
    // a polymorphic `apply`'s by-name or inline argument is bound like any other (`liftArgs` leaves one only for
    // a method that takes no type parameters)
    val pb = new PolyByName
    pb[String](mark("i", 1)) += mark("rhs", 12); flush()
    pb(mark("i", 1)) += mark("rhs", 12); flush()
    val pi = new PolyInline
    pi[Int](mark("i", 1)) += mark("rhs", 5); flush()
    pi(mark("i", 1)) += mark("rhs", 5); flush()
    val fun = new Fun
    fun(x => x + 1) += 2; flush()
    val ctx = new Ctx
    ctx(summon[String].length) += 2; flush()
    new Child().go(); flush()
    c.apply(mark("index", 0)) += mark("rhs", 1); flush()
    // what the index defines, defined again for the second typing
    val arr = Array(1, 2, 3, 4)
    arr({ object O { val i = mark("O", 0) }; O.i }) += 1000; flush()
    arr({ class K(val i: Int); object K { def one = new K(mark("K", 1)) }; K.one.i }) += 100; flush()
    arr({ enum En { case A, B }; object En { val z = mark("En", 2) }; En.z + En.A.ordinal }) += 10; flush()
    arr({ def g = mark("g", 3); g }) += 1; flush()
    n += { object R { val v = mark("R", 7) }; R.v }; flush()
    println(arr.toList)
    println(n)
