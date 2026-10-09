// A default argument sees the parameters of the earlier clauses only, so a name shared with a
// member of the enclosing class binds the member.
final class Box(val size: Int, val label: String):
  def copyWith(size: Int = size, label: String = label): Box = Box(size, label)
  def scaled(factor: Int = size): Int = factor * 2
  def tagged(prefix: String = label + "!")(width: Int = prefix.length + size): String = prefix + width
  override def toString: String = s"Box($size, $label)"

object Registry:
  val limit: Int = 10
  def register(limit: Int = limit, name: String = "x" + limit): String = s"$name/$limit"

class Config(val depth: Int)(val width: Int = depth * 2, val label: String = "d" + depth):
  override def toString: String = s"Config($depth, $width, $label)"

extension (b: Box)
  def grown(size: Int = b.size + 1): Box = Box(size, b.label)

@main def run(): Unit =
  val b = Box(1, "a").copyWith(label = "b")
  println(b)
  println(b.copyWith(size = 5))
  println(b.scaled())
  println(b.scaled(7))
  println(b.tagged()())
  println(b.tagged("p")(3))
  println(Registry.register())
  println(Registry.register(3))
  println(Config(4)())
  println(Config(4)(1, "w"))
  println(b.grown())
  println(b.grown(9))
