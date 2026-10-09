package sua

// Programs the std's forms must leave as written: an explicit `<:<.refl`
// evidence is the argument the source chose, not one for the typer to find again; a program's
// own `indexOf(Int)` and `char2int` are no widened argument of the JDK's `String.indexOf`.
object Api:
  given local: (Int <:< Int) = null
  def consume(using ev: Int <:< Int): Boolean = ev == null
  def chosen(): Boolean = consume(using <:<.refl[Int])

object Encoding:
  def char2int(c: Char): Int = c.toInt + 1

class Search:
  def indexOf(x: Int): Int = x

object Lookup:
  def test(c: Char): Int = new Search().indexOf(Encoding.char2int(c))
