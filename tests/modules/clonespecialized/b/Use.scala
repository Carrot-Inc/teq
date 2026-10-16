package clonespecializeduse

import clonespecialized.*

class E extends H

object Main:
  def main(args: Array[String]): Unit =
    val x: G[String] = new E
    println(x.clone("x"))
