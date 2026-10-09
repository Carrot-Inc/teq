// The objects, traits and enums an inline body defines, and the case classes, case objects and
// enum class cases a local inline method's body defines (which scalac 3.8.4 refuses in a member
// method alone), are the stored body's own: copied with their class graph at each expansion (a
// companion with its class, an enum with its companion, its cases and their values, a trait
// with what mixes it in, a class only a pattern tests), named per expansion, their bodies walked
// where nothing creates them (`twice` in the trait's `show`). scalac accepts these and duplicates
// the classes, but its run of an enum's value cases and of a local case class is broken (an
// AbstractMethodError for `ordinal`, the anonymous class's name for `toString`, `productPrefix`
// empty, `==` false); these print the members' own answers.
object Lib:
  inline def counter(start: Int): Int =
    object C:
      var n = start
      def next(): Int = { n += 1; n }
    C.next() + C.next()
  inline def twice(x: Int): Int = x * 2
  inline def shape(k: Int): String =
    trait Shape { def area: Int; def show: String = "area " + twice(area) }
    class Square(s: Int) extends Shape { def area = s * s }
    val t = new Shape { def area = k }
    new Square(k).show + " " + t.show
  inline def lazyObj: String =
    var log = ""
    object O { log += "init "; val v = 1 }
    log += "before "
    val a = O.v
    val b = O.v
    log + (a + b)
  inline def companions(k: Int): Int =
    class P(val v: Int)
    object P { def make(x: Int): P = new P(x * k) }
    P.make(2).v
  inline def colours(i: Int): String =
    enum Colour { case Red, Green, Blue }
    val c = Colour.fromOrdinal(i)
    val name = c match
      case Colour.Red => "r"
      case Colour.Green => "g"
      case Colour.Blue => "b"
    s"$name ${c.ordinal} $c ${Colour.values.length} ${Colour.valueOf("Green") == Colour.Green}"
  inline def tested(x: Any): Boolean =
    class Marker
    x.isInstanceOf[Marker]

@main def run(): Unit =
  println(Lib.counter(1))
  println(Lib.counter(10))
  println(Lib.shape(3))
  println(Lib.shape(4))
  println(Lib.lazyObj)
  println(Lib.companions(3) + Lib.companions(4))
  println(Lib.colours(0) + " | " + Lib.colours(2))
  println(Lib.tested(1))
  def local(x: Int): String =
    inline def f(y: Int): String =
      object K { val v = x + y }
      case class C(a: Int, b: String)
      case object Z
      enum E { case A(n: Int); case B }
      val c = C(K.v, "x")
      val d = c.copy(b = "y")
      val m = d match
        case C(a, "y") => a
        case _ => -1
      s"$c $d $m ${c == C(K.v, "x")} $Z ${E.A(y).n} ${E.B.ordinal}"
    f(1) + " | " + f(2)
  println(local(5))
