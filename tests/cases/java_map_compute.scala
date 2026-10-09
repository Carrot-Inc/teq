// `java.util.Map.computeIfAbsent` on a `HashMap`: the mapping runs for a missing key only, and a
// null result is not stored.
import java.util.HashMap

object Main:
  def main(args: Array[String]): Unit =
    val m = new HashMap[String, Integer]()
    var calls = 0
    def len(k: String): Integer =
      calls += 1
      k.length
    println(m.computeIfAbsent("abc", k => len(k)))
    println(m.computeIfAbsent("abc", k => len(k)))
    println(calls)
    println(m.computeIfAbsent("none", _ => null))
    println(s"${m.containsKey("none")} ${m.size()}")
    val jm: java.util.Map[String, Integer] = m
    println(jm.computeIfAbsent("de", k => len(k)) + " " + calls)
