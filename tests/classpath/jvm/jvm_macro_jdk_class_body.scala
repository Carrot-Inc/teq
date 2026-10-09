// jars: scala-library sourcecode-0.4.4
// std: lean scala-library
// A macro that makes a JDK class at compile time on the JVM target: sourcecode 0.4.4's `FullName`
// caches in a `java.util.concurrent.ConcurrentHashMap`, which the interpreter runs from the std's
// body of the class (`@jvmClass`) as it does on JavaScript.
object Names:
  def whoAmI(using n: sourcecode.FullName): String = n.value

@main def run(): Unit =
  println(Names.whoAmI)
  val cache = new java.util.concurrent.ConcurrentHashMap[String, Integer]()
  cache.computeIfAbsent("a", _ => 1)
  println(cache.get("a"))
