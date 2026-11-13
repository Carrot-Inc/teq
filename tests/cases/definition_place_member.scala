// A member outside every block, its result inferred, first needed from inside another construct: it is typed in
// its own place, so that what stands around the code that asked is none of it.

// Expects 3: a method of inferred result first needed inside a secondary constructor's call of the primary one; its
// body creates the class through that secondary constructor, which is its own to call. scalac prints 3.
object SelfCall:
  class Counted(val n: Int):
    def this(s: String) = this(Counted.sized(s))

  object Counted:
    def sized(s: String) =
      if s.isEmpty then 0 else new Counted("").n + s.length

  object Test:
    def main(args: Array[String]): Unit = println(new Counted("abc").n)

// Expects "any, any" and "other, any": a method of inferred result first needed inside a case that fixes the
// class's type parameter; its body's overload takes a value of that parameter as the class has it, not as the case
// has it. scalac prints any, any and other, any.
object CaseBounds:
  sealed trait Tag[A]
  case object IntTag extends Tag[Int]
  case object StrTag extends Tag[String]

  class Keeper[A](held: A):
    def which(x: Int): String = "int " + (x + 1)
    def which(x: Any): String = "any"
    def run(tag: Tag[A]): String =
      val first = tag match
        case IntTag => shown
        case _ => "other"
      first + ", " + shown
    def shown = which(held)

  object Test:
    def main(args: Array[String]): Unit =
      println(new Keeper(2).run(IntTag))
      println(new Keeper("b").run(StrTag))

// Expects "local": a method of inferred result first needed in a given's right-hand side asks for the given's
// type; the given being defined there is none of the method's, which finds it. scalac prints local.
object GivenUnderWay:
  trait Mark:
    def name: String

  trait Farther:
    given outer: Mark with
      def name: String = "outer"

  object Members extends Farther:
    def wrap(m: => Mark): Mark = new Mark:
      def name: String = if m eq this then "local" else m.name
    given local: Mark = picked
    def picked = wrap(summon[Mark])

  object Test:
    def main(args: Array[String]): Unit = println(Members.local.name)

// Expects "wide any": scalac compiles it; `seen` is first needed by the default of `taken`, whose getter is first read
// while the call's two alternatives are tested with conversions (a `Plain` passed for a `Wide`).
object ConversionRound:
  import scala.language.implicitConversions
  class Plain
  class Wide
  class Other

  object Rounds:
    def kind(x: String): String = "string"
    def kind(x: Any): String = "any"
    given Conversion[Int, String] = _.toString
    given Conversion[Plain, Wide] = _ => new Wide
    def run: String = taken(new Plain)
    def taken(w: Wide, s: String = seen): String = "wide " + s
    def taken(o: Other): String = "other"
    def seen = kind(1)

  object Test:
    def main(args: Array[String]): Unit = println(Rounds.run)

// Expects 3: scalac compiles it (the member `add` does not take an Int, the extension in scope is another class's,
// the conversion to `Wide` gives the method); the extension `add` of inferred result is first needed in that retry.
object ExtensionRetry:
  import scala.language.implicitConversions
  class Box(val n: Int):
    def add(s: String): String = s + n
  class Wide(val n: Int):
    def add(k: Int): Int = n + k
  class Other

  object Keeper:
    given Conversion[Box, Wide] = b => new Wide(b.n)
    extension (k: Int) def twice: Int = k * 2
    def run: Int = new Box(1).add(2)
    extension (o: Other) def add(k: Int) = k.twice

  object Test:
    def main(args: Array[String]): Unit = println(Keeper.run)

// Expects 13: scalac compiles it; `bump`, a member of the enclosing class, is first needed in the self call of the
// nested class's secondary constructor, and its body reads `base` of its own instance.
object OuterParameter:
  class Outer(val base: Int):
    class Inner(val n: Int):
      def this(s: String) = this(s.length + bump)
    def bump = base + 1
    def make(s: String): Int = new Inner(s).n

  object Test:
    def main(args: Array[String]): Unit = println(new Outer(10).make("ab"))

// Expects 5: scalac compiles it; `fresh` is first needed as the argument of `new a.Inner(..)`, and its body creates
// an `Inner` of its own instance with the default read from that instance.
object CreationPrefix:
  object Keeper:
    val a: Outer = new Outer(1)
    val b: Outer = new Outer(5)
    def run: Int = new a.Inner(b.fresh).n

  class Outer(val base: Int):
    class Inner(val n: Int = base)
    def fresh = new Inner().n

  object Test:
    def main(args: Array[String]): Unit = println(Keeper.run)

// Expects 4: scalac compiles it; `size`, a member of the class itself, is first needed on another instance in the
// self call of the secondary constructor, and its body reads `n` of its own instance.
object SelfCallMember:
  class Counted(val n: Int):
    def this(s: String) = this(new Counted(1).size + s.length)
    def size = n + 1

  object Test:
    def main(args: Array[String]): Unit = println(new Counted("ab").n)

// Expects 5: scalac compiles it; `size`, a member of the class itself, is first needed on another instance in the
// arguments of the class's parent, and its body reads `this` and `k` of its own instance.
object ParentArguments:
  class Base(val n: Int)
  class Sub(val k: Int, peer: Option[Sub]) extends Base(peer.map(p => p.size).getOrElse(0) + k):
    def size = this.k + 2

  object Test:
    def main(args: Array[String]): Unit = println(new Sub(1, Some(new Sub(2, None))).n)

// Expects "any1 any0": scalac compiles it; the extractor's `unapply`, of inferred result, is first needed where the
// pattern of a `case` is typed, and its body matches a generator's pattern against boxes of its own type parameter.
object PatternUnderWay:
  sealed trait Box[A]
  final case class IntBox(n: Int) extends Box[Int]
  final case class StrBox(s: String) extends Box[String]
  class Arg[T](val boxes: List[Box[T]], val held: T)

  object Keeper:
    def which(x: Int): String = "int"
    def which(x: Any): String = "any"
    def run[A](arg: Arg[A]): String = arg match
      case Pick(s) => s

  object Pick:
    def unapply[T](arg: Arg[T]) =
      val hits = for case IntBox(k) <- arg.boxes yield k
      Some(Keeper.which(arg.held) + hits.size)

  object Test:
    def main(args: Array[String]): Unit =
      println(Keeper.run(new Arg(List(IntBox(1)), 2)) + " " + Keeper.run(new Arg(List(StrBox("a")), "b")))

// Expects "1 fruit": scalac compiles it; the override `Sub.extra`, of inferred result, is first needed where a member
// is selected on a union whose second alternative is a refined type of the base class, and its body joins two case classes.
object JoinUnderWay:
  object Keeper:
    def pick(one: Sub | (Holder { def extra: Int })): Int = one.extra
    def run: Int = pick(new Sub)

  class Holder:
    def extra: Int = 0
  class Sub extends Holder:
    override def extra =
      val picked = if Probe.flag then new Apple else new Berry
      val back: Apple | Berry = picked
      Probe.log = Probe.name(back)
      1

  case class Apple()
  case class Berry()
  object Probe:
    var log = ""
    val flag: Boolean = true
    def name(x: Apple | Berry): String = "fruit"
    def name(x: Any): String = "any"

  object Test:
    def main(args: Array[String]): Unit =
      val r = Keeper.run
      println(r.toString + " " + Probe.log)

// Expects "7 3": scalac compiles it; `pick`, a member of the bound's class of inferred result that overloads an
// inherited method, is first needed where a member is selected on a value of the type parameter `T`, and through it
// `later`, a member of the same class of inferred result, whose body selects on a value of the same `T`.
object BoundLookup:
  class Keeper[T <: Base]:
    def run(x: T): Int = x.pick + 1
    def later(y: T) = y.other + 1

  class Root:
    def pick(k: Int): Int = k
  class Base extends Root:
    def pick = Probe.viaLater * 2
    def other: Int = 2

  object Probe:
    def viaLater = new Keeper[Base].later(new Base)

  object Test:
    def main(args: Array[String]): Unit = println(new Keeper[Base].run(new Base).toString + " " + Probe.viaLater)

// Expects "top1": scalac compiles it; `held` is first needed by the first statement of a block whose alias `Local`,
// defined further down, is first named inside a local method that defines a class with the name the alias stands for;
// the body of `held` is a block with an alias of its own.
object BlockAliases:
  class Marker:
    def name: String = "top"

  object Keeper:
    def run: String =
      val n = held
      def early: String =
        class Marker:
          def name: String = "inner"
        val x = new Local
        x.name
      type Local = Marker
      early + n
    def held =
      type Mine = Int
      val k: Mine = 1
      k

  object Test:
    def main(args: Array[String]): Unit = println(Keeper.run)

@main def run(): Unit =
  SelfCall.Test.main(Array())
  CaseBounds.Test.main(Array())
  GivenUnderWay.Test.main(Array())
  ConversionRound.Test.main(Array())
  ExtensionRetry.Test.main(Array())
  OuterParameter.Test.main(Array())
  CreationPrefix.Test.main(Array())
  SelfCallMember.Test.main(Array())
  ParentArguments.Test.main(Array())
  PatternUnderWay.Test.main(Array())
  JoinUnderWay.Test.main(Array())
  BoundLookup.Test.main(Array())
  BlockAliases.Test.main(Array())
