//> using platform jvm
// split-watch: jvm
// A local class constructed (`new Later`) before its declaration, after a local lazy val of the same name: dotty's
// MegaPhase meets the class's TypeDef at its declaration and a `New` as its type tree alone, so the local val is
// numbered first (`x$lzy1`), the class's lazy val second (`x$lzy2`, its handle and `x$lzyINIT2`), the member third.
// The names by reflection: the local class's fields and initialiser, the enclosing class's member's.
class A:
  def go(): Unit =
    def make() = new Later
    lazy val x = 1
    class Later { lazy val x = 2 }
    println(x)
    val obj = make()
    println(obj.x)
    println((obj.getClass.getDeclaredFields.map(_.getName) ++ obj.getClass.getDeclaredMethods.map(_.getName)).filter(_.contains("lzy")).sorted.mkString(","))
  lazy val x = 3

@main def run(): Unit =
  new A().go()
  println((classOf[A].getDeclaredFields.map(_.getName) ++ classOf[A].getDeclaredMethods.map(_.getName).filter(_.startsWith("x$lzyINIT3"))).filter(_.contains("lzy")).sorted.mkString(","))
