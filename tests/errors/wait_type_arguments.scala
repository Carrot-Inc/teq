// Object's wait, notify and notifyAll take no type parameters: a type application on them is scalac's error, E134
// among wait's overloads, and for notify and notifyAll that the method takes none.
object Main:
  def main(args: Array[String]): Unit =
    val o = new Object
    o.synchronized { o.wait[Int](1L) }
    o.synchronized { o.notify[Int]() }
    o.synchronized { o.notifyAll[String]() }
// expect: 6:22: error: None of the overloaded alternatives of method wait in class Object with types
// expect:  (x$0: Long, x$1: Int): Unit
// expect:  (x$0: Long): Unit
// expect:  (): Unit
// expect: match type arguments [Int] and arguments (1L)
// expect: 7:22: error: method notify in class Object does not take type parameters
// expect: 8:22: error: method notifyAll in class Object does not take type parameters
// expect: 3 errors found
