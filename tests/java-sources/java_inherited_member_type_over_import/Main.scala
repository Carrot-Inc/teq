// A simple type name in a class's body is the member type its parent gives, ahead of a single-type import of the name
// (JLS 6.4.1), as javac and scalac bind it.
object Main {
  def main(args: Array[String]): Unit = println(r.C.make().one())
}
