package ncc

// The review's client: the companion of a case class nested in a class, an object of each
// enclosing instance, is a module class, which the JVM's class makes `java.io.Serializable`
// (dotty's `BCodeSkelBuilder`), over teq's products as over scalac's.
@main def client(): Unit =
  val o = new nca.O(1)
  val c: Any = o.K
  println(c.isInstanceOf[java.io.Serializable])
  println(c.toString)
