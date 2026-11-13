// A default argument first needed by a call inside another construct: its getter is typed in its method's place.

// Expects "any, any" then "other": a default argument of a class's method first needed inside a `case` that
// fixes the class's type parameter; the default's overloaded call takes a value of that parameter.
// scalac prints any, any and other.
object CaseBounds:
  sealed trait Tag[A]
  case object IntTag extends Tag[Int]
  case object StrTag extends Tag[String]
  class Keeper[A](held: A):
    def which(x: Int): String = "int " + (x + 1)
    def which(x: Any): String = "any"
    def run(tag: Tag[A]): String =
      val first = tag match
        case IntTag => shown()
        case _ => "other"
      first + (if first == "other" then "" else ", " + shown())
    def shown(text: String = which(held)): String = text
  object Test:
    def main(args: Array[String]): Unit =
      println(new Keeper(2).run(IntTag))
      println(new Keeper("b").run(StrTag))

// Expects "4 3": a default argument of a method first needed inside the self call of a secondary constructor of a
// class nested in the method's class; the default names a member of the enclosing class. scalac prints 4 3.
object OuterParameter:
  object Test:
    def main(args: Array[String]): Unit =
      val o = new Outer
      println(new o.Inner("abc").n.toString + " " + o.other)
  class Outer:
    val base = 1
    class Inner(val n: Int):
      def this(s: String) = this(sized(s))
    def sized(s: String, extra: Int = base): Int = extra + s.length
    def other: Int = sized("ab")

// Expects "b b": a default argument first needed among the arguments of `new a.Inner(..)`; the default makes an
// Inner of its own instance, whose default reads that instance. scalac prints b b.
object CreationPrefix:
  object Test:
    def main(args: Array[String]): Unit =
      val a = new Outer("a")
      val b = new Outer("b")
      println(new a.Inner(b.fresh()).text + " " + b.again)
  class Outer(val tag: String):
    class Inner(val text: String = tag)
    def fresh(text: String = new Inner().text): String = text
    def again: String = fresh()

// Expects 9: a default argument of a method of a class first needed inside the self call of the class's own
// secondary constructor; the default names a member of the class. scalac prints 9.
object SelfCallMember:
  object Test:
    val n = 50
    def main(args: Array[String]): Unit = println(new Counted("abc").n)
  class Counted(val n: Int):
    def this(s: String) = this(new Counted(s.length).scaled())
    def scaled(by: Int = n): Int = n * by

// Expects 3: a default argument first needed inside the self call of a secondary constructor; the default
// creates the class through that constructor. scalac prints 3.
object SelfCall:
  class Counted(val n: Int):
    def this(s: String) = this(Sizer.sized(s))
  object Sizer:
    def sized(s: String, base: => Int = new Counted("").n): Int =
      if s.isEmpty then 0 else base + s.length
  object Test:
    def main(args: Array[String]): Unit = println(new Counted("abc").n)

// Expects 42: a default argument first needed by a call inside a quote; the default calls an inline method, and
// ordinary code calls the method later. scalac prints 42.
object QuoteLevel:
  import scala.quoted.*
  object Macros:
    def impl(using Quotes): Expr[Int] = '{ Late.pick() }
  object Test:
    def main(args: Array[String]): Unit = println(Late.again)
  object Late:
    inline def twice(x: Int): Int = x * 2
    def pick(n: Int = twice(21)): Int = n
    def again: Int = pick()

@main def run(): Unit =
  CaseBounds.Test.main(Array())
  OuterParameter.Test.main(Array())
  CreationPrefix.Test.main(Array())
  SelfCallMember.Test.main(Array())
  SelfCall.Test.main(Array())
  QuoteLevel.Test.main(Array())
