package cloneoverloaduse

import cloneoverload.*

class E extends D

object Main:
  def c(x: D): Object = x.clone()
  def main(args: Array[String]): Unit =
    println(c(new E))
    println(new E().clone(3))
