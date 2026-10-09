// The receiver of an inline method is evaluated once, before the arguments, and `this.type` is its
// path; an extension's receiver is bound by its parameter's mode, once for a plain one and on each
// use for an `inline` one, as scalac 3.8.4 expands them and the expansion by substitution keeps.
class Counter:
  var n = 0
  def next(): Int = { n += 1; n }

class Box(val v: Int):
  inline def self: this.type = this
  inline def twiceV: Int = v + v

extension (x: Int)
  inline def doubled: Int = x + x
extension (inline x: Int)
  inline def doubledInline: Int = x + x

object Main:
  def box(c: Counter): Box = { c.next(); Box(3) }
  def main(args: Array[String]): Unit =
    val b = Box(1)
    val s: b.type = b.self
    println(s eq b)
    val c1 = Counter(); println((box(c1).twiceV, c1.n))
    val c2 = Counter(); println((c2.next().doubled, c2.n))
    val c3 = Counter(); println((c3.next().doubledInline, c3.n))
