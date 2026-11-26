// A top-level given object and a top-level val have one owner, the file's module.
trait T
val marker = 1
given G: T with {}
object Main { def main(args: Array[String]): Unit = println(Inspect.same[G.type, marker.type]) }
