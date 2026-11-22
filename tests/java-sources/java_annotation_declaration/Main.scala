// An annotation declaration read from its source, applied to Scala definitions as one read from a class file: named
// arguments, its defaults left out, an array member, a nested annotation.
class Host {
  @Ann(s = "a") def f: Int = 1
  @Ann(x = 7, s = "b", arr = Array("p", "q"), in = new Inner(v = 3)) def g: Int = 2
}
object Main {
  def main(args: Array[String]): Unit = println(s"${classOf[Ann].isAnnotation} ${(new Host).f}${(new Host).g}")
}
