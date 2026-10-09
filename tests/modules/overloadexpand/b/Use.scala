package oeb

import oea.Fmt

object Use:
  def a: String = Fmt.show[Int]("alpha", 1, 10)
  def b: String = Fmt.show[String]("beta", 2, "z")
  def c: String = Fmt.show[Int]("x", 1, 2) + Fmt.show[Int]("y", 2, 3)
  def l: String = Fmt.show("long", 7L) + Fmt.show("again", 8L)
  def m(n: Long): String = Fmt.show("n", n) + Fmt.show[Long]("t", 1, n)
