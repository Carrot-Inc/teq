trait Base:
  opaque type T = Int
  def make(i: Int): T = i
  def show(t: T): String = s"base($t)"

object O extends Base

def narrowed(x: O.type & Base { type T = Int }): x.T = 2
def widened(x: Base { type T = Int }): x.T = 3
def bounded(x: O.type & Base { type T <: Int })(v: x.T): Int = v
def throughBound[X <: O.type & Base { type T = Int }](x: X): x.T = 4
def throughBoundBounds[X <: O.type & Base { type T <: Int }](x: X)(v: x.T): Int = v
type Refined = O.type & Base { type T = Int }
def throughAlias(x: Refined): x.T = 5
def throughAliasBound[X <: Refined](x: X): x.T = 6

@main def run(): Unit =
  println(O.show(O.make(1)))
  println("typed")
