package clonewideuse

import clonewide.T

class E extends T

object Main:
  def main(args: Array[String]): Unit = println(new E().clone(1L, 7))
