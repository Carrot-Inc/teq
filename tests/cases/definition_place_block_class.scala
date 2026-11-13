// A member of a class made in a block, its result inferred, first needed from inside another construct: it is typed
// in what stood where its class was entered.

// Expects "4 3": a def of inferred result of a block's class first needed inside the self call of a secondary
// constructor of a class nested in it; its body names a member of the enclosing class. scalac prints 4 3.
object OuterParameter:
  object Test:
    def run: String =
      class Outer:
        val base = 1
        class Inner(val n: Int):
          def this(s: String) = this(sized(s))
        def sized(s: String) = base + s.length
      val o = new Outer
      new o.Inner("abc").n.toString + " " + o.sized("ab")
    def main(args: Array[String]): Unit = println(run)

// Expects "b b": a def of inferred result of a block's class first needed among the arguments of
// `new other.Inner(..)` in a member above it; its body makes an Inner of its own instance, whose default reads
// that instance. scalac prints b b.
object CreationPrefix:
  object Test:
    def run: String =
      class Outer(val tag: String):
        class Inner(val text: String = tag)
        def joined(other: Outer): String = new other.Inner(fresh).text
        def fresh = new Inner().text
      val b = new Outer("b")
      b.joined(new Outer("a")) + " " + b.fresh
    def main(args: Array[String]): Unit = println(run)

// Expects "6 6": a def of inferred result of a block's class first needed inside the self call of the class's
// own secondary constructor; its body names a member of the class and `this`. scalac prints 6 6.
object SelfCallMember:
  object Test:
    def run: String =
      val n = 50
      class Counted(val n: Int):
        def this(s: String) = this(new Counted(s.length).twice)
        def twice = n * 2
        def again = this.n
      val c = new Counted("abc")
      c.n.toString + " " + c.again
    def main(args: Array[String]): Unit = println(run)

// Expects 6: a def of inferred result of a block's class first needed inside the self call of the class's own
// secondary constructor; its body names `this`. scalac prints 6.
object SelfCallThis:
  object Test:
    def run: String =
      class Counted(val n: Int):
        def this(s: String) = this(new Counted(s.length).twice)
        def twice = this.n * 2
      new Counted("abc").n.toString
    def main(args: Array[String]): Unit = println(run)

// Expects "any 1": a def of inferred result of a block's object, an extractor's unapply, first needed by a pattern
// of a def above the object; its body holds a pattern definition against a type the method's parameter is left
// in, then an overloaded call on a value of that parameter. scalac prints any 1.
object PatternUnderWay:
  sealed trait Box[A]
  final case class IntBox(n: Int) extends Box[Int]
  final case class StrBox(s: String) extends Box[String]
  object Test:
    def which(x: Any): String = "any"
    def which(x: Int): String = "int"
    def run[A](box: Box[A], held: A): String =
      def caller: String = (box, held) match
        case Ext(text) => text
      object Ext:
        def unapply(x: (Box[A], A)) =
          val IntBox(n) = x._1: @unchecked
          Some(which(x._2) + " " + n)
      caller
    def main(args: Array[String]): Unit = println(run(IntBox(1), 2))

// Expects 3: a def of inferred result of a block's object first needed inside the self call of a secondary
// constructor of a block's class; its body creates the class through that constructor. scalac prints 3.
object SelfCall:
  object Test:
    def run: String =
      class Counted(val n: Int):
        def this(s: String) = this(Sizer.sized(s))
      object Sizer:
        def sized(s: String) =
          if s.isEmpty then 0 else new Counted("").n + s.length
      new Counted("abc").n.toString
    def main(args: Array[String]): Unit = println(run)

// Expects "got 42": a def of inferred result of a block's object, whose body calls an inline method, first needed
// inside the body of a block's inline method above the object. scalac prints got 42.
object InlineCheckObject:
  object Test:
    inline def twice(x: Int): Int = x * 2
    def run: String =
      inline def shown: String = "got " + Single.later
      object Single:
        def later = twice(21)
      shown
    def main(args: Array[String]): Unit = println(run)

// Expects "got 42": a def of inferred result of a block's class, whose body calls an inline method, first needed
// inside the body of an inline method of the class above it. scalac prints got 42.
object InlineCheckClass:
  object Test:
    inline def twice(x: Int): Int = x * 2
    def run: String =
      class Keeper:
        inline def shown: String = "got " + later
        def later = twice(21)
      new Keeper().shown
    def main(args: Array[String]): Unit = println(run)

@main def run(): Unit =
  OuterParameter.Test.main(Array())
  CreationPrefix.Test.main(Array())
  SelfCallMember.Test.main(Array())
  SelfCallThis.Test.main(Array())
  PatternUnderWay.Test.main(Array())
  SelfCall.Test.main(Array())
  InlineCheckObject.Test.main(Array())
  InlineCheckClass.Test.main(Array())
