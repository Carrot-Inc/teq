package uab

import uaa.*
import java.util.concurrent.TimeoutException

object Q:
  val e: Exception = new TimeoutException()

@main def run(): Unit =
  println(P.f[String](null))
  println(P.g[Array](null))
  println(Q.e.getClass.getSimpleName)
