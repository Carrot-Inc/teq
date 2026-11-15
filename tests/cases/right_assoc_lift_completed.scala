// `Typer.typedInfixOp` lifts the arguments of the COMPLETED application `r.op(l)` (`LiftImpure.liftArgs`): an
// auto-tupled operand is bound as the tuple, an operand a conversion adapts is bound converted, an operand is bound
// ahead of a receiver a conversion wraps, and the argument of an inline parameter is not bound at all (it is
// evaluated where the body reads it, twice here). Master evaluated the receiver first in the first three and bound
// the fourth once.
import scala.language.implicitConversions
class PairBox:
  def +:(p: (Int, Int)): Int = p._1 + p._2
class C(val n: Int)
class Target:
  def +:(x: C): Int = x.n
class Receiver
class Converted:
  def +:(x: Int): Int = x
class InlineBox:
  inline def +:(inline x: Int): Int = x + x
given Conversion[Int, C] with
  def apply(x: Int): C = { print("c"); new C(x) }
given Conversion[Receiver, Converted] with
  def apply(x: Receiver): Converted = { print("r"); new Converted }
def left: Int = { print("l"); 1 }
def second: Int = { print("b"); 2 }
def pair: PairBox = { print("r"); new PairBox }
def target: Target = { print("r"); new Target }
def in: InlineBox = { print("r"); new InlineBox }
@main def run(): Unit =
  print("tuple:"); println((left, second) +: pair)
  print("converted-argument:"); println(1 +: target)
  val receiver = new Receiver
  print("converted-receiver:"); println(left +: receiver)
  print("inline:"); println(left +: in)
