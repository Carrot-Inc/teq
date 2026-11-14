// An inline method that implements an abstract inline method has the result the abstract one declares, read before
// its own body: it may call itself, or the method beside it that calls it back, with no result written (scalac
// takes the overridden member's type, `Namer.inferredResultType`).
trait Counter:
  inline def steps(inline n: Int): Int
  inline def hops(inline n: Int): Int

object Stairs extends Counter:
  def up(x: Int): Int = x + 1
  inline def steps(inline n: Int) = inline if n <= 0 then 0 else up(hops(n - 1))
  inline def hops(inline n: Int) = inline if n <= 0 then 0 else up(steps(n - 1))

trait Marker[T]:
  inline def marked(inline n: Int, x: T): T

object Bangs extends Marker[String]:
  inline def marked(inline n: Int, x: String) = inline if n <= 0 then x else marked(n - 1, x + "!")

trait Depth:
  inline def depth(inline n: Int): Int

object Plain extends Depth:
  inline def depth(inline n: Int) = inline if n <= 0 then 0 else depth(n - 1) + 1

@main def run(): Unit =
  println(Stairs.steps(3).toString + " " + Stairs.hops(2))
  println(Bangs.marked(2, "a"))
  println(Plain.depth(3))
  val wide = new Wide
  val narrow: Narrow = wide
  println(Reader.doubled(wide, 1).toString + " " + narrow.size(1))

// The overridden member's result is itself inferred: the override's is read off it.
object Reader:
  inline def doubled(w: Wide, n: Int) = w.size(n) * 2

class Narrow:
  def size(n: Int) = n + 1

class Wide extends Narrow:
  override inline def size(n: Int) = n + 10
