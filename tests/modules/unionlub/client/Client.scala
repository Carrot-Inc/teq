package ulc

import ula.*

object Client:
  def main(args: Array[String]): Unit =
    println(L.call(null))
    println(L.pure(new PB))
    println(L.matched[String](Array(new B)))
    println(L.nullable(new S("s")))
    val a = new java.util.ArrayList[Int]()
    a.add(1)
    println(L.size(a))
    println(L.find(Ids.of(java.util.UUID.randomUUID())))
    println(L.late(Array[java.util.concurrent.TimeoutException]()))
    println(L.serial(new SC))
    println(L.pureNull(new PT))
    println(L.pureLate(new PZ))
    println(L.pureUnionNull(new TU1))
