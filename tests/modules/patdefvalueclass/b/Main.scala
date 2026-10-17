package pvc

import Bodies.Body

// A tuple pattern definition over a match of tuple literals (`SimpleTuple`), its components the
// upstream's value class and opaque type, each passed on as itself.
object Main {
  def request(payload: Option[Int]): String = {
    val (finalHeaders, entityBody) = payload match {
      case Some(p) => (Headers("a", "b"), Body(p, p))
      case None => (Headers.empty, Body())
    }
    Client.send(finalHeaders, entityBody)
  }

  def main(args: Array[String]): Unit = {
    println(request(Some(3)))
    println(request(None))
  }
}
