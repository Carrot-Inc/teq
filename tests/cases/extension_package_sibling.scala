extension (base: Int) {
  def identity: Int => Int = value => base + value
  def viaSibling(value: Int): Int = identity(value)
}
extension (base: String) {
  def identity: String => String = value => base + value
}
@main def runPackageExtensionSibling(): Unit = println(10.viaSibling(2))
