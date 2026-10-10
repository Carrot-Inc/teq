package aab

import aaa.{Api, Shape}

object Use:
  def main(args: Array[String]): Unit =
    println(Api.calls())
    println(Api.use(new AnyRef { def f(x: Int): Int = x + 1 }))
    println(Api.shape(new Shape { type Measure = Int; def size = 3 }))
