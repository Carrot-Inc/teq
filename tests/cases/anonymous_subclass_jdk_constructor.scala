// An anonymous subclass of an empty body reaches a protected constructor as its super call, a JDK class's
// too: named whole, through an alias, through a renaming import, as a by-name and a default argument and in
// a lambda. On the JVM such a creation is a class of its own, since the JVM lets a subclass alone call a JDK
// class's protected constructor.
import java.util.logging.{Level => Lv}
type L = java.util.logging.Level
object Names:
  type Lvl = java.util.logging.Level
  def make(n: Int): Lvl = new Lvl("N", n) {}
def twice(l: => java.util.logging.Level): Int = l.intValue + l.intValue
def default(l: java.util.logging.Level = new java.util.logging.Level("D", 6) {}): Int = l.intValue
@main def main(): Unit =
  println((new java.util.logging.Level("X", 5) {}).intValue)
  println((new L("A", 4) {}).intValue)
  println((new Lv("R", 3) {}).intValue)
  println(twice(new java.util.logging.Level("B", 5) {}))
  println(default())
  println(default(new java.util.logging.Level("E", 7) {}))
  val make: Int => java.util.logging.Level = n => new java.util.logging.Level("M", n) {}
  println(make(8).intValue)
  val named: java.util.logging.Level = new java.util.logging.Level("V", 9) {}
  println(named.getName)
  println(Names.make(10).intValue)
  println((new RuntimeException("x") {}).getMessage)
