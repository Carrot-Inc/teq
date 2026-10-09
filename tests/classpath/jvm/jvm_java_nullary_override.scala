// jars: scala-library
// std: lean scala-library
// A parameterless Scala member overriding a Java method declared with `()`, as scalac allows.
final class Countdown(from: Int) extends java.util.Iterator[Int]:
  private var left = from
  override def hasNext: Boolean = left > 0
  def next(): Int =
    left -= 1
    left + 1

final class Named(label: String) extends java.util.function.Supplier[String]:
  val get: String = label

final class Answer extends java.util.function.IntSupplier:
  def getAsInt: Int = 42

@main def run(): Unit =
  val it = Countdown(3)
  while it.hasNext do print(it.next())
  println()
  println(Named("n").get)
  val supplier: java.util.function.IntSupplier = Answer()
  println(supplier.getAsInt())
