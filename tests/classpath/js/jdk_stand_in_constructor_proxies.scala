// jars: javafix
// targets: js jvm interp
// A JDK class the std stands for without a companion has the JDK class's module once a term names it, with an `apply`
// proxy of each of the class's constructors (`NamerOps.addConstructorProxies`, which `ClassfileParser.parseClass` runs
// once both roots are in): `ArrayList[Int]()` is `ArrayList.apply[Int]()`, a creation, for a top-level stand-in and a
// nested one (`AbstractMap.SimpleEntry`).
import java.util.ArrayList

@main def run(): Unit =
  val l = ArrayList[Int]()
  l.add(1)
  println(l)
  println(java.util.concurrent.atomic.AtomicInteger(5).incrementAndGet())
  val e = java.util.AbstractMap.SimpleEntry("b", 2)
  println(e.getKey)
  println(e.getValue)
