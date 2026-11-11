// teq accepts an inline parameter as a type prefix, `(b.get: b.T)` with `inline b: Box`, and prints 7. scalac 3.8.4
// rejects it with E083 (the .expected): "(b : Box) is not a valid type prefix, since it is not an immutable path" (an
// inline parameter is no stable path). In the typer, not the writer. Found by reviewer #14 of the workers exports
// helper (push 9); the same before the SELECTtpt port.
trait Box:
  type T
  def get: T

class IntBox2 extends Box:
  type T = Int
  def get: Int = 7

object IP:
  inline def q(inline b: Box): Any = (b.get: b.T)

@main def run(): Unit =
  println(IP.q(new IntBox2))
