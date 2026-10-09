// A method named where a Java functional interface is expected eta-expands into an instance
// of it, as a lambda does: the JDK's `ConcurrentHashMap.computeIfAbsent` takes a
// `java.util.function.Function` and `compute` a `BiFunction`.
import java.util.concurrent.ConcurrentHashMap
import java.util.function.{BiFunction, Function}

object Main:
  val cache = new ConcurrentHashMap[String, Int]
  def parse(s: String): Int = s.length * 10
  def join(k: String, v: String): String = if v == null then k + "!" else v + k
  def main(args: Array[String]): Unit =
    println(cache.computeIfAbsent("abc", parse))
    println(cache.computeIfAbsent("abc", _ => 0))
    println(cache.computeIfAbsent("de", (k: String) => k.length))
    println(cache.compute("de", (k, v) => v + k.length))
    val names = new ConcurrentHashMap[String, String]
    println(names.compute("x", join))
    println(names.compute("x", join))
    println(names.computeIfAbsent("y", k => k.toUpperCase))
    val f: Function[String, Int] = s => s.length
    val g: Function[String, Int] = parse
    val h: BiFunction[String, String, String] = join
    println(f.apply("four") + g.apply("four") + " " + h.apply("a", "b"))
