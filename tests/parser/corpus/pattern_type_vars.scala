// Type variables bound by a type pattern in an ordinary match (`case m: Memo[a]`), with the
// bounds the scrutinee gives them: an upper bound through a covariant parameter, fixed through
// an invariant one, against a path-dependent scrutinee (`Ev[c.Start]`) as well; and the
// wildcard arguments of a type pattern inferred from the scrutinee (`case r: Const[_]`).
sealed trait Ev[+A]
final case class Now[A](value: A) extends Ev[A]
final case class Memo[A](ev: Ev[A], var result: Option[A] = None) extends Ev[A]
final case class Fn[A, B](ev: Ev[A], f: A => B) extends Ev[B]
trait FM[A]:
  type Start
  def start(): Ev[Start]
  def run: Start => Ev[A]
def evaluate[A](c: FM[A]): Ev[A] = c.start() match
  case m: Memo[a] =>
    m.result match
      case Some(a) => c.run(a)
      case None => c.run(evalNow(m.ev))
  case Now(v) => c.run(v)
  case fn: Fn[a, c.Start] =>
    val x: a = evalNow[a](fn.ev)
    c.run(fn.f(x))
def evalNow[A](e: Ev[A]): A = e match
  case Now(v) => v
  case m: Memo[a] => evalNow[a](m.ev)
  case fn: Fn[a, A] => fn.f(evalNow[a](fn.ev))
object Dep:
  def run(): Unit =
    val fm = new FM[String]:
      type Start = Int
      def start() = Memo(Now(4))
      def run = i => Now((i * 10).toString)
    println(evaluate(fm))
final class Const[A](val value: A) extends Function1[Any, A]:
  def apply(x: Any): A = value
def tag[A, B](run: A => B, extra: B => String): Any => String = run match
  case r: Const[_] => r.andThen(b => "const " + extra(b))
  case _ => a => "plain " + extra(run(a.asInstanceOf[A]))
object Main:
  def run[A](e: Ev[A]): A = e match
    case Now(v) => v
    case m: Memo[a] =>
      val v: a = run[a](m.ev)
      m.result = Some(v)
      v
    case fn: Fn[a, A] =>
      val x: a = run[a](fn.ev)
      fn.f(x)
  def main(args: Array[String]): Unit =
    println(run(Now(1)))
    val m = Memo(Now("s"))
    println(run(m) + " " + m.result)
    println(run(Fn(Now(20), (i: Int) => i + 1)))
    println(run(Fn(Memo(Now(2)), (i: Int) => i.toString * 2)))
    Dep.run()
    println(tag[Int, Int](new Const(5), i => (i * 2).toString)(0))
    println(tag[Int, Int](_ + 1, i => (i * 2).toString)(0))
