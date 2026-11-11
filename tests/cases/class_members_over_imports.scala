object FirstNames {
  def value: Int = 1
  type Item = String
}
object SecondNames {
  def value: Int = 2
  type Item = Boolean
}
class ClassNames { self =>
  import FirstNames.*
  import SecondNames.*
  println(value)
  println(self.value)
  val item: Item = 5
  def value: Int = 3
  type Item = Int
  def fromMethod: Int = { import FirstNames.*; def value: Int = 4; value }
}
trait InheritedName { def value: Int = 6 }
class InheritedNames extends InheritedName {
  import FirstNames.*
  import SecondNames.*
  println(value)
}
@main def run(): Unit = {
  val names = new ClassNames
  println(names.item)
  println(names.fromMethod)
  new InheritedNames
  ()
}
