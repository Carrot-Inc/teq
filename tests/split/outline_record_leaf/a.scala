package a

object Trace:
  var lines: List[String] = Nil
  def note(s: String): Unit = lines = s :: lines

def side(): Int = { println("side"); 3 }

inline def f(x: Int): Int =
  val size = x * 2
  Trace.note("f of " + size)
  val head = if size > 5 then size * 10 else size + 1
  Trace.note("f head " + head)
  head + size * 0

transparent inline def g(): Int = f(side())
