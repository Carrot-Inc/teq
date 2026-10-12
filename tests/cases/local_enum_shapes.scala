// More shapes of a local enum, its companion a local object of each run of the block: an explicit
// companion before the enum and one whose members read the values, a match outside the enum, a
// generic enum, an enum extending a trait, the values' strings and ordinals, an enum of a lambda's
// block and of a local class's method.
trait Named:
  def label: String

def before(n: Int): String =
  object E:
    def first = A
  enum E:
    case A, B
    def tag = s"$this$n"
  E.first.tag + E.B.tag

def members(): String =
  enum E:
    case A, B
  object E:
    def all = values.toList.map(_.toString).mkString("+")
    def second = fromOrdinal(1)
  E.all + " " + E.second

def outside(i: Int): String =
  enum E:
    case A, B, C
  E.fromOrdinal(i) match
    case E.A => "a"
    case E.B => "b"
    case E.C => "c"

def generic(n: Int): String =
  enum Opt[+T]:
    case Some(x: T)
    case None
    def orElse[U >: T](u: U): U = this match
      case Some(x) => x
      case None => u
  s"${Opt.Some(n).orElse(0)} ${Opt.None.orElse(n + 1)}"

def labelled(prefix: String): String =
  enum E extends Named:
    case A, B
    def label = prefix + toString
  E.values.map(_.label).mkString(",")

def inLambda(): String =
  val f = (k: Int) =>
    enum E:
      case A
      def get = k
    E.A.get
  s"${f(1)} ${f(2)}"

def inClass(n: Int): String =
  class C:
    def make(m: Int): String =
      enum E:
        case A
        def get = n + m
      s"${E.A.get} ${E.A.ordinal} ${E.A}"
  (new C).make(1)

@main def run(): Unit =
  println(before(1))
  println(members())
  println(outside(1))
  println(generic(4))
  println(labelled("x"))
  println(inLambda())
  println(inClass(10))
