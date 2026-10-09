package tgdb

import tgda.*

object G extends Gives

@main def run(): Unit =
  given Int = 41
  println(G.gp)
  println(G.use)
  given String = "s"
  println(G.gt[String])
  println(new Gives {}.gp(using 1))
