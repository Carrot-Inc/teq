package rtb

import rta.{Api, Box, Holder, P2, P3}

object box extends Box:
  type T = String
  def get: String = "member"
class QB extends P2:
  val a: box.type = box
  type T = box.T
object holder extends Holder:
  val b: box.type = box
class DB extends P3:
  val a: holder.type = holder
  type T = box.T

object Use:
  def main(args: Array[String]): Unit =
    val r = Api.make
    val s: String = Api.read(r)
    val t: r.a.type = r.b
    val i = Api.inferred
    val u: i.a.type = i.b
    println(s + " " + t + " " + u)
    val q: Api.Q = QB()
    def same(q: Api.Q)(x: q.T): q.a.T = x
    val m: q.a.T = same(q)(Api.same(q)(q.a.get))
    println(m)
    val d: Api.D = DB()
    def deep(d: Api.D)(x: d.T): d.a.b.T = x
    val k: d.a.b.T = deep(d)(Api.deep(d)(d.a.b.get))
    println(k)
