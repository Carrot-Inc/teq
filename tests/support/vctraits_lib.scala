package vct

// Universal traits whose concrete `hashCode`, `toString`, `equals` and `canEqual` a value class of
// another build inherits (tests/classpath/jvm/vc_box_jar_traits): no synthesized member of its box.
trait H extends Any {
  override def hashCode: Int = 42
  override def toString: String = "H!"
  override def equals(o: Any): Boolean = { println("equals"); true }
}
trait CE extends Any {
  def canEqual(a: Any): Boolean = { println("canEqual"); false }
}
