// java.util.Objects' members that check or default a value, as the JDK's bodies run them: the
// default taken only for a null, its own null refused with the parameter's name, the supplier
// asked only when needed, an index checked against a length.
import java.util.Objects

@main def run(): Unit =
  println(Objects.requireNonNullElse[String](null, "fallback"))
  println(Objects.requireNonNullElse("given", "fallback"))
  try Objects.requireNonNullElse[String](null, null) catch case e: NullPointerException => println(s"NPE ${e.getMessage}")
  var asked = 0
  val supplier: java.util.function.Supplier[String] = () => { asked += 1; "made" }
  println(Objects.requireNonNullElseGet[String](null, supplier))
  println(Objects.requireNonNullElseGet("given", supplier))
  println(asked)
  try Objects.requireNonNullElseGet[String](null, () => null) catch case e: NullPointerException => println(s"NPE ${e.getMessage}")
  println(Objects.toString(null, "none"))
  println(Objects.toString(List(1), "none"))
  val byLength: java.util.Comparator[String] = (a, b) => a.length - b.length
  println(Objects.compare("aa", "b", byLength))
  println(Objects.checkIndex(2, 3))
  try Objects.checkIndex(3, 3) catch case e: IndexOutOfBoundsException => println(e.getMessage)
