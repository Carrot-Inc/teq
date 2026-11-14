// A type parameter whose bound has a class's member beside a refinement of the same name: the class's member holds
// a refinement asked of the parameter where the bound's own refinement is wider than the one asked (the two are
// one member, met): through another parameter, in a value's intersection, as a `val`, with parameters, with an
// unrelated result, as a class's argument and a class's parameter. scalac 3.8.4 prints the same line.
class Other { def self: Int = 7; val size: Int = 8; def add(n: Int): Int = n + 9; def text: String = "t" }
trait Named
class Both extends Other with Named
class Loop[X] { def self: Int = 10 }

def beside[T <: Other & AnyRef { def self: Any }](t: T): AnyRef { def self: Int } = t
def throughSecond[U <: AnyRef { def self: Any }, T <: U & Other](t: T): AnyRef { def self: Int } = t
def inValue[T <: Named { def self: Any }](x: T & Other): (Named & Other) { def self: Int } = x
def asVal[T <: Other & AnyRef { def size: Int }](t: T): AnyRef { val size: Int } = t
def withParameters[T <: Other & AnyRef { def add(n: Int): Any }](t: T): AnyRef { def add(n: Int): Int } = t
def unrelated[T <: Other & AnyRef { def text: Int }](t: T): AnyRef { def text: String } = t
class Asked[F[X] <: { def self: Int }] { def tag: Int = 11 }
class Passes[G[X] <: Loop[X] & AnyRef { def self: Any }] extends Asked[G]
class Box[T <: Other & AnyRef { def self: Any }](t: T) { def got: AnyRef { def self: Int } = t }

@main def main(): Unit =
  val o = new Other
  val b = new Both
  val kept = List[AnyRef](beside(o), throughSecond[Other, Other](o), asVal(o), withParameters(o), new Box(o).got)
  println(s"${kept.forall(_ eq o)} ${inValue[Both](b) eq b} ${new Passes[Loop].tag}")
