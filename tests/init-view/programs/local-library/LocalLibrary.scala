abstract class LocalLibrary {
  val first = {
    def read(): Int = later
    read()
  }
  def later: Int
}
class Child extends LocalLibrary {
  val later = 1
}
