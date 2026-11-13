// A block's own method, its result inferred, first needed from a nested scope of the block or from inside another
// construct: it is typed in its block's frames and in what stood where the block opened.

// Expects "string 1": a block's method of inferred result first needed inside another method of the block that
// defines a value of a name its body reads; the body reads the block's value. scalac prints string 1.
object NestedScope:
  object Test:
    def which(x: Any): String = "any"
    def which(x: Int): String = "int"
    def which(x: String): String = "string"
    def run: String =
      val held = "outer"
      def caller: String =
        val held = 1
        later + " " + held
      def later = which(held)
      caller
    def main(args: Array[String]): Unit =
      println(run)

// Expects "any" and "other any": a block's method of inferred result first needed inside a case that fixes the
// enclosing method's type parameter; its body's overload takes a value of that parameter as the block has it.
// scalac prints any and other any.
object CaseBounds:
  sealed trait Tag[A]
  case object IntTag extends Tag[Int]
  case object StrTag extends Tag[String]
  object Test:
    def which(x: Any): String = "any"
    def which(x: Int): String = "int"
    def run[A](tag: Tag[A], held: A): String =
      def caller: String = tag match
        case IntTag => later
        case StrTag => "other " + later
      def later = which(held)
      caller
    def main(args: Array[String]): Unit =
      println(run(IntTag, 2))
      println(run(StrTag, "b"))

// Expects "b b": a block's def of inferred result first needed among the arguments of `new other.Inner(..)`; its
// body makes an Inner of its own enclosing instance, whose default reads that instance. scalac prints b b.
object CreationPrefix:
  class Outer(val tag: String):
    class Inner(val text: String = tag)
    def run(other: Outer): String =
      def caller: String = new other.Inner(later).text
      def later = new Inner().text
      caller + " " + later
  object Test:
    def main(args: Array[String]): Unit = println(new Outer("b").run(new Outer("a")))

// Expects 3: a block's def of inferred result first needed inside the self call of a secondary constructor of a
// block's class; its body creates the class through that constructor. scalac prints 3.
object SelfCall:
  object Test:
    def run: String =
      class Counted(val n: Int):
        def this(s: String) = this(sized(s))
      def sized(s: String) =
        if s.isEmpty then 0 else new Counted("").n + s.length
      new Counted("abc").n.toString
    def main(args: Array[String]): Unit = println(run)

// Expects "got 42": a block's def of inferred result, whose body calls an inline method, first needed inside the
// body of a block's inline method above it. scalac prints got 42.
object InlineCheck:
  object Test:
    inline def twice(x: Int): Int = x * 2
    def run: String =
      inline def shown: String = "got " + later
      def later = twice(21)
      shown
    def main(args: Array[String]): Unit = println(run)

@main def run(): Unit =
  NestedScope.Test.main(Array())
  CaseBounds.Test.main(Array())
  CreationPrefix.Test.main(Array())
  SelfCall.Test.main(Array())
  InlineCheck.Test.main(Array())
