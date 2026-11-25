// teq: --wconf msg=Ignoring:s
// The members dotty's variance check leaves local, which scalac runs (typer/VarianceChecker.scala at 3.8.4): a
// `private[this]` class parameter (a LocalParamAccessor, not checked), and a private class-parameter var no other
// instance assigns (isLocal: checked at its getter's variance), a class's, a case class's and an enum case's, assigned
// through `this` in each form.
class ThisParam[-T](private[this] val row: List[T]):
  def size: Int = row.size
class ThisVar[+T](private[this] var row: List[T]):
  def size: Int = row.size
class PrivateVar[+T](private var row: List[T]):
  def add(): Int = { row = row ++ Nil; row ++= Nil; this.row = row; val g = () => { row = Nil }; g(); row.size }
case class CaseVar[+T](private var row: List[T]):
  def size: Int = row.size
enum EnumVar[+T]:
  case C(private var row: List[T])
@main def run(): Unit =
  val a: ThisParam[Int] = new ThisParam[Any](List(1, 2))
  println(a.size)
  val b: ThisVar[Any] = new ThisVar[Int](List(1))
  println(b.size)
  val c: PrivateVar[Any] = new PrivateVar[Int](List(1, 2, 3))
  println(c.add())
  val d: CaseVar[Any] = CaseVar[Int](List(4))
  println(d.size)
  println(EnumVar.C(List(5)).ordinal)
