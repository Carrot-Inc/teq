// Extension methods are found in the implicit scope of the receiver's type: the companions of
// its classes and the objects those classes are nested in, also behind an opaque type or alias.
object Outer:
  case class Inner(v: Int)
  extension (i: Inner) def doubled: Int = i.v * 2
  object Deep:
    class Deeper
  extension (d: Deep.Deeper) def name: String = "deeper"
  extension (xs: List[Inner]) def total: Int = xs.map(_.v).sum

object Ids:
  opaque type Email = String
  object Email:
    def apply(s: String): Email = s
    extension (e: Email) def domain: String = e.split("@")(1)
  extension (e: Email) def local: String = e.split("@")(0)

object A:
  object opaques:
    opaque type FlagSet = Long
    def FlagSet(bits: Long): FlagSet = bits
    def toBits(fs: FlagSet): Long = fs
  val someFlag = FlagSet(1)
  type FlagSet = opaques.FlagSet
  def FlagSet(bits: Long): FlagSet = opaques.FlagSet(bits)
  extension (xs: FlagSet)
    def bits: Long = opaques.toBits(xs)
    def |(ys: FlagSet): FlagSet = FlagSet(xs.bits | ys.bits)

object B:
  type Variance = A.FlagSet
  val f: A.FlagSet = A.someFlag
  val v: Variance = A.someFlag
  def run(): Unit =
    println(f.bits)
    println(v.bits)
    println((f | A.FlagSet(2)).bits)

@main def run(): Unit =
  println(Outer.Inner(2).doubled)
  println(Outer.Deep.Deeper().name)
  println(List(Outer.Inner(1), Outer.Inner(2)).total)
  val e = Ids.Email("a@b.c")
  println(e.domain)
  println(e.local)
  B.run()
