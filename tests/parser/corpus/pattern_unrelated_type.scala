// A typed pattern whose type is unrelated to the scrutinee's binds their intersection, so the
// members of both are in reach, as in scalac.
trait Named:
  def name: String
trait Sized:
  def size: Int
class Both extends Named with Sized:
  def name = "both"
  def size = 2
class OnlyNamed extends Named:
  def name = "named"

object Test:
  def describe(n: Named): String = n match
    case s: Sized => s"${s.name} has size ${s.size}"
    case other => s"${other.name} has no size"
  def sized(n: Named): Int = n match
    case s: Sized if s.size > 1 => s.size + s.name.length
    case _ => 0
  def main(args: Array[String]): Unit =
    println(describe(new Both))
    println(describe(new OnlyNamed))
    println(sized(new Both))
    println(sized(new OnlyNamed))
