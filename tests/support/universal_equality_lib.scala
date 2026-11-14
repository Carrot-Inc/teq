package unieq

// Universal traits that override `equals` and `hashCode`, for a value class of another build.
trait Odd extends Any {
  override def equals(x: Any): Boolean = false
  override def hashCode: Int = 7
}
trait Even extends Any {
  override def equals(x: Any): Boolean = false
  override def hashCode: Int = 8
}
