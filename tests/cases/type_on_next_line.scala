// A type that starts on the next, indented line: after the `=` of an alias, ordinary, private
// or opaque (bounded or not), and the `=>` of a function type or a match type's case, the type
// alone in its indentation region, as scalac's `typ()` takes `INDENT Type OUTDENT`; after the `:`
// of a val, a var, a def's result, a parameter (a class's and a using clause's too) and a named
// given, where scalac opens no region, the statements after the definition at that indentation
// being the enclosing sequence's. A bound or a parent on the next line was accepted already.
trait A:
  def name: String = "a"
trait B:
  def name: String = "b"
trait C:
  def name: String = "c"

object Rules:
  private type FunctionLike =
    A | B | C
  type Pair =
    (Int, String)
  opaque type Id =
    Int
  opaque type Code <: Int =
    Int
  type Render =
    Int =>
      String
  type Chosen[X] =
    X match
      case Int =>
        String
      case String =>
        Int
  type Bounded <:
    AnyRef
  def id(n: Int): Id = n
  def code(n: Int): Code = n
  def describe(f: FunctionLike): String = f match
    case a: A => s"A ${a.name}"
    case b: B => s"B ${b.name}"
    case c: C => s"C ${c.name}"

class Parent
class Child(val size:
  Int) extends
  Parent

def scaled(x:
  Int, factor: Int =
  2): Int = x * factor

def greeting(using who:
  String): String = s"hello $who"

trait Show[T]:
  def show(t: T): String
object ShowInt extends Show[Int]:
  def show(t: Int): String = s"#$t"
given showInt:
  Show[Int] = ShowInt

val total:
  Int = 40
var counter:
  Int = 1
def label:
  String = "label"

object Nested:
  val first:
    Int = 1
    def second = first + 1
  def third = second + 1

object Layouts:
  def both:
    String =
    val f = "f"
    f + "g"
  val split:
    Int
  = 7
  val _:
    Int = 8
  class **
  given star:
    ** = new **
  def hasStar: Boolean = summon[**] eq star
  def partial:
    PartialFunction[Int, Int] =
    case n if n > 0 => n + 1
  def scaledBy(x: Int, by:
    Int =
    val two = 2
    two * 2
  ): Int = x * by
  given named:
    Long
  = 5L

object Braced:
  var log = ""
  def f:
    Unit =
    { log += "f" }
    log += "g"
  log += "start"

@main def run(): Unit =
  println(Rules.describe(new B {}))
  val p: Rules.Pair = (1, "one")
  println(p)
  println(Rules.id(3))
  println(Rules.code(4) + 1)
  val r: Rules.Render = n => s"r$n"
  println(r(5))
  val chosen: Rules.Chosen[Int] = "chosen"
  println(chosen)
  println(Child(6).size)
  println(scaled(7) + scaled(7, 3))
  given String = "you"
  println(greeting)
  println(summon[Show[Int]].show(8))
  counter += 1
  println(total + counter)
  println(label)
  println(Nested.third)
  println(Layouts.both + Layouts.split + Layouts.hasStar)
  println(Layouts.partial(1) + Layouts.scaledBy(3) + Layouts.named)
  Braced.f
  println(Braced.log)
  val local:
    Int = 9
  println(local)
  println((10:
    Int) + ("eleven":
    String).length)
  println(scaled:
    12)
