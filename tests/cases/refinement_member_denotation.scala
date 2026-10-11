// A member a refinement of the receiver's type declares is the refinement's denotation where it
// conforms to the class's (dotty's `Types.findMember` `goRefined`): its parameters' names and its
// result type the call, generic ones too; the class's member is the one called, with its
// defaults.
trait Base { def run[T](x: T): Any; def scaled(x: Int, by: Int = 3): Any; val tag: Any }
class Impl extends Base:
  def run[T](x: T): T = x
  def scaled(x: Int, by: Int = 3): Int = x * by
  val tag: String = "impl"

type Precise = Base { def run[T](x: T): T; def scaled(n: Int, by: Int): Int; val tag: String }

trait B { def f(x: Int): Any }
def named(b: B { def f(y: Int): String }): String = b.f(y = 1)

trait Ctx { type T; val value: T }
def dependent(fun: (c: Ctx) => c.T, c: Ctx): c.T = fun(c)

@main def run(): Unit =
  val p: Precise = Impl()
  val s: String = p.run[String]("generic")
  val i: Int = p.run(41) + 1
  val n: Int = p.scaled(n = 2) + p.scaled(2, 10)
  val t: String = p.tag
  println(s"$s $i $n $t")
  println(named(new B { def f(x: Int): String = "named " + x }))
  val c = new Ctx { type T = String; val value = "dependent" }
  val v: c.T = dependent(_.value, c)
  println(v)
