// jars: scala-library
// Runs on the JVM against the JDK's classes, called through their class file descriptors.
import java.util.{ArrayList, HashMap}
object Main:
  def main(args: Array[String]): Unit =
    val xs = new ArrayList[String]()
    xs.add("b")
    xs.add("a")
    println(xs.size())
    println(xs.get(0))
    println(xs)
    val m = new HashMap[String, Int]()
    m.put("k", 1)
    println(m.get("k"))
    println(m.containsKey("k"))
    val date = java.time.LocalDate.of(2024, 1, 2).plusDays(3)
    println(date)
    println(date.getDayOfMonth)
    val sb = new java.lang.StringBuilder()
    println(sb.append("a").append(1).append(2.5).toString)
    println(Integer.parseInt("42") + 1)
    println(Math.max(1, 2.5))
    println(java.util.Arrays.asList("a", "b").size())
    println("abc".repeat(2))
    println(String.valueOf(7))
    println(java.util.Optional.of(1).map(x => x + 1).get())
    println(java.util.Objects.hash(1, "a") == java.util.Objects.hash(1, "a"))
