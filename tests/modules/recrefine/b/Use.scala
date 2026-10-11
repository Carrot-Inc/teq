package rrb

import rra.Api

object Use:
  def test(r: Api.R)(x: r.T): r.U = Api.same(r)(x)

  def main(args: Array[String]): Unit =
    val b = Api.Box("ok")
    val r: Api.R { type T = String } = b
    val s: String = test(r)(b.value)
    println(s)
    val i = Api.inferred("inferred")
    val widen: i.U => String = x => x
    val u: String = (b.value: i.T)
    println(u + widen.toString.take(0))
