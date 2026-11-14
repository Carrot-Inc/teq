package oldsrc
import scala.language.`2.13`

class Counter { def next(): Int = 7 }
class Base { def label(): String = "base" }
class Meter(val n: Int) { def plus(o: Meter): Meter = new Meter(n + o.n) }
