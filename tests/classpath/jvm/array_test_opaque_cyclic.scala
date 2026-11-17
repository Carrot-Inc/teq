// jars: scala-library
// An opaque type that stands for itself: scalac refuses the definition (E140, a cyclic reference) and teq does not
// yet. Until it does, an array test that names one is built as it was before the test read the element's class:
// told uncheckable, any array at run time; neither the check of the element nor its erasure is entered, which do
// not end on such a type (a typed pattern over one is built the same way). The expectation is teq's own lines:
// scalac has none for this program.
object Rec:
  opaque type Chain = Array[Chain]
  opaque type Self = Self | Null
  opaque type Fix[F[_]] = F[Fix[F]]
  def chain(x: Any): Boolean = x.isInstanceOf[Array[Chain]]
  def self(x: Any): Boolean = x.isInstanceOf[Array[Self]]
  def fix(x: Any): Boolean = x.isInstanceOf[Array[Fix[Array]]]
  def pattern(x: Any): Int = x match { case _: Array[Chain] => 1; case _ => 0 }
def outside(x: Any): Boolean = x.isInstanceOf[Array[Rec.Fix[Array]]]
@main def run(): Unit =
  val strs: Any = Array("a"); val nested: Any = Array(Array(1)); val str: Any = "s"
  println("chain " + Rec.chain(strs) + " " + Rec.chain(nested) + " " + Rec.chain(str))
  println("self " + Rec.self(strs) + " " + Rec.self(str))
  println("fix " + Rec.fix(strs) + " " + Rec.fix(nested) + " " + Rec.fix(str))
  println("pattern " + Rec.pattern(strs) + " " + Rec.pattern(str))
  println("outside " + outside(strs) + " " + outside(str))
