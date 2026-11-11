// jars: scala-library package-overloads-lib
// Inherited overloads have trait owners. Filtering for package-object owners
// leaves NoDenotation, whose aggregate still supplies the NoSource file sentinel.
import packageoverloads.*
object Checked:
  val number: Int = choose(3)
  val text: Int = choose("four")
