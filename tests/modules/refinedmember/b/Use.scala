package rmb

import rma.{Api, Ctx, Runner}

object Use:
  def main(args: Array[String]): Unit =
    val r = Api.get
    val s: String = r.f[String]("first")
    val t: String = r.g(y = 2)
    val u: String = r.g(3, 100)
    println(s + " " + t + " " + u)
    println(Api.twice(r))
    val c = new Ctx { type T = String; val value = "dependent" }
    val k = new Runner { def run(c: Ctx): c.T = c.value }
    val v: c.T = Api.read(k, c)
    println(v)
