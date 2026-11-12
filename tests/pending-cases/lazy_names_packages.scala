//> using platform jvm
// Pending on lazy-val-names (queued): teq numbers each package clause of one source on its own counter, where
// dotty's CompilationUnit.freshNames is the source's, shared by its clauses; teq's names are unique and the same on
// every run, scalac's numbers are the expectation, here 3 for `b.B`'s `x`.
// Lazy vals in two package clauses of one file, a package-level one and a class's in the first, a class's in the
// second: the declared fields of each class name its holder by the unit's counter.
package a {
  lazy val x = 1
  class A { lazy val x = 2 }
}
package b {
  class B { lazy val x = 3 }
}
object Main {
  def main(args: Array[String]): Unit = {
    println(a.x)
    println(new a.A().x)
    println(new b.B().x)
    println(classOf[a.A].getDeclaredFields.map(_.getName).sorted.mkString(","))
    println(classOf[b.B].getDeclaredFields.map(_.getName).sorted.mkString(","))
  }
}
