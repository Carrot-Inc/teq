package pnb

import pna.PolyNamed

object UsePolyNamed:
  def test(f: [A] => (first: A, second: A) => A): String = PolyNamed.keep(f)[String](first = "one", second = "two")
  def main(args: Array[String]): Unit = println(test([A] => (first: A, second: A) => second))
