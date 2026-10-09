package dnb

import dna.*

object UseNested:
  val f: (c: Ctx) => (x: c.T) => c.T = c => x => x
  def run(c: Ctx)(x: c.T): c.T = Nested.keep(f)(c)(x)
  def main(args: Array[String]): Unit =
    val c = new Ctx { type T = String; val t = "nested" }
    val s: String = run(c)(c.t)
    println(s)
